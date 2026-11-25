use crate::models::{
    AutoChangeItem, AutoOptimizePreview, CutSuggestion, DeckCard, DeckOptimizationReport,
    DustBudget, MetaDeck, SimilarDeckMatch, StageCpSummary, UpgradeSuggestion, UserDeck,
};
use std::collections::HashMap;

const CP_PER_CARD: u32 = 30;
const CP_PER_DISMANTLE: u32 = 10;

fn is_strict_top_deck(d: &MetaDeck) -> bool {
    d.category == "tournament" || d.category == "master1" || d.category == "duelist_cup"
}

/// Achievement weight inside a cohesive archetype pool — ZERO penalty by global Tier
/// so Rogue decks are NEVER hijacked by Tier 1/2 decks sharing generic handtraps!
fn achievement_weight_balanced(d: &MetaDeck) -> f64 {
    match d.category.as_str() {
        "tournament" => {
            let pl = d.placement.as_deref().unwrap_or("").to_lowercase();
            if pl.contains("1st") || pl.contains("2nd") || pl.contains("top 4") {
                1.28
            } else {
                1.22
            }
        }
        "duelist_cup" | "master1" => 1.18,
        "master5" | _ => 1.00,
    }
}

/// Precomputed per-snapshot statistics (document frequency, per-deck card counts and TF-IDF norms).
/// Built ONCE per meta sync/expansion and reused by every analysis instead of recomputing from scratch.
pub struct MetaIndex {
    pub n: usize,
    pub df: HashMap<String, usize>,
    pub deck_counts: Vec<HashMap<String, u32>>,
    pub deck_norms: Vec<f64>,
}

impl MetaIndex {
    pub fn build(meta_decks: &[MetaDeck]) -> Self {
        let mut df: HashMap<String, usize> = HashMap::new();
        let mut deck_counts: Vec<HashMap<String, u32>> = Vec::with_capacity(meta_decks.len());
        for deck in meta_decks {
            let mut counts: HashMap<String, u32> = HashMap::new();
            for c in deck.main.iter().chain(deck.extra.iter()) {
                let e = counts.entry(c.id.clone()).or_insert(0);
                *e = (*e + c.amount).min(3);
            }
            for id in counts.keys() {
                *df.entry(id.clone()).or_insert(0) += 1;
            }
            deck_counts.push(counts);
        }

        let mut idx = Self {
            n: meta_decks.len(),
            df,
            deck_counts,
            deck_norms: Vec::new(),
        };
        let norms: Vec<f64> = idx
            .deck_counts
            .iter()
            .map(|counts| {
                counts
                    .iter()
                    .map(|(id, q)| {
                        let w = (*q as f64) * idx.idf(id);
                        w * w
                    })
                    .sum::<f64>()
                    .sqrt()
            })
            .collect();
        idx.deck_norms = norms;
        idx
    }

    pub fn idf(&self, card_id: &str) -> f64 {
        let df = *self.df.get(card_id).unwrap_or(&0) as f64;
        ((self.n as f64 + 1.0) / (df + 1.0)).ln() + 1.0
    }

    pub fn matches(&self, meta_decks: &[MetaDeck]) -> bool {
        self.deck_counts.len() == meta_decks.len()
    }
}

/// Fast archetype detector (uses the precomputed MetaIndex) so the backend can auto-load deep
/// Ranked + Championship lists of a Rogue/specific archetype before running the full analysis.
pub fn detect_archetype_fast(
    user_deck: &UserDeck,
    meta_decks: &[MetaDeck],
    index: &MetaIndex,
) -> Option<String> {
    if let Some(target) = user_deck
        .target_archetype
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return Some(target.to_string());
    }

    if (user_deck.main.is_empty() && user_deck.extra.is_empty())
        || meta_decks.is_empty()
        || !index.matches(meta_decks)
    {
        return None;
    }

    let user_counts = deck_card_map(&user_deck.main, &user_deck.extra);
    let user_vec: HashMap<&str, f64> = user_counts
        .iter()
        .map(|(id, (qty, _, _, _))| (id.as_str(), (*qty as f64) * index.idf(id)))
        .collect();
    let user_norm = l2_norm(&user_vec);
    if user_norm == 0.0 {
        return None;
    }

    let mut sims: Vec<(&str, f64)> = Vec::with_capacity(meta_decks.len());
    for (i, deck) in meta_decks.iter().enumerate() {
        let counts = &index.deck_counts[i];
        let mut dot = 0.0f64;
        for (cid, uw) in &user_vec {
            if let Some(mq) = counts.get(*cid) {
                dot += uw * (*mq as f64) * index.idf(cid);
            }
        }
        let denom = user_norm * index.deck_norms[i];
        let raw_sim = if denom > 0.0 { dot / denom } else { 0.0 };
        if raw_sim >= 0.08 {
            sims.push((deck.deck_type.as_str(), raw_sim));
        }
    }

    sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut weights: HashMap<&str, f64> = HashMap::new();
    for (arch, sim) in sims.into_iter().take(5) {
        *weights.entry(arch).or_insert(0.0) += sim;
    }
    weights
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(arch, _)| arch.to_string())
}

/// Lazy-learning KNN optimizer using Cohesive-Archetype TF-IDF Cosine Similarity.
/// Eliminates Rogue deck bias by detecting the user's archetype via pure TF-IDF first (zero Tier penalty),
/// locking onto cohesive archetype lists (Master V, Master I, Duelist Cup & Championships),
/// and supporting 3 modes: "balanced" (default), "strict_top", and "pure_sim".
pub fn optimize_user_deck_knn(
    user_deck: &UserDeck,
    meta_decks: &[MetaDeck],
    index: &MetaIndex,
    budget: &DustBudget,
    k: usize,
    knn_mode: &str,
) -> DeckOptimizationReport {
    let total_meta = meta_decks.len();
    if total_meta == 0 {
        return empty_report();
    }

    // 1. Reuse precomputed document frequencies (rebuild locally only if the index is stale)
    let local_index;
    let index = if index.matches(meta_decks) {
        index
    } else {
        local_index = MetaIndex::build(meta_decks);
        &local_index
    };
    let df_map = &index.df;
    let n_f = index.n as f64;
    let idf = |card_id: &str| -> f64 { index.idf(card_id) };

    // 2. Build user's card count map & TF-IDF vector
    let mut user_counts: HashMap<String, (u32, String, String, String)> = HashMap::new();
    for c in user_deck.main.iter().chain(user_deck.extra.iter()) {
        user_counts
            .entry(c.id.clone())
            .and_modify(|e| e.0 = (e.0 + c.amount).min(3))
            .or_insert_with(|| {
                (
                    c.amount.min(3),
                    c.name.clone(),
                    c.rarity.clone(),
                    c.section.clone(),
                )
            });
    }

    let user_vec: HashMap<&str, f64> = user_counts
        .iter()
        .map(|(id, (qty, _, _, _))| (id.as_str(), (*qty as f64) * idf(id)))
        .collect();
    let user_norm = l2_norm(&user_vec);

    let target_filter = user_deck
        .target_archetype
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    // 3. Stage 1: Compute PURE TF-IDF similarity across all meta decks (zero Tier bias!)
    let mut raw_scored: Vec<(&MetaDeck, f64, u32)> = Vec::with_capacity(meta_decks.len());
    for (i, deck) in meta_decks.iter().enumerate() {
        if user_counts.is_empty() {
            raw_scored.push((deck, 1.0, 0));
            continue;
        }

        let counts = &index.deck_counts[i];
        let mut dot = 0.0f64;
        let mut shared_cards = 0u32;

        for (cid, uw) in &user_vec {
            if let Some(mq) = counts.get(*cid) {
                dot += uw * (*mq as f64) * idf(cid);
                let u_qty = user_counts.get(*cid).map(|e| e.0).unwrap_or(0);
                shared_cards += u_qty.min(*mq);
            }
        }

        let denom = user_norm * index.deck_norms[i];
        let raw_sim = if denom > 0.0 { dot / denom } else { 0.0 };
        raw_scored.push((deck, raw_sim, shared_cards));
    }

    // 4. Detect true archetype from explicit target OR top pure-TF-IDF matches (unbiased!)
    let detected_archetype = if let Some(target) = target_filter {
        target.to_string()
    } else if user_counts.is_empty() {
        "Meta Geral".to_string()
    } else {
        let mut pure_top = raw_scored.clone();
        pure_top.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut arch_weights: HashMap<&str, f64> = HashMap::new();
        for (d, sim, _) in pure_top.iter().take(6).filter(|(_, s, _)| *s > 0.04) {
            *arch_weights.entry(d.deck_type.as_str()).or_insert(0.0) += *sim;
        }
        arch_weights
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| name.to_string())
            .unwrap_or_else(|| "Desconhecido".to_string())
    };

    // 5. Lock onto cohesive archetype pool so Tier 1 decks sharing staples NEVER hijack Rogue decks!
    let cohesive_candidates: Vec<(&MetaDeck, f64, u32)> = if let Some(target) = target_filter {
        let exact: Vec<(&MetaDeck, f64, u32)> = raw_scored
            .iter()
            .copied()
            .filter(|(d, _, _)| {
                d.deck_type.eq_ignore_ascii_case(target)
                    || d.engines.iter().any(|e| e.eq_ignore_ascii_case(target))
            })
            .collect();
        if exact.is_empty() {
            raw_scored
                .iter()
                .copied()
                .filter(|(_, sim, _)| *sim > 0.02)
                .collect()
        } else {
            exact
        }
    } else if !user_counts.is_empty() && detected_archetype != "Desconhecido" {
        let same_arch_or_high_sim: Vec<(&MetaDeck, f64, u32)> = raw_scored
            .iter()
            .copied()
            .filter(|(d, sim, _)| {
                d.deck_type.eq_ignore_ascii_case(&detected_archetype)
                    || d.engines
                        .iter()
                        .any(|e| e.eq_ignore_ascii_case(&detected_archetype))
                    || *sim >= 0.30
            })
            .collect();
        if same_arch_or_high_sim.len() >= 3 {
            same_arch_or_high_sim
        } else {
            raw_scored
                .iter()
                .copied()
                .filter(|(_, sim, _)| *sim > 0.02)
                .collect()
        }
    } else {
        raw_scored
    };

    // 6. Apply selected KNN Mode ("balanced" | "strict_top" | "pure_sim")
    let requested_mode = match knn_mode {
        "strict_top" | "pure_sim" | "balanced" => knn_mode,
        _ => "balanced",
    };

    let mut competitive_only_used = false;
    let mut knn_mode_used = requested_mode.to_string();

    let active_pool: Vec<(&MetaDeck, f64, u32)> = if requested_mode == "strict_top" {
        let strict: Vec<(&MetaDeck, f64, u32)> = cohesive_candidates
            .iter()
            .copied()
            .filter(|(d, _, _)| is_strict_top_deck(d))
            .collect();
        if strict.len() >= 3 {
            competitive_only_used = true;
            strict
        } else {
            knn_mode_used = "balanced_fallback".to_string();
            cohesive_candidates
        }
    } else {
        cohesive_candidates
    };

    let mut scored: Vec<(&MetaDeck, f64, f64, u32)> = active_pool
        .into_iter()
        .map(|(deck, raw_sim, shared_cards)| {
            let mult = if knn_mode_used == "pure_sim" {
                1.0
            } else {
                achievement_weight_balanced(deck)
            };
            (deck, raw_sim, raw_sim * mult, shared_cards)
        })
        .collect();

    scored.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.3.cmp(&a.3))
            .then_with(|| a.0.ur_price.cmp(&b.0.ur_price))
    });

    let k_eff = k.clamp(3, 15).min(scored.len());
    let neighbors = &scored[..k_eff];

    if neighbors.is_empty() {
        return empty_report();
    }

    let similar_decks: Vec<SimilarDeckMatch> = neighbors
        .iter()
        .map(|(d, raw_sim, _, shared)| SimilarDeckMatch {
            deck_id: d.id.clone(),
            deck_type: d.deck_type.clone(),
            tier: d.tier,
            category: d.category.clone(),
            badge_label: d.badge_label.clone(),
            placement: d.placement.clone(),
            engines: d.engines.clone(),
            similarity_pct: (raw_sim * 1000.0).round() / 10.0,
            shared_cards: *shared,
            cover_card_id: d.cover_card_id.clone(),
        })
        .collect();

    // 6. Aggregate weighted card frequencies & recommended copy counts across K neighbors
    struct CardAgg {
        name: String,
        rarity: String,
        section: String,
        weight_present: f64,
        weighted_qty_sum: f64,
    }
    let mut agg_map: HashMap<String, CardAgg> = HashMap::new();
    let mut total_weight = 0.0f64;

    for (deck, _, rank_score, _) in neighbors {
        let w = rank_score.max(0.1);
        total_weight += w;
        let counts = deck_card_map(&deck.main, &deck.extra);
        for (cid, (qty, name, rarity, section)) in counts {
            agg_map
                .entry(cid)
                .and_modify(|a| {
                    a.weight_present += w;
                    a.weighted_qty_sum += w * (qty as f64);
                })
                .or_insert(CardAgg {
                    name,
                    rarity,
                    section,
                    weight_present: w,
                    weighted_qty_sum: w * (qty as f64),
                });
        }
    }

    // 7. Build prioritized Upgrade Suggestions (Stages 1, 2, 3)
    let mut upgrades: Vec<UpgradeSuggestion> = Vec::new();
    for (cid, agg) in &agg_map {
        let usage_ratio = if total_weight > 0.0 {
            agg.weight_present / total_weight
        } else {
            0.0
        };
        let neighbor_usage_pct = (usage_ratio * 1000.0).round() / 10.0;
        if neighbor_usage_pct < 22.0 {
            continue;
        }

        let avg_when_played = if agg.weight_present > 0.0 {
            agg.weighted_qty_sum / agg.weight_present
        } else {
            1.0
        };
        let recommended_qty = (avg_when_played.round() as u32).clamp(1, 3);
        let current_qty = user_counts.get(cid).map(|e| e.0).unwrap_or(0);

        if recommended_qty <= current_qty {
            continue;
        }

        let add_qty = recommended_qty - current_qty;
        let cp_cost = add_qty * CP_PER_CARD;
        let global_df = *df_map.get(cid).unwrap_or(&0) as f64;
        let global_meta_pct = ((global_df / n_f) * 1000.0).round() / 10.0;

        let (stage, stage_label) = if agg.rarity != "UR" {
            (1u8, "Etapa 1 — Barato (0 UR)".to_string())
        } else if neighbor_usage_pct >= 60.0 || global_meta_pct >= 40.0 {
            (2u8, "Etapa 2 — UR Prioritária".to_string())
        } else {
            (3u8, "Etapa 3 — Tech / Refinamento".to_string())
        };

        let cheap_bonus = match agg.rarity.as_str() {
            "N" | "R" => 15.0,
            "SR" => 8.0,
            _ => 0.0,
        };
        let priority_score =
            ((neighbor_usage_pct * 0.65 + global_meta_pct * 0.25 + cheap_bonus) * 10.0).round()
                / 10.0;

        upgrades.push(UpgradeSuggestion {
            id: cid.clone(),
            name: agg.name.clone(),
            rarity: agg.rarity.clone(),
            section: agg.section.clone(),
            current_qty,
            recommended_qty,
            add_qty,
            neighbor_usage_pct,
            global_meta_pct,
            cp_cost,
            stage,
            stage_label,
            priority_score,
        });
    }

    upgrades.sort_by(|a, b| {
        a.stage
            .cmp(&b.stage)
            .then_with(|| {
                b.priority_score
                    .partial_cmp(&a.priority_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.name.cmp(&b.name))
    });

    // 8. Build Possible Cuts
    let mut cuts: Vec<CutSuggestion> = Vec::new();
    for (cid, (current_qty, name, rarity, section)) in &user_counts {
        let usage_pct = agg_map
            .get(cid)
            .map(|a| {
                if total_weight > 0.0 {
                    ((a.weight_present / total_weight) * 1000.0).round() / 10.0
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0);

        if usage_pct < 15.0 {
            let reason = if usage_pct == 0.0 {
                format!(
                    "0% de uso nos {} decks competitivos mais parecidos — fora do padrão vencedor.",
                    k_eff
                )
            } else {
                format!(
                    "Apenas {:.0}% de uso nos {} decks competitivos — baixa consistência.",
                    usage_pct, k_eff
                )
            };
            cuts.push(CutSuggestion {
                id: cid.clone(),
                name: name.clone(),
                rarity: rarity.clone(),
                section: section.clone(),
                current_qty: *current_qty,
                recommended_qty: 0,
                neighbor_usage_pct: usage_pct,
                reason,
            });
        } else if let Some(agg) = agg_map.get(cid) {
            let avg_qty = (agg.weighted_qty_sum / agg.weight_present.max(0.001)).round() as u32;
            if *current_qty > avg_qty.max(1) && *current_qty >= 2 {
                cuts.push(CutSuggestion {
                    id: cid.clone(),
                    name: name.clone(),
                    rarity: rarity.clone(),
                    section: section.clone(),
                    current_qty: *current_qty,
                    recommended_qty: avg_qty.max(1),
                    neighbor_usage_pct: usage_pct,
                    reason: format!(
                        "Maioria competitiva usa {}x (você usa {}x) — libera espaço para upgrades.",
                        avg_qty.max(1),
                        current_qty
                    ),
                });
            }
        }
    }

    cuts.sort_by(|a, b| {
        a.neighbor_usage_pct
            .partial_cmp(&b.neighbor_usage_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });

    // 9. Staged CP Roadmap
    let stage_defs = [
        (
            1u8,
            "Etapa 1 — Melhorias Baratas",
            "Cartas N / R / SR essenciais (0 UR CP)",
        ),
        (
            2u8,
            "Etapa 2 — URs Prioritárias",
            "Core de alto impacto e Staples Universais",
        ),
        (
            3u8,
            "Etapa 3 — Refinamento & Techs",
            "Opções situacionais para fechar 100% Meta",
        ),
    ];

    let mut stages: Vec<StageCpSummary> = Vec::with_capacity(3);
    let mut total_ur_cp = 0u32;
    let mut total_sr_cp = 0u32;
    let mut total_r_cp = 0u32;
    let mut total_n_cp = 0u32;

    for (st_num, title, subtitle) in stage_defs {
        let mut ur = 0u32;
        let mut sr = 0u32;
        let mut r = 0u32;
        let mut n = 0u32;
        let mut copies = 0u32;

        for up in upgrades.iter().filter(|u| u.stage == st_num) {
            copies += up.add_qty;
            match up.rarity.as_str() {
                "UR" => ur += up.cp_cost,
                "SR" => sr += up.cp_cost,
                "R" => r += up.cp_cost,
                _ => n += up.cp_cost,
            }
        }

        total_ur_cp += ur;
        total_sr_cp += sr;
        total_r_cp += r;
        total_n_cp += n;

        let can_afford =
            ur <= budget.ur_cp && sr <= budget.sr_cp && r <= budget.r_cp && n <= budget.n_cp;

        stages.push(StageCpSummary {
            stage: st_num,
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            ur_cp: ur,
            sr_cp: sr,
            r_cp: r,
            n_cp: n,
            card_copies: copies,
            can_afford_with_budget: can_afford,
        });
    }

    let ur_deficit = total_ur_cp.saturating_sub(budget.ur_cp);
    let sr_deficit = total_sr_cp.saturating_sub(budget.sr_cp);
    let ur_dismantle_needed = (ur_deficit + CP_PER_DISMANTLE - 1) / CP_PER_DISMANTLE;
    let sr_dismantle_needed = (sr_deficit + CP_PER_DISMANTLE - 1) / CP_PER_DISMANTLE;

    // 10. Build Automatic Before-vs-After Optimization Preview with clear reasons!
    let auto_optimize = build_auto_optimize_preview(
        user_deck,
        &user_counts,
        &upgrades,
        &cuts,
        neighbors,
        &idf,
        k_eff,
    );

    DeckOptimizationReport {
        detected_archetype,
        k_neighbors: k_eff,
        knn_mode_used,
        competitive_only_used,
        similar_decks,
        upgrades,
        cuts,
        stages,
        total_ur_cp,
        total_sr_cp,
        total_r_cp,
        total_n_cp,
        ur_deficit,
        sr_deficit,
        ur_dismantle_needed,
        sr_dismantle_needed,
        auto_optimize,
    }
}

fn build_auto_optimize_preview(
    user_deck: &UserDeck,
    user_counts: &HashMap<String, (u32, String, String, String)>,
    upgrades: &[UpgradeSuggestion],
    cuts: &[CutSuggestion],
    neighbors: &[(&MetaDeck, f64, f64, u32)],
    idf: &impl Fn(&str) -> f64,
    k_eff: usize,
) -> AutoOptimizePreview {
    let before_main_count: u32 = user_deck.main.iter().map(|c| c.amount).sum();
    let before_extra_count: u32 = user_deck.extra.iter().map(|c| c.amount).sum();

    let mut opt_map: HashMap<String, DeckCard> = HashMap::new();
    for c in user_deck.main.iter().chain(user_deck.extra.iter()) {
        opt_map.insert(c.id.clone(), c.clone());
    }

    let mut removed_changes: Vec<AutoChangeItem> = Vec::new();
    let mut added_changes: Vec<AutoChangeItem> = Vec::new();

    // Step A: Apply all clear cuts (cards with <15% usage or excess copies)
    for cut in cuts {
        let delta = cut.current_qty.saturating_sub(cut.recommended_qty);
        if delta == 0 {
            continue;
        }
        if cut.recommended_qty == 0 {
            opt_map.remove(&cut.id);
        } else if let Some(entry) = opt_map.get_mut(&cut.id) {
            entry.amount = cut.recommended_qty;
        }
        removed_changes.push(AutoChangeItem {
            action: "remove".to_string(),
            id: cut.id.clone(),
            name: cut.name.clone(),
            rarity: cut.rarity.clone(),
            section: cut.section.clone(),
            from_qty: cut.current_qty,
            to_qty: cut.recommended_qty,
            delta,
            cp_cost: 0,
            neighbor_usage_pct: cut.neighbor_usage_pct,
            reason: cut.reason.clone(),
        });
    }

    // Step B: Add highest-impact upgrades (usage >= 45%, respecting 60 Main / 15 Extra limits)
    let mut cur_main: u32 = opt_map
        .values()
        .filter(|c| c.section == "main")
        .map(|c| c.amount)
        .sum();
    let mut cur_extra: u32 = opt_map
        .values()
        .filter(|c| c.section == "extra")
        .map(|c| c.amount)
        .sum();

    // Target main size: keep near original (or 40 if original was smaller)
    let target_main_cap = before_main_count.clamp(40, 60);

    let mut sorted_ups: Vec<&UpgradeSuggestion> = upgrades
        .iter()
        .filter(|u| u.neighbor_usage_pct >= 45.0)
        .collect();
    sorted_ups.sort_by(|a, b| {
        b.neighbor_usage_pct
            .partial_cmp(&a.neighbor_usage_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.priority_score
                    .partial_cmp(&a.priority_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut ur_cp_needed = 0u32;
    let mut sr_cp_needed = 0u32;
    let mut r_cp_needed = 0u32;
    let mut n_cp_needed = 0u32;

    for up in sorted_ups {
        let is_extra = up.section == "extra";
        let room = if is_extra {
            15u32.saturating_sub(cur_extra)
        } else {
            target_main_cap.saturating_sub(cur_main)
        };
        if room == 0 {
            continue;
        }
        let add_now = up.add_qty.min(room);
        if add_now == 0 {
            continue;
        }

        let new_qty = (up.current_qty + add_now).min(3);
        opt_map.insert(
            up.id.clone(),
            DeckCard {
                id: up.id.clone(),
                name: up.name.clone(),
                rarity: up.rarity.clone(),
                amount: new_qty,
                section: up.section.clone(),
            },
        );
        if is_extra {
            cur_extra += add_now;
        } else {
            cur_main += add_now;
        }

        let cost = add_now * CP_PER_CARD;
        match up.rarity.as_str() {
            "UR" => ur_cp_needed += cost,
            "SR" => sr_cp_needed += cost,
            "R" => r_cp_needed += cost,
            _ => n_cp_needed += cost,
        }

        let staple_note = if up.global_meta_pct >= 40.0 {
            format!(" • Staple em {:.0}% de todo o meta", up.global_meta_pct)
        } else {
            String::new()
        };
        let reason = format!(
            "Presente em {:.0}% dos {} decks competitivos (média {}x){}.",
            up.neighbor_usage_pct, k_eff, up.recommended_qty, staple_note
        );

        added_changes.push(AutoChangeItem {
            action: "add".to_string(),
            id: up.id.clone(),
            name: up.name.clone(),
            rarity: up.rarity.clone(),
            section: up.section.clone(),
            from_qty: up.current_qty,
            to_qty: new_qty,
            delta: add_now,
            cp_cost: cost,
            neighbor_usage_pct: up.neighbor_usage_pct,
            reason,
        });
    }

    // Build final sorted Main & Extra vectors
    let mut optimized_main: Vec<DeckCard> = opt_map
        .values()
        .filter(|c| c.section == "main")
        .cloned()
        .collect();
    let mut optimized_extra: Vec<DeckCard> = opt_map
        .values()
        .filter(|c| c.section == "extra")
        .cloned()
        .collect();
    optimized_main.sort_by(|a, b| a.name.cmp(&b.name));
    optimized_extra.sort_by(|a, b| a.name.cmp(&b.name));

    let after_main_count: u32 = optimized_main.iter().map(|c| c.amount).sum();
    let after_extra_count: u32 = optimized_extra.iter().map(|c| c.amount).sum();

    // Compute average TF-IDF Cosine Similarity against the K competitive neighbors BEFORE vs AFTER
    let before_counts_only: HashMap<String, u32> = user_counts
        .iter()
        .map(|(k, v)| (k.clone(), v.0))
        .collect();
    let after_counts_only: HashMap<String, u32> = opt_map
        .iter()
        .map(|(k, v)| (k.clone(), v.amount))
        .collect();

    let before_meta_score = avg_similarity_to_neighbors(&before_counts_only, neighbors, idf);
    let after_meta_score = avg_similarity_to_neighbors(&after_counts_only, neighbors, idf);

    AutoOptimizePreview {
        before_meta_score,
        after_meta_score,
        before_main_count,
        after_main_count,
        before_extra_count,
        after_extra_count,
        removed_changes,
        added_changes,
        optimized_main,
        optimized_extra,
        ur_cp_needed,
        sr_cp_needed,
        r_cp_needed,
        n_cp_needed,
    }
}

fn avg_similarity_to_neighbors(
    counts: &HashMap<String, u32>,
    neighbors: &[(&MetaDeck, f64, f64, u32)],
    idf: &impl Fn(&str) -> f64,
) -> f64 {
    if counts.is_empty() || neighbors.is_empty() {
        return 0.0;
    }
    let vec_a: HashMap<&str, f64> = counts
        .iter()
        .map(|(id, qty)| (id.as_str(), (*qty as f64) * idf(id)))
        .collect();
    let norm_a = l2_norm(&vec_a);
    if norm_a == 0.0 {
        return 0.0;
    }

    let mut sum = 0.0f64;
    for (deck, _, _, _) in neighbors {
        let m = deck_card_map(&deck.main, &deck.extra);
        let mut dot = 0.0f64;
        let mut sq = 0.0f64;
        for (cid, (qty, _, _, _)) in &m {
            let w = (*qty as f64) * idf(cid);
            sq += w * w;
            if let Some(wa) = vec_a.get(cid.as_str()) {
                dot += wa * w;
            }
        }
        let denom = norm_a * sq.sqrt();
        if denom > 0.0 {
            sum += dot / denom;
        }
    }
    ((sum / (neighbors.len() as f64)) * 1000.0).round() / 10.0
}

fn deck_card_map(
    main: &[DeckCard],
    extra: &[DeckCard],
) -> HashMap<String, (u32, String, String, String)> {
    let mut map: HashMap<String, (u32, String, String, String)> = HashMap::new();
    for c in main.iter().chain(extra.iter()) {
        map.entry(c.id.clone())
            .and_modify(|e| e.0 = (e.0 + c.amount).min(3))
            .or_insert_with(|| {
                (
                    c.amount.min(3),
                    c.name.clone(),
                    c.rarity.clone(),
                    c.section.clone(),
                )
            });
    }
    map
}

fn l2_norm(v: &HashMap<&str, f64>) -> f64 {
    v.values().map(|x| x * x).sum::<f64>().sqrt()
}

fn empty_report() -> DeckOptimizationReport {
    DeckOptimizationReport {
        detected_archetype: "Sem dados do Meta".to_string(),
        k_neighbors: 0,
        knn_mode_used: "balanced".to_string(),
        competitive_only_used: false,
        similar_decks: vec![],
        upgrades: vec![],
        cuts: vec![],
        stages: vec![],
        total_ur_cp: 0,
        total_sr_cp: 0,
        total_r_cp: 0,
        total_n_cp: 0,
        ur_deficit: 0,
        sr_deficit: 0,
        ur_dismantle_needed: 0,
        sr_dismantle_needed: 0,
        auto_optimize: AutoOptimizePreview {
            before_meta_score: 0.0,
            after_meta_score: 0.0,
            before_main_count: 0,
            after_main_count: 0,
            before_extra_count: 0,
            after_extra_count: 0,
            removed_changes: vec![],
            added_changes: vec![],
            optimized_main: vec![],
            optimized_extra: vec![],
            ur_cp_needed: 0,
            sr_cp_needed: 0,
            r_cp_needed: 0,
            n_cp_needed: 0,
        },
    }
}
