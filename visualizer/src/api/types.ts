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

export interface Move {
  index: number;           // 0-based turn number
  player: Player;
  coord: Coord;
  captured: Coord[];       // opponent stones removed by this move
  thinkMs: number;         // time the actor spent on this move
  source: MoveSource;
  analysisId?: string;     // present when source === "ai"
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
  /** Engine search depth (only meaningful for `vsai` games). */
  aiDepth?: number;
}

/**
 * Aggregate result of one search, shown in the analysis panel.
 *
 * The engine only surfaces the chosen move, the root-side score, and the
 * search-cost counters now — the verbose tree / candidate breakdown used to
 * live here but was removed along with the engine's verbose search mode.
 */
export interface Analysis {
  id: string;
  gameId: string;
  moveIndex: number;
  chosen: Coord | null;
  rootScore: number;
  thinkMs: number;
  depth: number;
  nodesVisited: number;
}
