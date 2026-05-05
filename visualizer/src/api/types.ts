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
 * One root-child of the search tree, surfaced as a "candidate move".
 *
 * Score is in the root mover's frame (already sign-flipped from the engine's
 * negamax convention). `subtreeNodes` is how many nodes the engine spent
 * exploring beneath this candidate.
 */
export interface CandidateMove {
  coord: Coord;
  score: number;
  subtreeNodes: number;
  pruned: boolean;
}

/** Aggregate result of one verbose search, ready for the analysis panel. */
export interface Analysis {
  id: string;
  gameId: string;
  moveIndex: number;
  chosen: Coord | null;
  rootScore: number;
  thinkMs: number;
  depth: number;
  nodesVisited: number;
  /** Top-level branches the search considered, ordered best→worst. */
  candidates: CandidateMove[];
  /** Best-child chain from root, in play order. */
  principalVariation: Coord[];
}

/**
 * One node in the min-max search tree, post-flatten.
 *
 * `score` is normalized to the *root* side-to-move's perspective so siblings
 * across plies stay comparable. `alpha`/`beta` are the window the node was
 * called with (still in side-to-move frame — it's what the engine actually
 * used to prune). `pruned` is true when this node returned on a β-cutoff.
 */
export interface TreeNode {
  id: string;
  coord: Coord;
  player: Player;
  score: number;
  depth: number;
  alpha: number;
  beta: number;
  pruned: boolean;
  children: TreeNode[];
}
