import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { ArrowLeft, Circle, Clock } from "lucide-react";
import type { Analysis } from "@/api/types";
import { getAnalysis, getGame, queryKeys } from "@/api/client";
import { boardAtMove, emptyBoard } from "@/api/fixtures";
import { Board } from "@/components/Board";
import { BoardSettings } from "@/components/BoardSettings";
import { EvalBar } from "@/components/EvalBar";
import { MoveTable } from "@/components/MoveTable";
import { TimelineScrubber } from "@/components/TimelineScrubber";
import { AnalysisPanel } from "@/components/AnalysisPanel";
import { useGameView } from "@/store/gameView";
import { coordLabel, formatMs } from "@/lib/format";
import { ANY_DEPTH, AUTO_MAX_BUDGET_MS, AUTO_START_BUDGET_MS } from "@/lib/search";

export function GameDetail() {
  const { id = "" } = useParams();

  const { data: game, isLoading } = useQuery({
    queryKey: queryKeys.game(id),
    queryFn: () => getGame(id),
    enabled: !!id,
  });

  const selectedMoveIndex = useGameView((s) => s.selectedMoveIndex);
  const setSelected = useGameView((s) => s.setSelected);
  const step = useGameView((s) => s.step);
  const showLastMove = useGameView((s) => s.showLastMove);
  const showCoordinates = useGameView((s) => s.showCoordinates);
  const showCrosshair = useGameView((s) => s.showCrosshair);
  const showHoverGhost = useGameView((s) => s.showHoverGhost);
  const showEvalBar = useGameView((s) => s.showEvalBar);
  const autoAnalyze = useGameView((s) => s.autoAnalyze);
  const toggle = useGameView((s) => s.toggle);
  const [hoveredCell, setHoveredCell] = useState<{ x: number; y: number } | null>(null);

  /**
   * Per-move "user clicked Analyze" set. Lives in component state so it
   * resets when navigating away — analysis results themselves stay in
   * react-query's cache and are reused if the user comes back.
   */
  const [requestedSet, setRequestedSet] = useState<Set<number>>(new Set());

  const maxIndex = (game?.moves.length ?? 0) - 1;

  useEffect(() => {
    if (game) setSelected(game.moves.length - 1);
  }, [game, setSelected]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement) return;
      if (e.key === "ArrowLeft") step(-1, maxIndex);
      else if (e.key === "ArrowRight") step(1, maxIndex);
      else if (e.key === "Home") setSelected(-1);
      else if (e.key === "End") setSelected(maxIndex);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [step, setSelected, maxIndex]);

  const board = useMemo(() => {
    if (!game || selectedMoveIndex < 0) return emptyBoard();
    return boardAtMove(game.moves, selectedMoveIndex);
  }, [game, selectedMoveIndex]);

  const selectedMove =
    game && selectedMoveIndex >= 0 ? game.moves[selectedMoveIndex] : undefined;

  /**
   * Analysis fires when:
   *   • the move was made by the AI (it's the engine's own choice — show it),
   *   • the user explicitly requested analysis on this move, OR
   *   • auto-analyze is on and this is a real move.
   *
   * Otherwise the panel sits idle and shows the "Analyze" button.
   */
  const wasAiMove = selectedMove?.source === "ai";
  const userRequested = requestedSet.has(selectedMoveIndex);
  const analysisEnabled =
    !!game &&
    selectedMoveIndex >= 0 &&
    (wasAiMove || userRequested || autoAnalyze);

  /**
   * Progressive automatic analysis for the selected move. While enabled, keep
   * re-searching the position with a doubling time budget so the result gets
   * better and better, updating the panel each pass. Selecting another move
   * cancels and restarts. `getAnalysis` also attaches the AI's recorded eval
   * (when the move was an AI move) so the panel can show both then and now.
   */
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [analyzing, setAnalyzing] = useState(false);

  useEffect(() => {
    if (!analysisEnabled || selectedMoveIndex < 0) {
      setAnalysis(null);
      setAnalyzing(false);
      return;
    }
    let cancelled = false;
    const idx = selectedMoveIndex;
    void (async () => {
      setAnalyzing(true);
      let budget = AUTO_START_BUDGET_MS;
      let first = true;
      while (!cancelled) {
        let a: Analysis | null;
        try {
          a = await getAnalysis(id, idx, { depth: ANY_DEPTH, timeoutMs: budget });
        } catch {
          break;
        }
        if (cancelled) return;
        if (!a) {
          setAnalysis(null);
          break;
        }
        setAnalysis(a);
        if (first) {
          setAnalyzing(false);
          first = false;
        }
        // Stop once we hit the budget cap — re-running it would just repeat.
        if (budget >= AUTO_MAX_BUDGET_MS) break;
        budget = Math.min(budget * 2, AUTO_MAX_BUDGET_MS);
      }
      if (!cancelled) setAnalyzing(false);
    })();
    return () => {
      cancelled = true;
    };
  }, [id, selectedMoveIndex, analysisEnabled]);

  /**
   * Trigger analysis for the current move. Marking it requested flips the
   * `analysisEnabled` predicate, which starts the progressive effect above.
   * Idempotent; safe to call when an analysis already exists.
   */
  const triggerAnalyze = () => {
    if (selectedMoveIndex < 0) return;
    setRequestedSet((prev) => {
      if (prev.has(selectedMoveIndex)) return prev;
      const next = new Set(prev);
      next.add(selectedMoveIndex);
      return next;
    });
  };

  if (isLoading || !game) {
    return <div className="p-10 text-sm text-ink-muted">Loading game…</div>;
  }

  const canAnalyze = selectedMoveIndex >= 0;

  return (
    <div className="flex min-h-screen flex-col">
      <header className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-b border-border bg-bg-1 px-4 py-3 sm:px-6">
        <div className="flex items-center gap-4">
          <Link to="/games" className="text-ink-muted hover:text-ink-strong">
            <ArrowLeft className="size-4" />
          </Link>
          <div>
            <h1 className="text-sm font-medium text-ink-strong">{game.title}</h1>
            <div className="flex items-center gap-3 text-xs text-ink-muted">
              <span className="flex items-center gap-1.5">
                <Circle className="size-2.5 fill-stone-black stroke-0" />
                {game.black}
              </span>
              <span>vs</span>
              <span className="flex items-center gap-1.5">
                <Circle className="size-2.5 fill-stone-white stroke-0" />
                {game.white}
              </span>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-4 text-xs text-ink-muted">
          <span>{game.moveCount} moves</span>
          <span className="flex items-center gap-1">
            <Clock className="size-3" />
            captures {game.captures.black}–{game.captures.white}
          </span>
        </div>
      </header>

      <div className="grid flex-1 grid-cols-1 gap-4 overflow-y-auto p-4 xl:grid-cols-[minmax(0,1fr)_340px_320px] xl:gap-6 xl:overflow-hidden xl:p-6">
        <div className="flex min-w-0 flex-col gap-3 xl:overflow-hidden">
          <div className="flex items-stretch gap-2">
            {showEvalBar && (
              <EvalBar
                rootScore={analysis?.rootScore ?? null}
                // The analysis searches the position *before* this move, so the
                // root side-to-move (whose frame `rootScore` is in) is the
                // player who made the move — not the side to move after it.
                rootSide={selectedMove ? selectedMove.player : "black"}
              />
            )}
            <div className="min-w-0 flex-1">
              <Board
                board={board}
                lastMove={selectedMove?.coord ?? null}
                showLastMove={showLastMove}
                showCoordinates={showCoordinates}
                showCrosshair={showCrosshair}
                showHoverGhost={showHoverGhost}
                currentPlayer={
                  selectedMove
                    ? selectedMove.player === "black"
                      ? "white"
                      : "black"
                    : "black"
                }
                onHoverCell={setHoveredCell}
              />
            </div>
          </div>
          <BoardSettings
            showCoordinates={showCoordinates}
            showCrosshair={showCrosshair}
            showHoverGhost={showHoverGhost}
            onToggleCoordinates={() => toggle("showCoordinates")}
            onToggleCrosshair={() => toggle("showCrosshair")}
            onToggleHoverGhost={() => toggle("showHoverGhost")}
          />
          <TimelineScrubber
            selectedIndex={selectedMoveIndex}
            maxIndex={maxIndex}
            onSelect={setSelected}
          />
          <MoveSummary />
        </div>

        <div className="flex min-w-0 flex-col gap-3 xl:overflow-hidden">
          <div className="text-[11px] font-medium uppercase tracking-wider text-ink-muted">
            Moves
          </div>
          <MoveTable
            moves={game.moves}
            selectedIndex={selectedMoveIndex}
            onSelect={setSelected}
          />
        </div>

        <div className="flex min-w-0 flex-col gap-3 xl:overflow-y-auto">
          <AnalysisPanel
            analysis={analysis}
            isLoading={analyzing}
            canAnalyze={canAnalyze}
            autoAnalyze={autoAnalyze}
            onAnalyze={triggerAnalyze}
            onToggleAutoAnalyze={() => toggle("autoAnalyze")}
          />
        </div>
      </div>
    </div>
  );

  function MoveSummary() {
    if (!selectedMove) {
      return (
        <div className="flex items-center justify-between rounded-md border border-border bg-bg-1 px-4 py-2 text-xs text-ink-muted">
          <span>Empty board — use arrow keys or the scrubber to step through moves.</span>
          <HoverLabel coord={hoveredCell} />
        </div>
      );
    }
    return (
      <div className="flex items-center justify-between rounded-md border border-border bg-bg-1 px-4 py-2 text-xs">
        <span className="flex items-center gap-2 text-ink-strong">
          <Circle
            className={
              selectedMove.player === "black"
                ? "size-3 fill-stone-black stroke-0"
                : "size-3 fill-stone-white stroke-0"
            }
          />
          Move {selectedMove.index + 1}
          <span className="font-mono">{coordLabel(selectedMove.coord)}</span>
          {selectedMove.captured.length > 0 && (
            <span className="text-accent-2">+{selectedMove.captured.length} capture</span>
          )}
        </span>
        <div className="flex items-center gap-3">
          <HoverLabel coord={hoveredCell} />
          <span className="font-mono text-ink-muted">
            {selectedMove.source} · {formatMs(selectedMove.thinkMs)}
          </span>
        </div>
      </div>
    );
  }
}

function HoverLabel({ coord }: { coord: { x: number; y: number } | null }) {
  return (
    <span className="font-mono text-[11px] text-ink-muted">
      {coord ? `cursor → ${coordLabel(coord)}` : ""}
    </span>
  );
}
