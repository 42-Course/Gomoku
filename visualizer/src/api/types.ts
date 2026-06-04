/**
 * Wire types for the future gomoku-server.
 * These are the single source of truth — the server should target them verbatim.
 */

export type Player = "black" | "white";
export type Cell = Player | null;
export type Board = Cell[][]; // 19x19, [y][x]

export type Coord = { x: number; y: number };

export type GameStatus =
  | { kind: "ongoing" }
  | { kind: "win"; player: Player }
  | { kind: "draw" };

export type MoveSource = "human" | "ai";

/**
 * Engine search metadata captured at the moment an AI move was chosen, so the
 * review screen can show what the engine actually evaluated then — not a fresh
 * recomputation. The chosen move is the parent `Move.coord` itself.
 */
export interface MoveAnalysis {
  /** Score from the moving side's perspective. */
  score: number;
  /** Depth bound requested for the search (`ANY_DEPTH` for time-bounded). */
  depth: number;
  /** Deepest iterative-deepening depth fully completed. */
  depthReached: number;
  /** Deepest ply explored overall. */
  maxPly: number;
  /** Total nodes (incl. leaves) visited across all iterations. */
  nodesVisited: number;
}

export interface Move {
  index: number;           // 0-based turn number
  player: Player;
  coord: Coord;
  captured: Coord[];       // opponent stones removed by this move
  thinkMs: number;         // time the actor spent on this move
  source: MoveSource;
  analysisId?: string;     // present when source === "ai"
  analysis?: MoveAnalysis; // engine eval captured for AI moves
}

/**
 * Where a game came from.
 *
 * - `fixture` — bundled demo content; lives in code, never edited or deleted.
 * - `local`   — the user played it on this device; lives in IndexedDB.
 *
 * (Future kinds: `lan`, `online` — same record shape, different provenance.)
 */
export type GameKind = "fixture" | "local";

/** Mode the game was played in. */
export type GameMode = "hotseat" | "vsai" | "lan" | "online";

export interface GameSummary {
  id: string;
  kind: GameKind;
  title: string;
  black: string;           // player name / "AI (depth 6)"
  white: string;
  status: GameStatus;
  moveCount: number;
  updatedAt: string;       // ISO
  lastCoord?: Coord;
}

export interface Game extends GameSummary {
  mode: GameMode;
  moves: Move[];
  captures: { black: number; white: number };
  createdAt: string;
  /** Engine search depth, or `ANY_DEPTH` for time-bounded (only `vsai`). */
  aiDepth?: number;
  /** Wall-clock budget (ms) for the AI's "any depth" searches (`vsai`). */
  aiTimeoutMs?: number;
  /** Which side the AI plays (only meaningful for `vsai` games). */
  aiSide?: Player;
}

/**
 * Aggregate result of one search, shown in the analysis panel.
 *
 * The engine surfaces the chosen move, the root-side score, and the
 * search-cost counters: the depth bound requested, the deepest iteration it
 * actually completed, the deepest ply it explored, and the total node count.
 */
export interface Analysis {
  id: string;
  gameId: string;
  moveIndex: number;
  chosen: Coord | null;
  rootScore: number;
  thinkMs: number;
  /** Depth bound requested for the search (`ANY_DEPTH` for time-bounded). */
  depth: number;
  /** Deepest iterative-deepening depth the search fully completed. */
  depthReached: number;
  /** Deepest ply the search explored overall. */
  maxPly: number;
  /** Total nodes (incl. leaves) visited across all iterations. */
  nodesVisited: number;
  /**
   * What the engine recorded when it actually played this move, if it was an
   * AI move. Shown alongside the fresh automatic analysis so the review can
   * compare "what the AI decided then" against "what the engine thinks now".
   * Carries the move the AI played and the time it spent, plus its search
   * counters.
   */
  recorded?: MoveAnalysis & {
    /** The move the AI actually played. */
    chosen: Coord;
    /** Time the AI spent choosing it. */
    thinkMs: number;
  };
}
