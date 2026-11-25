import React, { useEffect, useState } from "react";
import {
  Check,
  Copy,
  Crosshair,
  Download,
  ExternalLink,
  GitBranch,
  Monitor,
  Package,
  Play,
  RotateCcw,
  SlidersHorizontal,
  Square,
  Wand2,
  X,
} from "lucide-react";
import { api } from "../api";
import {
  cardImageUrl,
  type AutoImportSummary,
  type DeckCard,
  type GameWindowStatus,
  type ImporterCalibration,
  type MetaDeck,
  type PackRecommendation,
} from "../types";
import { DustPill, RarityBadge, TierBadge } from "./Badges";

interface DeckModalProps {
  deck: MetaDeck;
  onClose: () => void;
  onCloneToMyDecks: (metaDeckId: string) => void;
}

export const DeckModal: React.FC<DeckModalProps> = ({
  deck,
  onClose,
  onCloneToMyDecks,
}) => {
  const [modalTab, setModalTab] = useState<"cards" | "packs" | "import">("cards");
  const [exporting, setExporting] = useState(false);
  const [copiedYdk, setCopiedYdk] = useState(false);

  const [packs, setPacks] = useState<PackRecommendation[] | null>(null);
  const [loadingPacks, setLoadingPacks] = useState(false);
  const [packsError, setPacksError] = useState<string | null>(null);

  const [sortedMain, setSortedMain] = useState<DeckCard[]>(deck.main);
  const [sortedExtra, setSortedExtra] = useState<DeckCard[]>(deck.extra);

  useEffect(() => {
    setSortedMain(deck.main);
    setSortedExtra(deck.extra);
    let active = true;
    api
      .sortDeckLists(deck.main, deck.extra)
      .then(([m, e]) => {
        if (active) {
          setSortedMain(m);
          setSortedExtra(e);
        }
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [deck]);

  const [cal, setCal] = useState<ImporterCalibration>({
    searchRx: 0.7781,
    searchRy: 0.1911,
    firstCardRx: 0.705,
    firstCardRy: 0.35,
    stepDelayMs: 700,
    clickDelayMs: 220,
    customTuned: false,
  });
  const [winStatus, setWinStatus] = useState<GameWindowStatus | null>(null);
  const [showFineTune, setShowFineTune] = useState(false);
  const [testingAim, setTestingAim] = useState(false);
  const [capturingTarget, setCapturingTarget] = useState<"search" | "card" | null>(null);
  const [importingToGame, setImportingToGame] = useState(false);
  const [importSummary, setImportSummary] = useState<AutoImportSummary | null>(null);
  const [importError, setImportError] = useState<string | null>(null);

  useEffect(() => {
    api.getImporterCalibration().then(setCal).catch(() => {});
  }, []);

  useEffect(() => {
    if (modalTab !== "import") return;
    let active = true;
    const poll = () => {
      api
        .getGameWindowStatus()
        .then((s) => active && setWinStatus(s))
        .catch(() => {});
    };
    poll();
    const id = window.setInterval(poll, 1500);
    return () => {
      active = false;
      window.clearInterval(id);
    };
  }, [modalTab]);

  useEffect(() => {
    if (modalTab !== "packs" || packs !== null) return;
    let cancelled = false;
    (async () => {
      setLoadingPacks(true);
      setPacksError(null);
      try {
        const res = await api.getDeckSecretPacks(deck.id);
        if (!cancelled) setPacks(res);
      } catch (err) {
        if (!cancelled) setPacksError(String(err));
      } finally {
        if (!cancelled) setLoadingPacks(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [modalTab, packs, deck.id]);

  const handleCapturePoint = async (target: "search" | "card") => {
    setCapturingTarget(target);
    setImportError(null);
    try {
      const [rx, ry] = await api.captureRelativePoint(3);
      const next: ImporterCalibration = {
        ...cal,
        searchRx: target === "search" ? rx : cal.searchRx,
        searchRy: target === "search" ? ry : cal.searchRy,
        firstCardRx: target === "card" ? rx : cal.firstCardRx,
        firstCardRy: target === "card" ? ry : cal.firstCardRy,
        customTuned: true,
      };
      const saved = await api.saveImporterCalibration(next);
      setCal(saved);
      const st = await api.getGameWindowStatus();
      setWinStatus(st);
    } catch (err) {
      setImportError(String(err));
    } finally {
      setCapturingTarget(null);
    }
  };

  const handleResetAuto = async () => {
    try {
      const saved = await api.resetImporterCalibration();
      setCal(saved);
      const st = await api.getGameWindowStatus();
      setWinStatus(st);
    } catch (err) {
      setImportError(String(err));
    }
  };

  const handleTestAim = async () => {
    setTestingAim(true);
    setImportError(null);
    try {
      const st = await api.testImporterAim();
      setWinStatus(st);
    } catch (err) {
      setImportError(String(err));
    } finally {
      setTestingAim(false);
    }
  };

  const handleDelayChange = async (field: "stepDelayMs" | "clickDelayMs", ms: number) => {
    const next = { ...cal, [field]: ms };
    setCal(next);
    try {
      await api.saveImporterCalibration(next);
    } catch {
      // ignore
    }
  };

  const handleStartAutoImport = async () => {
    setImportingToGame(true);
    setImportSummary(null);
    setImportError(null);
    try {
      const summary = await api.runAutoImportDeck(deck.id);
      setImportSummary(summary);
    } catch (err) {
      setImportError(String(err));
    } finally {
      setImportingToGame(false);
    }
  };

  const handleExportYdk = async (mode: "copy" | "download") => {
    try {
      setExporting(true);
      const ydk = await api.exportDeckYdk(deck.id);
      if (mode === "copy") {
        await navigator.clipboard.writeText(ydk);
        setCopiedYdk(true);
        setTimeout(() => setCopiedYdk(false), 2000);
      } else {
        await api.saveYdkToDisk(deck.deckType, ydk);
      }
    } catch (err) {
      alert(`Erro ao exportar .YDK: ${err}`);
    } finally {
      setExporting(false);
    }
  };

  const renderCardRow = (c: DeckCard) => (
    <div
      key={`${c.section}-${c.id}`}
      className="flex items-center gap-3 p-2 rounded-lg bg-[#151b28] border border-slate-800/90"
    >
      <img
        src={cardImageUrl(c.id, 200)}
        alt={c.name}
        loading="lazy"
        className="w-10 h-14 object-cover rounded shadow bg-slate-900 shrink-0"
      />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <RarityBadge rarity={c.rarity} compact />
          <span className="text-sm font-medium text-slate-100 truncate" title={c.name}>
            {c.name}
          </span>
        </div>
        <div className="mt-1 text-xs text-slate-400">
          Quantidade: <strong className="text-amber-300">{c.amount}x</strong>
        </div>
      </div>
    </div>
  );

  const mainCount = deck.main.reduce((s, c) => s + c.amount, 0);
  const extraCount = deck.extra.reduce((s, c) => s + c.amount, 0);

  return (
    <div
      className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4"
      onClick={onClose}
    >
      <div
        className="bg-[#0f131c] border border-slate-700/80 rounded-2xl w-full max-w-5xl max-h-[90vh] flex flex-col shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="p-5 border-b border-slate-800 bg-gradient-to-r from-[#131926] to-[#0f131c] flex flex-wrap items-start justify-between gap-4">
          <div className="flex items-center gap-4">
            {deck.coverCardId && (
              <img
                src={cardImageUrl(deck.coverCardId, 260)}
                alt={deck.deckType}
                className="w-16 h-22 object-cover rounded-lg border border-slate-700 shadow-lg"
              />
            )}
            <div>
              <div className="flex items-center gap-2 flex-wrap">
                <TierBadge tier={deck.tier} />
                <span
                  className={`px-2 py-0.5 text-[11px] font-bold rounded border ${
                    deck.category === "tournament"
                      ? "bg-amber-500/20 text-amber-300 border-amber-400/35"
                      : deck.category === "master1"
                      ? "bg-fuchsia-500/20 text-fuchsia-200 border-fuchsia-400/35"
                      : deck.category === "duelist_cup"
                      ? "bg-cyan-500/20 text-cyan-300 border-cyan-400/35"
                      : "bg-indigo-500/20 text-indigo-300 border-indigo-400/30"
                  }`}
                >
                  {deck.badgeLabel || deck.placement || "Top Deck"}
                </span>
              </div>
              <h2 className="text-xl font-extrabold text-white mt-1.5">{deck.deckType}</h2>
              <p className="text-xs text-slate-400 mt-0.5">
                {deck.eventName || deck.badgeLabel || "Top Deck"} • {mainCount} Main / {extraCount} Extra
              </p>
              <div className="flex items-center gap-1.5 mt-2 flex-wrap">
                <DustPill rarity="UR" cp={deck.urPrice} />
                <DustPill rarity="SR" cp={deck.srPrice} />
              </div>
            </div>
          </div>

          <button
            onClick={onClose}
            className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 cursor-pointer"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Action Toolbar */}
        <div className="px-5 py-3 border-b border-slate-800/80 bg-[#111622] flex flex-wrap items-center justify-between gap-3">
          <div className="flex bg-[#0b0e14] p-0.5 rounded-lg border border-slate-800">
            <button
              onClick={() => setModalTab("cards")}
              className={`px-3 py-1 rounded-md text-xs font-bold cursor-pointer ${
                modalTab === "cards"
                  ? "bg-amber-500 text-slate-950"
                  : "text-slate-400 hover:text-white"
              }`}
            >
              Lista de Cartas
            </button>
            <button
              onClick={() => setModalTab("packs")}
              className={`px-3 py-1 rounded-md text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer ${
                modalTab === "packs"
                  ? "bg-fuchsia-500 text-white"
                  : "text-fuchsia-300 hover:text-white"
              }`}
            >
              <Package className="w-3.5 h-3.5" />
              Secret Packs
            </button>
            <button
              onClick={() => setModalTab("import")}
              className={`px-3 py-1 rounded-md text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer ${
                modalTab === "import"
                  ? "bg-emerald-500 text-slate-950"
                  : "text-emerald-300 hover:text-white"
              }`}
            >
              <Wand2 className="w-3.5 h-3.5" />
              Montar no Jogo
            </button>
          </div>

          <div className="flex items-center gap-2 flex-wrap">
            <button
              onClick={() => onCloneToMyDecks(deck.id)}
              className="px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer"
            >
              <GitBranch className="w-3.5 h-3.5" />
              Clonar para Meus Decks
            </button>
            <button
              disabled={exporting}
              onClick={() => handleExportYdk("copy")}
              className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 text-xs font-semibold inline-flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
            >
              {copiedYdk ? (
                <Check className="w-3.5 h-3.5 text-emerald-400" />
              ) : (
                <Copy className="w-3.5 h-3.5" />
              )}
              {copiedYdk ? "YDK Copiado!" : "Copiar .YDK"}
            </button>
            <button
              disabled={exporting}
              onClick={() => handleExportYdk("download")}
              className="px-3 py-1.5 rounded-lg bg-amber-500 hover:bg-amber-400 text-slate-950 text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
            >
              <Download className="w-3.5 h-3.5" />
              .YDK
            </button>
            <a
              href={deck.mdmUrl}
              target="_blank"
              rel="noreferrer"
              className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-300 text-xs font-semibold inline-flex items-center gap-1.5"
            >
              <ExternalLink className="w-3.5 h-3.5" />
              MDM
            </a>
          </div>
        </div>

        {/* Body */}
        <div className="p-5 overflow-y-auto space-y-6 flex-1">
          {modalTab === "cards" ? (
            <>
              <div>
                <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 mb-2.5">
                  Main Deck ({mainCount})
                </h3>
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2">
                  {sortedMain.map(renderCardRow)}
                </div>
              </div>
              {sortedExtra.length > 0 && (
                <div>
                  <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 mb-2.5">
                    Extra Deck ({extraCount})
                  </h3>
                  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2">
                    {sortedExtra.map(renderCardRow)}
                  </div>
                </div>
              )}
            </>
          ) : modalTab === "import" ? (
            <div className="space-y-4 max-w-3xl">
              <div>
                <h3 className="text-sm font-bold text-white flex items-center gap-2">
                  <Wand2 className="w-4 h-4 text-emerald-400" />
                  Auto-Montar Inteligente (Detecção Automática Multi-Monitor)
                </h3>
                <p className="text-xs text-slate-400 mt-1">
                  O DuelCraft detecta automaticamente onde está a janela do <strong>Master Duel</strong> (em qualquer monitor, janela ou tela cheia) e calcula a posição da busca e da 1ª carta em tempo real — sem precisar recalibrar quando mover a tela! Aperte <kbd className="px-1.5 py-0.5 rounded bg-slate-800 text-amber-300 font-mono text-[11px]">ESC</kbd> para parar a qualquer momento.
                </p>
              </div>

              {/* Live Window Detection Status */}
              <div
                className={`rounded-xl border p-4 flex flex-wrap items-center justify-between gap-3 ${
                  winStatus?.found && !winStatus.minimized
                    ? "bg-emerald-950/20 border-emerald-500/40"
                    : "bg-amber-950/20 border-amber-500/30"
                }`}
              >
                <div className="flex items-center gap-3">
                  <div
                    className={`w-9 h-9 rounded-lg flex items-center justify-center ${
                      winStatus?.found && !winStatus.minimized
                        ? "bg-emerald-500/20 text-emerald-300"
                        : "bg-amber-500/20 text-amber-300"
                    }`}
                  >
                    <Monitor className="w-5 h-5" />
                  </div>
                  <div>
                    <div className="text-xs font-extrabold text-white">
                      {winStatus?.found
                        ? winStatus.minimized
                          ? "Master Duel detectado (Minimizado — será restaurado ao iniciar)"
                          : `Master Duel detectado • ${winStatus.clientWidth}x${winStatus.clientHeight} em (${winStatus.clientX}, ${winStatus.clientY})`
                        : "Aguardando janela do Master Duel..."}
                    </div>
                    <div className="text-[11px] text-slate-400 mt-0.5">
                      {winStatus?.found && !winStatus.minimized
                        ? `Mira automática: Busca (${winStatus.searchX}, ${winStatus.searchY}) • 1ª Carta (${winStatus.firstCardX}, ${winStatus.firstCardY})`
                        : "Deixe o Master Duel aberto na tela de Edição de Deck (em qualquer monitor)."}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  <button
                    disabled={testingAim || importingToGame}
                    onClick={handleTestAim}
                    className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-xs font-bold text-slate-200 inline-flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
                  >
                    <Crosshair className="w-3.5 h-3.5 text-amber-400" />
                    {testingAim ? "Testando..." : "Testar Mira"}
                  </button>
                  <button
                    onClick={() => setShowFineTune((v) => !v)}
                    className="px-2.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-xs font-semibold text-slate-300 inline-flex items-center gap-1 cursor-pointer"
                  >
                    <SlidersHorizontal className="w-3.5 h-3.5" />
                    Ajuste Fino
                  </button>
                </div>
              </div>

              {/* Optional Relative Fine-Tuning (Saved as window ratios, still works across monitors!) */}
              {showFineTune && (
                <div className="bg-[#141a28] border border-slate-800 rounded-xl p-4 space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-bold text-slate-200">
                      Ajuste Fino Relativo {cal.customTuned ? "(Personalizado)" : "(Padrão 16:9)"}
                    </span>
                    {cal.customTuned && (
                      <button
                        onClick={handleResetAuto}
                        className="text-[11px] text-amber-400 hover:underline inline-flex items-center gap-1 cursor-pointer"
                      >
                        <RotateCcw className="w-3 h-3" /> Restaurar Padrão Automático
                      </button>
                    )}
                  </div>
                  <p className="text-[11px] text-slate-400">
                    Mesmo ajustando aqui, o DuelCraft salva em <strong>proporção relativa da janela</strong> — você pode continuar movendo o jogo entre monitores sem recalibrar!
                  </p>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-2.5">
                    <button
                      disabled={capturingTarget !== null || importingToGame}
                      onClick={() => handleCapturePoint("search")}
                      className="py-2 px-3 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-xs font-bold text-amber-300 inline-flex items-center justify-center gap-2 cursor-pointer disabled:opacity-50"
                    >
                      <Crosshair className="w-3.5 h-3.5" />
                      {capturingTarget === "search"
                        ? "Aponte na Busca (3s)..."
                        : `Ajustar Busca (${(cal.searchRx * 100).toFixed(1)}%, ${(cal.searchRy * 100).toFixed(1)}%)`}
                    </button>
                    <button
                      disabled={capturingTarget !== null || importingToGame}
                      onClick={() => handleCapturePoint("card")}
                      className="py-2 px-3 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-xs font-bold text-amber-300 inline-flex items-center justify-center gap-2 cursor-pointer disabled:opacity-50"
                    >
                      <Crosshair className="w-3.5 h-3.5" />
                      {capturingTarget === "card"
                        ? "Aponte na 1ª Carta (3s)..."
                        : `Ajustar 1ª Carta (${(cal.firstCardRx * 100).toFixed(1)}%, ${(cal.firstCardRy * 100).toFixed(1)}%)`}
                    </button>
                  </div>
                </div>
              )}

              <div className="bg-[#141a28] border border-slate-800 rounded-xl p-4 flex flex-wrap items-center justify-between gap-4">
                <div className="flex flex-wrap items-center gap-5">
                  <div className="space-y-1">
                    <label className="text-xs font-bold text-slate-200 block">
                      Espera da busca: <span className="text-amber-400">{cal.stepDelayMs}ms</span>
                    </label>
                    <input
                      type="range"
                      min={300}
                      max={1800}
                      step={50}
                      value={cal.stepDelayMs}
                      onChange={(e) => handleDelayChange("stepDelayMs", Number(e.target.value))}
                      className="w-40 accent-amber-500"
                    />
                  </div>
                  <div className="space-y-1">
                    <label className="text-xs font-bold text-slate-200 block">
                      Entre cópias: <span className="text-emerald-400">{cal.clickDelayMs}ms</span>
                    </label>
                    <input
                      type="range"
                      min={140}
                      max={500}
                      step={20}
                      value={cal.clickDelayMs}
                      onChange={(e) => handleDelayChange("clickDelayMs", Number(e.target.value))}
                      className="w-40 accent-emerald-500"
                    />
                  </div>
                </div>
                {importingToGame ? (
                  <button
                    onClick={() => api.cancelAutoImport()}
                    className="px-5 py-2.5 rounded-xl bg-rose-500 hover:bg-rose-400 text-white text-xs font-extrabold inline-flex items-center gap-2 cursor-pointer"
                  >
                    <Square className="w-3.5 h-3.5" /> Parar (ESC)
                  </button>
                ) : (
                  <button
                    disabled={capturingTarget !== null || testingAim}
                    onClick={handleStartAutoImport}
                    className="px-5 py-2.5 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 text-xs font-extrabold inline-flex items-center gap-2 shadow-lg shadow-emerald-500/20 cursor-pointer disabled:opacity-40"
                  >
                    <Play className="w-3.5 h-3.5" /> Montar Automaticamente no Master Duel
                  </button>
                )}
              </div>

              {importSummary && (
                <div className="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-200 text-xs">
                  {importSummary.message}
                </div>
              )}
              {importError && (
                <div className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/30 text-rose-300 text-xs">
                  {importError}
                </div>
              )}
            </div>
          ) : (
            <div className="space-y-3">
              <h3 className="text-sm font-bold text-white flex items-center gap-2">
                <Package className="w-4 h-4 text-fuchsia-400" />
                Secret Packs & Selection Packs deste Deck
              </h3>
              {loadingPacks ? (
                <div className="py-10 text-center text-xs text-slate-400">
                  Consultando packs no Master Duel Meta...
                </div>
              ) : packsError ? (
                <div className="p-3 rounded-lg bg-rose-500/10 border border-rose-500/30 text-xs text-rose-300">
                  {packsError}
                </div>
              ) : (
                <div className="space-y-3">
                  {(packs ?? []).map((pack) => (
                    <div
                      key={pack.packName}
                      className={`rounded-xl border p-4 ${
                        pack.packType === "Master Pack / Craft"
                          ? "bg-slate-900/50 border-slate-800"
                          : "bg-[#141a28] border-fuchsia-500/30"
                      }`}
                    >
                      <div className="flex flex-wrap items-center justify-between gap-2 mb-3">
                        <div>
                          <span className="text-[10px] font-bold uppercase tracking-wider px-2 py-0.5 rounded bg-slate-800 text-fuchsia-300 border border-fuchsia-500/30">
                            {pack.packType}
                          </span>
                          <h4 className="text-sm font-extrabold text-white mt-1">
                            {pack.packName}
                          </h4>
                        </div>
                        <div className="flex items-center gap-2">
                          {pack.urMissingCopies > 0 && (
                            <span className="text-xs font-bold px-2.5 py-1 rounded-lg bg-fuchsia-500/15 text-fuchsia-200 border border-fuchsia-400/30">
                              {pack.urMissingCopies} UR
                            </span>
                          )}
                          {pack.srMissingCopies > 0 && (
                            <span className="text-xs font-bold px-2.5 py-1 rounded-lg bg-amber-500/15 text-amber-200 border border-amber-400/30">
                              {pack.srMissingCopies} SR
                            </span>
                          )}
                        </div>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        {pack.cards.map((c) => (
                          <div
                            key={c.id}
                            className="flex items-center gap-2 bg-[#0d111a] border border-slate-800 rounded-lg px-2.5 py-1.5"
                          >
                            <img
                              src={cardImageUrl(c.id, 200)}
                              alt={c.name}
                              loading="lazy"
                              className="w-6 h-8 object-cover rounded"
                            />
                            <RarityBadge rarity={c.rarity} compact />
                            <span className="text-xs text-slate-200 font-medium">{c.name}</span>
                            <span className="text-[11px] font-bold text-amber-300">x{c.missing}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
