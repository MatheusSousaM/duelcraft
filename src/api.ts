import { invoke } from "@tauri-apps/api/core";
import type {
  AutoImportSummary,
  CardSearchFilter,
  DeckCard,
  DeckOptimizationReport,
  DustBudget,
  GameWindowStatus,
  ImporterCalibration,
  KnnMode,
  MetaSnapshot,
  PackCardRef,
  PackRecommendation,
  SearchCardResult,
  UserDeck,
} from "./types";

export const api = {
  getMetaSnapshot: () => invoke<MetaSnapshot | null>("get_meta_snapshot"),
  syncMetaFromMdm: () => invoke<MetaSnapshot>("sync_meta_from_mdm"),
  loadArchetypeDecks: (query: string) =>
    invoke<MetaSnapshot>("load_archetype_decks", { query }),
  getDustBudget: () => invoke<DustBudget>("get_dust_budget"),
  setDustBudget: (budget: DustBudget) => invoke<DustBudget>("set_dust_budget", { budget }),

  // User Decks & KNN Optimizer
  getUserDecks: () => invoke<UserDeck[]>("get_user_decks"),
  saveUserDeck: (deck: UserDeck) => invoke<UserDeck[]>("save_user_deck", { deck }),
  deleteUserDeck: (deckId: string) => invoke<UserDeck[]>("delete_user_deck", { deckId }),
  cloneMetaDeckToUser: (metaDeckId: string) =>
    invoke<UserDeck>("clone_meta_deck_to_user", { metaDeckId }),
  importYdkAsUserDeck: (name: string, ydkContent: string) =>
    invoke<UserDeck>("import_ydk_as_user_deck", { name, ydkContent }),
  analyzeUserDeckKnn: (deckId: string, kNeighbors = 8, knnMode: KnnMode = "balanced") =>
    invoke<DeckOptimizationReport>("analyze_user_deck_knn", {
      deckId,
      kNeighbors,
      knnMode,
    }),

  // Secret Packs & Cards
  getDeckSecretPacks: (deckId: string) =>
    invoke<PackRecommendation[]>("get_deck_secret_packs", { deckId }),
  getCardsSecretPacks: (cards: PackCardRef[]) =>
    invoke<PackRecommendation[]>("get_cards_secret_packs", { cards }),
  ensureCardsCatalog: (forceRefresh = false) =>
    invoke<number>("ensure_cards_catalog", { forceRefresh }),
  sortDeckLists: (main: DeckCard[], extra: DeckCard[]) =>
    invoke<[DeckCard[], DeckCard[]]>("sort_deck_lists", { main, extra }),
  searchMdmCards: (filter: CardSearchFilter) =>
    invoke<SearchCardResult[]>("search_mdm_cards", { filter }),
  getRelatedCards: (cardId: string, cardName?: string) =>
    invoke<SearchCardResult[]>("get_related_cards", { cardId, cardName: cardName ?? null }),
  exportDeckYdk: (deckId: string) => invoke<string>("export_deck_ydk", { deckId }),
  exportUserDeckYdk: (deckId: string) => invoke<string>("export_user_deck_ydk", { deckId }),
  saveYdkToDisk: (suggestedName: string, ydkContent: string) =>
    invoke<string | null>("save_ydk_to_disk", { suggestedName, ydkContent }),

  // Auto-Importer (Automatic Window & Multi-Monitor Detection)
  getImporterCalibration: () => invoke<ImporterCalibration>("get_importer_calibration"),
  getGameWindowStatus: () => invoke<GameWindowStatus>("get_game_window_status"),
  resetImporterCalibration: () => invoke<ImporterCalibration>("reset_importer_calibration"),
  saveImporterCalibration: (calibration: ImporterCalibration) =>
    invoke<ImporterCalibration>("save_importer_calibration", { calibration }),
  captureRelativePoint: (delaySecs: number) =>
    invoke<[number, number]>("capture_relative_point", { delaySecs }),
  testImporterAim: () => invoke<GameWindowStatus>("test_importer_aim"),
  runAutoImportDeck: (deckId: string) =>
    invoke<AutoImportSummary>("run_auto_import_deck", { deckId }),
  runAutoImportUserDeck: (deckId: string) =>
    invoke<AutoImportSummary>("run_auto_import_user_deck", { deckId }),
  cancelAutoImport: () => invoke<void>("cancel_auto_import"),
};
