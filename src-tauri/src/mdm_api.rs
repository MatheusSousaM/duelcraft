use crate::models::{
    tier_from_power, DeckCard, MetaArchetype, MetaDeck, MetaSnapshot, PackCardRef,
    PackRecommendation, SearchCardResult,
};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

const MDM_BASE: &str = "https://www.masterduelmeta.com/api/v1";

pub struct MdmClient {
    http: Client,
}

impl MdmClient {
    pub fn new() -> Result<Self, String> {
        let http = Client::builder()
            .user_agent("DuelCraft/0.3 (Master Duel Meta & KNN Deck Optimizer)")
            .timeout(Duration::from_secs(25))
            .build()
            .map_err(|e| format!("Falha ao inicializar HTTP client: {}", e))?;
        Ok(Self { http })
    }

    pub async fn sync_meta_snapshot(&self) -> Result<MetaSnapshot, String> {
        let deck_types_url = format!(
            "{}/deck-types?sort=-tournamentPower&limit=300&fields=name,card,tier,tournamentPower,popRank,deckBreakdown.avgUrPrice,deckBreakdown.avgSrPrice,deckBreakdown.total",
            MDM_BASE
        );
        let recent_decks_url = format!("{}/top-decks?sort=-created&limit=220", MDM_BASE);
        let tourney_decks_url = format!(
            "{}/top-decks?tournamentType[$exists]=true&sort=-created&limit=180",
            MDM_BASE
        );

        let (dt_res, recent_res, tourney_res) = tokio::join!(
            self.http.get(&deck_types_url).send(),
            self.http.get(&recent_decks_url).send(),
            self.http.get(&tourney_decks_url).send()
        );

        let raw_types: Vec<RawDeckType> = dt_res
            .map_err(|e| format!("Erro de rede ao buscar Tier List: {}", e))?
            .error_for_status()
            .map_err(|e| format!("API MDM retornou erro (deck-types): {}", e))?
            .json()
            .await
            .map_err(|e| format!("Erro ao decodificar Tier List: {}", e))?;

        let mut raw_decks: Vec<RawTopDeck> = recent_res
            .map_err(|e| format!("Erro de rede ao buscar Top Decks: {}", e))?
            .error_for_status()
            .map_err(|e| format!("API MDM retornou erro (top-decks): {}", e))?
            .json()
            .await
            .map_err(|e| format!("Erro ao decodificar Top Decks: {}", e))?;

        if let Ok(t_resp) = tourney_res {
            if let Ok(t_ok) = t_resp.error_for_status() {
                if let Ok(t_list) = t_ok.json::<Vec<RawTopDeck>>().await {
                    raw_decks.extend(t_list);
                }
            }
        }

        let mut cover_by_type_id: HashMap<String, String> = HashMap::new();
        let mut power_by_type_id: HashMap<String, (f64, Option<u8>)> = HashMap::new();

        let archetypes: Vec<MetaArchetype> = raw_types
            .into_iter()
            .map(|dt| {
                let power = dt.tournament_power.unwrap_or(0.0);
                let (tier, tier_label) = tier_from_power(dt.tier, power);
                if let Some(ref c) = dt.card {
                    cover_by_type_id.insert(dt.id.clone(), c.clone());
                }
                power_by_type_id.insert(dt.id.clone(), (power, dt.tier));

                let bd = dt.deck_breakdown.unwrap_or_default();
                MetaArchetype {
                    id: dt.id,
                    name: dt.name,
                    cover_card_id: dt.card,
                    tier,
                    tier_label,
                    tournament_power: power,
                    pop_rank: dt.pop_rank.unwrap_or(0.0),
                    avg_ur_price: bd.avg_ur_price.unwrap_or(0.0).round() as u32,
                    avg_sr_price: bd.avg_sr_price.unwrap_or(0.0).round() as u32,
                    deck_count: bd.total.unwrap_or(0),
                }
            })
            .collect();

        let mut seen_deck_ids: HashSet<String> = HashSet::new();
        let mut decks: Vec<MetaDeck> = Vec::with_capacity(raw_decks.len());

        for td in raw_decks {
            if !seen_deck_ids.insert(td.id.clone()) {
                continue;
            }
            if let Some(mapped) = map_raw_top_deck(td, &cover_by_type_id, &power_by_type_id) {
                decks.push(mapped);
            }
        }

        Ok(MetaSnapshot {
            last_synced: Utc::now().to_rfc3339(),
            archetypes,
            decks,
        })
    }

    /// Deep-fetches Ranked (Master I / Master V / Rating) AND Championship/Tournament decks
    /// for a specific archetype (e.g. "Resonators") so Rogue & specific archetypes have rich data!
    pub async fn fetch_archetype_decks(
        &self,
        deck_type_name: &str,
        existing_archetypes: &[MetaArchetype],
    ) -> Result<Vec<MetaDeck>, String> {
        let trimmed = deck_type_name.trim();
        if trimmed.is_empty() {
            return Ok(vec![]);
        }

        // 1. Resolve archetype _id from existing list or directly from /deck-types?name=...
        let lower = trimmed.to_lowercase();
        let mut resolved_arch: Option<(String, Option<String>, f64, Option<u8>)> =
            existing_archetypes
                .iter()
                .find(|a| a.name.to_lowercase() == lower)
                .or_else(|| {
                    existing_archetypes
                        .iter()
                        .find(|a| a.name.to_lowercase().contains(&lower))
                })
                .map(|a| (a.id.clone(), a.cover_card_id.clone(), a.tournament_power, Some(a.tier)));

        if resolved_arch.is_none() {
            let lookup_url = format!(
                "{}/deck-types?name={}&limit=5&fields=_id,name,card,tier,tournamentPower",
                MDM_BASE,
                urlencoding::encode(trimmed)
            );
            if let Ok(resp) = self.http.get(&lookup_url).send().await {
                if let Ok(list) = resp.json::<Vec<RawDeckType>>().await {
                    if let Some(dt) = list.into_iter().next() {
                        resolved_arch = Some((
                            dt.id,
                            dt.card,
                            dt.tournament_power.unwrap_or(0.0),
                            dt.tier,
                        ));
                    }
                }
            }
        }

        let Some((dt_id, cover_opt, power, tier_opt)) = resolved_arch else {
            return Ok(vec![]);
        };

        let recent_url = format!(
            "{}/top-decks?deckType={}&sort=-created&limit=45",
            MDM_BASE,
            urlencoding::encode(&dt_id)
        );
        let tourney_url = format!(
            "{}/top-decks?deckType={}&tournamentType[$exists]=true&sort=-created&limit=35",
            MDM_BASE,
            urlencoding::encode(&dt_id)
        );

        let (r_res, t_res) = tokio::join!(
            self.http.get(&recent_url).send(),
            self.http.get(&tourney_url).send()
        );

        let mut combined: Vec<RawTopDeck> = Vec::new();
        if let Ok(resp) = r_res {
            if let Ok(list) = resp.json::<Vec<RawTopDeck>>().await {
                combined.extend(list);
            }
        }
        if let Ok(resp) = t_res {
            if let Ok(list) = resp.json::<Vec<RawTopDeck>>().await {
                combined.extend(list);
            }
        }

        let mut cover_map = HashMap::new();
        if let Some(c) = cover_opt {
            cover_map.insert(dt_id.clone(), c);
        }
        let mut power_map = HashMap::new();
        power_map.insert(dt_id, (power, tier_opt));

        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for td in combined {
            if !seen.insert(td.id.clone()) {
                continue;
            }
            if let Some(mapped) = map_raw_top_deck(td, &cover_map, &power_map) {
                out.push(mapped);
            }
        }
        Ok(out)
    }

    pub async fn get_pack_recommendations(
        &self,
        target_cards: &[PackCardRef],
    ) -> Result<Vec<PackRecommendation>, String> {
        let targets: Vec<&PackCardRef> = target_cards
            .iter()
            .filter(|c| c.missing > 0 && (c.rarity == "UR" || c.rarity == "SR"))
            .collect();

        if targets.is_empty() {
            return Ok(vec![]);
        }

        let target_map: HashMap<String, &PackCardRef> =
            targets.iter().map(|c| (c.id.clone(), *c)).collect();
        let all_ids: Vec<String> = target_map.keys().cloned().collect();

        let mut card_sources: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let now = Utc::now();

        for chunk in all_ids.chunks(35) {
            let params: Vec<String> = chunk
                .iter()
                .enumerate()
                .map(|(i, id)| format!("_id[$in][{}]={}", i, urlencoding::encode(id)))
                .collect();
            let url = format!(
                "{}/cards?{}&limit=100&fields=name,rarity,obtain",
                MDM_BASE,
                params.join("&")
            );
            let raw: Vec<RawCardObtain> = self
                .http
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Erro ao buscar Secret Packs: {}", e))?
                .error_for_status()
                .map_err(|e| format!("API MDM erro ao buscar packs: {}", e))?
                .json()
                .await
                .map_err(|e| format!("Erro ao decodificar packs: {}", e))?;

            for c in raw {
                let mut valid_packs: Vec<(String, String)> = Vec::new();
                for ob in c.obtain.unwrap_or_default() {
                    if let Some(src) = ob.source {
                        let ptype = src.source_type.unwrap_or_default();
                        let pname = src.name.unwrap_or_default();
                        if pname.is_empty() {
                            continue;
                        }
                        if let Some(exp_str) = src.expires {
                            if let Ok(exp_dt) = exp_str.parse::<DateTime<Utc>>() {
                                if exp_dt < now {
                                    continue;
                                }
                            }
                        }
                        if ptype == "Secret Pack"
                            || ptype == "Selection Pack"
                            || ptype == "Structure Deck"
                        {
                            valid_packs.push((pname, ptype));
                        }
                    }
                }
                card_sources.insert(c.id, valid_packs);
            }
        }

        struct PackAcc {
            pack_type: String,
            ur_copies: u32,
            sr_copies: u32,
            cards: Vec<PackCardRef>,
            seen: HashSet<String>,
        }
        let mut packs: HashMap<String, PackAcc> = HashMap::new();

        for (cid, status) in &target_map {
            let sources = card_sources.get(cid).cloned().unwrap_or_default();
            let entries = if sources.is_empty() {
                vec![(
                    "Master Pack / Apenas Craft".to_string(),
                    "Master Pack / Craft".to_string(),
                )]
            } else {
                sources
            };

            for (pname, ptype) in entries {
                let acc = packs.entry(pname).or_insert_with(|| PackAcc {
                    pack_type: ptype,
                    ur_copies: 0,
                    sr_copies: 0,
                    cards: Vec::new(),
                    seen: HashSet::new(),
                });
                if acc.seen.insert(cid.clone()) {
                    if status.rarity == "UR" {
                        acc.ur_copies += status.missing;
                    } else if status.rarity == "SR" {
                        acc.sr_copies += status.missing;
                    }
                    acc.cards.push(PackCardRef {
                        id: status.id.clone(),
                        name: status.name.clone(),
                        rarity: status.rarity.clone(),
                        missing: status.missing,
                    });
                }
            }
        }

        let mut out: Vec<PackRecommendation> = packs
            .into_iter()
            .map(|(pack_name, mut acc)| {
                acc.cards.sort_by(|a, b| {
                    (b.rarity == "UR")
                        .cmp(&(a.rarity == "UR"))
                        .then(a.name.cmp(&b.name))
                });
                PackRecommendation {
                    pack_name,
                    pack_type: acc.pack_type,
                    ur_missing_copies: acc.ur_copies,
                    sr_missing_copies: acc.sr_copies,
                    total_missing_copies: acc.ur_copies + acc.sr_copies,
                    cards: acc.cards,
                }
            })
            .collect();

        out.sort_by(|a, b| {
            let a_craft = a.pack_type == "Master Pack / Craft";
            let b_craft = b.pack_type == "Master Pack / Craft";
            a_craft
                .cmp(&b_craft)
                .then_with(|| b.ur_missing_copies.cmp(&a.ur_missing_copies))
                .then_with(|| b.sr_missing_copies.cmp(&a.sr_missing_copies))
                .then_with(|| a.pack_name.cmp(&b.pack_name))
        });

        Ok(out)
    }

    /// Fetches the entire released Master Duel card catalog (~14k+ cards) across all pages.
    /// Parses each card via serde_json::Value so a single unusual card (e.g. atk="?" or float)
    /// can NEVER fail deserialization of an entire 3,000-card page.
    pub async fn fetch_all_cards_catalog(&self) -> Result<Vec<SearchCardResult>, String> {
        let fields = "name,rarity,konamiID,gameId,ocgRelease,alternateArt,type,monsterType,attribute,race,level,linkRating,atk,def,description";
        let mut by_norm_name: HashMap<String, (bool, SearchCardResult)> =
            HashMap::with_capacity(16000);

        let mut page = 1u32;
        loop {
            // Fetch up to 3 pages in parallel per wave
            let mut handles = Vec::new();
            for p in page..(page + 3) {
                let client = self.http.clone();
                let url = format!(
                    "{}/cards?limit=3000&page={}&fields={}",
                    MDM_BASE, p, fields
                );
                handles.push(tokio::spawn(async move {
                    let resp = client.get(&url).send().await.ok()?;
                    if !resp.status().is_success() {
                        return None;
                    }
                    resp.json::<Vec<serde_json::Value>>().await.ok()
                }));
            }

            let mut any_full_page = false;
            for h in handles {
                if let Ok(Some(batch)) = h.await {
                    if batch.len() >= 3000 {
                        any_full_page = true;
                    }
                    for val in batch {
                        let Some(id) = val.get("_id").and_then(|v| v.as_str()) else {
                            continue;
                        };
                        let Some(name) = val.get("name").and_then(|v| v.as_str()) else {
                            continue;
                        };
                        let Some(rarity) = val.get("rarity").and_then(|v| v.as_str()) else {
                            continue;
                        };
                        if rarity.is_empty() {
                            continue;
                        }
                        let is_alt_art = val
                            .get("alternateArt")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let norm_name = name.to_lowercase();
                        if let Some((existing_is_alt, _)) = by_norm_name.get(&norm_name) {
                            // Prefer standard art (alternateArt == false) over alternate art
                            if !*existing_is_alt || is_alt_art {
                                continue;
                            }
                        }

                        let monster_type: Vec<String> = val
                            .get("monsterType")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();

                        let is_extra = is_extra_deck_monster(&Some(monster_type.clone()));

                        let konami_id = val.get("konamiID").and_then(|v| {
                            v.as_str()
                                .map(|s| s.to_string())
                                .or_else(|| v.as_u64().map(|n| n.to_string()))
                        });

                        let parse_u32 = |key: &str| -> Option<u32> {
                            val.get(key).and_then(|v| {
                                v.as_u64()
                                    .map(|n| n as u32)
                                    .or_else(|| v.as_f64().map(|f| f as u32))
                                    .or_else(|| v.as_str().and_then(|s| s.parse::<u32>().ok()))
                            })
                        };
                        let parse_i32 = |key: &str| -> Option<i32> {
                            val.get(key).and_then(|v| {
                                v.as_i64()
                                    .map(|n| n as i32)
                                    .or_else(|| v.as_f64().map(|f| f as i32))
                                    .or_else(|| v.as_str().and_then(|s| s.parse::<i32>().ok()))
                            })
                        };

                        let card_item = SearchCardResult {
                            id: id.to_string(),
                            name: name.to_string(),
                            rarity: rarity.to_string(),
                            konami_id,
                            game_id: parse_u32("gameId"),
                            ocg_release: val
                                .get("ocgRelease")
                                .and_then(|v| v.as_str())
                                .or_else(|| val.get("release").and_then(|v| v.as_str()))
                                .map(|s| s.to_string()),
                            card_type: val
                                .get("type")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            monster_type,
                            attribute: val
                                .get("attribute")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            race: val
                                .get("race")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            level: parse_u32("level"),
                            link_rating: parse_u32("linkRating"),
                            atk: parse_i32("atk"),
                            def: parse_i32("def"),
                            description: val
                                .get("description")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            is_extra,
                        };
                        by_norm_name.insert(norm_name, (is_alt_art, card_item));
                    }
                }
            }

            if !any_full_page || page >= 16 {
                break;
            }
            page += 3;
        }

        if by_norm_name.is_empty() {
            return Err(
                "Não foi possível baixar o catálogo de cartas do Master Duel Meta.".to_string(),
            );
        }

        let mut catalog: Vec<SearchCardResult> =
            by_norm_name.into_values().map(|(_, c)| c).collect();
        catalog.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(catalog)
    }

    pub async fn resolve_ydk_cards(
        &self,
        konami_ids: &[String],
    ) -> Result<HashMap<String, (String, String, String, bool)>, String> {
        let unique: Vec<String> = konami_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let mut resolved = HashMap::new();
        for chunk in unique.chunks(40) {
            let params: Vec<String> = chunk
                .iter()
                .enumerate()
                .map(|(i, id)| format!("konamiID[$in][{}]={}", i, urlencoding::encode(id)))
                .collect();
            let url = format!(
                "{}/cards?{}&limit=100&fields=name,rarity,konamiID,monsterType",
                MDM_BASE,
                params.join("&")
            );
            let raw: Vec<RawCardFull> = self
                .http
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Erro ao resolver IDs YDK: {}", e))?
                .error_for_status()
                .map_err(|e| format!("API MDM erro ao resolver YDK: {}", e))?
                .json()
                .await
                .map_err(|e| format!("Erro ao decodificar cartas YDK: {}", e))?;

            for c in raw {
                if let Some(kid) = c.konami_id {
                    let is_extra = is_extra_deck_monster(&c.monster_type);
                    resolved.insert(
                        kid,
                        (c.id, c.name, c.rarity.unwrap_or_else(|| "N".to_string()), is_extra),
                    );
                }
            }
        }
        Ok(resolved)
    }

    pub async fn export_cards_to_ydk(
        &self,
        main: &[DeckCard],
        extra: &[DeckCard],
    ) -> Result<String, String> {
        let all_ids: Vec<String> = main
            .iter()
            .chain(extra.iter())
            .map(|c| c.id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let mut id_to_konami: HashMap<String, String> = HashMap::new();
        for chunk in all_ids.chunks(40) {
            let params: Vec<String> = chunk
                .iter()
                .enumerate()
                .map(|(i, id)| format!("_id[$in][{}]={}", i, urlencoding::encode(id)))
                .collect();
            let url = format!(
                "{}/cards?{}&limit=100&fields=konamiID",
                MDM_BASE,
                params.join("&")
            );
            let raw: Vec<RawCardIdOnly> = self
                .http
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Erro ao buscar Konami IDs para YDK: {}", e))?
                .error_for_status()
                .map_err(|e| format!("API MDM erro ao exportar YDK: {}", e))?
                .json()
                .await
                .map_err(|e| format!("Erro ao decodificar Konami IDs: {}", e))?;

            for c in raw {
                if let Some(kid) = c.konami_id {
                    id_to_konami.insert(c.id, kid);
                }
            }
        }

        let mut out = String::from("#created by DuelCraft (Master Duel Meta)\n#main\n");
        for card in main {
            if let Some(kid) = id_to_konami.get(&card.id) {
                for _ in 0..card.amount {
                    out.push_str(kid);
                    out.push('\n');
                }
            }
        }
        out.push_str("#extra\n");
        for card in extra {
            if let Some(kid) = id_to_konami.get(&card.id) {
                for _ in 0..card.amount {
                    out.push_str(kid);
                    out.push('\n');
                }
            }
        }
        out.push_str("!side\n");
        Ok(out)
    }
}

fn is_extra_deck_monster(monster_types: &Option<Vec<String>>) -> bool {
    if let Some(types) = monster_types {
        for t in types {
            let lower = t.to_lowercase();
            if lower == "fusion" || lower == "synchro" || lower == "xyz" || lower == "link" {
                return true;
            }
        }
    }
    false
}

fn map_raw_top_deck(
    td: RawTopDeck,
    cover_by_type_id: &HashMap<String, String>,
    power_by_type_id: &HashMap<String, (f64, Option<u8>)>,
) -> Option<MetaDeck> {
    let dt = td.deck_type?;
    let main: Vec<DeckCard> = td
        .main
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            let c = entry.card?;
            Some(DeckCard {
                id: c.id,
                name: c.name,
                rarity: c.rarity.unwrap_or_else(|| "N".to_string()),
                amount: entry.amount.unwrap_or(1),
                section: "main".to_string(),
            })
        })
        .collect();

    let extra: Vec<DeckCard> = td
        .extra
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            let c = entry.card?;
            Some(DeckCard {
                id: c.id,
                name: c.name,
                rarity: c.rarity.unwrap_or_else(|| "N".to_string()),
                amount: entry.amount.unwrap_or(1),
                section: "extra".to_string(),
            })
        })
        .collect();

    if main.is_empty() {
        return None;
    }

    let (power, explicit_tier) = power_by_type_id
        .get(&dt.id)
        .copied()
        .unwrap_or((dt.tournament_power.unwrap_or(0.0), None));
    let (tier, _) = tier_from_power(explicit_tier, power);

    let cover_card_id = cover_by_type_id
        .get(&dt.id)
        .cloned()
        .or_else(|| extra.first().or_else(|| main.first()).map(|c| c.id.clone()));

    let engines = td
        .engines
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.name)
        .collect();

    let tt_name = td.tournament_type.and_then(|t| t.name).filter(|s| !s.is_empty());
    let rt_name = td.ranked_type.and_then(|r| r.name).filter(|s| !s.is_empty());
    let custom_name = td.custom_tournament_name.filter(|s| !s.is_empty());
    let placement = td.tournament_placement.filter(|s| !s.is_empty());

    let is_tournament = tt_name.is_some() || custom_name.is_some() || placement.is_some();

    let event_name = custom_name
        .clone()
        .or_else(|| tt_name.clone())
        .or_else(|| rt_name.clone());

    let (category, badge_label) = classify_deck_category_and_badge(
        is_tournament,
        tt_name.as_deref(),
        custom_name.as_deref(),
        rt_name.as_deref(),
        placement.as_deref(),
        td.duel_rating,
    );

    let mdm_url = td
        .url
        .map(|u| {
            if u.starts_with("http") {
                u
            } else {
                format!("https://www.masterduelmeta.com{}", u)
            }
        })
        .unwrap_or_else(|| "https://www.masterduelmeta.com/top-decks".to_string());

    Some(MetaDeck {
        id: td.id,
        deck_type: dt.name,
        deck_type_id: dt.id,
        cover_card_id,
        tournament_power: power,
        tier,
        category,
        badge_label,
        placement,
        event_name,
        engines,
        created: td.created.unwrap_or_default(),
        mdm_url,
        ur_price: td.ur_price.unwrap_or(0),
        sr_price: td.sr_price.unwrap_or(0),
        main,
        extra,
    })
}

fn classify_deck_category_and_badge(
    is_tournament: bool,
    tt_name: Option<&str>,
    custom_name: Option<&str>,
    rt_name: Option<&str>,
    placement: Option<&str>,
    duel_rating: Option<f64>,
) -> (String, String) {
    if is_tournament {
        let ev = custom_name.or(tt_name).unwrap_or("Torneio");
        let label = if let Some(pl) = placement {
            format!("{} • {}", pl, ev)
        } else {
            ev.to_string()
        };
        let lower_ev = ev.to_lowercase();
        if lower_ev.contains("duelist cup") || lower_ev.contains("dlv") || lower_ev.contains("wcs") {
            return ("duelist_cup".to_string(), label);
        }
        return ("tournament".to_string(), label);
    }

    let r_str = rt_name.unwrap_or("");
    let r_lower = r_str.to_lowercase();

    if r_lower.contains("duelist cup") || r_lower.contains("dlv") || r_lower.contains("wcs") {
        return (
            "duelist_cup".to_string(),
            if r_str.is_empty() {
                "Duelist Cup".to_string()
            } else {
                r_str.to_string()
            },
        );
    }

    if r_lower.contains("rating") || duel_rating.is_some() {
        let label = if let Some(dr) = duel_rating {
            format!("Rated {:.0}", dr)
        } else if !r_str.is_empty() {
            r_str.to_string()
        } else {
            "Master I (Rated)".to_string()
        };
        return ("master1".to_string(), label);
    }

    if (r_lower.contains("master i")
        && !r_lower.contains("master ii")
        && !r_lower.contains("master iii")
        && !r_lower.contains("master iv"))
        || r_lower.contains("master 1")
    {
        return ("master1".to_string(), "Master I".to_string());
    }

    if r_lower.contains("master v") || r_lower.contains("master 5") {
        return ("master5".to_string(), "Master V".to_string());
    }

    if !r_str.is_empty() {
        return ("other".to_string(), r_str.to_string());
    }

    ("other".to_string(), "Top Deck".to_string())
}

#[derive(Debug, Deserialize)]
struct RawDeckType {
    #[serde(rename = "_id")]
    id: String,
    name: String,
    card: Option<String>,
    tier: Option<u8>,
    #[serde(rename = "tournamentPower")]
    tournament_power: Option<f64>,
    #[serde(rename = "popRank")]
    pop_rank: Option<f64>,
    #[serde(rename = "deckBreakdown")]
    deck_breakdown: Option<RawDeckBreakdown>,
}

#[derive(Debug, Deserialize, Default)]
struct RawDeckBreakdown {
    total: Option<u32>,
    #[serde(rename = "avgUrPrice")]
    avg_ur_price: Option<f64>,
    #[serde(rename = "avgSrPrice")]
    avg_sr_price: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawTopDeck {
    #[serde(rename = "_id")]
    id: String,
    main: Option<Vec<RawDeckEntry>>,
    extra: Option<Vec<RawDeckEntry>>,
    #[serde(rename = "deckType")]
    deck_type: Option<RawDeckTypeRef>,
    #[serde(rename = "tournamentPlacement")]
    tournament_placement: Option<String>,
    #[serde(rename = "customTournamentName")]
    custom_tournament_name: Option<String>,
    #[serde(rename = "tournamentType")]
    tournament_type: Option<RawTournamentType>,
    #[serde(rename = "rankedType")]
    ranked_type: Option<RawTournamentType>,
    #[serde(rename = "duelRating")]
    duel_rating: Option<f64>,
    engines: Option<Vec<RawNamedRef>>,
    #[serde(rename = "urPrice")]
    ur_price: Option<u32>,
    #[serde(rename = "srPrice")]
    sr_price: Option<u32>,
    created: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawDeckEntry {
    card: Option<RawCardShort>,
    amount: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RawCardShort {
    #[serde(rename = "_id")]
    id: String,
    name: String,
    rarity: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawDeckTypeRef {
    #[serde(rename = "_id")]
    id: String,
    name: String,
    #[serde(rename = "tournamentPower")]
    tournament_power: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawTournamentType {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawNamedRef {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawCardFull {
    #[serde(rename = "_id")]
    id: String,
    name: String,
    rarity: Option<String>,
    #[serde(rename = "konamiID")]
    konami_id: Option<String>,
    #[serde(rename = "monsterType")]
    monster_type: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RawCardIdOnly {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "konamiID")]
    konami_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCardObtain {
    #[serde(rename = "_id")]
    id: String,
    obtain: Option<Vec<RawObtainEntry>>,
}

#[derive(Debug, Deserialize)]
struct RawObtainEntry {
    source: Option<RawObtainSource>,
}

#[derive(Debug, Deserialize)]
struct RawObtainSource {
    name: Option<String>,
    #[serde(rename = "type")]
    source_type: Option<String>,
    expires: Option<String>,
}
