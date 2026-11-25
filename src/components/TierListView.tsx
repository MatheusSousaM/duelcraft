import React from "react";
import { Flame, TrendingUp } from "lucide-react";
import { cardImageUrl, type MetaArchetype } from "../types";
import { TierBadge } from "./Badges";

interface TierListViewProps {
  archetypes: MetaArchetype[];
  onSelectArchetype: (name: string) => void;
}

export const TierListView: React.FC<TierListViewProps> = ({
  archetypes,
  onSelectArchetype,
}) => {
  const tiers = [
    { level: 1, title: "Tier 1 — Meta Dominante", accent: "border-rose-500/40 bg-rose-500/5" },
    { level: 2, title: "Tier 2 — Fortíssimos", accent: "border-amber-500/40 bg-amber-500/5" },
    { level: 3, title: "Tier 3 — Competitivos", accent: "border-sky-500/40 bg-sky-500/5" },
    { level: 4, title: "Rogue & Alto Potencial", accent: "border-slate-700/80 bg-slate-900/40" },
  ];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between flex-wrap gap-2">
        <div>
          <h2 className="text-lg font-bold text-white flex items-center gap-2">
            <Flame className="w-5 h-5 text-amber-400" />
            Tier List — Power Rankings (Master Duel Meta)
          </h2>
          <p className="text-xs text-slate-400 mt-0.5">
            Clique em qualquer arquétipo para filtrar os decks e calcular quanto de pó falta na sua coleção.
          </p>
        </div>
      </div>

      {tiers.map(({ level, title, accent }) => {
        const list = archetypes.filter((a) => a.tier === level);
        if (list.length === 0) return null;

        return (
          <section key={level} className={`rounded-xl border p-4 ${accent}`}>
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <TierBadge tier={level} />
                <h3 className="text-sm font-bold text-slate-100">{title}</h3>
              </div>
              <span className="text-xs text-slate-400">{list.length} arquétipos</span>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3">
              {list.map((arch) => (
                <button
                  key={arch.id}
                  onClick={() => onSelectArchetype(arch.name)}
                  className="text-left group bg-[#121723] hover:bg-[#171e2e] border border-slate-800 hover:border-amber-500/50 rounded-xl p-3 flex items-center gap-3 transition-all cursor-pointer"
                >
                  <img
                    src={cardImageUrl(arch.coverCardId, 200)}
                    alt={arch.name}
                    loading="lazy"
                    className="w-12 h-16 object-cover rounded-lg bg-slate-900 border border-slate-700/60 shrink-0 group-hover:scale-105 transition-transform"
                  />
                  <div className="min-w-0 flex-1">
                    <div className="font-bold text-sm text-white truncate group-hover:text-amber-300 transition-colors">
                      {arch.name}
                    </div>
                    <div className="mt-1 flex items-center gap-2 text-xs text-slate-400">
                      <span className="inline-flex items-center gap-1 text-amber-300 font-semibold">
                        <TrendingUp className="w-3 h-3" />
                        Power {arch.tournamentPower.toFixed(1)}
                      </span>
                      {arch.popRank > 0 && <span>• Pop {arch.popRank.toFixed(1)}%</span>}
                    </div>
                    {(arch.avgUrPrice > 0 || arch.avgSrPrice > 0) && (
                      <div className="mt-1.5 flex items-center gap-2 text-[11px] text-slate-400">
                        <span className="text-fuchsia-300 font-semibold">~{arch.avgUrPrice} UR</span>
                        <span className="text-amber-300 font-semibold">~{arch.avgSrPrice} SR</span>
                      </div>
                    )}
                  </div>
                </button>
              ))}
            </div>
          </section>
        );
      })}
    </div>
  );
};
