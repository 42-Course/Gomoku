import { useState } from "react";
import {
  Bot,
  Eye,
  EyeOff,
  GitBranch,
  Sparkles,
  Zap,
  ZapOff,
} from "lucide-react";
import type { Analysis } from "@/api/types";
import { coordLabel, formatMs } from "@/lib/format";
import { cn } from "@/lib/cn";

interface AnalysisPanelProps {
  analysis: Analysis | null | undefined;
  isLoading?: boolean;
  /** True when the user could ask for analysis on the current move. */
  canAnalyze: boolean;
  showCandidates: boolean;
  showPrincipalVariation: boolean;
  autoAnalyze: boolean;
  onAnalyze: () => void;
  onToggleCandidates: () => void;
  onTogglePrincipalVariation: () => void;
  onToggleAutoAnalyze: () => void;
}

type Tab = "summary" | "candidates" | "pv";

/**
 * Surfaces what the engine actually computed: the chosen move, the search
 * window (depth & node count), the root-side score, the candidates the
 * engine considered, and the principal variation.
 *
 * When no analysis exists yet, offers a one-click "Analyze" button and a
 * persistent "Auto" toggle that runs analysis automatically on every move.
 */
export function AnalysisPanel({
  analysis,
  isLoading,
  canAnalyze,
  showCandidates,
  showPrincipalVariation,
  autoAnalyze,
  onAnalyze,
  onToggleCandidates,
  onTogglePrincipalVariation,
  onToggleAutoAnalyze,
}: AnalysisPanelProps) {
  const [tab, setTab] = useState<Tab>("summary");

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
        <div className="flex gap-1">
          <TabBtn active={tab === "summary"} onClick={() => setTab("summary")}>
            Summary
          </TabBtn>
          <TabBtn active={tab === "candidates"} onClick={() => setTab("candidates")}>
            Candidates
          </TabBtn>
          <TabBtn active={tab === "pv"} onClick={() => setTab("pv")}>
            PV
          </TabBtn>
        </div>
      </div>

      {tab === "summary" && (
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
          <Stat
            label="branches"
            value={analysis.candidates.length.toString()}
            mono
          />
        </div>
      )}

      {tab === "candidates" && (
        <div className="max-h-[280px] overflow-y-auto">
          {analysis.candidates.length === 0 && (
            <div className="p-3 text-xs text-ink-muted">
              No candidates were generated.
            </div>
          )}
          {analysis.candidates.map((c, i) => (
            <div
              key={`${c.coord.x}-${c.coord.y}-${i}`}
              className={cn(
                "grid grid-cols-[2rem_3rem_1fr_3.5rem] items-center gap-2 border-b border-border/50 px-3 py-1.5 text-xs last:border-b-0",
                c.pruned && "opacity-50",
              )}
            >
              <span className="font-mono text-ink-muted">#{i + 1}</span>
              <span className="font-mono text-ink-strong">
                {coordLabel(c.coord)}
              </span>
              <div className="flex items-center gap-2 text-[10px] text-ink-muted">
                <span className="font-mono">
                  {c.subtreeNodes.toLocaleString()} n
                </span>
                {c.pruned && (
                  <span className="rounded bg-danger/20 px-1 py-0 uppercase text-danger">
                    pruned
                  </span>
                )}
              </div>
              <span
                className={cn(
                  "text-right font-mono",
                  scoreColor(c.score),
                )}
              >
                {formatScore(c.score)}
              </span>
            </div>
          ))}
        </div>
      )}

      {tab === "pv" && (
        <div className="max-h-[280px] overflow-y-auto p-3">
          {analysis.principalVariation.length === 0 ? (
            <div className="text-xs text-ink-muted">
              No principal variation — search bottomed out at depth 0.
            </div>
          ) : (
            <ol className="flex flex-wrap gap-1.5 font-mono text-xs">
              {analysis.principalVariation.map((c, i) => (
                <li
                  key={`pv-${i}`}
                  className="flex items-center gap-1 rounded bg-bg-2 px-2 py-1"
                >
                  <span className="text-ink-muted">{i + 1}.</span>
                  <span className="text-ink-strong">{coordLabel(c)}</span>
                </li>
              ))}
            </ol>
          )}
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2 border-t border-border bg-bg-1 p-2">
        <ToggleBtn
          active={showCandidates}
          onClick={onToggleCandidates}
          icon={
            showCandidates ? (
              <Eye className="size-3.5" />
            ) : (
              <EyeOff className="size-3.5" />
            )
          }
        >
          Candidates
        </ToggleBtn>
        <ToggleBtn
          active={showPrincipalVariation}
          onClick={onTogglePrincipalVariation}
          icon={<GitBranch className="size-3.5" />}
        >
          PV
        </ToggleBtn>
        <div className="ml-auto">
          <AutoToggle active={autoAnalyze} onClick={onToggleAutoAnalyze} />
        </div>
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

function scoreColor(score: number): string {
  if (score > 300) return "text-accent";
  if (score < -300) return "text-danger";
  return "text-ink-strong";
}

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <div className="overflow-hidden rounded-md border border-border bg-bg-1">
      {children}
    </div>
  );
}

function TabBtn({
  active,
  children,
  ...rest
}: React.ButtonHTMLAttributes<HTMLButtonElement> & { active: boolean }) {
  return (
    <button
      className={cn(
        "rounded px-2 py-0.5 text-xs transition-colors",
        active
          ? "bg-bg-3 text-ink-strong"
          : "text-ink-muted hover:text-ink-strong",
      )}
      {...rest}
    >
      {children}
    </button>
  );
}

function ToggleBtn({
  active,
  icon,
  children,
  ...rest
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  active: boolean;
  icon: React.ReactNode;
}) {
  return (
    <button
      className={cn(
        "flex items-center gap-1.5 rounded px-2 py-1 text-xs transition-colors",
        active
          ? "bg-accent/15 text-accent"
          : "text-ink-muted hover:bg-bg-2 hover:text-ink-strong",
      )}
      {...rest}
    >
      {icon}
      {children}
    </button>
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
