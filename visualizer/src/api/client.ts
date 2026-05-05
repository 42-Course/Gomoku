/**
 * Data access layer for the visualizer.
 *
 * Game records (the move list, players, captures, timestamps) live in the
 * fixtures for now and will move to IndexedDB later — they're persisted
 * data, not engine output. Analyses and search trees, on the other hand,
 * are *recomputed on demand* by `engine-wasm`: we replay the saved game
 * up to the requested move and ask the engine for a verbose search.
 */

import type { Analysis, Game, GameSummary, TreeNode } from "./types";
import { getGameFixture, GAMES as FIXTURE_GAMES } from "./fixtures";
import { getEngine } from "@/engine/EngineClient";
import { flatTreeToNested, treeToAnalysis } from "@/engine/adapters";
import {
  deleteLocalGame,
  getLocalGame,
  listLocalGames,
} from "@/storage/games";

const ANALYSIS_DEPTH = 4;

function summarize(g: Game): GameSummary {
  return {
    id: g.id,
    kind: g.kind,
    title: g.title,
    black: g.black,
    white: g.white,
    status: g.status,
    moveCount: g.moves.length,
    updatedAt: g.updatedAt,
    lastCoord: g.moves[g.moves.length - 1]?.coord,
  };
}

/**
 * The user's saved games. Fixtures live in the Lab, not here — Games is
 * "what you've played", not "everything that exists".
 */
export async function listGames(): Promise<GameSummary[]> {
  const local = await listLocalGames();
  return local.map(summarize);
}

/**
 * Bundled demo content for the Lab. These are read-only learning aids —
 * never written to IndexedDB, never deletable.
 */
export async function listFixtures(): Promise<GameSummary[]> {
  return FIXTURE_GAMES.map(summarize);
}

/**
 * Resolve a game by id from either source. The review screen doesn't
 * care where a game comes from — it renders the same way for fixtures
 * and saved games.
 */
export async function getGame(id: string): Promise<Game> {
  const local = await getLocalGame(id);
  if (local) return local;
  const fixture = getGameFixture(id);
  if (fixture) return fixture;
  throw new Error(`game ${id} not found`);
}

/** Permanently remove a saved game. Fixtures cannot be deleted. */
export async function deleteGame(id: string): Promise<void> {
  await deleteLocalGame(id);
}

/**
 * Run a verbose search from the position *just before* `moveIndex`.
 *
 * That's the position the AI was facing when it chose `moves[moveIndex]`.
 * The engine doesn't know about move sources or game IDs — we replay the
 * saved game's coordinates to drive its handle to the right state.
 */
/**
 * Replay the saved coords through the engine. May fail when a fixture move
 * violates Gomoku rules — fixtures are random walks, the engine is strict.
 * Returns true on success so callers can degrade gracefully.
 */
async function replaySafely(
  coords: ReadonlyArray<{ x: number; y: number }>,
  upTo: number,
): Promise<boolean> {
  try {
    await getEngine().replay(coords, upTo);
    return true;
  } catch {
    return false;
  }
}

/**
 * Run a verbose search from the position *just before* `moveIndex`.
 *
 * That's the position whoever-was-on-move was facing when they played
 * `moves[moveIndex]`. Works for AI moves (where it shows what the engine
 * actually did) and human moves (where it shows what the engine *would*
 * have done). Returns null when the saved move list can't be replayed
 * (random-walk fixtures sometimes hit illegal positions).
 */
export async function getAnalysis(
  gameId: string,
  moveIndex: number,
): Promise<Analysis | null> {
  if (moveIndex < 0) return null;
  const g = await getGame(gameId);
  if (!g.moves[moveIndex]) return null;

  const ok = await replaySafely(g.moves.map((m) => m.coord), moveIndex - 1);
  if (!ok) return null;

  const { result, tree, thinkMs } = await getEngine().bestMoveVerbose(
    ANALYSIS_DEPTH,
  );
  return treeToAnalysis({
    id: `a_${gameId}_${moveIndex}`,
    gameId,
    moveIndex,
    depth: ANALYSIS_DEPTH,
    nodesVisited: result.nodes_visited,
    thinkMs,
    chosen: result.move ? { x: result.move.x, y: result.move.y } : null,
    tree,
  });
}

export async function getTree(
  gameId: string,
  moveIndex: number,
): Promise<TreeNode | null> {
  if (moveIndex < 0) return null;
  const g = await getGame(gameId);
  if (!g.moves[moveIndex]) return null;

  const ok = await replaySafely(g.moves.map((m) => m.coord), moveIndex - 1);
  if (!ok) return null;

  const { tree } = await getEngine().bestMoveVerbose(ANALYSIS_DEPTH);
  return flatTreeToNested(tree);
}

export const queryKeys = {
  games: ["games"] as const,
  fixtures: ["fixtures"] as const,
  game: (id: string) => ["game", id] as const,
  analysis: (gameId: string, moveIndex: number) =>
    ["analysis", gameId, moveIndex] as const,
  tree: (gameId: string, moveIndex: number) =>
    ["tree", gameId, moveIndex] as const,
};
