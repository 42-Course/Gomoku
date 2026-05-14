import { Bot, Sparkles, Zap, ZapOff } from "lucide-react";
import type { Analysis } from "@/api/types";
import { coordLabel, formatMs } from "@/lib/format";
import { cn } from "@/lib/cn";

interface AnalysisPanelProps {
  analysis: Analysis | null | undefined;
  isLoading?: boolean;
  /** True when the user could ask for analysis on the current move. */
  canAnalyze: boolean;
  autoAnalyze: boolean;
  onAnalyze: () => void;
  onToggleAutoAnalyze: () => void;
}

/**
 * Surfaces what the engine actually computed: the chosen move, the search
 * window (depth & node count), and the root-side score.
 *
 * When no analysis exists yet, offers a one-click "Analyze" button and a
 * persistent "Auto" toggle that runs analysis automatically on every move.
 */
export function AnalysisPanel({
  analysis,
  isLoading,
  canAnalyze,
  autoAnalyze,
  onAnalyze,
  onToggleAutoAnalyze,
}: AnalysisPanelProps) {
  if (isLoading) {
    return (
      <Shell>
        <div className="p-4 text-sm text-ink-muted">Running search…</div>
      </Shell>
    );
  }

  if (!analysis) {
    return (
      <Shell>
        <div className="flex flex-col items-center gap-3 p-6 text-center text-sm text-ink-muted">
          <Sparkles className="size-5 opacity-50" />
          <div>No analysis for this move yet.</div>
          {canAnalyze ? (
            <button
              onClick={onAnalyze}
              className={cn(
                "inline-flex items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-bg-0",
                "transition-colors hover:bg-accent/85",
              )}
            >
              <Bot className="size-3.5" /> Analyze this position
            </button>
          ) : (
            <div className="text-xs">
              The saved moves can't be replayed cleanly (rule violation), so
              the engine has nothing to analyze from.
            </div>
          )}
          <AutoToggle active={autoAnalyze} onClick={onToggleAutoAnalyze} />
        </div>
      </Shell>
    );
  }

  return (
    <Shell>
      <div className="flex items-center justify-between border-b border-border bg-bg-2 px-3 py-1.5">
        <div className="flex items-center gap-2 text-xs font-medium text-ink-strong">
          <Bot className="size-3.5 text-accent" /> AI search
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3 p-3 text-xs">
        <Stat
          label="chosen"
          value={analysis.chosen ? coordLabel(analysis.chosen) : "—"}
          mono
        />
        <Stat label="score" value={formatScore(analysis.rootScore)} mono />
        <Stat label="depth" value={analysis.depth.toString()} mono />
        <Stat
          label="nodes"
          value={analysis.nodesVisited.toLocaleString()}
          mono
        />
        <Stat label="think" value={formatMs(analysis.thinkMs)} mono />
      </div>

      <div className="flex items-center justify-end border-t border-border bg-bg-1 p-2">
        <AutoToggle active={autoAnalyze} onClick={onToggleAutoAnalyze} />
      </div>
    </Shell>
  );
}

function AutoToggle({
  active,
  onClick,
}: {
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "inline-flex items-center gap-1.5 rounded px-2 py-1 text-xs transition-colors",
        active
          ? "bg-accent/15 text-accent"
          : "text-ink-muted hover:bg-bg-2 hover:text-ink-strong",
      )}
      title={
        active
          ? "Auto-analyze is on — every move you land on runs a search"
          : "Auto-analyze is off — click Analyze on each move you want"
      }
    >
      {active ? <Zap className="size-3.5" /> : <ZapOff className="size-3.5" />}
      Auto
    </button>
  );
}

function formatScore(score: number): string {
  if (score >= 1_000_000) return "+mate";
  if (score <= -1_000_000) return "-mate";
  return `${score > 0 ? "+" : ""}${score}`;
}

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <div className="overflow-hidden rounded-md border border-border bg-bg-1">
      {children}
    </div>
  );
}

function Stat({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="flex flex-col rounded-md border border-border/50 bg-bg-0/50 px-2.5 py-1.5">
      <span className="text-[10px] uppercase tracking-wider text-ink-muted">
        {label}
      </span>
      <span
        className={cn("text-sm text-ink-strong", mono && "font-mono")}
      >
        {value}
      </span>
    </div>
  );
}
