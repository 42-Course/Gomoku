import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import {
  Bot,
  Lightbulb,
  RotateCcw,
  Save,
  Sparkles,
  Users,
  X,
} from "lucide-react";
import type {
  Analysis,
  Board as BoardGrid,
  Coord,
  Game,
  GameMode,
  GameStatus,
  Move,
  Player,
  TreeNode,
} from "@/api/types";
import { BOARD_SIZE, boardAtMove, emptyBoard } from "@/api/fixtures";
import { Board } from "@/components/Board";
import { BoardSettings } from "@/components/BoardSettings";
import { AnalysisPanel } from "@/components/AnalysisPanel";
import { TreeExplorer } from "@/components/TreeExplorer";
import { EvalBar } from "@/components/EvalBar";
import { useGameView } from "@/store/gameView";
import { EngineClient } from "@/engine/EngineClient";
import { flatTreeToNested, treeToAnalysis } from "@/engine/adapters";
import { saveLocalGame } from "@/storage/games";
import { ensureIdentity } from "@/storage/identity";
import {
  clearLastPlay,
  readLastPlay,
  writeLastPlay,
} from "@/storage/lastPlay";
import { coordLabel } from "@/lib/format";
import { cn } from "@/lib/cn";

const ANALYSIS_DEPTH = 4;

/**
 * Live game page. Owns its own EngineClient so the review screen's shared
 * client doesn't get yanked out from under it. Mode comes from `?mode=`
 * (vsai|hotseat). The engine handle's state is always in sync with the
 * displayed position because we play into it move-by-move.
 */
export function Play() {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const queryMode: GameMode = params.get("mode") === "hotseat" ? "hotseat" : "vsai";
  const resumeRequested = params.get("resume") === "1";

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

  // Snapshot of any persisted session, captured once on first render so a
  // later write to localStorage doesn't re-trigger the resume effect.
  const persistedRef = useRef(resumeRequested ? readLastPlay() : null);
  const persisted = persistedRef.current;

  // Mode is locked to whatever the page mounted with (or what was resumed).
  // Switching mode mid-page would invalidate the engine state, so we just
  // require the user to navigate via Home to flip it.
  const [mode] = useState<GameMode>(persisted?.mode ?? queryMode);
  const [moves, setMoves] = useState<Move[]>([]);
  const [status, setStatus] = useState<GameStatus>({ kind: "ongoing" });
  const [captures, setCaptures] = useState<{ black: number; white: number }>({
    black: 0,
    white: 0,
  });
  const [aiDepth, setAiDepth] = useState(persisted?.aiDepth ?? 4);
  const [aiSide, setAiSide] = useState<Player>(persisted?.aiSide ?? "white");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [restored, setRestored] = useState<boolean>(!persisted);

  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [tree, setTree] = useState<TreeNode | null>(null);
  const [analysisLoading, setAnalysisLoading] = useState(false);
  const [autoAnalyze, setAutoAnalyze] = useState(false);
  const [showCandidates, setShowCandidates] = useState(true);
  const [showPV, setShowPV] = useState(true);
  const [hoveredTreeCoord, setHoveredTreeCoord] = useState<Coord | null>(null);
  const [hoveredCell, setHoveredCell] = useState<Coord | null>(null);
  const [suggestion, setSuggestion] = useState<Coord | null>(null);

  const showCoordinates = useGameView((s) => s.showCoordinates);
  const showCrosshair = useGameView((s) => s.showCrosshair);
  const showHoverGhost = useGameView((s) => s.showHoverGhost);
  const showEvalBar = useGameView((s) => s.showEvalBar);
  const toggleView = useGameView((s) => s.toggle);

  const [savedId, setSavedId] = useState<string | null>(persisted?.savedId ?? null);

  /**
   * In vs-AI, the engine moves so fast that if it's auto-fired the moment
   * the page mounts (or after a New game), the user never has time to flip
   * the AI side. The "started" gate keeps the AI silent until the user
   * explicitly clicks Start, which is also when settings get locked.
   */
  const [started, setStarted] = useState<boolean>(
    persisted?.started ?? (queryMode === "hotseat"),
  );

  /**
   * Resume on mount: replay every persisted move through the engine so its
   * internal state matches what the UI thinks is on the board. Runs exactly
   * once, gated by `restored`.
   */
  useEffect(() => {
    if (restored) return;
    let cancelled = false;
    (async () => {
      if (!persisted) {
        setRestored(true);
        return;
      }
      try {
        for (const m of persisted.moves) {
          await engine.play(m.coord.x, m.coord.y);
        }
        if (cancelled) return;
        setMoves(persisted.moves);
        setStatus(persisted.status);
        setCaptures(persisted.captures);
      } catch {
        // Replay failed — wipe the bad record and start fresh.
        clearLastPlay();
      } finally {
        if (!cancelled) setRestored(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [restored, persisted, engine]);

  /**
   * Persist the live session whenever it changes. Skipped while we're
   * still restoring the previous session to avoid a clobber race.
   */
  useEffect(() => {
    if (!restored) return;
    if (moves.length === 0 && status.kind === "ongoing") {
      // Empty board with no commitment — wipe rather than keep an empty record.
      clearLastPlay();
      return;
    }
    writeLastPlay({
      mode,
      aiDepth: mode === "vsai" ? aiDepth : undefined,
      aiSide: mode === "vsai" ? aiSide : undefined,
      moves,
      status,
      captures,
      updatedAt: new Date().toISOString(),
      started,
      savedId: savedId ?? undefined,
    });
  }, [restored, mode, aiDepth, aiSide, moves, status, captures, started, savedId]);

  const board: BoardGrid = useMemo(
    () => (moves.length === 0 ? emptyBoard() : boardAtMove(moves, moves.length - 1)),
    [moves],
  );
  const lastMove = moves[moves.length - 1];
  const currentPlayer: Player =
    moves.length === 0 ? "black" : moves[moves.length - 1].player === "black" ? "white" : "black";
  const isOver = status.kind !== "ongoing";

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

  /** Run a verbose search at the current position and update the panel. */
  const runAnalyze = useCallback(async () => {
    setAnalysisLoading(true);
    try {
      const { result, tree: flat, thinkMs } = await engine.bestMoveVerbose(
        ANALYSIS_DEPTH,
      );
      const a = treeToAnalysis({
        id: `play_${Date.now()}`,
        gameId: "play",
        moveIndex: moves.length,
        depth: ANALYSIS_DEPTH,
        nodesVisited: result.nodes_visited,
        thinkMs,
        chosen: result.move ? { x: result.move.x, y: result.move.y } : null,
        tree: flat,
      });
      setAnalysis(a);
      setTree(flatTreeToNested(flat));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setAnalysisLoading(false);
    }
  }, [engine, moves.length]);

  /** Ask the engine for a move suggestion (one-shot, no tree). */
  const runSuggest = useCallback(async () => {
    try {
      const { result } = await engine.bestMove(ANALYSIS_DEPTH);
      if (result.move) setSuggestion({ x: result.move.x, y: result.move.y });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [engine]);

  /**
   * Trigger an AI reply when it's the AI's turn. Refreshes analysis
   * afterwards if auto-analyze is on.
   */
  const triggerAiTurnIfNeeded = useCallback(async () => {
    if (mode !== "vsai" || isOver) return;
    if (!started) return;
    if (currentPlayer !== aiSide) return;
    setBusy(true);
    try {
      const t0 = performance.now();
      const { result } = await engine.bestMove(aiDepth);
      const elapsed = performance.now() - t0;
      if (result.move) {
        await playMove(result.move.x, result.move.y, "ai", elapsed);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }, [aiDepth, aiSide, currentPlayer, engine, isOver, mode, playMove]);

  // After every move settles, optionally auto-analyze and pump the AI turn.
  useEffect(() => {
    setSuggestion(null);
    if (isOver) return;
    if (mode === "vsai" && started && currentPlayer === aiSide && !busy) {
      void triggerAiTurnIfNeeded();
      return;
    }
    if (autoAnalyze) void runAnalyze();
    else {
      setAnalysis(null);
      setTree(null);
    }
  }, [
    moves.length,
    aiSide,
    autoAnalyze,
    busy,
    currentPlayer,
    isOver,
    mode,
    runAnalyze,
    started,
    triggerAiTurnIfNeeded,
  ]);

  const onCellClick = async (c: Coord) => {
    if (busy || isOver) return;
    if (mode === "vsai" && !started) return;
    if (mode === "vsai" && currentPlayer === aiSide) return;
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
    if (busy || moves.length === 0) return;
    setBusy(true);
    try {
      // In vsai, undo the AI reply *and* the human move so it's the
      // human's turn again — otherwise the AI would just replay.
      const popCount = mode === "vsai" && moves.length >= 2 ? 2 : 1;
      for (let i = 0; i < popCount; i++) {
        await engine.undo();
      }
      setMoves((prev) => prev.slice(0, prev.length - popCount));
      setStatus({ kind: "ongoing" });
      // captures: re-derive from the snapshot to stay honest.
      const snap = await engine.snapshot();
      setCaptures({ black: snap.captures[0], white: snap.captures[1] });
      setAnalysis(null);
      setTree(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const newGame = async () => {
    if (busy) return;
    setBusy(true);
    try {
      await engine.reset();
      setMoves([]);
      setStatus({ kind: "ongoing" });
      setCaptures({ black: 0, white: 0 });
      setAnalysis(null);
      setTree(null);
      setSuggestion(null);
      setSavedId(null);
      setStarted(mode === "hotseat");
      clearLastPlay();
    } finally {
      setBusy(false);
    }
  };

  /**
   * Persist the current session as a Game in IndexedDB. Returns the id so
   * callers can navigate to the review screen for it (used by the
   * "Save & analyze" banner).
   */
  const save = async (): Promise<string | null> => {
    if (moves.length === 0) return null;
    const me = await ensureIdentity();
    const id = savedId ?? `local_${Date.now()}`;
    const black = mode === "vsai" && aiSide === "black" ? `AI (depth ${aiDepth})` : me.displayName;
    const white = mode === "vsai" && aiSide === "white" ? `AI (depth ${aiDepth})` : me.displayName;
    const game: Game = {
      id,
      kind: "local",
      mode,
      title:
        mode === "vsai"
          ? `vs AI (depth ${aiDepth})`
          : `Hot-seat · ${moves.length} moves`,
      black,
      white,
      status,
      moveCount: moves.length,
      moves,
      captures,
      createdAt: new Date(savedId ? Date.now() - 1 : Date.now()).toISOString(),
      updatedAt: new Date().toISOString(),
      aiDepth: mode === "vsai" ? aiDepth : undefined,
    };
    await saveLocalGame(game);
    setSavedId(id);
    return id;
  };

  /** Save and jump straight to the review screen. */
  const saveAndAnalyze = async () => {
    const id = await save();
    if (id) navigate(`/games/${id}`);
  };

  return (
    <div className="flex h-screen flex-col">
      <header className="flex items-center justify-between border-b border-border bg-bg-1 px-6 py-3">
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate(-1)}
            className="text-ink-muted hover:text-ink-strong"
            aria-label="Back"
          >
            <X className="size-4" />
          </button>
          <ModeBadge mode={mode} />
          <span className="text-xs text-ink-muted">
            captures {captures.black}–{captures.white}
          </span>
          <StatusBadge status={status} />
        </div>

        <div className="flex items-center gap-2">
          {mode === "vsai" && (
            <DepthControl depth={aiDepth} setDepth={setAiDepth} disabled={busy || started} />
          )}
          {mode === "vsai" && (
            <SideControl side={aiSide} setSide={setAiSide} disabled={busy || started} />
          )}
          {mode === "vsai" && !started && (
            <Btn onClick={() => setStarted(true)} disabled={busy} active>
              Start
            </Btn>
          )}
          <Btn onClick={runSuggest} disabled={busy || isOver}>
            <Lightbulb className="size-3.5" /> Suggest
          </Btn>
          <Btn onClick={runAnalyze} disabled={busy} active={!!analysis}>
            <Sparkles className="size-3.5" /> Analyze
          </Btn>
          <Btn onClick={undo} disabled={busy || moves.length === 0}>
            <RotateCcw className="size-3.5" /> Undo
          </Btn>
          <Btn onClick={save} disabled={moves.length === 0}>
            <Save className="size-3.5" /> {savedId ? "Saved" : "Save"}
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
          {isOver && moves.length > 0 && !savedId && (
            <GameOverBanner status={status} onSave={saveAndAnalyze} />
          )}
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
                candidates={analysis?.candidates ?? []}
                showCandidates={showCandidates && !!analysis}
                principalVariation={analysis?.principalVariation ?? []}
                showPrincipalVariation={showPV && !!analysis}
                showCoordinates={showCoordinates}
                showCrosshair={showCrosshair}
                showHoverGhost={showHoverGhost}
                currentPlayer={currentPlayer}
                highlightCoord={suggestion ?? hoveredTreeCoord}
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
            mode={mode}
            current={currentPlayer}
            aiSide={aiSide}
            isOver={isOver}
            busy={busy}
            started={started}
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
            showCandidates={showCandidates}
            showPrincipalVariation={showPV}
            autoAnalyze={autoAnalyze}
            onAnalyze={runAnalyze}
            onToggleCandidates={() => setShowCandidates((v) => !v)}
            onTogglePrincipalVariation={() => setShowPV((v) => !v)}
            onToggleAutoAnalyze={() => setAutoAnalyze((v) => !v)}
          />
          <TreeExplorer
            root={tree}
            isLoading={analysisLoading}
            onHoverCoord={setHoveredTreeCoord}
          />
        </div>
      </div>
    </div>
  );
}

function GameOverBanner({
  status,
  onSave,
}: {
  status: GameStatus;
  onSave: () => void;
}) {
  const verdict =
    status.kind === "draw"
      ? "Draw."
      : status.kind === "win"
        ? `${status.player === "black" ? "Black" : "White"} wins.`
        : "";
  return (
    <div className="flex items-center justify-between gap-3 rounded-md border border-accent/40 bg-accent/10 px-4 py-2.5 text-sm">
      <div className="flex flex-col">
        <span className="font-medium text-ink-strong">Game over · {verdict}</span>
        <span className="text-xs text-ink-muted">
          Save it to your library so you can review it move-by-move with full
          analysis.
        </span>
      </div>
      <button
        onClick={onSave}
        className="inline-flex items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-bg-0 transition-colors hover:bg-accent/85"
      >
        <Save className="size-3.5" /> Save & analyze
      </button>
    </div>
  );
}

function ModeBadge({ mode }: { mode: GameMode }) {
  const isAi = mode === "vsai";
  return (
    <span className="inline-flex items-center gap-1.5 rounded-md bg-bg-2 px-2 py-1 text-xs text-ink-strong">
      {isAi ? <Bot className="size-3.5" /> : <Users className="size-3.5" />}
      {isAi ? "vs AI" : "hot-seat"}
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

function DepthControl({
  depth,
  setDepth,
  disabled,
}: {
  depth: number;
  setDepth: (n: number) => void;
  disabled: boolean;
}) {
  return (
    <label className="flex items-center gap-1.5 rounded-md bg-bg-2 px-2 py-1 text-xs text-ink-muted">
      depth
      <input
        type="number"
        min={1}
        max={6}
        value={depth}
        disabled={disabled}
        onChange={(e) =>
          setDepth(Math.max(1, Math.min(6, parseInt(e.target.value, 10) || 1)))
        }
        className="w-10 rounded bg-bg-1 px-1 font-mono text-ink-strong outline-none disabled:opacity-50"
      />
    </label>
  );
}

function SideControl({
  side,
  setSide,
  disabled,
}: {
  side: Player;
  setSide: (p: Player) => void;
  disabled: boolean;
}) {
  return (
    <div className="flex items-center gap-1 rounded-md bg-bg-2 p-0.5 text-xs">
      {(["black", "white"] as Player[]).map((p) => (
        <button
          key={p}
          disabled={disabled}
          onClick={() => setSide(p)}
          className={cn(
            "rounded px-2 py-0.5 transition-colors",
            side === p ? "bg-bg-3 text-ink-strong" : "text-ink-muted",
            disabled && "opacity-50",
          )}
        >
          AI: {p}
        </button>
      ))}
    </div>
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
  mode,
  current,
  aiSide,
  isOver,
  busy,
  started,
  moveCount,
  suggestion,
  hoveredCell,
}: {
  mode: GameMode;
  current: Player;
  aiSide: Player;
  isOver: boolean;
  busy: boolean;
  started: boolean;
  moveCount: number;
  suggestion: Coord | null;
  hoveredCell: Coord | null;
}) {
  let label: string;
  if (isOver) label = "Game over";
  else if (mode === "vsai" && !started)
    label = "Pick a side and depth, then click Start";
  else if (mode === "vsai" && current === aiSide)
    label = busy ? "AI is thinking…" : `AI to move (${aiSide})`;
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
        {!hoveredCell && !suggestion && moveCount === 0 && started && (
          <span className="text-ink-muted">
            Click any intersection. {BOARD_SIZE}×{BOARD_SIZE} board.
          </span>
        )}
      </div>
    </div>
  );
}
