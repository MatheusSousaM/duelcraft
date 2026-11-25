import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  ArrowRight,
  BrainCircuit,
  Check,
  Copy,
  CopyPlus,
  Crosshair,
  Download,
  FileUp,
  LayoutGrid,
  Package,
  Plus,
  Scissors,
  Sparkles,
  Square,
  Trash2,
  TrendingUp,
  Trophy,
  Wand2,
} from "lucide-react";
import { api } from "../api";
import {
  cardImageUrl,
  type DeckOptimizationReport,
  type DustBudget,
  type KnnMode,
  type MetaArchetype,
  type PackRecommendation,
  type UpgradeSuggestion,
  type UserDeck,
} from "../types";
import { DustPill, RarityBadge, TierBadge } from "./Badges";
import { MasterDuelDeckBuilder } from "./MasterDuelDeckBuilder";

interface MyDecksViewProps {
  userDecks: UserDeck[];
  activeDeckId: string | null;
  archetypes: MetaArchetype[];
  budget: DustBudget;
  onSelectDeckId: (id: string | null) => void;
  onUserDecksChange: (decks: UserDeck[]) => void;
  onOpenMetaDeck: (metaDeckId: string) => void;
  onMetaExpanded?: () => void;
}

export const MyDecksView: React.FC<MyDecksViewProps> = ({
  userDecks,
  activeDeckId,
  archetypes,
  budget,
  onSelectDeckId,
  onUserDecksChange,
  onOpenMetaDeck,
  onMetaExpanded,
}) => {
  const activeDeck = useMemo(
    () => userDecks.find((d) => d.id === activeDeckId) ?? userDecks[0] ?? null,
    [userDecks, activeDeckId]
  );

  const [subView, setSubView] = useState<"editor" | "optimizer">("editor");
  const [report, setReport] = useState<DeckOptimizationReport | null>(null);
  const [kNeighbors, setKNeighbors] = useState<number>(8);
  const [knnMode, setKnnMode] = useState<KnnMode>("balanced");
  const [stageFilter, setStageFilter] = useState<number | "ALL">("ALL");

  // YDK import panel
  const [showYdkImport, setShowYdkImport] = useState(false);
  const [ydkName, setYdkName] = useState("");
  const [ydkText, setYdkText] = useState("");
  const [importingYdk, setImportingYdk] = useState(false);

  // Secret packs for recommended upgrades
  const [upgradePacks, setUpgradePacks] = useState<PackRecommendation[] | null>(null);
  const [loadingPacks, setLoadingPacks] = useState(false);

  // Auto-import & YDK export state
  const [runningAutoImport, setRunningAutoImport] = useState(false);
  const [autoImportMsg, setAutoImportMsg] = useState<string | null>(null);
  const [exportingYdk, setExportingYdk] = useState(false);
  const [copiedYdk, setCopiedYdk] = useState(false);

  const runAnalysis = useCallback(
    async (deckId: string) => {
      try {
        const rep = await api.analyzeUserDeckKnn(deckId, kNeighbors, knnMode);
        setReport(rep);
        setUpgradePacks(null);
        onMetaExpanded?.();
      } catch (err) {
        console.error(err);
      }
    },
    [kNeighbors, knnMode, onMetaExpanded]
  );

  // Reset stale report when switching active deck
  useEffect(() => {
    setReport(null);
    setUpgradePacks(null);
  }, [activeDeck?.id]);

  // Only run optimizer analysis when the Optimizer tab is actually open (debounced 180ms)
  useEffect(() => {
    if (!activeDeck) {
      setReport(null);
      return;
    }
    if (subView !== "optimizer") {
      return;
    }
    const timer = window.setTimeout(() => {
      runAnalysis(activeDeck.id);
    }, 180);
    return () => window.clearTimeout(timer);
  }, [activeDeck, subView, budget, kNeighbors, knnMode, runAnalysis]);

  const handleApplyAutoOptimize = async (asCopy: boolean) => {
    if (!activeDeck || !report) return;
    const opt = report.autoOptimize;
    if (asCopy) {
      const copyDeck: UserDeck = {
        id: `deck-${Date.now()}`,
        name: `${activeDeck.name} (Otimizado)`,
        targetArchetype: activeDeck.targetArchetype ?? report.detectedArchetype,
        updatedAt: new Date().toISOString(),
        main: opt.optimizedMain,
        extra: opt.optimizedExtra,
      };
      const updated = await api.saveUserDeck(copyDeck);
      onUserDecksChange(updated);
      onSelectDeckId(copyDeck.id);
      setSubView("editor");
    } else {
      await persistDeck({
        ...activeDeck,
        main: opt.optimizedMain,
        extra: opt.optimizedExtra,
      });
    }
  };

  const handleCreateDeck = async () => {
    const newDeck: UserDeck = {
      id: `deck-${Date.now()}`,
      name: `Meu Deck ${userDecks.length + 1}`,
      targetArchetype: null,
      updatedAt: new Date().toISOString(),
      main: [],
      extra: [],
    };
    const updated = await api.saveUserDeck(newDeck);
    onUserDecksChange(updated);
    onSelectDeckId(newDeck.id);
    setSubView("editor");
  };

  const handleDeleteDeck = async (id: string) => {
    if (!confirm("Remover este deck?")) return;
    const updated = await api.deleteUserDeck(id);
    onUserDecksChange(updated);
    onSelectDeckId(updated[0]?.id ?? null);
  };

  const persistDeck = async (next: UserDeck) => {
    const updated = await api.saveUserDeck(next);
    onUserDecksChange(updated);
  };

  const handleSetCardQty = async (
    card: { id: string; name: string; rarity: string; section: "main" | "extra" },
    qty: number
  ) => {
    if (!activeDeck) return;
    const clamped = Math.max(0, Math.min(3, qty));
    const listKey = card.section === "extra" ? "extra" : "main";
    const otherKey = listKey === "extra" ? "main" : "extra";
    const list = [...activeDeck[listKey]];
    const idx = list.findIndex((c) => c.id === card.id);
    if (clamped === 0) {
      if (idx >= 0) list.splice(idx, 1);
    } else if (idx >= 0) {
      list[idx] = { ...list[idx], amount: clamped };
    } else {
      list.push({
        id: card.id,
        name: card.name,
        rarity: card.rarity,
        amount: clamped,
        section: card.section,
      });
    }
    const next: UserDeck = {
      ...activeDeck,
      [listKey]: list,
      [otherKey]: activeDeck[otherKey],
    };
    await persistDeck(next);
  };

  const handleApplyUpgrade = async (up: UpgradeSuggestion) => {
    const section = up.section === "extra" ? "extra" : "main";
    await handleSetCardQty(
      { id: up.id, name: up.name, rarity: up.rarity, section },
      up.recommendedQty
    );
  };

  const handleImportYdk = async () => {
    if (!ydkText.trim()) return;
    setImportingYdk(true);
    try {
      const created = await api.importYdkAsUserDeck(ydkName || "Deck .YDK", ydkText);
      const all = await api.getUserDecks();
      onUserDecksChange(all);
      onSelectDeckId(created.id);
      setShowYdkImport(false);
      setYdkText("");
      setYdkName("");
    } catch (err) {
      alert(`Erro ao importar .YDK: ${err}`);
    } finally {
      setImportingYdk(false);
    }
  };

  const handleLoadUpgradePacks = async () => {
    if (!report || report.upgrades.length === 0) return;
    setLoadingPacks(true);
    try {
      const targetCards = report.upgrades
        .filter((u) => stageFilter === "ALL" || u.stage === stageFilter)
        .map((u) => ({
          id: u.id,
          name: u.name,
          rarity: u.rarity,
          missing: u.addQty,
        }));
      const packs = await api.getCardsSecretPacks(targetCards);
      setUpgradePacks(packs);
    } catch (err) {
      alert(`Erro ao buscar Secret Packs: ${err}`);
    } finally {
      setLoadingPacks(false);
    }
  };

  const handleAutoImportToGame = async () => {
    if (!activeDeck) return;
    setRunningAutoImport(true);
    setAutoImportMsg(null);
    try {
      const res = await api.runAutoImportUserDeck(activeDeck.id);
      setAutoImportMsg(res.message);
    } catch (err) {
      setAutoImportMsg(`Erro: ${err}`);
    } finally {
      setRunningAutoImport(false);
    }
  };

  const handleExportUserYdk = async (mode: "copy" | "save") => {
    if (!activeDeck) return;
    setExportingYdk(true);
    try {
      const ydk = await api.exportUserDeckYdk(activeDeck.id);
      if (mode === "copy") {
        await navigator.clipboard.writeText(ydk);
        setCopiedYdk(true);
        window.setTimeout(() => setCopiedYdk(false), 2000);
      } else {
        const savedPath = await api.saveYdkToDisk(activeDeck.name, ydk);
        if (savedPath) {
          setAutoImportMsg(`Deck exportado com sucesso em: ${savedPath}`);
        }
      }
    } catch (err) {
      setAutoImportMsg(`Erro ao exportar .YDK: ${err}`);
    } finally {
      setExportingYdk(false);
    }
  };

  const mainCount = activeDeck?.main.reduce((s, c) => s + c.amount, 0) ?? 0;
  const extraCount = activeDeck?.extra.reduce((s, c) => s + c.amount, 0) ?? 0;

  const filteredUpgrades = useMemo(() => {
    if (!report) return [];
    return report.upgrades.filter(
      (u) => stageFilter === "ALL" || u.stage === stageFilter
    );
  }, [report, stageFilter]);

  return (
    <div className="space-y-4">
      {/* Top Deck Selector & Mode Switch Bar */}
      <div className="bg-[#121723] border border-slate-800 rounded-xl p-3 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2 flex-wrap">
          {userDecks.map((d) => (
            <button
              key={d.id}
              onClick={() => onSelectDeckId(d.id)}
              className={`px-3 py-1.5 rounded-lg text-xs font-bold transition-all cursor-pointer ${
                activeDeck?.id === d.id
                  ? "bg-amber-500 text-slate-950 shadow"
                  : "bg-[#0b0e14] text-slate-300 border border-slate-800 hover:border-slate-600"
              }`}
            >
              {d.name} ({d.main.reduce((s, c) => s + c.amount, 0)}+
              {d.extra.reduce((s, c) => s + c.amount, 0)})
            </button>
          ))}
          <button
            onClick={handleCreateDeck}
            className="px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer"
          >
            <Plus className="w-3.5 h-3.5" /> Novo Deck
          </button>
          <button
            onClick={() => setShowYdkImport((v) => !v)}
            className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer"
          >
            <FileUp className="w-3.5 h-3.5" /> Importar .YDK
          </button>
        </div>

        {activeDeck && (
          <div className="flex items-center gap-2 flex-wrap">
            {/* Switch: Master Duel Visual Editor vs KNN Optimizer & CP Plan */}
            <div className="flex bg-[#0b0e14] p-0.5 rounded-lg border border-slate-800">
              <button
                onClick={() => setSubView("editor")}
                className={`px-3 py-1.5 rounded-md text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer ${
                  subView === "editor"
                    ? "bg-amber-500 text-slate-950"
                    : "text-slate-300 hover:text-white"
                }`}
              >
                <LayoutGrid className="w-3.5 h-3.5" />
                Editor (Master Duel)
              </button>
              <button
                onClick={() => setSubView("optimizer")}
                className={`px-3 py-1.5 rounded-md text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer ${
                  subView === "optimizer"
                    ? "bg-emerald-500 text-slate-950"
                    : "text-emerald-300 hover:text-white"
                }`}
              >
                <BrainCircuit className="w-3.5 h-3.5" />
                Otimizador & Plano CP
                {report && report.upgrades.length > 0 && (
                  <span className="px-1.5 py-0.5 rounded-full bg-slate-950/30 text-[10px]">
                    {report.upgrades.length}
                  </span>
                )}
              </button>
            </div>

            {/* YDK Copy & Export to Disk */}
            <div className="flex items-center gap-1">
              <button
                disabled={exportingYdk || (mainCount === 0 && extraCount === 0)}
                onClick={() => handleExportUserYdk("copy")}
                title="Copiar lista .YDK para a área de transferência"
                className="px-2.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer disabled:opacity-40"
              >
                {copiedYdk ? (
                  <Check className="w-3.5 h-3.5 text-emerald-400" />
                ) : (
                  <Copy className="w-3.5 h-3.5" />
                )}
                {copiedYdk ? "Copiado!" : "Copiar .YDK"}
              </button>
              <button
                disabled={exportingYdk || (mainCount === 0 && extraCount === 0)}
                onClick={() => handleExportUserYdk("save")}
                title="Salvar arquivo .ydk no computador (EDOPro, YGO Omega, DuelingBook, backup)"
                className="px-2.5 py-1.5 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 border border-amber-500/40 text-amber-300 text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer disabled:opacity-40"
              >
                <Download className="w-3.5 h-3.5" />
                Exportar .YDK
              </button>
            </div>

            {runningAutoImport ? (
              <button
                onClick={() => api.cancelAutoImport()}
                className="px-3 py-1.5 rounded-lg bg-rose-500 text-white text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer"
              >
                <Square className="w-3.5 h-3.5" /> Parar (ESC)
              </button>
            ) : (
              <div className="flex items-center gap-1.5">
                <button
                  onClick={async () => {
                    setAutoImportMsg(null);
                    try {
                      const st = await api.testImporterAim();
                      setAutoImportMsg(
                        `Mira verificada em ${st.clientWidth}x${st.clientHeight} (${st.clientX}, ${st.clientY}) → Busca (${st.searchX}, ${st.searchY}) • 1ª Carta (${st.firstCardX}, ${st.firstCardY})`
                      );
                    } catch (err) {
                      setAutoImportMsg(`Erro: ${err}`);
                    }
                  }}
                  title="Testar onde o mouse vai mirar na janela atual do Master Duel (sem clicar)"
                  className="p-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-amber-300 cursor-pointer"
                >
                  <Crosshair className="w-4 h-4" />
                </button>
                <button
                  disabled={mainCount === 0}
                  onClick={handleAutoImportToGame}
                  className="px-3 py-1.5 rounded-lg bg-emerald-500/20 hover:bg-emerald-500/30 border border-emerald-500/40 text-emerald-300 text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer disabled:opacity-40"
                >
                  <Wand2 className="w-3.5 h-3.5" /> Montar no Jogo
                </button>
              </div>
            )}
            <button
              onClick={() => handleDeleteDeck(activeDeck.id)}
              title="Excluir deck"
              className="p-1.5 rounded-lg bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/30 text-rose-400 cursor-pointer"
            >
              <Trash2 className="w-4 h-4" />
            </button>
          </div>
        )}
      </div>

      {autoImportMsg && (
        <div className="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-200 text-xs">
          {autoImportMsg}
        </div>
      )}

      {showYdkImport && (
        <div className="bg-[#121723] border border-indigo-500/40 rounded-xl p-4 space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-bold text-white">Importar Decklist (.YDK)</h3>
            <label className="px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-xs font-bold text-slate-200 cursor-pointer">
              Carregar arquivo .ydk
              <input
                type="file"
                accept=".ydk,.txt"
                className="hidden"
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  if (!f) return;
                  setYdkName(f.name.replace(/\.ydk$/i, ""));
                  const r = new FileReader();
                  r.onload = () => typeof r.result === "string" && setYdkText(r.result);
                  r.readAsText(f);
                }}
              />
            </label>
          </div>
          <input
            type="text"
            value={ydkName}
            onChange={(e) => setYdkName(e.target.value)}
            placeholder="Nome do deck (ex: Meu Branded)"
            className="w-full px-3 py-1.5 rounded-lg bg-[#0b0e14] border border-slate-700 text-xs text-white"
          />
          <textarea
            rows={4}
            value={ydkText}
            onChange={(e) => setYdkText(e.target.value)}
            placeholder={"#main\n14558127\n..."}
            className="w-full p-2.5 rounded-lg bg-[#0b0e14] border border-slate-700 text-xs font-mono text-slate-200"
          />
          <button
            disabled={importingYdk || !ydkText.trim()}
            onClick={handleImportYdk}
            className="px-4 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold cursor-pointer disabled:opacity-40"
          >
            {importingYdk ? "Resolvendo cartas..." : "Criar Deck a partir do .YDK"}
          </button>
        </div>
      )}

      {!activeDeck ? (
        <div className="bg-[#121723] border border-slate-800 rounded-2xl p-12 text-center space-y-3">
          <LayoutGrid className="w-10 h-10 text-amber-400 mx-auto" />
          <h2 className="text-lg font-extrabold text-white">
            Monte seu Deck no Editor estilo Master Duel
          </h2>
          <p className="text-xs text-slate-400 max-w-lg mx-auto">
            Crie seu deck aqui com filtros de <strong>Tipo, Subtipo (Fusão/Sincro/Xyz/Link), Atributo, Nível, Link, Raça e Raridade</strong> — depois clique em <strong>Otimizador & Plano CP</strong> para ver quais cartas melhorar e quanto de pó juntar!
          </p>
          <button
            onClick={handleCreateDeck}
            className="px-5 py-2.5 rounded-xl bg-amber-500 hover:bg-amber-400 text-slate-950 text-xs font-extrabold inline-flex items-center gap-2 cursor-pointer"
          >
            <Plus className="w-4 h-4" /> Criar Meu Primeiro Deck
          </button>
        </div>
      ) : subView === "editor" ? (
        <MasterDuelDeckBuilder
          deck={activeDeck}
          archetypes={archetypes}
          onDeckChange={persistDeck}
          onSetCardQty={handleSetCardQty}
        />
      ) : (
        /* ===== KNN OPTIMIZER & CP PLAN VIEW ===== */
        <div className="space-y-4">
          {report && (
            <>
              {/* KNN Header & Similar Decks */}
              <div className="bg-[#121723] border border-slate-800 rounded-xl p-4 space-y-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <div className="flex items-center gap-2">
                      <BrainCircuit className="w-4 h-4 text-amber-400" />
                      <h3 className="text-sm font-extrabold text-white">
                        Otimizador de Deck • Arquétipo:{" "}
                        <span className="text-amber-400">{report.detectedArchetype}</span>
                      </h3>
                    </div>
                    <p className="text-xs text-slate-400 mt-0.5">
                      Comparando <strong>{activeDeck.name}</strong> com os{" "}
                      <strong>{report.kNeighbors} decks vencedores</strong> mais parecidos no Master Duel Meta.
                    </p>
                  </div>
                  <div className="flex items-center gap-3 flex-wrap text-xs">
                    <div className="flex items-center bg-[#0b0e14] p-0.5 rounded-lg border border-slate-800">
                      {(
                        [
                          {
                            v: "balanced" as const,
                            l: "Sem Viés Rogue (Mestre + Campeonatos)",
                            tip: "Foca 100% no seu arquétipo real sem penalizar decks Rogue, usando listas de Master V, Master I e Campeonatos",
                          },
                          {
                            v: "strict_top" as const,
                            l: "Só Campeonatos & M1",
                            tip: "Filtra estritamente Campeonatos, Duelist Cup e Master I do seu arquétipo (com fallback automático se houver < 3 listas)",
                          },
                          {
                            v: "pure_sim" as const,
                            l: "Similaridade Pura",
                            tip: "Compara diretamente as cartas sem peso extra por colocação",
                          },
                        ]
                      ).map(({ v, l, tip }) => (
                        <button
                          key={v}
                          onClick={() => setKnnMode(v)}
                          title={tip}
                          className={`px-2.5 py-1 rounded-md font-bold inline-flex items-center gap-1 cursor-pointer ${
                            knnMode === v
                              ? "bg-indigo-600 text-white"
                              : "text-slate-400 hover:text-slate-200"
                          }`}
                        >
                          {v === "strict_top" && (
                            <Trophy className="w-3 h-3 text-amber-400" />
                          )}
                          {l}
                        </button>
                      ))}
                    </div>

                    <div className="flex items-center gap-1.5">
                      <span className="text-slate-400">Decks base:</span>
                      {[5, 8, 12].map((k) => (
                        <button
                          key={k}
                          onClick={() => setKNeighbors(k)}
                          className={`px-2 py-0.5 rounded font-bold cursor-pointer ${
                            kNeighbors === k
                              ? "bg-amber-500 text-slate-950"
                              : "bg-slate-800 text-slate-400 hover:text-white"
                          }`}
                        >
                          {k}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                {report.knnModeUsed === "balanced_fallback" && (
                  <div className="text-[11px] text-amber-300 bg-amber-500/10 border border-amber-500/25 rounded-lg px-2.5 py-1">
                    Este arquétipo possui menos de 3 listas exclusivas de Campeonato/M1 recentes — incluindo também listas Master V do mesmo arquétipo para evitar viés.
                  </div>
                )}

                {report.similarDecks.length > 0 && (
                  <div className="flex gap-2 overflow-x-auto pb-1">
                    {report.similarDecks.slice(0, 6).map((sim) => (
                      <button
                        key={sim.deckId}
                        onClick={() => onOpenMetaDeck(sim.deckId)}
                        className="shrink-0 text-left bg-[#0d111a] hover:bg-[#151c2b] border border-slate-800 hover:border-amber-500/40 rounded-lg p-2 flex items-center gap-2.5 min-w-[230px] cursor-pointer transition-colors"
                      >
                        <img
                          src={cardImageUrl(sim.coverCardId, 200)}
                          alt={sim.deckType}
                          loading="lazy"
                          className="w-9 h-12 object-cover rounded bg-slate-900 shrink-0"
                        />
                        <div className="min-w-0">
                          <div className="flex items-center gap-1.5">
                            <TierBadge tier={sim.tier} />
                            <span className="text-[11px] font-black text-emerald-400">
                              {sim.similarityPct}%
                            </span>
                          </div>
                          <div className="text-xs font-bold text-white truncate mt-0.5">
                            {sim.deckType}
                          </div>
                          <div className="text-[10px] text-amber-300/90 font-semibold truncate">
                            {sim.badgeLabel || sim.placement || "Top Deck"} • {sim.sharedCards} comuns
                          </div>
                        </div>
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* ===== AUTOMATIC BEFORE-VS-AFTER OPTIMIZER DIFF ===== */}
              {(report.autoOptimize.addedChanges.length > 0 ||
                report.autoOptimize.removedChanges.length > 0) && (
                <div className="bg-gradient-to-br from-[#121927] to-[#101522] border border-emerald-500/35 rounded-xl p-4 space-y-4">
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <div>
                      <h3 className="text-sm font-extrabold text-white flex items-center gap-2">
                        <Wand2 className="w-4 h-4 text-emerald-400" />
                        Otimização Automática — Antes vs Depois (e o Porquê)
                      </h3>
                      <p className="text-xs text-slate-400 mt-0.5">
                        Substituição automática baseada nos {report.kNeighbors} decks competitivos mais parecidos, respeitando os limites de Main/Extra Deck.
                      </p>
                    </div>

                    <div className="flex items-center gap-2 flex-wrap">
                      <button
                        onClick={() => handleApplyAutoOptimize(true)}
                        className="px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer"
                      >
                        <CopyPlus className="w-3.5 h-3.5" />
                        Salvar como Cópia Otimizada
                      </button>
                      <button
                        onClick={() => handleApplyAutoOptimize(false)}
                        className="px-3 py-1.5 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-slate-950 text-xs font-extrabold inline-flex items-center gap-1.5 cursor-pointer"
                      >
                        <Check className="w-3.5 h-3.5" />
                        Aplicar no Meu Deck
                      </button>
                    </div>
                  </div>

                  {/* Before vs After Metrics Strip */}
                  <div className="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
                    <div className="bg-[#0b0e14] border border-slate-800 rounded-lg p-3 flex items-center justify-between">
                      <div>
                        <div className="text-[10px] uppercase font-bold text-slate-400">
                          Alinhamento com o Meta
                        </div>
                        <div className="mt-1 flex items-center gap-2 text-sm font-black">
                          <span className="text-slate-400">
                            {report.autoOptimize.beforeMetaScore}%
                          </span>
                          <ArrowRight className="w-4 h-4 text-emerald-400" />
                          <span className="text-emerald-400">
                            {report.autoOptimize.afterMetaScore}%
                          </span>
                        </div>
                      </div>
                      {report.autoOptimize.afterMetaScore > report.autoOptimize.beforeMetaScore && (
                        <span className="px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-300 text-xs font-black">
                          +
                          {(
                            report.autoOptimize.afterMetaScore -
                            report.autoOptimize.beforeMetaScore
                          ).toFixed(1)}
                          %
                        </span>
                      )}
                    </div>

                    <div className="bg-[#0b0e14] border border-slate-800 rounded-lg p-3">
                      <div className="text-[10px] uppercase font-bold text-slate-400">
                        Tamanho (Main / Extra)
                      </div>
                      <div className="mt-1 flex items-center gap-2 text-xs font-bold text-slate-200">
                        <span>
                          {report.autoOptimize.beforeMainCount} /{" "}
                          {report.autoOptimize.beforeExtraCount}
                        </span>
                        <ArrowRight className="w-3.5 h-3.5 text-amber-400" />
                        <span className="text-amber-300">
                          {report.autoOptimize.afterMainCount} /{" "}
                          {report.autoOptimize.afterExtraCount}
                        </span>
                      </div>
                    </div>

                    <div className="bg-[#0b0e14] border border-slate-800 rounded-lg p-3">
                      <div className="text-[10px] uppercase font-bold text-slate-400 mb-1">
                        Custo de CP desta Otimização
                      </div>
                      <div className="flex items-center gap-1.5 flex-wrap">
                        <DustPill rarity="UR" cp={report.autoOptimize.urCpNeeded} />
                        <DustPill rarity="SR" cp={report.autoOptimize.srCpNeeded} />
                        <DustPill rarity="R" cp={report.autoOptimize.rCpNeeded} />
                        <DustPill rarity="N" cp={report.autoOptimize.nCpNeeded} />
                        {report.autoOptimize.urCpNeeded === 0 &&
                          report.autoOptimize.srCpNeeded === 0 &&
                          report.autoOptimize.rCpNeeded === 0 &&
                          report.autoOptimize.nCpNeeded === 0 && (
                            <span className="text-xs text-emerald-400 font-bold">0 CP</span>
                          )}
                      </div>
                    </div>
                  </div>

                  {/* Side-by-side Diff: SAI (Antes) vs ENTRA (Depois) with Reasons */}
                  <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
                    {/* REMOVED */}
                    <div className="bg-[#0b0e14] border border-rose-500/25 rounded-xl p-3 space-y-2">
                      <div className="text-xs font-extrabold text-rose-300 uppercase tracking-wider">
                        Sai do Deck ({report.autoOptimize.removedChanges.length})
                      </div>
                      {report.autoOptimize.removedChanges.length === 0 ? (
                        <p className="text-xs text-slate-500 py-3">Nenhuma carta removida.</p>
                      ) : (
                        <div className="space-y-1.5 max-h-60 overflow-y-auto pr-1">
                          {report.autoOptimize.removedChanges.map((ch) => (
                            <div
                              key={`rem-${ch.id}`}
                              className="flex items-start gap-2 p-2 rounded-lg bg-rose-950/20 border border-rose-500/20"
                            >
                              <img
                                src={cardImageUrl(ch.id, 200)}
                                alt={ch.name}
                                loading="lazy"
                                className="w-7 h-10 object-cover rounded shrink-0"
                              />
                              <div className="min-w-0 flex-1">
                                <div className="flex items-center gap-1.5">
                                  <span className="text-[11px] font-black text-rose-400">
                                    -{ch.delta}x
                                  </span>
                                  <RarityBadge rarity={ch.rarity} compact />
                                  <span className="text-xs font-bold text-slate-100 truncate">
                                    {ch.name}
                                  </span>
                                </div>
                                <div className="text-[11px] text-rose-200/80 mt-0.5">
                                  <strong>Por quê:</strong> {ch.reason}
                                </div>
                              </div>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>

                    {/* ADDED */}
                    <div className="bg-[#0b0e14] border border-emerald-500/25 rounded-xl p-3 space-y-2">
                      <div className="text-xs font-extrabold text-emerald-300 uppercase tracking-wider">
                        Entra no Deck ({report.autoOptimize.addedChanges.length})
                      </div>
                      {report.autoOptimize.addedChanges.length === 0 ? (
                        <p className="text-xs text-slate-500 py-3">Nenhuma carta adicionada.</p>
                      ) : (
                        <div className="space-y-1.5 max-h-60 overflow-y-auto pr-1">
                          {report.autoOptimize.addedChanges.map((ch) => (
                            <div
                              key={`add-${ch.id}`}
                              className="flex items-start gap-2 p-2 rounded-lg bg-emerald-950/20 border border-emerald-500/20"
                            >
                              <img
                                src={cardImageUrl(ch.id, 200)}
                                alt={ch.name}
                                loading="lazy"
                                className="w-7 h-10 object-cover rounded shrink-0"
                              />
                              <div className="min-w-0 flex-1">
                                <div className="flex items-center gap-1.5 flex-wrap">
                                  <span className="text-[11px] font-black text-emerald-400">
                                    +{ch.delta}x
                                  </span>
                                  <RarityBadge rarity={ch.rarity} compact />
                                  <span className="text-xs font-bold text-slate-100 truncate">
                                    {ch.name}
                                  </span>
                                  <span className="text-[10px] font-bold text-amber-300 ml-auto">
                                    {ch.cpCost} {ch.rarity}
                                  </span>
                                </div>
                                <div className="text-[11px] text-emerald-200/80 mt-0.5">
                                  <strong>Por quê:</strong> {ch.reason}
                                </div>
                              </div>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              )}

              {/* CP Roadmap Stages */}
              <div className="bg-[#121723] border border-slate-800 rounded-xl p-4 space-y-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <h3 className="text-sm font-bold text-white flex items-center gap-2">
                    <TrendingUp className="w-4 h-4 text-emerald-400" />
                    Planejamento de CP para Melhorar seu Deck
                  </h3>
                  <div className="flex items-center gap-1.5 flex-wrap">
                    <span className="text-[11px] text-slate-400">Total todas etapas:</span>
                    <DustPill rarity="UR" cp={report.totalUrCp} />
                    <DustPill rarity="SR" cp={report.totalSrCp} />
                    <DustPill rarity="R" cp={report.totalRCp} />
                  </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-3 gap-2.5">
                  {report.stages.map((st) => {
                    const active = stageFilter === st.stage;
                    return (
                      <button
                        key={st.stage}
                        onClick={() => setStageFilter(active ? "ALL" : st.stage)}
                        className={`text-left p-3 rounded-xl border transition-all cursor-pointer ${
                          active
                            ? "bg-amber-500/10 border-amber-500/60"
                            : "bg-[#0d111a] border-slate-800 hover:border-slate-700"
                        }`}
                      >
                        <div className="flex items-center justify-between gap-1">
                          <span className="text-xs font-extrabold text-white">{st.title}</span>
                          {st.cardCopies > 0 && st.canAffordWithBudget && (
                            <span className="text-[10px] font-black px-1.5 py-0.5 rounded bg-emerald-500/20 text-emerald-300">
                              DÁ AGORA
                            </span>
                          )}
                        </div>
                        <p className="text-[11px] text-slate-400 mt-0.5">{st.subtitle}</p>
                        <div className="mt-2 flex items-center gap-1 flex-wrap">
                          {st.cardCopies === 0 ? (
                            <span className="text-[11px] text-emerald-400 font-semibold">
                              Concluída!
                            </span>
                          ) : (
                            <>
                              <DustPill rarity="UR" cp={st.urCp} />
                              <DustPill rarity="SR" cp={st.srCp} />
                              <DustPill rarity="R" cp={st.rCp} />
                              <DustPill rarity="N" cp={st.nCp} />
                            </>
                          )}
                        </div>
                      </button>
                    );
                  })}
                </div>

                {(report.urDeficit > 0 || report.srDeficit > 0) && (
                  <div className="p-3 rounded-lg bg-[#0b0e14] border border-slate-800 text-xs text-slate-300 flex flex-wrap items-center justify-between gap-2">
                    <div>
                      Com seu saldo atual de pó no topo, ainda faltam{" "}
                      {report.urDeficit > 0 && (
                        <strong className="text-fuchsia-300">
                          {report.urDeficit} UR CP (~{report.urDismantleNeeded} URs para desmantelar)
                        </strong>
                      )}
                      {report.urDeficit > 0 && report.srDeficit > 0 && " e "}
                      {report.srDeficit > 0 && (
                        <strong className="text-amber-300">
                          {report.srDeficit} SR CP (~{report.srDismantleNeeded} SRs)
                        </strong>
                      )}
                      {" "}para completar todas as etapas.
                    </div>
                    <button
                      disabled={loadingPacks || report.upgrades.length === 0}
                      onClick={handleLoadUpgradePacks}
                      className="px-3 py-1.5 rounded-lg bg-fuchsia-600/20 hover:bg-fuchsia-600/30 border border-fuchsia-500/40 text-fuchsia-200 text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer disabled:opacity-40"
                    >
                      <Package className="w-3.5 h-3.5" />
                      {loadingPacks ? "Consultando..." : "Secret Packs dos Upgrades"}
                    </button>
                  </div>
                )}

                {upgradePacks && (
                  <div className="space-y-2 pt-1">
                    <div className="text-xs font-bold text-fuchsia-300">
                      Melhores Secret Packs para puxar seus upgrades antes de gastar CP:
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-2">
                      {upgradePacks.slice(0, 6).map((p) => (
                        <div
                          key={p.packName}
                          className="p-2.5 rounded-lg bg-[#0d111a] border border-fuchsia-500/25"
                        >
                          <div className="flex items-center justify-between gap-2">
                            <span className="text-xs font-bold text-white truncate">
                              {p.packName}
                            </span>
                            <span className="text-[10px] text-fuchsia-300 font-bold shrink-0">
                              {p.urMissingCopies} UR • {p.srMissingCopies} SR
                            </span>
                          </div>
                          <div className="text-[11px] text-slate-400 mt-1 truncate">
                            {p.cards.map((c) => `${c.name} (x${c.missing})`).join(", ")}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>

              {/* Upgrades + Possible Cuts side-by-side */}
              <div className="grid grid-cols-1 xl:grid-cols-12 gap-4 items-start">
                <div className="xl:col-span-7 bg-[#121723] border border-slate-800 rounded-xl p-4 space-y-3">
                  <div className="flex items-center justify-between flex-wrap gap-2">
                    <h3 className="text-sm font-bold text-white flex items-center gap-2">
                      <Sparkles className="w-4 h-4 text-amber-400" />
                      Cartas Recomendadas para Mirar ({filteredUpgrades.length})
                    </h3>
                    {stageFilter !== "ALL" && (
                      <button
                        onClick={() => setStageFilter("ALL")}
                        className="text-xs text-amber-400 hover:underline cursor-pointer"
                      >
                        Mostrar todas as etapas
                      </button>
                    )}
                  </div>

                  {filteredUpgrades.length === 0 ? (
                    <p className="text-xs text-emerald-400 py-6 text-center font-semibold">
                      Seu deck já está super alinhado com os decks vencedores nesta etapa!
                    </p>
                  ) : (
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-2 max-h-[460px] overflow-y-auto pr-1">
                      {filteredUpgrades.map((up) => (
                        <div
                          key={up.id}
                          className="flex items-center gap-2.5 p-2 rounded-lg bg-[#0d111a] border border-slate-800 hover:border-slate-700"
                        >
                          <img
                            src={cardImageUrl(up.id, 200)}
                            alt={up.name}
                            loading="lazy"
                            className="w-9 h-12 object-cover rounded shrink-0 bg-slate-900"
                          />
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-1.5">
                              <RarityBadge rarity={up.rarity} compact />
                              <span
                                className="text-xs font-semibold text-white truncate"
                                title={up.name}
                              >
                                {up.name}
                              </span>
                            </div>
                            <div className="text-[11px] text-slate-400 mt-0.5">
                              {up.currentQty}x →{" "}
                              <strong className="text-amber-300">{up.recommendedQty}x</strong> •{" "}
                              <strong className="text-emerald-400">{up.neighborUsagePct}%</strong>{" "}
                              •{" "}
                              <span className="text-fuchsia-300 font-semibold">
                                {up.cpCost} {up.rarity}
                              </span>
                            </div>
                          </div>
                          <button
                            onClick={() => handleApplyUpgrade(up)}
                            className="px-2.5 py-1.5 rounded-lg bg-emerald-500/15 hover:bg-emerald-500/30 border border-emerald-500/40 text-emerald-300 text-[11px] font-bold inline-flex items-center gap-1 shrink-0 cursor-pointer"
                          >
                            <Check className="w-3 h-3" /> Aplicar
                          </button>
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                <div className="xl:col-span-5 bg-[#121723] border border-slate-800 rounded-xl p-4 space-y-3">
                  <h3 className="text-sm font-bold text-white flex items-center gap-2">
                    <Scissors className="w-4 h-4 text-rose-400" />
                    Possíveis Cortes no seu Deck ({report.cuts.length})
                  </h3>
                  {report.cuts.length === 0 ? (
                    <p className="text-xs text-slate-400 py-6 text-center">
                      Nenhum corte óbvio detectado — sua lista está limpa!
                    </p>
                  ) : (
                    <div className="space-y-2 max-h-[460px] overflow-y-auto pr-1">
                      {report.cuts.map((cut) => (
                        <div
                          key={cut.id}
                          className="flex items-center gap-2.5 p-2 rounded-lg bg-rose-950/15 border border-rose-500/25"
                        >
                          <img
                            src={cardImageUrl(cut.id, 200)}
                            alt={cut.name}
                            loading="lazy"
                            className="w-8 h-11 object-cover rounded shrink-0"
                          />
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-1.5">
                              <RarityBadge rarity={cut.rarity} compact />
                              <span className="text-xs font-semibold text-slate-100 truncate">
                                {cut.name}
                              </span>
                            </div>
                            <div className="text-[11px] text-rose-300/90 mt-0.5">{cut.reason}</div>
                          </div>
                          <button
                            onClick={() =>
                              handleSetCardQty(
                                {
                                  id: cut.id,
                                  name: cut.name,
                                  rarity: cut.rarity,
                                  section: cut.section === "extra" ? "extra" : "main",
                                },
                                cut.recommendedQty
                              )
                            }
                            className="px-2.5 py-1.5 rounded-lg bg-rose-500/20 hover:bg-rose-500/30 border border-rose-500/40 text-rose-200 text-[11px] font-bold shrink-0 cursor-pointer"
                          >
                            {cut.recommendedQty === 0 ? "Remover" : `Para ${cut.recommendedQty}x`}
                          </button>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
};
