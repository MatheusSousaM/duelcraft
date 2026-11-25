import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  BrainCircuit,
  DownloadCloud,
  Flame,
  Layers,
  RefreshCw,
  Search,
  Sparkles,
  Trophy,
} from "lucide-react";
import { api } from "./api";
import { DustPill, RarityBadge, TierBadge } from "./components/Badges";
import { DeckModal } from "./components/DeckModal";
import { MyDecksView } from "./components/MyDecksView";
import { TierListView } from "./components/TierListView";
import {
  cardImageUrl,
  type DustBudget,
  type MetaDeck,
  type MetaSnapshot,
  type UserDeck,
} from "./types";

type TabId = "mydecks" | "meta" | "tierlist";
type SortBy = "recent" | "power" | "cheapest";
type CategoryFilter = "ALL" | "tournament" | "master1" | "duelist_cup" | "master5";
type PeriodFilter = "ALL" | "7d" | "30d";

const CATEGORY_LABELS: Record<string, string> = {
  master1: "Master I",
  master5: "Master V",
  tournament: "Campeonato",
  duelist_cup: "Duelist Cup",
  other: "Top Deck",
};

function deckBadgeClass(category: string): string {
  switch (category) {
    case "tournament":
      return "bg-amber-500/20 text-amber-300 border-amber-400/35";
    case "duelist_cup":
      return "bg-cyan-500/20 text-cyan-300 border-cyan-400/35";
    case "master1":
      return "bg-fuchsia-500/20 text-fuchsia-200 border-fuchsia-400/35";
    case "master5":
      return "bg-indigo-500/20 text-indigo-300 border-indigo-400/30";
    default:
      return "bg-slate-700/40 text-slate-300 border-slate-600/40";
  }
}

function getDeckBadgeText(d: MetaDeck): string {
  if (d.badgeLabel && d.badgeLabel.trim().length > 0) return d.badgeLabel;
  if (d.placement) return d.placement;
  return CATEGORY_LABELS[d.category] || "Top Deck";
}

export function App() {
  const [tab, setTab] = useState<TabId>("mydecks");
  const [meta, setMeta] = useState<MetaSnapshot | null>(null);
  const [userDecks, setUserDecks] = useState<UserDeck[]>([]);
  const [activeUserDeckId, setActiveUserDeckId] = useState<string | null>(null);
  const [budget, setBudget] = useState<DustBudget>({
    urCp: 0,
    srCp: 0,
    rCp: 0,
    nCp: 0,
  });
  const [budgetInputs, setBudgetInputs] = useState<Record<keyof DustBudget, string>>({
    urCp: "",
    srCp: "",
    rCp: "",
    nCp: "",
  });

  const [syncing, setSyncing] = useState(false);
  const [loadingArchetype, setLoadingArchetype] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const fetchedArchsRef = useRef<Set<string>>(new Set());

  // Meta Filters (All FREE — no paywall!)
  const [searchTerm, setSearchTerm] = useState("");
  const [tierFilter, setTierFilter] = useState<number | "ALL">("ALL");
  const [categoryFilter, setCategoryFilter] = useState<CategoryFilter>("ALL");
  const [engineFilter, setEngineFilter] = useState<string>("ALL");
  const [periodFilter, setPeriodFilter] = useState<PeriodFilter>("ALL");
  const [sortBy, setSortBy] = useState<SortBy>("power");
  const [selectedMetaDeckId, setSelectedMetaDeckId] = useState<string | null>(null);

  const metaDecks = meta?.decks ?? [];
  const metaArchetypes = meta?.archetypes ?? [];

  const matchedArchetypeForSearch = useMemo(() => {
    const q = searchTerm.trim().toLowerCase();
    if (q.length < 3 || metaArchetypes.length === 0) return null;
    return (
      metaArchetypes.find((a) => a.name.toLowerCase() === q) ??
      metaArchetypes.find((a) => a.name.toLowerCase().startsWith(q)) ??
      metaArchetypes.find((a) => a.name.toLowerCase().includes(q)) ??
      null
    );
  }, [searchTerm, metaArchetypes]);

  const handleLoadArchetypeDecks = useCallback(async (archQuery: string) => {
    const clean = archQuery.trim();
    if (!clean) return;
    setLoadingArchetype(clean);
    fetchedArchsRef.current.add(clean.toLowerCase());
    try {
      const updated = await api.loadArchetypeDecks(clean);
      setMeta(updated);
    } catch (err) {
      console.error("Falha ao expandir decks do arquétipo:", err);
    } finally {
      setLoadingArchetype(null);
    }
  }, []);

  // Auto-expand Ranked + Championship decks when searching a specific/Rogue archetype (e.g. "Reso" -> "Resonators")
  useEffect(() => {
    if (tab !== "meta" || !matchedArchetypeForSearch) return;
    const key = matchedArchetypeForSearch.name.toLowerCase();
    if (fetchedArchsRef.current.has(key)) return;

    const existingCount = metaDecks.filter(
      (d) => d.deckType.toLowerCase() === key
    ).length;
    const existingTourney = metaDecks.filter(
      (d) => d.deckType.toLowerCase() === key && d.category === "tournament"
    ).length;

    if (existingCount >= 18 && existingTourney > 0) {
      fetchedArchsRef.current.add(key);
      return;
    }

    const timer = window.setTimeout(() => {
      handleLoadArchetypeDecks(matchedArchetypeForSearch.name);
    }, 320);

    return () => window.clearTimeout(timer);
  }, [tab, matchedArchetypeForSearch, metaDecks, handleLoadArchetypeDecks]);

  const refreshCachedMeta = useCallback(async () => {
    try {
      const snap = await api.getMetaSnapshot();
      if (snap) setMeta(snap);
    } catch {
      // ignore
    }
  }, []);

  const popularEngines = useMemo(() => {
    const counts = new Map<string, number>();
    for (const d of metaDecks) {
      for (const eng of d.engines) {
        counts.set(eng, (counts.get(eng) ?? 0) + 1);
      }
    }
    return Array.from(counts.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 20)
      .map(([name, count]) => ({ name, count }));
  }, [metaDecks]);

  const handleSync = useCallback(async () => {
    setSyncing(true);
    setError(null);
    try {
      fetchedArchsRef.current.clear();
      const snap = await api.syncMetaFromMdm();
      setMeta(snap);
    } catch (err) {
      setError(`Falha ao sincronizar com Master Duel Meta: ${err}`);
    } finally {
      setSyncing(false);
    }
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const [cachedMeta, decks, bud] = await Promise.all([
          api.getMetaSnapshot(),
          api.getUserDecks(),
          api.getDustBudget(),
        ]);
        setUserDecks(decks);
        setActiveUserDeckId(decks[0]?.id ?? null);
        setBudget(bud);
        setBudgetInputs({
          urCp: bud.urCp > 0 ? String(bud.urCp) : "",
          srCp: bud.srCp > 0 ? String(bud.srCp) : "",
          rCp: bud.rCp > 0 ? String(bud.rCp) : "",
          nCp: bud.nCp > 0 ? String(bud.nCp) : "",
        });

        const needsFreshSync =
          !cachedMeta ||
          cachedMeta.decks.length === 0 ||
          !cachedMeta.decks[0]?.category ||
          !cachedMeta.decks[0]?.badgeLabel;
        if (!needsFreshSync && cachedMeta) {
          setMeta(cachedMeta);
        } else {
          await handleSync();
        }
      } catch (err) {
        setError(`Erro ao inicializar aplicativo: ${err}`);
      }
    })();
  }, [handleSync]);

  const handleBudgetChange = (field: keyof DustBudget, rawValue: string) => {
    const digits = rawValue.replace(/\D+/g, "").replace(/^0+(?=\d)/, "");
    if (digits === "") {
      setBudgetInputs((prev) => ({ ...prev, [field]: "" }));
      setBudget((prev) => {
        const next = { ...prev, [field]: 0 };
        api.setDustBudget(next).catch((err) => setError(`Erro ao salvar pó CP: ${err}`));
        return next;
      });
      return;
    }

    const parsed = Math.min(99990, parseInt(digits, 10) || 0);
    setBudgetInputs((prev) => ({ ...prev, [field]: digits === "0" ? "0" : String(parsed) }));
    setBudget((prev) => {
      const next = { ...prev, [field]: parsed };
      api.setDustBudget(next).catch((err) => setError(`Erro ao salvar pó CP: ${err}`));
      return next;
    });
  };

  const handleCloneMetaToUser = async (metaDeckId: string) => {
    try {
      const created = await api.cloneMetaDeckToUser(metaDeckId);
      const updated = await api.getUserDecks();
      setUserDecks(updated);
      setActiveUserDeckId(created.id);
      setSelectedMetaDeckId(null);
      setTab("mydecks");
    } catch (err) {
      setError(`Erro ao clonar deck: ${err}`);
    }
  };

  const filteredMetaDecks = useMemo(() => {
    const q = searchTerm.trim().toLowerCase();
    const nowMs = Date.now();
    const maxAgeMs =
      periodFilter === "7d"
        ? 7 * 86400 * 1000
        : periodFilter === "30d"
        ? 30 * 86400 * 1000
        : null;

    const list = metaDecks.filter((d) => {
      if (tierFilter !== "ALL" && d.tier !== tierFilter) return false;
      if (categoryFilter !== "ALL" && d.category !== categoryFilter) return false;
      if (engineFilter !== "ALL" && !d.engines.includes(engineFilter)) return false;
      if (maxAgeMs && d.created) {
        const createdMs = Date.parse(d.created);
        if (!Number.isNaN(createdMs) && nowMs - createdMs > maxAgeMs) return false;
      }
      if (q) {
        const matchType = d.deckType.toLowerCase().includes(q);
        const matchEngine = d.engines.some((e) => e.toLowerCase().includes(q));
        if (!matchType && !matchEngine) return false;
      }
      return true;
    });

    const sorted = [...list];
    if (sortBy === "power") {
      sorted.sort((a, b) => b.tournamentPower - a.tournamentPower || a.urPrice - b.urPrice);
    } else if (sortBy === "cheapest") {
      sorted.sort((a, b) => a.urPrice - b.urPrice || a.srPrice - b.srPrice);
    } else {
      sorted.sort((a, b) => (b.created || "").localeCompare(a.created || ""));
    }
    return sorted;
  }, [metaDecks, searchTerm, tierFilter, categoryFilter, engineFilter, periodFilter, sortBy]);

  const selectedMetaDeck = useMemo(
    () => metaDecks.find((d) => d.id === selectedMetaDeckId) ?? null,
    [metaDecks, selectedMetaDeckId]
  );

  return (
    <div className="min-h-screen flex flex-col bg-[#0b0e14] text-slate-100">
      {/* Top Header */}
      <header className="sticky top-0 z-30 border-b border-slate-800/90 bg-[#0d111a]/95 backdrop-blur-md">
        <div className="max-w-[1440px] mx-auto px-5 py-3 flex flex-wrap items-center justify-between gap-4">
          {/* Brand + Nav */}
          <div className="flex items-center gap-6">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-amber-400 via-orange-500 to-rose-500 flex items-center justify-center shadow-lg shadow-amber-500/20">
                <Sparkles className="w-4 h-4 text-slate-950" />
              </div>
              <h1 className="text-base font-black tracking-tight text-white leading-none">
                DUEL<span className="text-amber-400">CRAFT</span>
              </h1>
            </div>

            <nav className="flex items-center gap-1 bg-[#141a26] p-1 rounded-xl border border-slate-800">
              <button
                onClick={() => setTab("mydecks")}
                className={`px-3.5 py-1.5 rounded-lg text-xs font-bold inline-flex items-center gap-1.5 transition-all cursor-pointer ${
                  tab === "mydecks"
                    ? "bg-amber-500 text-slate-950 shadow"
                    : "text-slate-300 hover:text-white"
                }`}
              >
                <BrainCircuit className="w-3.5 h-3.5" />
                Meus Decks & Otimizador
              </button>
              <button
                onClick={() => setTab("meta")}
                className={`px-3.5 py-1.5 rounded-lg text-xs font-bold inline-flex items-center gap-1.5 transition-all cursor-pointer ${
                  tab === "meta"
                    ? "bg-amber-500 text-slate-950 shadow"
                    : "text-slate-300 hover:text-white"
                }`}
              >
                <Layers className="w-3.5 h-3.5" />
                Meta Decks ({metaDecks.length})
              </button>
              <button
                onClick={() => setTab("tierlist")}
                className={`px-3.5 py-1.5 rounded-lg text-xs font-bold inline-flex items-center gap-1.5 transition-all cursor-pointer ${
                  tab === "tierlist"
                    ? "bg-amber-500 text-slate-950 shadow"
                    : "text-slate-300 hover:text-white"
                }`}
              >
                <Flame className="w-3.5 h-3.5" />
                Tier List
              </button>
            </nav>
          </div>

          {/* Right: Dust Budget + Sync */}
          <div className="flex items-center gap-3 flex-wrap">
            <div className="flex items-center gap-2 bg-[#141a26] border border-slate-800 rounded-xl px-3 py-1.5">
              <span className="text-[11px] font-bold uppercase tracking-wider text-slate-400 mr-1">
                Seu Pó:
              </span>
              {(
                [
                  { key: "urCp", label: "UR" },
                  { key: "srCp", label: "SR" },
                  { key: "rCp", label: "R" },
                  { key: "nCp", label: "N" },
                ] as const
              ).map(({ key, label }) => (
                <label key={key} className="inline-flex items-center gap-1">
                  <RarityBadge rarity={label} compact />
                  <input
                    type="text"
                    inputMode="numeric"
                    placeholder="0"
                    value={budgetInputs[key]}
                    onFocus={(e) => e.currentTarget.select()}
                    onBlur={() => {
                      if (budgetInputs[key] === "0") {
                        setBudgetInputs((prev) => ({ ...prev, [key]: "" }));
                      }
                    }}
                    onChange={(e) => handleBudgetChange(key, e.target.value)}
                    className="w-14 bg-[#0b0e14] border border-slate-700/80 rounded px-1.5 py-0.5 text-xs font-bold text-slate-100 text-right placeholder-slate-600 focus:placeholder-transparent focus:outline-none focus:border-amber-500"
                  />
                </label>
              ))}
            </div>

            <button
              onClick={handleSync}
              disabled={syncing}
              className="px-3.5 py-2 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold inline-flex items-center gap-2 shadow shadow-indigo-600/20 cursor-pointer disabled:opacity-60"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${syncing ? "animate-spin" : ""}`} />
              {syncing ? "Sincronizando MDM..." : "Atualizar Meta"}
            </button>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 max-w-[1440px] w-full mx-auto px-5 py-5 space-y-5">
        {error && (
          <div className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/40 text-rose-200 text-xs flex items-center justify-between">
            <span>{error}</span>
            <button
              onClick={() => setError(null)}
              className="text-rose-300 hover:text-white font-bold ml-4 cursor-pointer"
            >
              Fechar
            </button>
          </div>
        )}

        {tab === "mydecks" && (
          <MyDecksView
            userDecks={userDecks}
            activeDeckId={activeUserDeckId}
            archetypes={meta?.archetypes ?? []}
            budget={budget}
            onSelectDeckId={setActiveUserDeckId}
            onUserDecksChange={setUserDecks}
            onOpenMetaDeck={(id) => setSelectedMetaDeckId(id)}
            onMetaExpanded={refreshCachedMeta}
          />
        )}

        {tab === "meta" && (
          <>
            {/* Filters Toolbar (All Ranks / Championships / Engines FREE) */}
            <div className="bg-[#121723] border border-slate-800 rounded-xl p-3 space-y-2.5">
              <div className="flex flex-wrap items-center justify-between gap-2.5">
                <div className="flex items-center gap-2 flex-wrap flex-1">
                  <div className="relative min-w-[210px] flex-1 max-w-xs">
                    <Search className="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
                    <input
                      type="text"
                      value={searchTerm}
                      onChange={(e) => setSearchTerm(e.target.value)}
                      placeholder="Buscar arquétipo ou engine (ex: Resonators)..."
                      className="w-full pl-9 pr-3 py-1.5 rounded-lg bg-[#0b0e14] border border-slate-700 text-xs text-slate-100 placeholder-slate-500 focus:outline-none focus:border-amber-500"
                    />
                  </div>

                  {loadingArchetype ? (
                    <span className="px-2.5 py-1 rounded-lg bg-amber-500/15 border border-amber-400/30 text-amber-300 text-xs font-bold inline-flex items-center gap-1.5">
                      <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                      Buscando Mestre & Campeonatos de {loadingArchetype}...
                    </span>
                  ) : (
                    searchTerm.trim().length >= 3 && (
                      <button
                        onClick={() =>
                          handleLoadArchetypeDecks(
                            matchedArchetypeForSearch?.name ?? searchTerm.trim()
                          )
                        }
                        title="Busca até 80 listas de Mestre + Campeonatos deste arquétipo direto no Master Duel Meta"
                        className="px-2.5 py-1 rounded-lg bg-amber-500/15 hover:bg-amber-500/25 border border-amber-500/35 text-amber-300 text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer"
                      >
                        <DownloadCloud className="w-3.5 h-3.5" />
                        + Listas de{" "}
                        {matchedArchetypeForSearch?.name ?? searchTerm.trim()}
                      </button>
                    )
                  )}

                  <div className="flex items-center bg-[#0b0e14] p-0.5 rounded-lg border border-slate-800">
                    {(
                      [
                        { v: "ALL", l: "Todos Ranks" },
                        { v: "tournament", l: "Campeonatos" },
                        { v: "master1", l: "Master I" },
                        { v: "duelist_cup", l: "Duelist Cup" },
                        { v: "master5", l: "Master V" },
                      ] as const
                    ).map(({ v, l }) => (
                      <button
                        key={v}
                        onClick={() => setCategoryFilter(v)}
                        className={`px-2.5 py-1 rounded-md text-xs font-bold cursor-pointer ${
                          categoryFilter === v
                            ? "bg-indigo-600 text-white"
                            : "text-slate-400 hover:text-slate-200"
                        }`}
                      >
                        {l}
                      </button>
                    ))}
                  </div>

                  <div className="flex items-center bg-[#0b0e14] p-0.5 rounded-lg border border-slate-800">
                    {(
                      [
                        { v: "ALL", l: "Todos Tiers" },
                        { v: 1, l: "T1" },
                        { v: 2, l: "T2" },
                        { v: 3, l: "T3" },
                        { v: 4, l: "Rogue" },
                      ] as const
                    ).map(({ v, l }) => (
                      <button
                        key={String(v)}
                        onClick={() => setTierFilter(v)}
                        className={`px-2 py-1 rounded-md text-xs font-bold cursor-pointer ${
                          tierFilter === v
                            ? "bg-slate-700 text-white"
                            : "text-slate-400 hover:text-slate-200"
                        }`}
                      >
                        {l}
                      </button>
                    ))}
                  </div>
                </div>
              </div>

              <div className="flex flex-wrap items-center justify-between gap-2.5 pt-2 border-t border-slate-800/70">
                <div className="flex items-center gap-2.5 flex-wrap">
                  <select
                    value={engineFilter}
                    onChange={(e) => setEngineFilter(e.target.value)}
                    className="bg-[#0b0e14] border border-slate-700 rounded-lg px-2.5 py-1 text-xs font-semibold text-slate-200 focus:outline-none focus:border-amber-500"
                  >
                    <option value="ALL">Todas as Engines</option>
                    {popularEngines.map((eng) => (
                      <option key={eng.name} value={eng.name}>
                        +{eng.name} ({eng.count})
                      </option>
                    ))}
                  </select>

                  <select
                    value={periodFilter}
                    onChange={(e) => setPeriodFilter(e.target.value as PeriodFilter)}
                    className="bg-[#0b0e14] border border-slate-700 rounded-lg px-2.5 py-1 text-xs font-semibold text-slate-200 focus:outline-none focus:border-amber-500"
                  >
                    <option value="ALL">Qualquer período</option>
                    <option value="7d">Últimos 7 dias</option>
                    <option value="30d">Últimos 30 dias</option>
                  </select>
                </div>

                <div className="flex items-center gap-2">
                  <span className="text-xs text-slate-400">
                    {filteredMetaDecks.length} deck(s) • Ordenar:
                  </span>
                  <select
                    value={sortBy}
                    onChange={(e) => setSortBy(e.target.value as SortBy)}
                    className="bg-[#0b0e14] border border-slate-700 rounded-lg px-2.5 py-1 text-xs font-semibold text-slate-200 focus:outline-none focus:border-amber-500"
                  >
                    <option value="power">Maior Poder Meta (Tier)</option>
                    <option value="cheapest">Mais barato (UR CP)</option>
                    <option value="recent">Mais recentes</option>
                  </select>
                </div>
              </div>
            </div>

            {/* Meta Decks Grid */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {filteredMetaDecks.map((d) => (
                <div
                  key={d.id}
                  onClick={() => setSelectedMetaDeckId(d.id)}
                  className="group bg-[#121723] hover:bg-[#161d2c] border border-slate-800 hover:border-amber-500/40 rounded-xl p-3.5 transition-all cursor-pointer flex gap-3.5"
                >
                  <img
                    src={cardImageUrl(d.coverCardId, 260)}
                    alt={d.deckType}
                    loading="lazy"
                    className="w-16 h-23 object-cover rounded-lg border border-slate-700/70 bg-slate-900 shrink-0 group-hover:scale-[1.03] transition-transform"
                  />
                  <div className="min-w-0 flex-1 flex flex-col justify-between">
                    <div>
                      <div className="flex items-center gap-1.5 flex-wrap">
                        <TierBadge tier={d.tier} />
                        <span
                          className={`text-[10px] font-bold px-1.5 py-0.5 rounded border inline-flex items-center gap-1 max-w-[210px] truncate ${deckBadgeClass(
                            d.category
                          )}`}
                          title={getDeckBadgeText(d)}
                        >
                          {d.category === "tournament" && (
                            <Trophy className="w-2.5 h-2.5 shrink-0" />
                          )}
                          <span className="truncate">{getDeckBadgeText(d)}</span>
                        </span>
                      </div>
                      <h3 className="text-sm font-extrabold text-white mt-1.5 truncate group-hover:text-amber-300 transition-colors">
                        {d.deckType}
                      </h3>
                      {d.engines.length > 0 && (
                        <div className="flex items-center gap-1 flex-wrap mt-1">
                          {d.engines.slice(0, 3).map((eng) => (
                            <span
                              key={eng}
                              className="text-[10px] px-1.5 py-0.5 rounded bg-slate-800/90 text-slate-400"
                            >
                              +{eng}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                    <div className="mt-2.5 flex items-center gap-1.5 flex-wrap">
                      <DustPill rarity="UR" cp={d.urPrice} />
                      <DustPill rarity="SR" cp={d.srPrice} />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </>
        )}

        {tab === "tierlist" && (
          <TierListView
            archetypes={meta?.archetypes ?? []}
            onSelectArchetype={(name) => {
              setSearchTerm(name);
              setTierFilter("ALL");
              setCategoryFilter("ALL");
              setEngineFilter("ALL");
              setTab("meta");
            }}
          />
        )}
      </main>

      {selectedMetaDeck && (
        <DeckModal
          deck={selectedMetaDeck}
          onClose={() => setSelectedMetaDeckId(null)}
          onCloneToMyDecks={handleCloneMetaToUser}
        />
      )}
    </div>
  );
}

export default App;
