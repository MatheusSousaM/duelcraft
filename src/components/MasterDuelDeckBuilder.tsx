import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  Filter,
  Link2,
  Minus,
  Plus,
  RotateCcw,
  Search,
  Sparkles,
  X,
} from "lucide-react";
import { api } from "../api";
import {
  cardImageUrl,
  type CardSearchFilter,
  type DeckCard,
  type MetaArchetype,
  type SearchCardResult,
  type UserDeck,
} from "../types";
import { RarityBadge } from "./Badges";

interface MasterDuelDeckBuilderProps {
  deck: UserDeck;
  archetypes: MetaArchetype[];
  onDeckChange: (next: UserDeck) => Promise<void>;
  onSetCardQty: (
    card: { id: string; name: string; rarity: string; section: "main" | "extra" },
    qty: number
  ) => Promise<void>;
}

const MONSTER_SUBTYPES = [
  { v: "Effect", l: "Efeito" },
  { v: "Normal", l: "Normal" },
  { v: "Tuner", l: "Regulador" },
  { v: "Fusion", l: "Fusão" },
  { v: "Synchro", l: "Sincro" },
  { v: "Xyz", l: "Xyz" },
  { v: "Link", l: "Link" },
  { v: "Ritual", l: "Ritual" },
  { v: "Pendulum", l: "Pêndulo" },
];

const ATTRIBUTES = [
  { v: "DARK", l: "TREVAS", color: "bg-purple-500/20 text-purple-300 border-purple-500/40" },
  { v: "LIGHT", l: "LUZ", color: "bg-amber-400/20 text-amber-200 border-amber-400/40" },
  { v: "FIRE", l: "FOGO", color: "bg-rose-500/20 text-rose-300 border-rose-500/40" },
  { v: "WATER", l: "ÁGUA", color: "bg-sky-500/20 text-sky-300 border-sky-500/40" },
  { v: "EARTH", l: "TERRA", color: "bg-orange-500/20 text-orange-300 border-orange-500/40" },
  { v: "WIND", l: "VENTO", color: "bg-emerald-500/20 text-emerald-300 border-emerald-500/40" },
  { v: "DIVINE", l: "DIVINO", color: "bg-yellow-500/20 text-yellow-200 border-yellow-500/40" },
];

const MONSTER_RACES = [
  "Aqua",
  "Beast",
  "Beast-Warrior",
  "Cyberse",
  "Dinosaur",
  "Divine-Beast",
  "Dragon",
  "Fairy",
  "Fiend",
  "Fish",
  "Illusion",
  "Insect",
  "Machine",
  "Plant",
  "Psychic",
  "Pyro",
  "Reptile",
  "Rock",
  "Sea Serpent",
  "Spellcaster",
  "Thunder",
  "Warrior",
  "Winged Beast",
  "Wyrm",
  "Zombie",
];

const SPELL_RACES = ["Normal", "Quick-Play", "Continuous", "Field", "Equip", "Ritual"];
const TRAP_RACES = ["Normal", "Continuous", "Counter"];

export const MasterDuelDeckBuilder: React.FC<MasterDuelDeckBuilderProps> = ({
  deck,
  archetypes,
  onDeckChange,
  onSetCardQty,
}) => {
  // Search & Master Duel filters
  const [query, setQuery] = useState("");
  const [cardType, setCardType] = useState<string>("");
  const [monsterType, setMonsterType] = useState<string>("");
  const [attribute, setAttribute] = useState<string>("");
  const [race, setRace] = useState<string>("");
  const [level, setLevel] = useState<number | undefined>(undefined);
  const [linkRating, setLinkRating] = useState<number | undefined>(undefined);
  const [rarity, setRarity] = useState<string>("");
  const [showFilters, setShowFilters] = useState(true);

  const [results, setResults] = useState<SearchCardResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [catalogTotal, setCatalogTotal] = useState<number>(0);
  const [loadingCatalog, setLoadingCatalog] = useState(false);
  const [relatedSource, setRelatedSource] = useState<string | null>(null);

  // Selected card for Left Inspector Panel
  const [inspected, setInspected] = useState<SearchCardResult | null>(null);

  // Load full Master Duel card catalog into local Rust cache once & align deck order
  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoadingCatalog(true);
      try {
        const count = await api.ensureCardsCatalog(false);
        if (!cancelled) {
          setCatalogTotal(count);
          if (deck.main.length > 0 || deck.extra.length > 0) {
            const [sortedMain, sortedExtra] = await api.sortDeckLists(deck.main, deck.extra);
            const changed =
              sortedMain.some((c, i) => c.id !== deck.main[i]?.id) ||
              sortedExtra.some((c, i) => c.id !== deck.extra[i]?.id);
            if (!cancelled && changed) {
              await onDeckChange({
                ...deck,
                main: sortedMain,
                extra: sortedExtra,
              });
            }
          }
        }
      } catch (err) {
        console.error("Erro ao carregar catálogo:", err);
      } finally {
        if (!cancelled) setLoadingCatalog(false);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [deck.id]);

  const runSearch = useCallback(
    async (overrides?: Partial<CardSearchFilter>) => {
      setSearching(true);
      try {
        const filter: CardSearchFilter = {
          query: overrides?.query !== undefined ? overrides.query : query || undefined,
          cardType: overrides?.cardType !== undefined ? overrides.cardType : cardType || undefined,
          monsterType:
            overrides?.monsterType !== undefined ? overrides.monsterType : monsterType || undefined,
          attribute:
            overrides?.attribute !== undefined ? overrides.attribute : attribute || undefined,
          race: overrides?.race !== undefined ? overrides.race : race || undefined,
          level: overrides && "level" in overrides ? overrides.level : level,
          linkRating: overrides && "linkRating" in overrides ? overrides.linkRating : linkRating,
          rarity: overrides?.rarity !== undefined ? overrides.rarity : rarity || undefined,
          limit: 80,
        };
        const res = await api.searchMdmCards(filter);
        setResults(res);
        setInspected((prev) => prev ?? res[0] ?? null);
      } catch (err) {
        console.error("Erro na busca de cartas:", err);
      } finally {
        setSearching(false);
      }
    },
    [query, cardType, monsterType, attribute, race, level, linkRating, rarity]
  );

  // Instant local search whenever text or filter changes
  useEffect(() => {
    if (relatedSource) return;
    const t = window.setTimeout(() => {
      runSearch();
    }, 90);
    return () => window.clearTimeout(t);
  }, [query, cardType, monsterType, attribute, race, level, linkRating, rarity, catalogTotal, relatedSource, runSearch]);

  const handleShowRelated = async (card: SearchCardResult) => {
    setSearching(true);
    setRelatedSource(card.name);
    try {
      const rel = await api.getRelatedCards(card.id, card.name);
      setResults(rel);
    } catch (err) {
      console.error("Erro ao buscar cartas relacionadas:", err);
    } finally {
      setSearching(false);
    }
  };

  const resetFilters = () => {
    setRelatedSource(null);
    setQuery("");
    setCardType("");
    setMonsterType("");
    setAttribute("");
    setRace("");
    setLevel(undefined);
    setLinkRating(undefined);
    setRarity("");
  };

  const activeFilterCount = [
    cardType,
    monsterType,
    attribute,
    race,
    level,
    linkRating,
    rarity,
  ].filter(Boolean).length;

  const qtyMap = useMemo(() => {
    const m: Record<string, number> = {};
    for (const c of deck.main) m[c.id] = c.amount;
    for (const c of deck.extra) m[c.id] = c.amount;
    return m;
  }, [deck]);

  const mainCount = deck.main.reduce((s, c) => s + c.amount, 0);
  const extraCount = deck.extra.reduce((s, c) => s + c.amount, 0);

  // Expand copies into individual visual slots (10-column Master Duel style grid!)
  const mainSlots = useMemo(() => {
    const slots: { card: DeckCard; copyIdx: number }[] = [];
    for (const card of deck.main) {
      for (let i = 0; i < card.amount; i++) {
        slots.push({ card, copyIdx: i });
      }
    }
    return slots;
  }, [deck.main]);

  const extraSlots = useMemo(() => {
    const slots: { card: DeckCard; copyIdx: number }[] = [];
    for (const card of deck.extra) {
      for (let i = 0; i < card.amount; i++) {
        slots.push({ card, copyIdx: i });
      }
    }
    return slots;
  }, [deck.extra]);

  const inspectDeckCard = async (c: DeckCard) => {
    const existing = results.find((r) => r.id === c.id);
    if (existing) {
      setInspected(existing);
      return;
    }
    // Show immediately with basic info, then enrich from API
    setInspected({
      id: c.id,
      name: c.name,
      rarity: c.rarity,
      konamiId: null,
      cardType: null,
      monsterType: [],
      attribute: null,
      race: null,
      level: null,
      linkRating: null,
      atk: null,
      def: null,
      description: "Carregando detalhes da carta...",
      isExtra: c.section === "extra",
    });
    try {
      const found = await api.searchMdmCards({ query: c.name, limit: 5 });
      const exact = found.find((f) => f.id === c.id) ?? found[0];
      if (exact) setInspected(exact);
    } catch {
      // keep basic preview
    }
  };

  const addOne = async (c: { id: string; name: string; rarity: string; isExtra: boolean }) => {
    const cur = qtyMap[c.id] ?? 0;
    if (cur >= 3) return;
    if (c.isExtra && extraCount >= 15) return;
    if (!c.isExtra && mainCount >= 60) return;
    await onSetCardQty(
      { id: c.id, name: c.name, rarity: c.rarity, section: c.isExtra ? "extra" : "main" },
      cur + 1
    );
  };

  const removeOne = async (c: DeckCard) => {
    const cur = qtyMap[c.id] ?? 0;
    if (cur <= 0) return;
    await onSetCardQty(
      { id: c.id, name: c.name, rarity: c.rarity, section: c.section },
      cur - 1
    );
  };

  const raceOptions =
    cardType === "Spell"
      ? SPELL_RACES
      : cardType === "Trap"
      ? TRAP_RACES
      : MONSTER_RACES;

  return (
    <div className="grid grid-cols-1 lg:grid-cols-12 gap-3 items-start">
      {/* ===== LEFT COLUMN: CARD INSPECTOR (Master Duel Left Panel) ===== */}
      <aside className="lg:col-span-3 bg-[#101622] border border-slate-800 rounded-xl p-3.5 space-y-3 lg:sticky lg:top-[76px]">
        {inspected ? (
          <>
            <div className="flex items-start justify-between gap-2">
              <h3 className="text-sm font-extrabold text-white leading-snug">
                {inspected.name}
              </h3>
              <RarityBadge rarity={inspected.rarity} />
            </div>

            <div className="flex justify-center bg-[#090c12] rounded-xl p-2 border border-slate-800/80">
              <img
                src={cardImageUrl(inspected.id, 420)}
                alt={inspected.name}
                className="w-44 h-auto rounded-lg shadow-xl object-contain"
              />
            </div>

            {/* Stats & Chips */}
            <div className="flex flex-wrap items-center gap-1.5 text-[11px]">
              {inspected.attribute && (
                <span className="px-2 py-0.5 rounded bg-indigo-500/20 text-indigo-200 border border-indigo-400/30 font-bold">
                  {inspected.attribute}
                </span>
              )}
              {inspected.linkRating ? (
                <span className="px-2 py-0.5 rounded bg-sky-500/20 text-sky-300 border border-sky-400/30 font-bold">
                  LINK-{inspected.linkRating}
                </span>
              ) : inspected.level ? (
                <span className="px-2 py-0.5 rounded bg-amber-500/20 text-amber-300 border border-amber-400/30 font-bold">
                  ★ {inspected.level}
                </span>
              ) : null}
              {inspected.race && (
                <span className="px-2 py-0.5 rounded bg-slate-800 text-slate-300 border border-slate-700 font-semibold">
                  {inspected.race}
                  {inspected.monsterType.length > 0
                    ? ` / ${inspected.monsterType.join(" / ")}`
                    : ""}
                </span>
              )}
            </div>

            {(inspected.atk !== null || inspected.def !== null) && (
              <div className="flex items-center gap-3 text-xs font-extrabold text-slate-200 bg-[#0b0e14] px-3 py-1.5 rounded-lg border border-slate-800">
                <span>ATK / {inspected.atk ?? "?"}</span>
                {inspected.linkRating ? (
                  <span className="text-sky-300">LINK-{inspected.linkRating}</span>
                ) : (
                  <span>DEF / {inspected.def ?? "?"}</span>
                )}
              </div>
            )}

            {/* Add / Remove Controls */}
            <div className="flex items-center justify-between gap-2 bg-[#0b0e14] p-2 rounded-lg border border-slate-800">
              <span className="text-xs text-slate-400">
                No deck:{" "}
                <strong className="text-amber-300 text-sm">
                  {qtyMap[inspected.id] ?? 0}x
                </strong>
              </span>
              <div className="flex items-center gap-1.5">
                <button
                  disabled={(qtyMap[inspected.id] ?? 0) === 0}
                  onClick={() =>
                    onSetCardQty(
                      {
                        id: inspected.id,
                        name: inspected.name,
                        rarity: inspected.rarity,
                        section: inspected.isExtra ? "extra" : "main",
                      },
                      (qtyMap[inspected.id] ?? 0) - 1
                    )
                  }
                  className="p-1.5 rounded-lg bg-rose-500/20 hover:bg-rose-500/30 border border-rose-500/40 text-rose-300 cursor-pointer disabled:opacity-30"
                >
                  <Minus className="w-3.5 h-3.5" />
                </button>
                <button
                  disabled={(qtyMap[inspected.id] ?? 0) >= 3}
                  onClick={() => addOne(inspected)}
                  className="px-3 py-1.5 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-slate-950 text-xs font-extrabold inline-flex items-center gap-1 cursor-pointer disabled:opacity-30"
                >
                  <Plus className="w-3.5 h-3.5" /> Adicionar
                </button>
              </div>
            </div>

            {/* Related Cards Button (Master Duel style!) */}
            <button
              onClick={() => handleShowRelated(inspected)}
              className="w-full py-1.5 px-3 rounded-lg bg-indigo-600/25 hover:bg-indigo-600/40 border border-indigo-500/40 text-indigo-200 text-xs font-bold inline-flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
            >
              <Link2 className="w-3.5 h-3.5" />
              Cartas Relacionadas
            </button>

            {inspected.description && (
              <div className="text-[11px] leading-relaxed text-slate-300 bg-[#0b0e14] p-2.5 rounded-lg border border-slate-800 max-h-40 overflow-y-auto whitespace-pre-line">
                {inspected.description}
              </div>
            )}
          </>
        ) : (
          <div className="py-16 text-center text-xs text-slate-500">
            Clique em qualquer carta para ver a arte, atributos e efeito aqui.
          </div>
        )}
      </aside>

      {/* ===== CENTER COLUMN: MAIN & EXTRA DECK VISUAL GRID (Master Duel Center) ===== */}
      <section className="lg:col-span-5 space-y-3">
        {/* Deck Header Controls */}
        <div className="bg-[#101622] border border-slate-800 rounded-xl p-3 flex flex-wrap items-center gap-2.5">
          <input
            type="text"
            value={deck.name}
            onChange={(e) => onDeckChange({ ...deck, name: e.target.value })}
            className="flex-1 min-w-[140px] px-2.5 py-1.5 rounded-lg bg-[#090c12] border border-slate-700 text-xs font-extrabold text-white focus:outline-none focus:border-amber-500"
          />
          <select
            value={deck.targetArchetype ?? ""}
            onChange={(e) =>
              onDeckChange({ ...deck, targetArchetype: e.target.value || null })
            }
            className="px-2.5 py-1.5 rounded-lg bg-[#090c12] border border-slate-700 text-xs font-semibold text-slate-200 focus:outline-none focus:border-amber-500"
          >
            <option value="">Arquétipo: Automático</option>
            {archetypes.map((a) => (
              <option key={a.id} value={a.name}>
                {a.name} ({a.tierLabel})
              </option>
            ))}
          </select>
        </div>

        {/* Main Deck Grid */}
        <div className="bg-[#0e1420] border border-slate-800 rounded-xl p-3">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <span className="text-xs font-black uppercase tracking-wider text-amber-300">
                Main Deck
              </span>
              <span
                className={`text-xs font-extrabold px-2 py-0.5 rounded ${
                  mainCount >= 40 && mainCount <= 60
                    ? "bg-emerald-500/20 text-emerald-300"
                    : "bg-amber-500/20 text-amber-300"
                }`}
              >
                {mainCount} / 60
              </span>
            </div>
            <span className="text-[10px] text-slate-500">
              Clique esq: detalhes • Clique dir: remover
            </span>
          </div>

          {mainSlots.length === 0 ? (
            <div className="py-12 text-center text-xs text-slate-500 border border-dashed border-slate-800 rounded-lg">
              Main Deck vazio — clique com o <strong>botão direito</strong> nas cartas à direita para adicionar!
            </div>
          ) : (
            <div className="grid grid-cols-5 sm:grid-cols-8 xl:grid-cols-10 gap-1">
              {mainSlots.map(({ card, copyIdx }) => (
                <button
                  key={`${card.id}-${copyIdx}`}
                  onClick={() => inspectDeckCard(card)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    removeOne(card);
                  }}
                  title={`${card.name} (Clique direito remove)`}
                  className="relative group rounded overflow-hidden border border-slate-800 hover:border-amber-400 transition-transform hover:scale-105 cursor-pointer bg-slate-900"
                >
                  <img
                    src={cardImageUrl(card.id, 200)}
                    alt={card.name}
                    loading="lazy"
                    className="w-full aspect-[421/614] object-cover block"
                  />
                  <span className="absolute top-0.5 right-0.5 scale-75 origin-top-right pointer-events-none">
                    <RarityBadge rarity={card.rarity} compact />
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Extra Deck Grid */}
        <div className="bg-[#111226] border border-indigo-500/30 rounded-xl p-3">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <span className="text-xs font-black uppercase tracking-wider text-indigo-300">
                Extra Deck
              </span>
              <span className="text-xs font-extrabold px-2 py-0.5 rounded bg-indigo-500/20 text-indigo-200">
                {extraCount} / 15
              </span>
            </div>
            <span className="text-[10px] text-slate-500">
              Fusão / Sincro / Xyz / Link
            </span>
          </div>

          {extraSlots.length === 0 ? (
            <div className="py-6 text-center text-xs text-slate-500 border border-dashed border-indigo-500/20 rounded-lg">
              Extra Deck vazio (0/15)
            </div>
          ) : (
            <div className="grid grid-cols-5 sm:grid-cols-8 xl:grid-cols-10 gap-1">
              {extraSlots.map(({ card, copyIdx }) => (
                <button
                  key={`${card.id}-${copyIdx}`}
                  onClick={() => inspectDeckCard(card)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    removeOne(card);
                  }}
                  title={`${card.name} (Clique direito remove)`}
                  className="relative group rounded overflow-hidden border border-indigo-900/80 hover:border-indigo-400 transition-transform hover:scale-105 cursor-pointer bg-slate-900"
                >
                  <img
                    src={cardImageUrl(card.id, 200)}
                    alt={card.name}
                    loading="lazy"
                    className="w-full aspect-[421/614] object-cover block"
                  />
                  <span className="absolute top-0.5 right-0.5 scale-75 origin-top-right pointer-events-none">
                    <RarityBadge rarity={card.rarity} compact />
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      </section>

      {/* ===== RIGHT COLUMN: CARD CATALOG + MASTER DUEL FILTERS ===== */}
      <aside className="lg:col-span-4 bg-[#101622] border border-slate-800 rounded-xl p-3 space-y-2.5">
        {/* Search Bar + Filter Toggle */}
        <div className="flex items-center gap-1.5">
          <div className="relative flex-1">
            <Search className="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              value={query}
              onChange={(e) => {
                if (relatedSource) setRelatedSource(null);
                setQuery(e.target.value);
              }}
              placeholder="Buscar carta por nome ou texto..."
              className="w-full pl-8 pr-2.5 py-1.5 rounded-lg bg-[#090c12] border border-slate-700 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-amber-500"
            />
          </div>
          <button
            onClick={() => setShowFilters((v) => !v)}
            className={`px-2.5 py-1.5 rounded-lg border text-xs font-bold inline-flex items-center gap-1 cursor-pointer ${
              showFilters || activeFilterCount > 0
                ? "bg-amber-500/20 border-amber-500/50 text-amber-300"
                : "bg-slate-800 border-slate-700 text-slate-300"
            }`}
          >
            <Filter className="w-3.5 h-3.5" />
            {activeFilterCount > 0 ? `(${activeFilterCount})` : "Filtros"}
          </button>
          {activeFilterCount > 0 && (
            <button
              onClick={resetFilters}
              title="Limpar filtros"
              className="p-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 cursor-pointer"
            >
              <RotateCcw className="w-3.5 h-3.5" />
            </button>
          )}
        </div>

        {/* Master Duel Style Filters Panel */}
        {showFilters && (
          <div className="bg-[#090c12] border border-slate-800 rounded-lg p-2.5 space-y-2 text-[11px]">
            {/* Card Type */}
            <div className="flex items-center gap-1 flex-wrap">
              <span className="text-slate-500 font-bold w-12">Tipo:</span>
              {[
                { v: "Monster", l: "Monstro", c: "bg-amber-500/20 text-amber-300 border-amber-500/50" },
                { v: "Spell", l: "Magia", c: "bg-emerald-500/20 text-emerald-300 border-emerald-500/50" },
                { v: "Trap", l: "Armadilha", c: "bg-fuchsia-500/20 text-fuchsia-300 border-fuchsia-500/50" },
              ].map((t) => (
                <button
                  key={t.v}
                  onClick={() => {
                    const next = cardType === t.v ? "" : t.v;
                    setCardType(next);
                    if (next !== "Monster") {
                      setMonsterType("");
                      setAttribute("");
                      setLevel(undefined);
                      setLinkRating(undefined);
                    }
                    setRace("");
                  }}
                  className={`px-2 py-0.5 rounded border font-bold cursor-pointer ${
                    cardType === t.v
                      ? t.c
                      : "bg-slate-900 border-slate-800 text-slate-400 hover:text-white"
                  }`}
                >
                  {t.l}
                </button>
              ))}
            </div>

            {/* Rarity */}
            <div className="flex items-center gap-1 flex-wrap">
              <span className="text-slate-500 font-bold w-12">Rar.:</span>
              {["UR", "SR", "R", "N"].map((r) => (
                <button
                  key={r}
                  onClick={() => setRarity(rarity === r ? "" : r)}
                  className={`px-2 py-0.5 rounded border font-bold cursor-pointer ${
                    rarity === r
                      ? "bg-amber-500 text-slate-950 border-amber-400"
                      : "bg-slate-900 border-slate-800 text-slate-400 hover:text-white"
                  }`}
                >
                  {r}
                </button>
              ))}
            </div>

            {/* Monster Subtypes */}
            {cardType !== "Spell" && cardType !== "Trap" && (
              <>
                <div className="flex items-center gap-1 flex-wrap">
                  <span className="text-slate-500 font-bold w-12">Subtipo:</span>
                  {MONSTER_SUBTYPES.map((st) => (
                    <button
                      key={st.v}
                      onClick={() => {
                        const next = monsterType === st.v ? "" : st.v;
                        setMonsterType(next);
                        if (next) setCardType("Monster");
                        if (next === "Link") setLevel(undefined);
                        else setLinkRating(undefined);
                      }}
                      className={`px-1.5 py-0.5 rounded border font-semibold cursor-pointer ${
                        monsterType === st.v
                          ? "bg-indigo-600 text-white border-indigo-400"
                          : "bg-slate-900 border-slate-800 text-slate-400 hover:text-white"
                      }`}
                    >
                      {st.l}
                    </button>
                  ))}
                </div>

                {/* Attribute */}
                <div className="flex items-center gap-1 flex-wrap">
                  <span className="text-slate-500 font-bold w-12">Atrib.:</span>
                  {ATTRIBUTES.map((a) => (
                    <button
                      key={a.v}
                      onClick={() => {
                        const next = attribute === a.v ? "" : a.v;
                        setAttribute(next);
                        if (next) setCardType("Monster");
                      }}
                      className={`px-1.5 py-0.5 rounded border font-bold cursor-pointer ${
                        attribute === a.v
                          ? a.color
                          : "bg-slate-900 border-slate-800 text-slate-400 hover:text-white"
                      }`}
                    >
                      {a.l}
                    </button>
                  ))}
                </div>

                {/* Level / Rank 1-12 */}
                {monsterType !== "Link" && (
                  <div className="flex items-center gap-1 flex-wrap">
                    <span className="text-slate-500 font-bold w-12">Nível:</span>
                    {[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].map((lv) => (
                      <button
                        key={lv}
                        onClick={() => {
                          const next = level === lv ? undefined : lv;
                          setLevel(next);
                          if (next) {
                            setCardType("Monster");
                            setLinkRating(undefined);
                          }
                        }}
                        className={`w-5 h-5 rounded border text-[10px] font-bold cursor-pointer ${
                          level === lv
                            ? "bg-amber-500 text-slate-950 border-amber-400"
                            : "bg-slate-900 border-slate-800 text-slate-400 hover:text-white"
                        }`}
                      >
                        {lv}
                      </button>
                    ))}
                  </div>
                )}

                {/* Link Rating 1-6 */}
                <div className="flex items-center gap-1 flex-wrap">
                  <span className="text-slate-500 font-bold w-12">Link:</span>
                  {[1, 2, 3, 4, 5, 6].map((lr) => (
                    <button
                      key={lr}
                      onClick={() => {
                        const next = linkRating === lr ? undefined : lr;
                        setLinkRating(next);
                        if (next) {
                          setCardType("Monster");
                          setMonsterType("Link");
                          setLevel(undefined);
                        }
                      }}
                      className={`px-2 h-5 rounded border text-[10px] font-bold cursor-pointer ${
                        linkRating === lr
                          ? "bg-sky-500 text-slate-950 border-sky-400"
                          : "bg-slate-900 border-slate-800 text-slate-400 hover:text-white"
                      }`}
                    >
                      L-{lr}
                    </button>
                  ))}
                </div>
              </>
            )}

            {/* Race / Spell-Trap Type */}
            <div className="flex items-center gap-1.5 pt-0.5">
              <span className="text-slate-500 font-bold w-12">
                {cardType === "Spell" || cardType === "Trap" ? "Ícone:" : "Raça:"}
              </span>
              <select
                value={race}
                onChange={(e) => setRace(e.target.value)}
                className="flex-1 bg-slate-900 border border-slate-800 rounded px-2 py-1 text-[11px] text-slate-200 focus:outline-none focus:border-amber-500"
              >
                <option value="">Todas</option>
                {raceOptions.map((r) => (
                  <option key={r} value={r}>
                    {r}
                  </option>
                ))}
              </select>
            </div>
          </div>
        )}

        {/* Related Cards Active Banner */}
        {relatedSource && (
          <div className="flex items-center justify-between gap-2 px-2.5 py-1.5 rounded-lg bg-indigo-500/20 border border-indigo-500/40 text-xs text-indigo-200">
            <span className="truncate">
              <Link2 className="w-3.5 h-3.5 inline mr-1" />
              Relacionadas: <strong>{relatedSource}</strong>
            </span>
            <button
              onClick={() => setRelatedSource(null)}
              className="p-0.5 rounded hover:bg-indigo-500/30 text-indigo-200 cursor-pointer shrink-0"
              title="Voltar para busca normal"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        {/* Catalog Grid Header */}
        <div className="flex items-center justify-between text-[11px] text-slate-400 px-0.5">
          <span>
            {loadingCatalog
              ? "Baixando catálogo completo (~13k cartas)..."
              : searching
              ? "Filtrando..."
              : `${results.length} carta(s)${catalogTotal > 0 ? ` de ${catalogTotal.toLocaleString("pt-BR")}` : ""}`}
          </span>
          <span className="text-slate-500">Clique dir: +1 no deck</span>
        </div>

        {/* Visual Card Catalog Grid (4 columns, Master Duel style!) */}
        <div className="grid grid-cols-4 gap-1.5 max-h-[540px] overflow-y-auto pr-1">
          {results.map((c) => {
            const qty = qtyMap[c.id] ?? 0;
            const isSelected = inspected?.id === c.id;
            return (
              <div
                key={c.id}
                onClick={() => setInspected(c)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setInspected(c);
                  addOne(c);
                }}
                title={`${c.name} (Clique esq: ver | Clique dir: adicionar)`}
                className={`relative group rounded-md overflow-hidden border transition-all cursor-pointer bg-slate-900 ${
                  isSelected
                    ? "border-amber-400 ring-1 ring-amber-400"
                    : "border-slate-800 hover:border-slate-500"
                }`}
              >
                <img
                  src={cardImageUrl(c.id, 200)}
                  alt={c.name}
                  loading="lazy"
                  className="w-full aspect-[421/614] object-cover block"
                />
                <span className="absolute top-0.5 left-0.5 scale-75 origin-top-left pointer-events-none">
                  <RarityBadge rarity={c.rarity} compact />
                </span>
                {qty > 0 && (
                  <span className="absolute top-0.5 right-0.5 px-1.5 py-0.5 rounded bg-amber-500 text-slate-950 text-[10px] font-black leading-none shadow">
                    {qty}
                  </span>
                )}
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setInspected(c);
                    addOne(c);
                  }}
                  title="Adicionar +1"
                  className="opacity-0 group-hover:opacity-100 absolute bottom-1 right-1 w-6 h-6 rounded-full bg-emerald-500 hover:bg-emerald-400 text-slate-950 flex items-center justify-center shadow-lg transition-opacity cursor-pointer"
                >
                  <Plus className="w-3.5 h-3.5" />
                </button>
              </div>
            );
          })}
          {!searching && results.length === 0 && (
            <div className="col-span-4 py-12 text-center text-xs text-slate-500">
              <Sparkles className="w-6 h-6 mx-auto mb-1.5 text-slate-700" />
              Nenhuma carta encontrada com esses filtros.
            </div>
          )}
        </div>
      </aside>
    </div>
  );
};
