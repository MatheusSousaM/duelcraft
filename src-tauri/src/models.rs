use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckCard {
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub amount: u32,
    pub section: String, // "main" | "extra"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaArchetype {
    pub id: String,
    pub name: String,
    pub cover_card_id: Option<String>,
    pub tier: u8, // 1, 2, 3, or 4 (Rogue)
    pub tier_label: String,
    pub tournament_power: f64,
    pub pop_rank: f64,
    pub avg_ur_price: u32,
    pub avg_sr_price: u32,
    pub deck_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaDeck {
    pub id: String,
    pub deck_type: String,
    pub deck_type_id: String,
    pub cover_card_id: Option<String>,
    pub tournament_power: f64,
    pub tier: u8,
    pub category: String, // "master1" | "master5" | "tournament" | "duelist_cup" | "other"
    #[serde(default)]
    pub badge_label: String, // e.g. "Top 4 • Shogun CS", "1st Place", "Master I", "Master V"
    pub placement: Option<String>,
    pub event_name: Option<String>,
    pub engines: Vec<String>,
    pub created: String,
    pub mdm_url: String,
    pub ur_price: u32,
    pub sr_price: u32,
    pub main: Vec<DeckCard>,
    pub extra: Vec<DeckCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetaSnapshot {
    pub last_synced: String,
    pub archetypes: Vec<MetaArchetype>,
    pub decks: Vec<MetaDeck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DustBudget {
    pub ur_cp: u32,
    pub sr_cp: u32,
    pub r_cp: u32,
    pub n_cp: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CardSearchFilter {
    pub query: Option<String>,
    pub card_type: Option<String>,    // "Monster" | "Spell" | "Trap"
    pub monster_type: Option<String>, // "Fusion" | "Synchro" | "Xyz" | "Link" | "Ritual" | "Pendulum" | "Tuner" | "Effect" | "Normal"
    pub attribute: Option<String>,    // "DARK" | "LIGHT" | "FIRE" | "WATER" | "EARTH" | "WIND" | "DIVINE"
    pub race: Option<String>,         // "Dragon", "Spellcaster", "Quick-Play", "Continuous", etc.
    pub level: Option<u32>,           // 1..=12
    pub link_rating: Option<u32>,     // 1..=6
    pub rarity: Option<String>,       // "UR" | "SR" | "R" | "N"
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCardResult {
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub konami_id: Option<String>,
    #[serde(default)]
    pub game_id: Option<u32>,
    #[serde(default)]
    pub ocg_release: Option<String>,
    pub card_type: Option<String>,
    pub monster_type: Vec<String>,
    pub attribute: Option<String>,
    pub race: Option<String>,
    pub level: Option<u32>,
    pub link_rating: Option<u32>,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub description: Option<String>,
    pub is_extra: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackCardRef {
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub missing: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackRecommendation {
    pub pack_name: String,
    pub pack_type: String,
    pub ur_missing_copies: u32,
    pub sr_missing_copies: u32,
    pub total_missing_copies: u32,
    pub cards: Vec<PackCardRef>,
}

// Exact 16:9 normalized coordinates measured on the live 2026 Master Duel Deck Editor:
// - "Search by Card Name" input box: (0.7781, 0.1911)
// - "Show Unowned Cards" toggle pill (Green = ON, Black = OFF): (0.9294, 0.1911)
// - First Card in catalog grid (Row 1, Col 1): (0.7050, 0.3500)
pub const DEFAULT_SEARCH_RX: f64 = 0.7781;
pub const DEFAULT_SEARCH_RY: f64 = 0.1911;
pub const DEFAULT_FIRST_CARD_RX: f64 = 0.7050;
pub const DEFAULT_FIRST_CARD_RY: f64 = 0.3500;
pub const UNOWNED_TOGGLE_RX: f64 = 0.9294;
pub const UNOWNED_TOGGLE_RY: f64 = 0.1911;
pub const DEFAULT_STEP_DELAY_MS: u64 = 700;
pub const DEFAULT_CLICK_DELAY_MS: u64 = 220;

fn default_search_rx() -> f64 {
    DEFAULT_SEARCH_RX
}
fn default_search_ry() -> f64 {
    DEFAULT_SEARCH_RY
}
fn default_first_card_rx() -> f64 {
    DEFAULT_FIRST_CARD_RX
}
fn default_first_card_ry() -> f64 {
    DEFAULT_FIRST_CARD_RY
}
fn default_step_delay_ms() -> u64 {
    DEFAULT_STEP_DELAY_MS
}
fn default_click_delay_ms() -> u64 {
    DEFAULT_CLICK_DELAY_MS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImporterCalibration {
    #[serde(default = "default_search_rx")]
    pub search_rx: f64,
    #[serde(default = "default_search_ry")]
    pub search_ry: f64,
    #[serde(default = "default_first_card_rx")]
    pub first_card_rx: f64,
    #[serde(default = "default_first_card_ry")]
    pub first_card_ry: f64,
    #[serde(default = "default_step_delay_ms")]
    pub step_delay_ms: u64,
    #[serde(default = "default_click_delay_ms")]
    pub click_delay_ms: u64,
    #[serde(default)]
    pub custom_tuned: bool,
}

impl Default for ImporterCalibration {
    fn default() -> Self {
        Self {
            search_rx: DEFAULT_SEARCH_RX,
            search_ry: DEFAULT_SEARCH_RY,
            first_card_rx: DEFAULT_FIRST_CARD_RX,
            first_card_ry: DEFAULT_FIRST_CARD_RY,
            step_delay_ms: DEFAULT_STEP_DELAY_MS,
            click_delay_ms: DEFAULT_CLICK_DELAY_MS,
            custom_tuned: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameWindowStatus {
    pub found: bool,
    pub minimized: bool,
    pub title: String,
    pub client_x: i32,
    pub client_y: i32,
    pub client_width: i32,
    pub client_height: i32,
    pub search_x: i32,
    pub search_y: i32,
    pub first_card_x: i32,
    pub first_card_y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoImportSummary {
    pub completed: bool,
    pub cancelled: bool,
    pub cards_processed: usize,
    pub total_cards: usize,
    pub message: String,
}

// ===== User Decks & KNN TF-IDF Optimizer =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDeck {
    pub id: String,
    pub name: String,
    pub target_archetype: Option<String>,
    pub updated_at: String,
    pub main: Vec<DeckCard>,
    pub extra: Vec<DeckCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarDeckMatch {
    pub deck_id: String,
    pub deck_type: String,
    pub tier: u8,
    pub category: String,
    pub badge_label: String,
    pub placement: Option<String>,
    pub engines: Vec<String>,
    pub similarity_pct: f64,
    pub shared_cards: u32,
    pub cover_card_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeSuggestion {
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub section: String,
    pub current_qty: u32,
    pub recommended_qty: u32,
    pub add_qty: u32,
    pub neighbor_usage_pct: f64,
    pub global_meta_pct: f64,
    pub cp_cost: u32,
    pub stage: u8, // 1 = Cheap (N/R/SR), 2 = High-Impact UR, 3 = Tech/Refinement
    pub stage_label: String,
    pub priority_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CutSuggestion {
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub section: String,
    pub current_qty: u32,
    pub recommended_qty: u32,
    pub neighbor_usage_pct: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageCpSummary {
    pub stage: u8,
    pub title: String,
    pub subtitle: String,
    pub ur_cp: u32,
    pub sr_cp: u32,
    pub r_cp: u32,
    pub n_cp: u32,
    pub card_copies: u32,
    pub can_afford_with_budget: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoChangeItem {
    pub action: String, // "add" | "remove"
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub section: String,
    pub from_qty: u32,
    pub to_qty: u32,
    pub delta: u32,
    pub cp_cost: u32,
    pub neighbor_usage_pct: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoOptimizePreview {
    pub before_meta_score: f64,
    pub after_meta_score: f64,
    pub before_main_count: u32,
    pub after_main_count: u32,
    pub before_extra_count: u32,
    pub after_extra_count: u32,
    pub removed_changes: Vec<AutoChangeItem>,
    pub added_changes: Vec<AutoChangeItem>,
    pub optimized_main: Vec<DeckCard>,
    pub optimized_extra: Vec<DeckCard>,
    pub ur_cp_needed: u32,
    pub sr_cp_needed: u32,
    pub r_cp_needed: u32,
    pub n_cp_needed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckOptimizationReport {
    pub detected_archetype: String,
    pub k_neighbors: usize,
    pub knn_mode_used: String,
    pub competitive_only_used: bool,
    pub similar_decks: Vec<SimilarDeckMatch>,
    pub upgrades: Vec<UpgradeSuggestion>,
    pub cuts: Vec<CutSuggestion>,
    pub stages: Vec<StageCpSummary>,
    pub total_ur_cp: u32,
    pub total_sr_cp: u32,
    pub total_r_cp: u32,
    pub total_n_cp: u32,
    pub ur_deficit: u32,
    pub sr_deficit: u32,
    pub ur_dismantle_needed: u32,
    pub sr_dismantle_needed: u32,
    pub auto_optimize: AutoOptimizePreview,
}

pub fn tier_from_power(explicit_tier: Option<u8>, power: f64) -> (u8, String) {
    if let Some(t) = explicit_tier {
        if (1..=3).contains(&t) {
            return (t, format!("Tier {}", t));
        }
    }
    if power >= 10.0 {
        (1, "Tier 1".to_string())
    } else if power >= 6.0 {
        (2, "Tier 2".to_string())
    } else if power >= 2.5 {
        (3, "Tier 3".to_string())
    } else {
        (4, "Rogue".to_string())
    }
}

/// Pre-indexed Master Duel card catalog built ONCE when the 14,000-card catalog loads.
/// Eliminates rebuilding 14k HashMaps on every deck edit/sort and eliminates `.to_lowercase()`
/// allocations on 14k names & descriptions on every search keystroke!
#[derive(Debug, Default)]
pub struct CatalogIndex {
    pub cards: Vec<SearchCardResult>,
    pub names_lower: Vec<String>,
    pub descs_lower: Vec<String>,
    by_id: std::collections::HashMap<String, usize>,
    by_name_lower: std::collections::HashMap<String, usize>,
}

impl CatalogIndex {
    pub fn build(cards: Vec<SearchCardResult>) -> Self {
        let len = cards.len();
        let mut names_lower = Vec::with_capacity(len);
        let mut descs_lower = Vec::with_capacity(len);
        let mut by_id = std::collections::HashMap::with_capacity(len);
        let mut by_name_lower = std::collections::HashMap::with_capacity(len);

        for (idx, c) in cards.iter().enumerate() {
            let nl = c.name.to_lowercase();
            let dl = c
                .description
                .as_deref()
                .map(|d| d.to_lowercase())
                .unwrap_or_default();
            by_id.insert(c.id.clone(), idx);
            by_name_lower.insert(nl.clone(), idx);
            names_lower.push(nl);
            descs_lower.push(dl);
        }

        Self {
            cards,
            names_lower,
            descs_lower,
            by_id,
            by_name_lower,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn lookup_by_id_or_name(&self, id: &str, name: &str) -> Option<&SearchCardResult> {
        if let Some(&idx) = self.by_id.get(id) {
            return self.cards.get(idx);
        }
        let nl = name.to_lowercase();
        self.by_name_lower
            .get(&nl)
            .and_then(|&idx| self.cards.get(idx))
    }

    pub fn lookup_index(&self, id: &str, name_hint: Option<&str>) -> Option<usize> {
        if let Some(&idx) = self.by_id.get(id) {
            return Some(idx);
        }
        name_hint.and_then(|n| self.by_name_lower.get(&n.to_lowercase()).copied())
    }

    pub fn sort_deck(&self, main: &mut [DeckCard], extra: &mut [DeckCard]) {
        if self.cards.is_empty() {
            return;
        }
        main.sort_by(|a, b| {
            let ma = self.lookup_by_id_or_name(&a.id, &a.name);
            let mb = self.lookup_by_id_or_name(&b.id, &b.name);
            main_md_sort_key(ma, &a.name).cmp(&main_md_sort_key(mb, &b.name))
        });
        extra.sort_by(|a, b| {
            let ma = self.lookup_by_id_or_name(&a.id, &a.name);
            let mb = self.lookup_by_id_or_name(&b.id, &b.name);
            extra_md_sort_key(ma, &a.name).cmp(&extra_md_sort_key(mb, &b.name))
        });
    }
}

pub fn main_md_sort_key<'a>(
    meta: Option<&'a SearchCardResult>,
    fallback_name: &str,
) -> (u8, u8, u32, &'a str, u32, String) {
    let Some(m) = meta else {
        return (3, 9, 99, "9999-99-99", u32::MAX, fallback_name.to_lowercase());
    };

    let has_mt = |t: &str| m.monster_type.iter().any(|x| x.eq_ignore_ascii_case(t));
    let ct = m.card_type.as_deref().unwrap_or("");

    let (type_rank, sub_rank, lvl) = match ct {
        "Monster" => {
            let is_pend = has_mt("Pendulum");
            let is_rit = has_mt("Ritual");
            let is_norm = has_mt("Normal");
            let sub = if !is_pend && !is_rit && is_norm {
                0
            } else if !is_pend && !is_rit {
                1
            } else if !is_pend && is_rit && is_norm {
                2
            } else if !is_pend && is_rit {
                3
            } else if is_pend && !is_rit && is_norm {
                4
            } else if is_pend && !is_rit {
                5
            } else {
                6
            };
            (0, sub, m.level.unwrap_or(0))
        }
        "Spell" => {
            let sub = match m.race.as_deref().unwrap_or("Normal") {
                "Normal" => 0,
                "Equip" => 1,
                "Field" => 2,
                "Ritual" => 3,
                "Quick-Play" => 4,
                "Continuous" => 5,
                _ => 6,
            };
            (1, sub, 0)
        }
        "Trap" => {
            let sub = match m.race.as_deref().unwrap_or("Normal") {
                "Normal" => 0,
                "Counter" => 1,
                "Continuous" => 2,
                _ => 3,
            };
            (2, sub, 0)
        }
        _ => (3, 9, 99),
    };

    let ocg = m.ocg_release.as_deref().unwrap_or("9999-99-99");
    let gid = m.game_id.unwrap_or(u32::MAX);
    (type_rank, sub_rank, lvl, ocg, gid, fallback_name.to_lowercase())
}

pub fn extra_md_sort_key<'a>(
    meta: Option<&'a SearchCardResult>,
    fallback_name: &str,
) -> (u8, u32, &'a str, u32, String) {
    let Some(m) = meta else {
        return (9, 99, "9999-99-99", u32::MAX, fallback_name.to_lowercase());
    };

    let has_mt = |t: &str| m.monster_type.iter().any(|x| x.eq_ignore_ascii_case(t));
    let is_pend = has_mt("Pendulum");

    let mech_rank = if has_mt("Fusion") {
        if is_pend { 1 } else { 0 }
    } else if has_mt("Synchro") {
        if is_pend { 3 } else { 2 }
    } else if has_mt("Xyz") {
        if is_pend { 5 } else { 4 }
    } else if has_mt("Link") {
        6
    } else {
        8
    };

    let rating = if has_mt("Link") {
        m.link_rating.unwrap_or(0)
    } else {
        m.level.unwrap_or(0)
    };

    let ocg = m.ocg_release.as_deref().unwrap_or("9999-99-99");
    let gid = m.game_id.unwrap_or(u32::MAX);
    (mech_rank, rating, ocg, gid, fallback_name.to_lowercase())
}

/// Unified Master Duel Card List ("Sort by Default") key:
/// Main Deck Monsters -> Extra Deck Monsters -> Spells -> Traps,
/// with exact internal Master Duel subtype + Level/Rank/Link + OCG release / gameId order.
pub fn catalog_md_sort_key<'a>(m: &'a SearchCardResult) -> (u8, u8, u32, &'a str, u32, &'a str) {
    if m.is_extra {
        let (mech, rating, ocg, gid, _) = extra_md_sort_key(Some(m), &m.name);
        (1, mech, rating, ocg, gid, m.name.as_str())
    } else {
        let (tr, sub, lvl, ocg, gid, _) = main_md_sort_key(Some(m), &m.name);
        let group = match tr {
            0 => 0, // Main Monster
            1 => 2, // Spell (after Extra Deck Monsters)
            2 => 3, // Trap
            _ => 4,
        };
        (group, sub, lvl, ocg, gid, m.name.as_str())
    }
}
