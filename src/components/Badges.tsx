import React from "react";

export const RarityBadge: React.FC<{ rarity: string; compact?: boolean }> = ({
  rarity,
  compact = false,
}) => {
  const r = (rarity || "N").toUpperCase();
  const base = compact
    ? "px-1.5 py-0.5 text-[10px] leading-none rounded font-bold tracking-wide inline-flex items-center justify-center"
    : "px-2 py-0.5 text-xs leading-none rounded font-bold tracking-wide inline-flex items-center justify-center";

  if (r === "UR") {
    return <span className={`${base} ur-badge shadow-sm shadow-fuchsia-500/20`}>UR</span>;
  }
  if (r === "SR") {
    return (
      <span className={`${base} bg-amber-400/20 text-amber-300 border border-amber-400/40`}>
        SR
      </span>
    );
  }
  if (r === "R") {
    return (
      <span className={`${base} bg-sky-500/20 text-sky-300 border border-sky-400/40`}>R</span>
    );
  }
  return (
    <span className={`${base} bg-slate-700/60 text-slate-300 border border-slate-600/60`}>N</span>
  );
};

export const TierBadge: React.FC<{ tier: number }> = ({ tier }) => {
  if (tier === 1) {
    return (
      <span className="px-2 py-0.5 text-[11px] font-extrabold uppercase tracking-wider rounded bg-rose-500/20 text-rose-300 border border-rose-500/40">
        Tier 1
      </span>
    );
  }
  if (tier === 2) {
    return (
      <span className="px-2 py-0.5 text-[11px] font-extrabold uppercase tracking-wider rounded bg-amber-500/20 text-amber-300 border border-amber-500/40">
        Tier 2
      </span>
    );
  }
  if (tier === 3) {
    return (
      <span className="px-2 py-0.5 text-[11px] font-extrabold uppercase tracking-wider rounded bg-sky-500/20 text-sky-300 border border-sky-500/40">
        Tier 3
      </span>
    );
  }
  return (
    <span className="px-2 py-0.5 text-[11px] font-bold uppercase tracking-wider rounded bg-slate-700/50 text-slate-400 border border-slate-600/40">
      Rogue
    </span>
  );
};

export const DustPill: React.FC<{ rarity: "UR" | "SR" | "R" | "N"; cp: number }> = ({
  rarity,
  cp,
}) => {
  if (cp <= 0) return null;
  const styles: Record<string, string> = {
    UR: "bg-fuchsia-500/15 text-fuchsia-200 border-fuchsia-400/30",
    SR: "bg-amber-500/15 text-amber-200 border-amber-400/30",
    R: "bg-sky-500/15 text-sky-200 border-sky-400/30",
    N: "bg-slate-700/40 text-slate-300 border-slate-600/40",
  };
  return (
    <span
      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-md border text-xs font-semibold ${styles[rarity]}`}
    >
      <RarityBadge rarity={rarity} compact />
      <span>{cp}</span>
    </span>
  );
};
