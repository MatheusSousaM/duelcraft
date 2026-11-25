use crate::models::{DustBudget, ImporterCalibration, MetaSnapshot, SearchCardResult, UserDeck};
use std::fs;
use std::path::PathBuf;

pub struct AppStorage {
    base_dir: PathBuf,
}

impl AppStorage {
    pub fn new(base_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&base_dir)
            .map_err(|e| format!("Falha ao criar diretório de dados: {}", e))?;
        Ok(Self { base_dir })
    }

    fn user_decks_path(&self) -> PathBuf {
        self.base_dir.join("user_decks.json")
    }

    fn dust_path(&self) -> PathBuf {
        self.base_dir.join("dust_budget.json")
    }

    fn meta_path(&self) -> PathBuf {
        self.base_dir.join("meta_cache_v2.json")
    }

    fn cards_cache_path(&self) -> PathBuf {
        self.base_dir.join("cards_cache_v3.json")
    }

    pub fn load_cards_cache(&self) -> Vec<SearchCardResult> {
        let path = self.cards_cache_path();
        if !path.exists() {
            return vec![];
        }
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_cards_cache(&self, cards: &[SearchCardResult]) -> Result<(), String> {
        let json = serde_json::to_string(cards)
            .map_err(|e| format!("Erro ao serializar catálogo de cartas: {}", e))?;
        fs::write(self.cards_cache_path(), json)
            .map_err(|e| format!("Erro ao salvar catálogo de cartas: {}", e))
    }

    fn calibration_path(&self) -> PathBuf {
        self.base_dir.join("importer_calibration_v6.json")
    }

    pub fn load_user_decks(&self) -> Vec<UserDeck> {
        let path = self.user_decks_path();
        if !path.exists() {
            return vec![];
        }
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_user_decks(&self, decks: &[UserDeck]) -> Result<(), String> {
        let json = serde_json::to_string_pretty(decks)
            .map_err(|e| format!("Erro ao serializar seus decks: {}", e))?;
        fs::write(self.user_decks_path(), json)
            .map_err(|e| format!("Erro ao salvar seus decks: {}", e))
    }

    pub fn load_calibration(&self) -> ImporterCalibration {
        let path = self.calibration_path();
        if !path.exists() {
            return ImporterCalibration::default();
        }
        let mut cal: ImporterCalibration = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        if cal.step_delay_ms < 200 {
            cal.step_delay_ms = 700;
        }
        if cal.click_delay_ms < 100 {
            cal.click_delay_ms = 220;
        }
        cal
    }

    pub fn save_calibration(&self, cal: &ImporterCalibration) -> Result<(), String> {
        let json = serde_json::to_string_pretty(cal)
            .map_err(|e| format!("Erro ao serializar calibração: {}", e))?;
        fs::write(self.calibration_path(), json)
            .map_err(|e| format!("Erro ao salvar calibração: {}", e))
    }

    pub fn load_dust_budget(&self) -> DustBudget {
        let path = self.dust_path();
        if !path.exists() {
            return DustBudget::default();
        }
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_dust_budget(&self, budget: &DustBudget) -> Result<(), String> {
        let json = serde_json::to_string_pretty(budget)
            .map_err(|e| format!("Erro ao serializar pó CP: {}", e))?;
        fs::write(self.dust_path(), json)
            .map_err(|e| format!("Erro ao salvar pó CP: {}", e))
    }

    pub fn load_meta_cache(&self) -> Option<MetaSnapshot> {
        let path = self.meta_path();
        if !path.exists() {
            return None;
        }
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    pub fn save_meta_cache(&self, snapshot: &MetaSnapshot) -> Result<(), String> {
        let json = serde_json::to_string(snapshot)
            .map_err(|e| format!("Erro ao serializar cache do meta: {}", e))?;
        fs::write(self.meta_path(), json)
            .map_err(|e| format!("Erro ao salvar cache do meta: {}", e))
    }

    /// Removes obsolete v1/v2 cache files (~14 MB) left over from older app versions.
    pub fn cleanup_legacy_caches(&self) {
        const LEGACY_FILES: &[&str] = &[
            "cards_cache.json",
            "cards_cache_v2.json",
            "meta_cache.json",
            "importer_calibration.json",
        ];
        for name in LEGACY_FILES {
            let p = self.base_dir.join(name);
            if p.exists() {
                let _ = fs::remove_file(p);
            }
        }
    }
}
