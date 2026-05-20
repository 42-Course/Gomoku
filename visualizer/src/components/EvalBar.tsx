import { cn } from "@/lib/cn";

interface EvalBarProps {
  /**
   * Root-mover's score from the current analysis. `null` means analysis
   * isn't available yet — the bar renders disabled (50/50 grey).
   */
  rootScore: number | null;
  /** Whose turn it was at the analyzed position (the "root mover"). */
  rootSide: "black" | "white";
}

const WIN_THRESHOLD = 1_000_000;

/**
 * Lichess-style vertical eval bar.
 *
 * The visualizer's analysis score is in the root-mover's frame. A score
 * of `+250` for black-to-move means "black is up by 250 cp"; the same
 * `+250` for white-to-move means "white is up by 250". We translate to a
 * stable white-side perspective ("how white is doing") so the bar's
 * direction has a fixed meaning regardless of whose turn it is.
 *
 * Win probability is a logistic squash of the centipawn-ish score. The
 * scaling constant is empirical — Gomoku scores live on a different
 * scale than chess but `~600` per "winning" feels right against the
 * engine's evaluator.
 */
export function EvalBar({ rootScore, rootSide }: EvalBarProps) {
  const disabled = rootScore === null;

  const whiteScore = rootScore === null
    ? 0
    : rootSide === "white"
      ? rootScore
      : -rootScore;

  // Clamp terminal mate scores so the bar pins instead of overflowing.
  const clamped =
    whiteScore >= WIN_THRESHOLD ? 9999 : whiteScore <= -WIN_THRESHOLD ? -9999 : whiteScore;
  const winProb = 1 / (1 + Math.exp(-clamped / 600)); // 0..1, white winning

  // The bar is split top=white, bottom=black; pct is how much of the
  // bar belongs to white.
  const whitePct = disabled ? 50 : Math.max(2, Math.min(98, winProb * 100));

  const label = disabled
    ? "—"
    : whiteScore >= WIN_THRESHOLD
      ? "+M"
      : whiteScore <= -WIN_THRESHOLD
        ? "-M"
        : `${whiteScore > 0 ? "+" : ""}${whiteScore}`;

  return (
    <div
      className={cn(
        "relative flex h-full w-7 flex-col overflow-hidden rounded border border-border",
        disabled && "opacity-40",
      )}
      title={
        disabled
          ? "Run analysis to fill the bar"
          : `White ${winProb >= 0.5 ? "+" : ""}${(winProb * 100 - 50).toFixed(0)}%`
      }
    >
      {/* white side (top) */}
      <div
        className="bg-stone-white transition-[height] duration-300 ease-out"
        style={{ height: `${whitePct}%` }}
      />
      {/* black side (bottom) — fills the rest */}
      <div className="flex-1 bg-stone-black" />

      {/* score chip pinned to whichever side is leading */}
      <div
        className={cn(
          "pointer-events-none absolute left-1/2 -translate-x-1/2 font-mono text-[10px] tracking-tight",
          disabled
            ? "top-1/2 -translate-y-1/2 text-ink-muted"
            : whitePct >= 50
              ? "top-1 text-ink-strong"
              : "bottom-1 text-bg-1",
        )}
      >
        {label}
      </div>
    </div>
  );
}
