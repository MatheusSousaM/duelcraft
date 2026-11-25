mod auto_import;
mod knn_optimizer;
mod mdm_api;
mod models;
mod storage;

use crate::knn_optimizer::{detect_archetype_fast, optimize_user_deck_knn, MetaIndex};
use crate::mdm_api::MdmClient;
use crate::models::{
    catalog_md_sort_key, AutoImportSummary, CardSearchFilter, CatalogIndex, DeckCard,
    DeckOptimizationReport, DustBudget, GameWindowStatus, ImporterCalibration, MetaSnapshot,
    PackCardRef, PackRecommendation, SearchCardResult, UserDeck,
};
use crate::storage::AppStorage;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use tauri::{Manager, State};

struct MetaBundle {
    snap: MetaSnapshot,
    index: MetaIndex,
}

impl MetaBundle {
    fn new(snap: MetaSnapshot) -> Arc<Self> {
        let index = MetaIndex::build(&snap.decks);
        Arc::new(Self { snap, index })
    }
}

struct AppState {
    storage: AppStorage,
    mdm: MdmClient,
    user_decks: Mutex<Vec<UserDeck>>,
    budget: Mutex<DustBudget>,
    meta: Mutex<Option<Arc<MetaBundle>>>,
    deep_fetched_archetypes: Mutex<HashSet<String>>,
    cards_catalog: Mutex<Arc<CatalogIndex>>,
    caches_ready: (Mutex<bool>, Condvar),
    calibration: Mutex<ImporterCalibration>,
    import_cancel: Arc<AtomicBool>,
}

fn wait_for_disk_caches(state: &AppState) {
    let (lock, cvar) = &state.caches_ready;
    let mut ready = lock.lock().unwrap();
    while !*ready {
        ready = cvar.wait(ready).unwrap();
    }
}

async fn get_or_load_catalog(
    state: &AppState,
    force_refresh: bool,
) -> Result<Arc<CatalogIndex>, String> {
    wait_for_disk_caches(state);
    if !force_refresh {
        let cur = Arc::clone(&state.cards_catalog.lock().unwrap());
        if !cur.is_empty() {
            return Ok(cur);
        }
    }
    let raw_cards = state.mdm.fetch_all_cards_catalog().await?;
    state.storage.save_cards_cache(&raw_cards)?;
    let built = Arc::new(CatalogIndex::build(raw_cards));
    {
        let mut decks = state.user_decks.lock().unwrap();
        if !decks.is_empty() {
            for d in decks.iter_mut() {
                built.sort_deck(&mut d.main, &mut d.extra);
            }
            let _ = state.storage.save_user_decks(&decks);
        }
    }
    *state.cards_catalog.lock().unwrap() = Arc::clone(&built);
    Ok(built)
}

#[tauri::command]
fn get_meta_snapshot(state: State<AppState>) -> Option<MetaSnapshot> {
    wait_for_disk_caches(&state);
    state
        .meta
        .lock()
        .unwrap()
        .as_ref()
        .map(|b| b.snap.clone())
}

#[tauri::command]
async fn sync_meta_from_mdm(state: State<'_, AppState>) -> Result<MetaSnapshot, String> {
    wait_for_disk_caches(&state);
    let snapshot = state.mdm.sync_meta_snapshot().await?;
    state.storage.save_meta_cache(&snapshot)?;
    state.deep_fetched_archetypes.lock().unwrap().clear();
    let bundle = MetaBundle::new(snapshot.clone());
    *state.meta.lock().unwrap() = Some(bundle);
    Ok(snapshot)
}

async fn merge_archetype_decks_into_state(
    state: &AppState,
    query: &str,
) -> Result<Arc<MetaBundle>, String> {
    wait_for_disk_caches(state);
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return state
            .meta
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| "Nenhum meta sincronizado.".to_string());
    }

    let archetypes = {
        let meta_opt = state.meta.lock().unwrap();
        meta_opt
            .as_ref()
            .map(|b| b.snap.archetypes.clone())
            .unwrap_or_default()
    };

    let fetched = state.mdm.fetch_archetype_decks(trimmed, &archetypes).await?;

    {
        let mut done = state.deep_fetched_archetypes.lock().unwrap();
        done.insert(trimmed.to_lowercase());
        for d in &fetched {
            done.insert(d.deck_type.to_lowercase());
        }
    }

    let updated_snap = {
        let meta_opt = state.meta.lock().unwrap();
        let mut snap = meta_opt
            .as_ref()
            .map(|b| b.snap.clone())
            .unwrap_or_else(|| MetaSnapshot {
                last_synced: Utc::now().to_rfc3339(),
                archetypes: Vec::new(),
                decks: Vec::new(),
            });

        let mut index_by_id: HashMap<String, usize> = snap
            .decks
            .iter()
            .enumerate()
            .map(|(i, d)| (d.id.clone(), i))
            .collect();

        for d in fetched {
            if let Some(&idx) = index_by_id.get(&d.id) {
                snap.decks[idx] = d;
            } else {
                index_by_id.insert(d.id.clone(), snap.decks.len());
                snap.decks.push(d);
            }
        }
        snap
    };

    let _ = state.storage.save_meta_cache(&updated_snap);
    let bundle = MetaBundle::new(updated_snap);
    *state.meta.lock().unwrap() = Some(Arc::clone(&bundle));
    Ok(bundle)
}

#[tauri::command]
async fn load_archetype_decks(
    state: State<'_, AppState>,
    query: String,
) -> Result<MetaSnapshot, String> {
    let bundle = merge_archetype_decks_into_state(&state, &query).await?;
    Ok(bundle.snap.clone())
}

#[tauri::command]
fn get_dust_budget(state: State<AppState>) -> DustBudget {
    state.budget.lock().unwrap().clone()
}

#[tauri::command]
fn set_dust_budget(state: State<AppState>, budget: DustBudget) -> Result<DustBudget, String> {
    state.storage.save_dust_budget(&budget)?;
    *state.budget.lock().unwrap() = budget.clone();
    Ok(budget)
}

// ===== User Decks CRUD & KNN Optimizer =====

#[tauri::command]
fn get_user_decks(state: State<AppState>) -> Vec<UserDeck> {
    let catalog = Arc::clone(&state.cards_catalog.lock().unwrap());
    let mut decks = state.user_decks.lock().unwrap();
    if !catalog.is_empty() {
        for d in decks.iter_mut() {
            catalog.sort_deck(&mut d.main, &mut d.extra);
        }
    }
    decks.clone()
}

#[tauri::command]
fn save_user_deck(state: State<AppState>, mut deck: UserDeck) -> Result<Vec<UserDeck>, String> {
    deck.updated_at = Utc::now().to_rfc3339();
    {
        let catalog = Arc::clone(&state.cards_catalog.lock().unwrap());
        if !catalog.is_empty() {
            catalog.sort_deck(&mut deck.main, &mut deck.extra);
        }
    }
    let mut decks = state.user_decks.lock().unwrap();
    if let Some(existing) = decks.iter_mut().find(|d| d.id == deck.id) {
        *existing = deck;
    } else {
        decks.insert(0, deck);
    }
    state.storage.save_user_decks(&decks)?;
    Ok(decks.clone())
}

#[tauri::command]
fn delete_user_deck(state: State<AppState>, deck_id: String) -> Result<Vec<UserDeck>, String> {
    let mut decks = state.user_decks.lock().unwrap();
    decks.retain(|d| d.id != deck_id);
    state.storage.save_user_decks(&decks)?;
    Ok(decks.clone())
}

#[tauri::command]
fn clone_meta_deck_to_user(state: State<AppState>, meta_deck_id: String) -> Result<UserDeck, String> {
    wait_for_disk_caches(&state);
    let meta_deck = {
        let meta_opt = state.meta.lock().unwrap();
        let bundle = meta_opt
            .as_ref()
            .ok_or_else(|| "Nenhum meta sincronizado.".to_string())?;
        bundle
            .snap
            .decks
            .iter()
            .find(|d| d.id == meta_deck_id)
            .cloned()
            .ok_or_else(|| "Deck do meta não encontrado.".to_string())?
    };

    let mut new_deck = UserDeck {
        id: format!("deck-{}", Utc::now().timestamp_millis()),
        name: format!("Meu {}", meta_deck.deck_type),
        target_archetype: Some(meta_deck.deck_type.clone()),
        updated_at: Utc::now().to_rfc3339(),
        main: meta_deck.main,
        extra: meta_deck.extra,
    };

    {
        let catalog = Arc::clone(&state.cards_catalog.lock().unwrap());
        if !catalog.is_empty() {
            catalog.sort_deck(&mut new_deck.main, &mut new_deck.extra);
        }
    }

    let mut decks = state.user_decks.lock().unwrap();
    decks.insert(0, new_deck.clone());
    state.storage.save_user_decks(&decks)?;
    Ok(new_deck)
}

#[tauri::command]
async fn import_ydk_as_user_deck(
    state: State<'_, AppState>,
    name: String,
    ydk_content: String,
) -> Result<UserDeck, String> {
    let mut main_counts: HashMap<String, u32> = HashMap::new();
    let mut extra_counts: HashMap<String, u32> = HashMap::new();
    let mut section = "main";

    for line in ydk_content.lines() {
        let trimmed = line.trim();
        if trimmed == "#main" {
            section = "main";
            continue;
        }
        if trimmed == "#extra" {
            section = "extra";
            continue;
        }
        if trimmed == "!side" {
            section = "side";
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') || section == "side" {
            continue;
        }
        if trimmed.chars().all(|c| c.is_ascii_digit()) {
            let map = if section == "extra" {
                &mut extra_counts
            } else {
                &mut main_counts
            };
            let entry = map.entry(trimmed.to_string()).or_insert(0);
            *entry = (*entry + 1).min(3);
        }
    }

    let all_ids: Vec<String> = main_counts
        .keys()
        .chain(extra_counts.keys())
        .cloned()
        .collect();
    if all_ids.is_empty() {
        return Err("Nenhum ID válido encontrado no arquivo/texto .ydk.".to_string());
    }

    let resolved = state.mdm.resolve_ydk_cards(&all_ids).await?;
    let mut main: Vec<DeckCard> = Vec::new();
    let mut extra: Vec<DeckCard> = Vec::new();

    for (kid, qty) in main_counts {
        if let Some((id, cname, rarity, is_ex)) = resolved.get(&kid) {
            let card = DeckCard {
                id: id.clone(),
                name: cname.clone(),
                rarity: rarity.clone(),
                amount: qty,
                section: if *is_ex { "extra".to_string() } else { "main".to_string() },
            };
            if *is_ex {
                extra.push(card);
            } else {
                main.push(card);
            }
        }
    }
    for (kid, qty) in extra_counts {
        if let Some((id, cname, rarity, _)) = resolved.get(&kid) {
            extra.push(DeckCard {
                id: id.clone(),
                name: cname.clone(),
                rarity: rarity.clone(),
                amount: qty,
                section: "extra".to_string(),
            });
        }
    }

    let mut new_deck = UserDeck {
        id: format!("deck-{}", Utc::now().timestamp_millis()),
        name: if name.trim().is_empty() {
            "Deck Importado (.YDK)".to_string()
        } else {
            name.trim().to_string()
        },
        target_archetype: None,
        updated_at: Utc::now().to_rfc3339(),
        main,
        extra,
    };

    {
        let catalog = Arc::clone(&state.cards_catalog.lock().unwrap());
        if !catalog.is_empty() {
            catalog.sort_deck(&mut new_deck.main, &mut new_deck.extra);
        }
    }

    let mut decks = state.user_decks.lock().unwrap();
    decks.insert(0, new_deck.clone());
    state.storage.save_user_decks(&decks)?;
    Ok(new_deck)
}

#[tauri::command]
async fn analyze_user_deck_knn(
    state: State<'_, AppState>,
    deck_id: String,
    k_neighbors: Option<usize>,
    knn_mode: Option<String>,
) -> Result<DeckOptimizationReport, String> {
    wait_for_disk_caches(&state);
    let user_deck = {
        let decks = state.user_decks.lock().unwrap();
        decks
            .iter()
            .find(|d| d.id == deck_id)
            .cloned()
            .ok_or_else(|| "Seu deck não foi encontrado.".to_string())?
    };

    let mut bundle_opt = state.meta.lock().unwrap().clone();

    // Auto-fetch deep Ranked + Championship lists when a Rogue/specific archetype has few local decks
    if let Some(ref b) = bundle_opt {
        if let Some(arch_name) = detect_archetype_fast(&user_deck, &b.snap.decks, &b.index) {
            let arch_lower = arch_name.to_lowercase();
            let already_fetched = state
                .deep_fetched_archetypes
                .lock()
                .unwrap()
                .contains(&arch_lower);

            if !already_fetched {
                let matching_count = b
                    .snap
                    .decks
                    .iter()
                    .filter(|d| d.deck_type.eq_ignore_ascii_case(&arch_name))
                    .count();
                let tourney_count = b
                    .snap
                    .decks
                    .iter()
                    .filter(|d| {
                        d.deck_type.eq_ignore_ascii_case(&arch_name) && d.category == "tournament"
                    })
                    .count();

                if matching_count < 15 || tourney_count == 0 {
                    if let Ok(updated_bundle) =
                        merge_archetype_decks_into_state(&state, &arch_name).await
                    {
                        bundle_opt = Some(updated_bundle);
                    }
                }
            }
        }
    }

    let budget = state.budget.lock().unwrap().clone();

    let mut report = if let Some(b) = bundle_opt {
        optimize_user_deck_knn(
            &user_deck,
            &b.snap.decks,
            &b.index,
            &budget,
            k_neighbors.unwrap_or(8),
            knn_mode.as_deref().unwrap_or("balanced"),
        )
    } else {
        let empty_idx = MetaIndex::build(&[]);
        optimize_user_deck_knn(
            &user_deck,
            &[],
            &empty_idx,
            &budget,
            k_neighbors.unwrap_or(8),
            knn_mode.as_deref().unwrap_or("balanced"),
        )
    };

    {
        let catalog = Arc::clone(&state.cards_catalog.lock().unwrap());
        if !catalog.is_empty() {
            catalog.sort_deck(
                &mut report.auto_optimize.optimized_main,
                &mut report.auto_optimize.optimized_extra,
            );
        }
    }

    Ok(report)
}

#[tauri::command]
async fn get_deck_secret_packs(
    state: State<'_, AppState>,
    deck_id: String,
) -> Result<Vec<PackRecommendation>, String> {
    wait_for_disk_caches(&state);
    let cards: Vec<PackCardRef> = {
        let meta_opt = state.meta.lock().unwrap();
        let bundle = meta_opt
            .as_ref()
            .ok_or_else(|| "Nenhum meta sincronizado.".to_string())?;
        let deck = bundle
            .snap
            .decks
            .iter()
            .find(|d| d.id == deck_id)
            .ok_or_else(|| "Deck não encontrado.".to_string())?;
        deck.main
            .iter()
            .chain(deck.extra.iter())
            .map(|c| PackCardRef {
                id: c.id.clone(),
                name: c.name.clone(),
                rarity: c.rarity.clone(),
                missing: c.amount,
            })
            .collect()
    };

    state.mdm.get_pack_recommendations(&cards).await
}

#[tauri::command]
async fn get_cards_secret_packs(
    state: State<'_, AppState>,
    cards: Vec<PackCardRef>,
) -> Result<Vec<PackRecommendation>, String> {
    state.mdm.get_pack_recommendations(&cards).await
}

#[tauri::command]
async fn ensure_cards_catalog(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<usize, String> {
    let cat = get_or_load_catalog(&state, force_refresh.unwrap_or(false)).await?;
    Ok(cat.len())
}

#[tauri::command]
async fn sort_deck_lists(
    state: State<'_, AppState>,
    mut main: Vec<DeckCard>,
    mut extra: Vec<DeckCard>,
) -> Result<(Vec<DeckCard>, Vec<DeckCard>), String> {
    let cat = get_or_load_catalog(&state, false).await?;
    cat.sort_deck(&mut main, &mut extra);
    Ok((main, extra))
}

#[tauri::command]
async fn search_mdm_cards(
    state: State<'_, AppState>,
    filter: CardSearchFilter,
) -> Result<Vec<SearchCardResult>, String> {
    let cat = get_or_load_catalog(&state, false).await?;
    Ok(filter_local_catalog(&cat, &filter))
}

#[tauri::command]
async fn get_related_cards(
    state: State<'_, AppState>,
    card_id: String,
    card_name: Option<String>,
) -> Result<Vec<SearchCardResult>, String> {
    let cat = get_or_load_catalog(&state, false).await?;
    Ok(find_related_cards_in_catalog(
        &cat,
        &card_id,
        card_name.as_deref(),
    ))
}

fn filter_local_catalog(
    cat: &CatalogIndex,
    filter: &CardSearchFilter,
) -> Vec<SearchCardResult> {
    let q = filter
        .query
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());
    let ct = filter.card_type.as_deref().filter(|s| !s.is_empty());
    let mt = filter.monster_type.as_deref().filter(|s| !s.is_empty());
    let attr = filter.attribute.as_deref().filter(|s| !s.is_empty());
    let race = filter.race.as_deref().filter(|s| !s.is_empty());
    let rar = filter.rarity.as_deref().filter(|s| !s.is_empty());
    let limit = filter.limit.unwrap_or(80).clamp(10, 200) as usize;

    let mut scored: Vec<(u8, &SearchCardResult)> = Vec::new();

    for (idx, c) in cat.cards.iter().enumerate() {
        if let Some(target_ct) = ct {
            if c.card_type.as_deref() != Some(target_ct) {
                continue;
            }
        }
        if let Some(target_mt) = mt {
            if !c
                .monster_type
                .iter()
                .any(|m| m.eq_ignore_ascii_case(target_mt))
            {
                continue;
            }
        }
        if let Some(target_attr) = attr {
            if !c
                .attribute
                .as_deref()
                .map(|a| a.eq_ignore_ascii_case(target_attr))
                .unwrap_or(false)
            {
                continue;
            }
        }
        if let Some(target_race) = race {
            if !c
                .race
                .as_deref()
                .map(|r| r.eq_ignore_ascii_case(target_race))
                .unwrap_or(false)
            {
                continue;
            }
        }
        if let Some(target_lvl) = filter.level {
            if c.level != Some(target_lvl) {
                continue;
            }
        }
        if let Some(target_lr) = filter.link_rating {
            if c.link_rating != Some(target_lr) {
                continue;
            }
        }
        if let Some(target_rar) = rar {
            if !c.rarity.eq_ignore_ascii_case(target_rar) {
                continue;
            }
        }

        let rank = if let Some(ref query_str) = q {
            let name_lower = &cat.names_lower[idx];
            if name_lower == query_str {
                0
            } else if name_lower.starts_with(query_str.as_str())
                || name_lower.contains(query_str.as_str())
            {
                1
            } else if cat.descs_lower[idx].contains(query_str.as_str()) {
                2
            } else {
                continue;
            }
        } else {
            0
        };

        scored.push((rank, c));
    }

    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| catalog_md_sort_key(a.1).cmp(&catalog_md_sort_key(b.1)))
    });
    scored.into_iter().take(limit).map(|(_, c)| c.clone()).collect()
}

fn find_related_cards_in_catalog(
    cat: &CatalogIndex,
    card_id: &str,
    card_name_hint: Option<&str>,
) -> Vec<SearchCardResult> {
    let Some(target_idx) = cat.lookup_index(card_id, card_name_hint) else {
        return vec![];
    };
    let target_card = &cat.cards[target_idx];
    let target_name_lower = &cat.names_lower[target_idx];
    let target_quoted_self = format!("\"{}\"", target_name_lower);

    let mut quoted_terms: Vec<String> = Vec::new();
    if let Some(ref desc) = target_card.description {
        let mut inside = false;
        let mut buf = String::new();
        for ch in desc.chars() {
            if ch == '"' {
                if inside {
                    let term = buf.trim().to_lowercase();
                    if term.len() >= 3
                        && term != *target_name_lower
                        && !quoted_terms.contains(&term)
                    {
                        quoted_terms.push(term);
                    }
                    buf.clear();
                }
                inside = !inside;
            } else if inside {
                buf.push(ch);
            }
        }
    }

    if let Some((prefix, _)) = target_card.name.split_once(" - ") {
        let p = prefix.trim().to_lowercase();
        if p.len() >= 3 && !quoted_terms.contains(&p) {
            quoted_terms.push(p);
        }
    }

    let quoted_patterns: Vec<String> = quoted_terms.iter().map(|qt| format!("\"{}\"", qt)).collect();
    let mut scored: Vec<(u8, &SearchCardResult)> = Vec::new();

    for (idx, c) in cat.cards.iter().enumerate() {
        if idx == target_idx {
            continue;
        }
        let cname_lower = &cat.names_lower[idx];
        let desc_lower = &cat.descs_lower[idx];

        let mentions_target = desc_lower.contains(&target_quoted_self);
        let target_mentions_card = quoted_terms.iter().any(|qt| qt == cname_lower);
        if mentions_target || target_mentions_card {
            scored.push((0, c));
            continue;
        }

        if quoted_terms.iter().any(|qt| cname_lower.contains(qt.as_str())) {
            scored.push((1, c));
            continue;
        }

        if quoted_patterns.iter().any(|qp| desc_lower.contains(qp.as_str())) {
            scored.push((2, c));
        }
    }

    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| catalog_md_sort_key(a.1).cmp(&catalog_md_sort_key(b.1)))
    });

    scored.into_iter().take(60).map(|(_, c)| c.clone()).collect()
}

async fn build_ydk_content(
    state: &AppState,
    mut main: Vec<DeckCard>,
    mut extra: Vec<DeckCard>,
) -> Result<String, String> {
    wait_for_disk_caches(state);
    let mut id_to_konami: HashMap<String, String> = HashMap::new();
    let mut missing_cards: Vec<DeckCard> = Vec::new();

    {
        let cat = Arc::clone(&state.cards_catalog.lock().unwrap());
        if !cat.is_empty() {
            cat.sort_deck(&mut main, &mut extra);
            for card in main.iter().chain(extra.iter()) {
                if id_to_konami.contains_key(&card.id) {
                    continue;
                }
                if let Some(meta) = cat.lookup_by_id_or_name(&card.id, &card.name) {
                    if let Some(ref kid) = meta.konami_id {
                        if !kid.is_empty() {
                            id_to_konami.insert(card.id.clone(), kid.clone());
                            continue;
                        }
                    }
                }
                missing_cards.push(card.clone());
            }
        } else {
            missing_cards.extend(main.iter().cloned());
            missing_cards.extend(extra.iter().cloned());
        }
    }

    if !missing_cards.is_empty() {
        // Fallback to MDM API only if catalog wasn't loaded or lacked a konamiID
        return state.mdm.export_cards_to_ydk(&main, &extra).await;
    }

    let mut out = String::from("#created by DuelCraft (Master Duel Meta)\n#main\n");
    for card in &main {
        if let Some(kid) = id_to_konami.get(&card.id) {
            for _ in 0..card.amount {
                out.push_str(kid);
                out.push('\n');
            }
        }
    }
    out.push_str("#extra\n");
    for card in &extra {
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

#[cfg(windows)]
fn prompt_save_ydk_dialog(default_file_name: &str) -> Option<std::path::PathBuf> {
    #[repr(C)]
    #[allow(non_snake_case)]
    struct OPENFILENAMEW {
        lStructSize: u32,
        hwndOwner: isize,
        hInstance: isize,
        lpstrFilter: *const u16,
        lpstrCustomFilter: *mut u16,
        nMaxCustFilter: u32,
        nFilterIndex: u32,
        lpstrFile: *mut u16,
        nMaxFile: u32,
        lpstrFileTitle: *mut u16,
        nMaxFileTitle: u32,
        lpstrInitialDir: *const u16,
        lpstrTitle: *const u16,
        Flags: u32,
        nFileOffset: u16,
        nFileExtension: u16,
        lpstrDefExt: *const u16,
        lCustData: isize,
        lpfnHook: isize,
        lpTemplateName: *const u16,
        pvReserved: *mut core::ffi::c_void,
        dwReserved: u32,
        FlagsEx: u32,
    }

    #[link(name = "comdlg32")]
    extern "system" {
        fn GetSaveFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
    }

    let sanitized: String = default_file_name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect();
    let stem = sanitized.trim().trim_end_matches(".ydk");
    let initial = format!(
        "{}.ydk",
        if stem.is_empty() { "meu-deck" } else { stem }
    );

    let mut file_buf = [0u16; 520];
    for (i, u) in initial.encode_utf16().take(500).enumerate() {
        file_buf[i] = u;
    }

    let filter: Vec<u16> = "Yu-Gi-Oh! Deck (.ydk)\0*.ydk\0Todos os arquivos (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let title: Vec<u16> = "Exportar Deck como .YDK\0".encode_utf16().collect();
    let def_ext: Vec<u16> = "ydk\0".encode_utf16().collect();

    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: 0,
        hInstance: 0,
        lpstrFilter: filter.as_ptr(),
        lpstrCustomFilter: std::ptr::null_mut(),
        nMaxCustFilter: 0,
        nFilterIndex: 1,
        lpstrFile: file_buf.as_mut_ptr(),
        nMaxFile: file_buf.len() as u32,
        lpstrFileTitle: std::ptr::null_mut(),
        nMaxFileTitle: 0,
        lpstrInitialDir: std::ptr::null(),
        lpstrTitle: title.as_ptr(),
        // OFN_OVERWRITEPROMPT (0x2) | OFN_NOCHANGEDIR (0x8) | OFN_PATHMUSTEXIST (0x800)
        Flags: 0x0000_0002 | 0x0000_0008 | 0x0000_0800,
        nFileOffset: 0,
        nFileExtension: 0,
        lpstrDefExt: def_ext.as_ptr(),
        lCustData: 0,
        lpfnHook: 0,
        lpTemplateName: std::ptr::null(),
        pvReserved: std::ptr::null_mut(),
        dwReserved: 0,
        FlagsEx: 0,
    };

    let ok = unsafe { GetSaveFileNameW(&mut ofn) };
    if ok == 0 {
        return None;
    }
    let end = file_buf
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(file_buf.len());
    let path_str = String::from_utf16_lossy(&file_buf[..end]);
    if path_str.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(path_str))
    }
}

#[cfg(not(windows))]
fn prompt_save_ydk_dialog(_default_file_name: &str) -> Option<std::path::PathBuf> {
    None
}

#[tauri::command]
async fn export_deck_ydk(state: State<'_, AppState>, deck_id: String) -> Result<String, String> {
    wait_for_disk_caches(&state);
    let (main, extra) = {
        let meta_opt = state.meta.lock().unwrap();
        let bundle = meta_opt
            .as_ref()
            .ok_or_else(|| "Nenhum meta sincronizado.".to_string())?;
        let deck = bundle
            .snap
            .decks
            .iter()
            .find(|d| d.id == deck_id)
            .ok_or_else(|| "Deck não encontrado.".to_string())?;
        (deck.main.clone(), deck.extra.clone())
    };
    build_ydk_content(&state, main, extra).await
}

#[tauri::command]
async fn export_user_deck_ydk(
    state: State<'_, AppState>,
    deck_id: String,
) -> Result<String, String> {
    let (main, extra) = {
        let decks = state.user_decks.lock().unwrap();
        let deck = decks
            .iter()
            .find(|d| d.id == deck_id)
            .ok_or_else(|| "Deck não encontrado.".to_string())?;
        (deck.main.clone(), deck.extra.clone())
    };
    build_ydk_content(&state, main, extra).await
}

#[tauri::command]
async fn save_ydk_to_disk(
    suggested_name: String,
    ydk_content: String,
) -> Result<Option<String>, String> {
    let chosen = tokio::task::spawn_blocking(move || prompt_save_ydk_dialog(&suggested_name))
        .await
        .map_err(|e| format!("Erro ao abrir janela Salvar .YDK: {}", e))?;

    let Some(path) = chosen else {
        return Ok(None);
    };

    std::fs::write(&path, ydk_content)
        .map_err(|e| format!("Falha ao gravar arquivo .YDK: {}", e))?;

    Ok(Some(path.to_string_lossy().to_string()))
}

// ===== Auto-Importer (Untapped-style Deck Editor Macro) =====

#[tauri::command]
fn get_importer_calibration(state: State<AppState>) -> ImporterCalibration {
    state.calibration.lock().unwrap().clone()
}

#[tauri::command]
fn get_game_window_status(state: State<AppState>) -> GameWindowStatus {
    let cal = state.calibration.lock().unwrap().clone();
    auto_import::get_game_window_status(&cal)
}

#[tauri::command]
fn reset_importer_calibration(state: State<AppState>) -> Result<ImporterCalibration, String> {
    let def = ImporterCalibration {
        step_delay_ms: state.calibration.lock().unwrap().step_delay_ms,
        ..ImporterCalibration::default()
    };
    state.storage.save_calibration(&def)?;
    *state.calibration.lock().unwrap() = def.clone();
    Ok(def)
}

#[tauri::command]
fn save_importer_calibration(
    state: State<AppState>,
    calibration: ImporterCalibration,
) -> Result<ImporterCalibration, String> {
    state.storage.save_calibration(&calibration)?;
    *state.calibration.lock().unwrap() = calibration.clone();
    Ok(calibration)
}

#[tauri::command]
async fn capture_relative_point(
    state: State<'_, AppState>,
    delay_secs: u64,
) -> Result<(f64, f64), String> {
    let cal = state.calibration.lock().unwrap().clone();
    tokio::task::spawn_blocking(move || {
        auto_import::capture_relative_point_after_delay(delay_secs, &cal)
    })
    .await
    .map_err(|e| format!("Erro interno ao capturar ponto relativo: {}", e))?
}

#[tauri::command]
async fn test_importer_aim(state: State<'_, AppState>) -> Result<GameWindowStatus, String> {
    let cal = state.calibration.lock().unwrap().clone();
    tokio::task::spawn_blocking(move || auto_import::test_aim_points(&cal))
        .await
        .map_err(|e| format!("Erro interno no teste de mira: {}", e))?
}

#[tauri::command]
async fn run_auto_import_deck(
    state: State<'_, AppState>,
    deck_id: String,
) -> Result<AutoImportSummary, String> {
    wait_for_disk_caches(&state);
    let cards = {
        let meta_opt = state.meta.lock().unwrap();
        let bundle = meta_opt
            .as_ref()
            .ok_or_else(|| "Nenhum meta sincronizado.".to_string())?;
        let deck = bundle
            .snap
            .decks
            .iter()
            .find(|d| d.id == deck_id)
            .ok_or_else(|| "Deck não encontrado.".to_string())?;
        let mut list = deck.main.clone();
        list.extend(deck.extra.clone());
        list
    };

    let cat = get_or_load_catalog(&state, false).await.unwrap_or_default();
    let prepared = auto_import::prepare_smart_import_cards(&cards, &cat.cards);
    let cal = state.calibration.lock().unwrap().clone();
    let cancel_flag = Arc::clone(&state.import_cancel);

    tokio::task::spawn_blocking(move || auto_import::run_deck_auto_import(prepared, cal, cancel_flag))
        .await
        .map_err(|e| format!("Erro na thread de Auto-Import: {}", e))?
}

#[tauri::command]
async fn run_auto_import_user_deck(
    state: State<'_, AppState>,
    deck_id: String,
) -> Result<AutoImportSummary, String> {
    let cards = {
        let decks = state.user_decks.lock().unwrap();
        let deck = decks
            .iter()
            .find(|d| d.id == deck_id)
            .ok_or_else(|| "Seu deck não foi encontrado.".to_string())?;
        let mut list = deck.main.clone();
        list.extend(deck.extra.clone());
        list
    };

    let cat = get_or_load_catalog(&state, false).await.unwrap_or_default();
    let prepared = auto_import::prepare_smart_import_cards(&cards, &cat.cards);
    let cal = state.calibration.lock().unwrap().clone();
    let cancel_flag = Arc::clone(&state.import_cancel);

    tokio::task::spawn_blocking(move || auto_import::run_deck_auto_import(prepared, cal, cancel_flag))
        .await
        .map_err(|e| format!("Erro na thread de Auto-Import: {}", e))?
}

#[tauri::command]
fn cancel_auto_import(state: State<AppState>) {
    state.import_cancel.store(true, Ordering::SeqCst);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("Falha ao localizar diretório app_data_dir");
            let storage = AppStorage::new(data_dir).expect("Falha ao inicializar AppStorage");
            let mdm = MdmClient::new().expect("Falha ao inicializar MdmClient");

            // Load tiny user files (<15 KB) immediately so the window opens with zero delay!
            let user_decks = storage.load_user_decks();
            let budget = storage.load_dust_budget();
            let calibration = storage.load_calibration();

            app.manage(AppState {
                storage,
                mdm,
                user_decks: Mutex::new(user_decks),
                budget: Mutex::new(budget),
                meta: Mutex::new(None),
                deep_fetched_archetypes: Mutex::new(HashSet::new()),
                cards_catalog: Mutex::new(Arc::new(CatalogIndex::default())),
                caches_ready: (Mutex::new(false), Condvar::new()),
                calibration: Mutex::new(calibration),
                import_cancel: Arc::new(AtomicBool::new(false)),
            });

            // Load heavy caches (~10 MB) + build search/meta indices + purge old caches in background!
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let state = handle.state::<AppState>();
                state.storage.cleanup_legacy_caches();

                let meta_bundle = state.storage.load_meta_cache().map(MetaBundle::new);
                let cat_index = Arc::new(CatalogIndex::build(state.storage.load_cards_cache()));

                if !cat_index.is_empty() {
                    let mut decks = state.user_decks.lock().unwrap();
                    for d in decks.iter_mut() {
                        cat_index.sort_deck(&mut d.main, &mut d.extra);
                    }
                }

                *state.meta.lock().unwrap() = meta_bundle;
                *state.cards_catalog.lock().unwrap() = cat_index;

                let (lock, cvar) = &state.caches_ready;
                let mut ready = lock.lock().unwrap();
                *ready = true;
                cvar.notify_all();
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_meta_snapshot,
            sync_meta_from_mdm,
            load_archetype_decks,
            get_dust_budget,
            set_dust_budget,
            get_user_decks,
            save_user_deck,
            delete_user_deck,
            clone_meta_deck_to_user,
            import_ydk_as_user_deck,
            analyze_user_deck_knn,
            get_deck_secret_packs,
            get_cards_secret_packs,
            ensure_cards_catalog,
            sort_deck_lists,
            search_mdm_cards,
            get_related_cards,
            export_deck_ydk,
            export_user_deck_ydk,
            save_ydk_to_disk,
            get_importer_calibration,
            get_game_window_status,
            reset_importer_calibration,
            save_importer_calibration,
            capture_relative_point,
            test_importer_aim,
            run_auto_import_deck,
            run_auto_import_user_deck,
            cancel_auto_import,
        ])
        .run(tauri::generate_context!())
        .expect("Erro ao executar aplicação Tauri DuelCraft");
}
