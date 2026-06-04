import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import {
  Circle,
  Lightbulb,
  RotateCcw,
  Sparkles,
  Timer,
  Trophy,
  X,
} from "lucide-react";
import type {
  Analysis,
  Board as BoardGrid,
  Coord,
  Game,
  GameStatus,
  Move,
  MoveAnalysis,
  Player,
} from "@/api/types";
import { queryKeys } from "@/api/client";
import { BOARD_SIZE, boardAtMove, emptyBoard } from "@/api/fixtures";
import { Board } from "@/components/Board";
import { BoardSettings } from "@/components/BoardSettings";
import { AnalysisPanel } from "@/components/AnalysisPanel";
import { MoveTable } from "@/components/MoveTable";
import { EvalBar } from "@/components/EvalBar";
import { useGameView } from "@/store/gameView";
import { EngineClient } from "@/engine/EngineClient";
import {
  createLocalGame,
  getLocalGame,
  saveLocalGame,
} from "@/storage/games";
import { coordLabel, formatMs } from "@/lib/format";
import {
  ANY_DEPTH,
  AUTO_MAX_BUDGET_MS,
  AUTO_START_BUDGET_MS,
  DEFAULT_ANY_BUDGET_MS,
  depthLabel,
  MAX_SLIDER_DEPTH,
  searchBudget,
} from "@/lib/search";
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

  // One engine instance for the lifetime of this page, fetched through a
  // getter rather than captured in a variable. The cleanup below disposes
  // the worker on unmount; under StrictMode React unmounts and *re-mounts*
  // the effects without re-rendering, so a captured instance would be a dead
  // (terminated) worker and every `engine.play(...)` would hang forever.
  // Re-creating lazily here hands the re-run effects a live engine instead.
  const engineRef = useRef<EngineClient | null>(null);
  const ensureEngine = useCallback(() => {
    if (!engineRef.current) engineRef.current = new EngineClient();
    return engineRef.current;
  }, []);
  useEffect(
    () => () => {
      engineRef.current?.dispose();
      engineRef.current = null;
    },
    [],
  );

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
  // Strength of the "Suggest" hint and the "Analyze" panel. `null` means
  // "follow the game's AI depth"; an explicit pick overrides it. Deriving the
  // effective depth this way avoids seeding state from the async game load.
  const [suggestDepthPick, setSuggestDepth] = useState<number | null>(null);
  const [analysisDepthPick, setAnalysisDepth] = useState<number | null>(null);

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
        // Capture one engine for this run so a cancelled StrictMode pass keeps
        // replaying into its own (soon-to-be-disposed) instance instead of
        // racing the surviving run's engine.
        const eng = ensureEngine();
        for (const m of g.moves) {
          if (cancelled) return;
          await eng.play(m.coord.x, m.coord.y);
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
  }, [id, ensureEngine, navigate]);

  const board: BoardGrid = useMemo(
    () => (moves.length === 0 ? emptyBoard() : boardAtMove(moves, moves.length - 1)),
    [moves],
  );
  const lastMove = moves[moves.length - 1];
  const currentPlayer: Player =
    moves.length === 0 ? "black" : moves[moves.length - 1].player === "black" ? "white" : "black";
  const isOver = status.kind !== "ongoing";

  // Effective strengths: an explicit pick wins, otherwise follow the game's
  // AI depth so reviewing matches how the game was played.
  const suggestDepth = suggestDepthPick ?? game?.aiDepth ?? ANALYSIS_DEPTH;
  const analysisDepth = analysisDepthPick ?? game?.aiDepth ?? ANALYSIS_DEPTH;

  // Per-player accumulated thinking time, summed from each move's recorded
  // think time — so it survives reloads and undo without extra bookkeeping.
  const accumulated = useMemo(() => {
    const acc = { black: 0, white: 0 };
    for (const m of moves) acc[m.player] += m.thinkMs;
    return acc;
  }, [moves]);

  // Live count-up clock for the current turn. The interval captures the
  // turn's start time in its closure (re-running whenever a move lands, so
  // `moves.length` changes) and ticks the elapsed time into state — no ref
  // reads or `Date.now()` during render.
  const [currentTurnMs, setCurrentTurnMs] = useState(0);
  // Whether the win/draw overlay has been dismissed for the current result.
  const [winAck, setWinAck] = useState(false);
  useEffect(() => {
    setCurrentTurnMs(0);
    setWinAck(false);
    if (isOver || !restored) return;
    const start = Date.now();
    const t = setInterval(() => setCurrentTurnMs(Date.now() - start), 200);
    return () => clearInterval(t);
  }, [isOver, restored, moves.length]);

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
    async (
      x: number,
      y: number,
      source: "human" | "ai",
      thinkMs: number,
      analysis?: MoveAnalysis,
    ) => {
      const before = performance.now();
      const result = await ensureEngine().play(x, y);
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
          analysis,
        },
      ]);
      setStatus(result.status);
      setCaptures({ black: result.captures[0], white: result.captures[1] });
    },
    [currentPlayer, ensureEngine],
  );

  /** Run a search at the current position and update the panel. */
  const runAnalyze = useCallback(async () => {
    setAnalysisLoading(true);
    try {
      const { depth, timeoutMs } = searchBudget(
        analysisDepth,
        game?.aiTimeoutMs ?? DEFAULT_ANY_BUDGET_MS,
      );
      const { result, thinkMs } = await ensureEngine().bestMove(depth, timeoutMs);
      setAnalysis({
        id: `play_${Date.now()}`,
        gameId: game?.id ?? "play",
        moveIndex: moves.length,
        chosen: result.move ? { x: result.move.x, y: result.move.y } : null,
        rootScore: result.score,
        thinkMs,
        depth: analysisDepth,
        depthReached: result.depth_reached,
        maxPly: result.max_ply,
        nodesVisited: Number(result.total_nodes),
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setAnalysisLoading(false);
    }
  }, [ensureEngine, game?.id, game?.aiTimeoutMs, analysisDepth, moves.length]);

  /** Ask the engine for a move suggestion (one-shot, no panel update). */
  const runSuggest = useCallback(async () => {
    try {
      const { depth, timeoutMs } = searchBudget(suggestDepth);
      const { result } = await ensureEngine().bestMove(depth, timeoutMs);
      if (result.move) setSuggestion({ x: result.move.x, y: result.move.y });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [ensureEngine, suggestDepth]);

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
      const { depth, timeoutMs } = searchBudget(
        game.aiDepth ?? ANALYSIS_DEPTH,
        game.aiTimeoutMs ?? DEFAULT_ANY_BUDGET_MS,
      );
      const { result } = await ensureEngine().bestMove(depth, timeoutMs);
      const elapsed = performance.now() - t0;
      if (result.move) {
        // Capture what the engine evaluated so the review screen can show
        // the AI's own assessment at the moment it moved.
        const evalMeta: MoveAnalysis = {
          score: result.score,
          depth,
          depthReached: result.depth_reached,
          maxPly: result.max_ply,
          nodesVisited: Number(result.total_nodes),
        };
        await playMove(result.move.x, result.move.y, "ai", elapsed, evalMeta);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }, [currentPlayer, ensureEngine, game, isOver, playMove]);

  // After every move settles, clear the stale suggestion and pump the AI turn.
  // Auto-analysis is handled by its own progressive effect below.
  useEffect(() => {
    if (!restored) return;
    setSuggestion(null);
    if (isOver) return;
    if (game?.mode === "vsai" && currentPlayer === game.aiSide && !busy) {
      void triggerAiTurnIfNeeded();
      return;
    }
    if (!autoAnalyze) setAnalysis(null);
  }, [
    restored,
    moves.length,
    autoAnalyze,
    busy,
    currentPlayer,
    game,
    isOver,
    triggerAiTurnIfNeeded,
  ]);

  /**
   * Progressive auto-analysis. While enabled and the side-to-move's position
   * is held (game live, not the AI's turn), keep searching with a doubling
   * time budget so the panel's result gets better the longer you sit on the
   * move. Changing position (moves.length) cancels and restarts; the per-pass
   * budget is capped so a queued move never waits long behind a search.
   */
  useEffect(() => {
    if (!autoAnalyze || !restored || isOver) return;
    if (game?.mode === "vsai" && currentPlayer === game.aiSide) return;

    let cancelled = false;
    const atMove = moves.length;
    void (async () => {
      setAnalysisLoading(true);
      let budget = AUTO_START_BUDGET_MS;
      let first = true;
      while (!cancelled) {
        let res: Awaited<ReturnType<EngineClient["bestMove"]>>;
        try {
          res = await ensureEngine().bestMove(ANY_DEPTH, budget);
        } catch (e) {
          if (!cancelled) setError(e instanceof Error ? e.message : String(e));
          break;
        }
        if (cancelled) return;
        const { result, thinkMs } = res;
        setAnalysis({
          id: `auto_${atMove}_${budget}`,
          gameId: game?.id ?? "play",
          moveIndex: atMove,
          chosen: result.move ? { x: result.move.x, y: result.move.y } : null,
          rootScore: result.score,
          thinkMs,
          depth: ANY_DEPTH,
          depthReached: result.depth_reached,
          maxPly: result.max_ply,
          nodesVisited: Number(result.total_nodes),
        });
        if (first) {
          setAnalysisLoading(false);
          first = false;
        }
        // Stop once we've reached the budget cap — the result has converged and
        // re-running at the same budget would just repeat it.
        if (budget >= AUTO_MAX_BUDGET_MS) break;
        budget = Math.min(budget * 2, AUTO_MAX_BUDGET_MS);
      }
      if (!cancelled) setAnalysisLoading(false);
    })();

    return () => {
      cancelled = true;
    };
  }, [autoAnalyze, restored, isOver, moves.length, currentPlayer, game, ensureEngine]);

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
        await ensureEngine().undo();
      }
      setMoves((prev) => prev.slice(0, prev.length - popCount));
      setStatus({ kind: "ongoing" });
      const snap = await ensureEngine().snapshot();
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
      aiTimeoutMs: game.aiTimeoutMs,
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
    <div className="flex min-h-screen flex-col">
      <header className="flex flex-wrap items-center justify-between gap-x-3 gap-y-2 border-b border-border bg-bg-1 px-4 py-3 sm:px-6">
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
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

        <div className="flex flex-wrap items-center gap-2">
          <div className="flex items-center gap-1.5">
            <Btn onClick={runSuggest} disabled={busy || isOver}>
              <Lightbulb className="size-3.5" /> Suggest
            </Btn>
            <StrengthSelect
              value={suggestDepth}
              onChange={setSuggestDepth}
              title="Suggestion strength"
            />
          </div>
          <div className="flex items-center gap-1.5">
            <Btn onClick={runAnalyze} disabled={busy} active={!!analysis}>
              <Sparkles className="size-3.5" /> Analyze
            </Btn>
            <StrengthSelect
              value={analysisDepth}
              onChange={setAnalysisDepth}
              title="Analysis depth"
            />
          </div>
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

      <div className="grid flex-1 grid-cols-1 gap-4 overflow-y-auto p-4 lg:grid-cols-[minmax(0,1fr)_340px] lg:gap-6 lg:overflow-hidden lg:p-6">
        <div className="flex min-w-0 flex-col gap-3 lg:overflow-hidden">
          <TurnClock
            current={currentPlayer}
            accumulated={accumulated}
            currentTurnMs={currentTurnMs}
            isOver={isOver}
          />
          <div className="flex items-stretch gap-2">
            {showEvalBar && (
              <EvalBar
                rootScore={analysis?.rootScore ?? null}
                rootSide={currentPlayer}
              />
            )}
            <div className="relative min-w-0 flex-1">
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
              {isOver && !winAck && (
                <GameOverOverlay
                  status={status}
                  busy={busy}
                  onNewGame={newGame}
                  onDismiss={() => setWinAck(true)}
                />
              )}
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

        <div className="flex min-w-0 flex-col gap-3 lg:overflow-y-auto">
          <AnalysisPanel
            analysis={analysis}
            isLoading={analysisLoading}
            canAnalyze={true}
            autoAnalyze={autoAnalyze}
            onAnalyze={runAnalyze}
            onToggleAutoAnalyze={() => setAutoAnalyze((v) => !v)}
          />
          <div className="flex flex-col gap-2">
            <div className="text-[11px] font-medium uppercase tracking-wider text-ink-muted">
              History
            </div>
            <MoveTable moves={moves} selectedIndex={moves.length - 1} />
          </div>
        </div>
      </div>
    </div>
  );
}

/**
 * Prominent end-of-game banner overlaid on the board so a win/draw is
 * impossible to miss. Dismissable so the final position stays reviewable.
 */
function GameOverOverlay({
  status,
  busy,
  onNewGame,
  onDismiss,
}: {
  status: GameStatus;
  busy: boolean;
  onNewGame: () => void;
  onDismiss: () => void;
}) {
  const winner = status.kind === "win" ? status.player : null;
  return (
    <div className="absolute inset-0 z-20 flex items-center justify-center rounded-md bg-bg-0/55 p-4 backdrop-blur-[2px]">
      <div className="relative flex flex-col items-center gap-4 rounded-2xl border border-border bg-bg-1/95 px-8 py-7 text-center shadow-2xl">
        <button
          onClick={onDismiss}
          aria-label="Dismiss"
          className="absolute right-3 top-3 text-ink-muted hover:text-ink-strong"
        >
          <X className="size-4" />
        </button>

        {winner ? (
          <>
            <div className="flex size-14 items-center justify-center rounded-full bg-accent/15">
              <Trophy className="size-7 text-accent" />
            </div>
            <div>
              <div className="flex items-center justify-center gap-2 text-xl font-semibold text-ink-strong">
                <Circle
                  className={cn(
                    "size-4",
                    winner === "black"
                      ? "fill-stone-black stroke-0"
                      : "fill-stone-white stroke-0",
                  )}
                />
                {winner === "black" ? "Black" : "White"} wins
              </div>
              <div className="mt-1 text-xs text-ink-muted">Game over</div>
            </div>
          </>
        ) : (
          <>
            <div className="flex size-14 items-center justify-center rounded-full bg-bg-3 text-2xl">
              🤝
            </div>
            <div className="text-xl font-semibold text-ink-strong">Draw</div>
          </>
        )}

        <div className="flex items-center gap-2">
          <button
            onClick={onNewGame}
            disabled={busy}
            className="rounded-md bg-accent px-4 py-1.5 text-xs font-medium text-bg-0 transition-colors hover:bg-accent/85 disabled:opacity-40"
          >
            New game
          </button>
          <button
            onClick={onDismiss}
            className="rounded-md bg-bg-2 px-4 py-1.5 text-xs text-ink-strong transition-colors hover:bg-bg-3"
          >
            Review
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * Compact strength picker: fixed depths 1..MAX plus an "Any" entry that
 * maps to the time-bounded sentinel depth. Used for the Suggest hint.
 */
function StrengthSelect({
  value,
  onChange,
  title,
}: {
  value: number;
  onChange: (depth: number) => void;
  title?: string;
}) {
  return (
    <select
      value={value}
      title={title}
      onChange={(e) => onChange(parseInt(e.target.value, 10))}
      className="rounded-md bg-bg-2 px-2 py-1.5 text-xs text-ink-strong outline-none hover:bg-bg-3 focus:ring-1 focus:ring-accent"
    >
      {Array.from({ length: MAX_SLIDER_DEPTH }, (_, i) => i + 1).map((d) => (
        <option key={d} value={d}>
          depth {d}
        </option>
      ))}
      <option value={ANY_DEPTH}>Any (timed)</option>
    </select>
  );
}

/**
 * Turn clock: a live count-up for the side to move plus each player's
 * accumulated thinking time across the game.
 */
function TurnClock({
  current,
  accumulated,
  currentTurnMs,
  isOver,
}: {
  current: Player;
  accumulated: { black: number; white: number };
  currentTurnMs: number;
  isOver: boolean;
}) {
  return (
    <div className="flex items-center justify-between rounded-md border border-border bg-bg-1 px-3 py-2 text-xs">
      <div className="flex items-center gap-2 text-ink-muted">
        <Timer className="size-3.5" />
        <span className="font-mono tabular-nums text-ink-strong">
          {isOver ? "—" : formatMs(currentTurnMs)}
        </span>
        {!isOver && <span>· {current} to move</span>}
      </div>
      <div className="flex items-center gap-3">
        <PlayerClock player="black" ms={accumulated.black} active={!isOver && current === "black"} />
        <PlayerClock player="white" ms={accumulated.white} active={!isOver && current === "white"} />
      </div>
    </div>
  );
}

function PlayerClock({
  player,
  ms,
  active,
}: {
  player: Player;
  ms: number;
  active: boolean;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded px-1.5 py-0.5 font-mono tabular-nums",
        active ? "bg-accent/15 text-accent" : "text-ink-muted",
      )}
    >
      <Circle
        className={cn(
          "size-2.5",
          player === "black" ? "fill-stone-black stroke-0" : "fill-stone-white stroke-0",
        )}
      />
      {formatMs(ms)}
    </span>
  );
}

function GameHeader({ game }: { game: Game }) {
  if (game.mode === "vsai") {
    return (
      <span className="inline-flex items-center gap-1.5 rounded-md bg-bg-2 px-2 py-1 text-xs text-ink-strong">
        vs AI · depth {game.aiDepth != null ? depthLabel(game.aiDepth) : "?"} · AI
        plays {game.aiSide}
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
