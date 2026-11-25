export interface DeckCard {
  id: string;
  name: string;
  rarity: string;
  amount: number;
  section: "main" | "extra";
}

export interface MetaArchetype {
  id: string;
  name: string;
  coverCardId: string | null;
  tier: number;
  tierLabel: string;
  tournamentPower: number;
  popRank: number;
  avgUrPrice: number;
  avgSrPrice: number;
  deckCount: number;
}

export interface MetaDeck {
  id: string;
  deckType: string;
  deckTypeId: string;
  coverCardId: string | null;
  tournamentPower: number;
  tier: number;
  category: "master1" | "master5" | "tournament" | "duelist_cup" | "other";
  badgeLabel: string;
  placement: string | null;
  eventName: string | null;
  engines: string[];
  created: string;
  mdmUrl: string;
  urPrice: number;
  srPrice: number;
  main: DeckCard[];
  extra: DeckCard[];
}

export interface MetaSnapshot {
  lastSynced: string;
  archetypes: MetaArchetype[];
  decks: MetaDeck[];
}

export interface DustBudget {
  urCp: number;
  srCp: number;
  rCp: number;
  nCp: number;
}

export interface CardSearchFilter {
  query?: string;
  cardType?: string;
  monsterType?: string;
  attribute?: string;
  race?: string;
  level?: number;
  linkRating?: number;
  rarity?: string;
  limit?: number;
}

export interface SearchCardResult {
  id: string;
  name: string;
  rarity: string;
  konamiId: string | null;
  gameId?: number | null;
  ocgRelease?: string | null;
  cardType: string | null;
  monsterType: string[];
  attribute: string | null;
  race: string | null;
  level: number | null;
  linkRating: number | null;
  atk: number | null;
  def: number | null;
  description: string | null;
  isExtra: boolean;
}

export interface PackCardRef {
  id: string;
  name: string;
  rarity: string;
  missing: number;
}

export interface PackRecommendation {
  packName: string;
  packType: string;
  urMissingCopies: number;
  srMissingCopies: number;
  totalMissingCopies: number;
  cards: PackCardRef[];
}

export interface ImporterCalibration {
  searchRx: number;
  searchRy: number;
  firstCardRx: number;
  firstCardRy: number;
  stepDelayMs: number;
  clickDelayMs: number;
  customTuned: boolean;
}

export interface GameWindowStatus {
  found: boolean;
  minimized: boolean;
  title: string;
  clientX: number;
  clientY: number;
  clientWidth: number;
  clientHeight: number;
  searchX: number;
  searchY: number;
  firstCardX: number;
  firstCardY: number;
}

export interface AutoImportSummary {
  completed: boolean;
  cancelled: boolean;
  cardsProcessed: number;
  totalCards: number;
  message: string;
}

export interface UserDeck {
  id: string;
  name: string;
  targetArchetype: string | null;
  updatedAt: string;
  main: DeckCard[];
  extra: DeckCard[];
}

export interface SimilarDeckMatch {
  deckId: string;
  deckType: string;
  tier: number;
  category: string;
  badgeLabel: string;
  placement: string | null;
  engines: string[];
  similarityPct: number;
  sharedCards: number;
  coverCardId: string | null;
}

export interface UpgradeSuggestion {
  id: string;
  name: string;
  rarity: string;
  section: string;
  currentQty: number;
  recommendedQty: number;
  addQty: number;
  neighborUsagePct: number;
  globalMetaPct: number;
  cpCost: number;
  stage: number;
  stageLabel: string;
  priorityScore: number;
}

export interface CutSuggestion {
  id: string;
  name: string;
  rarity: string;
  section: string;
  currentQty: number;
  recommendedQty: number;
  neighborUsagePct: number;
  reason: string;
}

export interface StageCpSummary {
  stage: number;
  title: string;
  subtitle: string;
  urCp: number;
  srCp: number;
  rCp: number;
  nCp: number;
  cardCopies: number;
  canAffordWithBudget: boolean;
}

export interface AutoChangeItem {
  action: "add" | "remove";
  id: string;
  name: string;
  rarity: string;
  section: string;
  fromQty: number;
  toQty: number;
  delta: number;
  cpCost: number;
  neighborUsagePct: number;
  reason: string;
}

export interface AutoOptimizePreview {
  beforeMetaScore: number;
  afterMetaScore: number;
  beforeMainCount: number;
  afterMainCount: number;
  beforeExtraCount: number;
  afterExtraCount: number;
  removedChanges: AutoChangeItem[];
  addedChanges: AutoChangeItem[];
  optimizedMain: DeckCard[];
  optimizedExtra: DeckCard[];
  urCpNeeded: number;
  srCpNeeded: number;
  rCpNeeded: number;
  nCpNeeded: number;
}

export type KnnMode = "balanced" | "strict_top" | "pure_sim";

export interface DeckOptimizationReport {
  detectedArchetype: string;
  kNeighbors: number;
  knnModeUsed: string;
  competitiveOnlyUsed: boolean;
  similarDecks: SimilarDeckMatch[];
  upgrades: UpgradeSuggestion[];
  cuts: CutSuggestion[];
  stages: StageCpSummary[];
  totalUrCp: number;
  totalSrCp: number;
  totalRCp: number;
  totalNCp: number;
  urDeficit: number;
  srDeficit: number;
  urDismantleNeeded: number;
  srDismantleNeeded: number;
  autoOptimize: AutoOptimizePreview;
}

export function cardImageUrl(cardId: string | null | undefined, size: 200 | 260 | 420 = 260): string {
  if (!cardId) return "";
  return `https://s3.duellinksmeta.com/cards/${cardId}_w${size}.webp`;
}
