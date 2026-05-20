import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { Lightbulb, RotateCcw, Sparkles, X } from "lucide-react";
import type {
  Analysis,
  Board as BoardGrid,
  Coord,
  Game,
  GameStatus,
  Move,
  Player,
} from "@/api/types";
import { queryKeys } from "@/api/client";
import { BOARD_SIZE, boardAtMove, emptyBoard } from "@/api/fixtures";
import { Board } from "@/components/Board";
import { BoardSettings } from "@/components/BoardSettings";
import { AnalysisPanel } from "@/components/AnalysisPanel";
import { EvalBar } from "@/components/EvalBar";
import { useGameView } from "@/store/gameView";
import { EngineClient } from "@/engine/EngineClient";
import {
  createLocalGame,
  getLocalGame,
  saveLocalGame,
} from "@/storage/games";
import { coordLabel } from "@/lib/format";
import { cn } from "@/lib/cn";

const ANALYSIS_DEPTH = 4;

/**
 * Live game page. Driven by a `?id=<localGameId>` URL parameter — the
 * Game record (created on Home) is the single source of truth. We replay
 * its moves through a page-owned EngineClient on mount, then mutate the
 * record on every move so the user can navigate away and resume cleanly.
 */
export function Play() {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const id = params.get("id");
  const queryClient = useQueryClient();

  // One engine instance for the lifetime of this page.
  const engineRef = useRef<EngineClient | null>(null);
  if (!engineRef.current) engineRef.current = new EngineClient();
  useEffect(
    () => () => {
      engineRef.current?.dispose();
      engineRef.current = null;
    },
    [],
  );
  const engine = engineRef.current;

  const [game, setGame] = useState<Game | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [restored, setRestored] = useState(false);

  const [moves, setMoves] = useState<Move[]>([]);
  const [status, setStatus] = useState<GameStatus>({ kind: "ongoing" });
  const [captures, setCaptures] = useState<{ black: number; white: number }>({
    black: 0,
    white: 0,
  });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [analysisLoading, setAnalysisLoading] = useState(false);
  const [autoAnalyze, setAutoAnalyze] = useState(false);
  const [hoveredCell, setHoveredCell] = useState<Coord | null>(null);
  const [suggestion, setSuggestion] = useState<Coord | null>(null);

  const showCoordinates = useGameView((s) => s.showCoordinates);
  const showCrosshair = useGameView((s) => s.showCrosshair);
  const showHoverGhost = useGameView((s) => s.showHoverGhost);
  const showEvalBar = useGameView((s) => s.showEvalBar);
  const toggleView = useGameView((s) => s.toggle);

  /**
   * Load the Game record by id and replay its moves into the engine so
   * the engine's internal state matches what we render. Runs once per
   * page mount; if the id is missing or unknown we bail back to Home.
   */
  useEffect(() => {
    if (!id) {
      navigate("/", { replace: true });
      return;
    }
    let cancelled = false;
    (async () => {
      const g = await getLocalGame(id);
      if (cancelled) return;
      if (!g) {
        setLoadError(`Game ${id} not found.`);
        setRestored(true);
        return;
      }
      try {
        for (const m of g.moves) {
          await engine.play(m.coord.x, m.coord.y);
        }
        if (cancelled) return;
        setGame(g);
        setMoves(g.moves);
        setStatus(g.status);
        setCaptures(g.captures);
      } catch (e) {
        if (!cancelled)
          setLoadError(e instanceof Error ? e.message : String(e));
      } finally {
        if (!cancelled) setRestored(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [id, engine, navigate]);

  const board: BoardGrid = useMemo(
    () => (moves.length === 0 ? emptyBoard() : boardAtMove(moves, moves.length - 1)),
    [moves],
  );
  const lastMove = moves[moves.length - 1];
  const currentPlayer: Player =
    moves.length === 0 ? "black" : moves[moves.length - 1].player === "black" ? "white" : "black";
  const isOver = status.kind !== "ongoing";

  /**
   * Persist the live game record back to IndexedDB whenever the displayed
   * state changes. Skipped while we're still restoring from disk.
   */
  useEffect(() => {
    if (!restored || !game) return;
    const updated: Game = {
      ...game,
      moves,
      moveCount: moves.length,
      status,
      captures,
      updatedAt: new Date().toISOString(),
      lastCoord: moves[moves.length - 1]?.coord,
    };
    void saveLocalGame(updated).then(() => {
      queryClient.invalidateQueries({ queryKey: queryKeys.games });
    });
  }, [restored, game, moves, status, captures, queryClient]);

  /**
   * Push one move into the engine and record it. `source` distinguishes
   * human input from AI replies in the saved record.
   */
  const playMove = useCallback(
    async (x: number, y: number, source: "human" | "ai", thinkMs: number) => {
      const before = performance.now();
      const result = await engine.play(x, y);
      const elapsed = thinkMs > 0 ? thinkMs : performance.now() - before;
      setMoves((prev) => [
        ...prev,
        {
          index: prev.length,
          player: currentPlayer,
          coord: { x, y },
          captured: result.captured.map((c) => ({ x: c.x, y: c.y })),
          thinkMs: Math.round(elapsed),
          source,
        },
      ]);
      setStatus(result.status);
      setCaptures({ black: result.captures[0], white: result.captures[1] });
    },
    [currentPlayer, engine],
  );

  /** Run a search at the current position and update the panel. */
  const runAnalyze = useCallback(async () => {
    setAnalysisLoading(true);
    try {
      const { result, thinkMs } = await engine.bestMove(ANALYSIS_DEPTH);
      setAnalysis({
        id: `play_${Date.now()}`,
        gameId: game?.id ?? "play",
        moveIndex: moves.length,
        chosen: result.move ? { x: result.move.x, y: result.move.y } : null,
        rootScore: result.score,
        thinkMs,
        depth: ANALYSIS_DEPTH,
        nodesVisited: Number(result.nodes_visited),
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setAnalysisLoading(false);
    }
  }, [engine, game?.id, moves.length]);

  /** Ask the engine for a move suggestion (one-shot, no panel update). */
  const runSuggest = useCallback(async () => {
    try {
      const { result } = await engine.bestMove(game?.aiDepth ?? ANALYSIS_DEPTH);
      if (result.move) setSuggestion({ x: result.move.x, y: result.move.y });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [engine, game?.aiDepth]);

  /**
   * Trigger an AI reply when it's the AI's turn. Refreshes analysis
   * afterwards if auto-analyze is on.
   */
  const triggerAiTurnIfNeeded = useCallback(async () => {
    if (!game || game.mode !== "vsai" || isOver) return;
    if (currentPlayer !== game.aiSide) return;
    setBusy(true);
    try {
      const t0 = performance.now();
      const { result } = await engine.bestMove(game.aiDepth ?? ANALYSIS_DEPTH);
      const elapsed = performance.now() - t0;
      if (result.move) {
        await playMove(result.move.x, result.move.y, "ai", elapsed);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }, [currentPlayer, engine, game, isOver, playMove]);

  // After every move settles, optionally auto-analyze and pump the AI turn.
  useEffect(() => {
    if (!restored) return;
    setSuggestion(null);
    if (isOver) return;
    if (game?.mode === "vsai" && currentPlayer === game.aiSide && !busy) {
      void triggerAiTurnIfNeeded();
      return;
    }
    if (autoAnalyze) void runAnalyze();
    else {
      setAnalysis(null);
    }
  }, [
    restored,
    moves.length,
    autoAnalyze,
    busy,
    currentPlayer,
    game,
    isOver,
    runAnalyze,
    triggerAiTurnIfNeeded,
  ]);

  const onCellClick = async (c: Coord) => {
    if (busy || isOver || !game) return;
    if (game.mode === "vsai" && currentPlayer === game.aiSide) return;
    setError(null);
    setBusy(true);
    try {
      await playMove(c.x, c.y, "human", 0);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const undo = async () => {
    if (busy || moves.length === 0 || !game) return;
    setBusy(true);
    try {
      // In vsai, undo the AI reply *and* the human move so it's the
      // human's turn again — otherwise the AI would just replay.
      const popCount =
        game.mode === "vsai" && moves.length >= 2 ? 2 : 1;
      for (let i = 0; i < popCount; i++) {
        await engine.undo();
      }
      setMoves((prev) => prev.slice(0, prev.length - popCount));
      setStatus({ kind: "ongoing" });
      const snap = await engine.snapshot();
      setCaptures({ black: snap.captures[0], white: snap.captures[1] });
      setAnalysis(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  /**
   * Start a new game with the same configuration as this one. The current
   * game stays in IndexedDB exactly as it is (unfinished or finished); we
   * just navigate to a freshly created record.
   */
  const newGame = async () => {
    if (busy || !game) return;
    const nextId = await createLocalGame({
      mode: game.mode,
      aiDepth: game.aiDepth,
      aiSide: game.aiSide,
    });
    queryClient.invalidateQueries({ queryKey: queryKeys.games });
    navigate(`/play?id=${nextId}`);
  };

  if (loadError) {
    return (
      <div className="p-10 text-sm text-ink-muted">
        {loadError}{" "}
        <button
          onClick={() => navigate("/")}
          className="text-accent hover:underline"
        >
          Go home
        </button>
      </div>
    );
  }

  if (!restored || !game) {
    return <div className="p-10 text-sm text-ink-muted">Loading game…</div>;
  }

  return (
    <div className="flex h-screen flex-col">
      <header className="flex items-center justify-between border-b border-border bg-bg-1 px-6 py-3">
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate("/")}
            className="text-ink-muted hover:text-ink-strong"
            aria-label="Back"
          >
            <X className="size-4" />
          </button>
          <GameHeader game={game} />
          <span className="text-xs text-ink-muted">
            captures {captures.black}–{captures.white}
          </span>
          <StatusBadge status={status} />
        </div>

        <div className="flex items-center gap-2">
          <Btn onClick={runSuggest} disabled={busy || isOver}>
            <Lightbulb className="size-3.5" /> Suggest
          </Btn>
          <Btn onClick={runAnalyze} disabled={busy} active={!!analysis}>
            <Sparkles className="size-3.5" /> Analyze
          </Btn>
          <Btn onClick={undo} disabled={busy || moves.length === 0}>
            <RotateCcw className="size-3.5" /> Undo
          </Btn>
          <Btn onClick={newGame} disabled={busy}>
            New
          </Btn>
        </div>
      </header>

      {error && (
        <div className="border-b border-danger/40 bg-danger/10 px-6 py-1.5 text-xs text-danger">
          {error}
        </div>
      )}

      <div className="grid flex-1 grid-cols-[1fr_340px] gap-6 overflow-hidden p-6">
        <div className="flex flex-col gap-3 overflow-hidden">
          <div className="flex items-stretch gap-2">
            {showEvalBar && (
              <EvalBar
                rootScore={analysis?.rootScore ?? null}
                rootSide={currentPlayer}
              />
            )}
            <div className="flex-1">
              <Board
                board={board}
                lastMove={lastMove?.coord ?? null}
                showLastMove
                showCoordinates={showCoordinates}
                showCrosshair={showCrosshair}
                showHoverGhost={showHoverGhost}
                currentPlayer={currentPlayer}
                highlightCoord={suggestion}
                onCellClick={onCellClick}
                onHoverCell={setHoveredCell}
              />
            </div>
          </div>
          <BoardSettings
            showCoordinates={showCoordinates}
            showCrosshair={showCrosshair}
            showHoverGhost={showHoverGhost}
            onToggleCoordinates={() => toggleView("showCoordinates")}
            onToggleCrosshair={() => toggleView("showCrosshair")}
            onToggleHoverGhost={() => toggleView("showHoverGhost")}
          />
          <TurnLine
            game={game}
            current={currentPlayer}
            isOver={isOver}
            busy={busy}
            moveCount={moves.length}
            suggestion={suggestion}
            hoveredCell={hoveredCell}
          />
        </div>

        <div className="flex flex-col gap-3 overflow-y-auto">
          <AnalysisPanel
            analysis={analysis}
            isLoading={analysisLoading}
            canAnalyze={true}
            autoAnalyze={autoAnalyze}
            onAnalyze={runAnalyze}
            onToggleAutoAnalyze={() => setAutoAnalyze((v) => !v)}
          />
        </div>
      </div>
    </div>
  );
}

function GameHeader({ game }: { game: Game }) {
  if (game.mode === "vsai") {
    return (
      <span className="inline-flex items-center gap-1.5 rounded-md bg-bg-2 px-2 py-1 text-xs text-ink-strong">
        vs AI · depth {game.aiDepth} · AI plays {game.aiSide}
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1.5 rounded-md bg-bg-2 px-2 py-1 text-xs text-ink-strong">
      hot-seat
    </span>
  );
}

function StatusBadge({ status }: { status: GameStatus }) {
  if (status.kind === "ongoing") return null;
  if (status.kind === "draw") {
    return (
      <span className="rounded-full bg-bg-3 px-2 py-0.5 text-xs text-ink-muted">
        draw
      </span>
    );
  }
  return (
    <span className="rounded-full bg-accent/15 px-2 py-0.5 text-xs text-accent">
      {status.player} wins
    </span>
  );
}

function Btn({
  children,
  active,
  ...rest
}: React.ButtonHTMLAttributes<HTMLButtonElement> & { active?: boolean }) {
  return (
    <button
      {...rest}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs transition-colors",
        active
          ? "bg-accent/15 text-accent"
          : "bg-bg-2 text-ink-strong hover:bg-bg-3",
        "disabled:cursor-not-allowed disabled:opacity-40",
      )}
    >
      {children}
    </button>
  );
}

function TurnLine({
  game,
  current,
  isOver,
  busy,
  moveCount,
  suggestion,
  hoveredCell,
}: {
  game: Game;
  current: Player;
  isOver: boolean;
  busy: boolean;
  moveCount: number;
  suggestion: Coord | null;
  hoveredCell: Coord | null;
}) {
  let label: string;
  if (isOver) label = "Game over";
  else if (game.mode === "vsai" && current === game.aiSide)
    label = busy ? "AI is thinking…" : `AI to move (${game.aiSide})`;
  else label = `${current} to move`;

  return (
    <div className="flex items-center justify-between rounded-md border border-border bg-bg-1 px-4 py-2 text-xs">
      <span className="text-ink-strong">
        Move {moveCount + 1} · {label}
      </span>
      <div className="flex items-center gap-3">
        {hoveredCell && (
          <span className="font-mono text-[11px] text-ink-muted">
            cursor → {coordLabel(hoveredCell)}
          </span>
        )}
        {suggestion && (
          <span className="font-mono text-accent">
            suggest → {coordLabel(suggestion)}
          </span>
        )}
        {!hoveredCell && !suggestion && moveCount === 0 && (
          <span className="text-ink-muted">
            Click any intersection. {BOARD_SIZE}×{BOARD_SIZE} board.
          </span>
        )}
      </div>
    </div>
  );
}
