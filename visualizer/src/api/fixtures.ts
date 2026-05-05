/**
 * Static game records for the visualizer.
 *
 * These are *persisted-data* stand-ins (move list, players, captures,
 * timestamps) that will live in IndexedDB once a real "play a game" UI
 * exists. Analysis and search trees are NOT here — the engine produces
 * those on demand from the moves below.
 *
 * Note on legality: random walks may emit moves the engine rejects (double
 * threes, occupied cells once captures clear, etc.). The client treats a
 * failed replay as "no analysis available" rather than a crash.
 */

import type { Board, Game, GameSummary, Move } from "./types";

export const BOARD_SIZE = 19;

export function emptyBoard(): Board {
  return Array.from({ length: BOARD_SIZE }, () =>
    Array.from({ length: BOARD_SIZE }, () => null),
  );
}

/**
 * Replay moves onto an empty board up to (and including) `upTo` index.
 * Respects captures: any coord recorded in `move.captured` is cleared.
 */
export function boardAtMove(moves: Move[], upTo: number): Board {
  const board = emptyBoard();
  const limit = Math.min(upTo, moves.length - 1);
  for (let i = 0; i <= limit; i++) {
    const m = moves[i];
    board[m.coord.y][m.coord.x] = m.player;
    for (const c of m.captured) board[c.y][c.x] = null;
  }
  return board;
}

/** Deterministic pseudo-random so fixtures don't flicker. */
function mulberry32(seed: number) {
  return () => {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function makeMoves(seed: number, n: number): Move[] {
  const rnd = mulberry32(seed);
  const used = new Set<string>();
  const moves: Move[] = [];
  let x = 9;
  let y = 9;
  for (let i = 0; i < n; i++) {
    for (let tries = 0; tries < 20; tries++) {
      const dx = Math.floor(rnd() * 5) - 2;
      const dy = Math.floor(rnd() * 5) - 2;
      const nx = Math.min(18, Math.max(0, x + dx));
      const ny = Math.min(18, Math.max(0, y + dy));
      const key = `${nx},${ny}`;
      if (!used.has(key)) {
        used.add(key);
        x = nx;
        y = ny;
        break;
      }
    }
    const player = i % 2 === 0 ? "black" : "white";
    const source = i % 2 === 1 ? "ai" : "human";
    moves.push({
      index: i,
      player,
      coord: { x, y },
      captured: [],
      thinkMs:
        source === "ai"
          ? Math.floor(200 + rnd() * 4800)
          : Math.floor(500 + rnd() * 12000),
      source,
      analysisId: source === "ai" ? `a_${seed}_${i}` : undefined,
    });
  }
  return moves;
}

function summaryOf(g: Game): GameSummary {
  const last = g.moves[g.moves.length - 1];
  return {
    id: g.id,
    kind: g.kind,
    title: g.title,
    black: g.black,
    white: g.white,
    status: g.status,
    moveCount: g.moves.length,
    updatedAt: g.updatedAt,
    lastCoord: last?.coord,
  };
}

function makeGame(
  id: string,
  title: string,
  black: string,
  white: string,
  seed: number,
  n: number,
  status: Game["status"] = { kind: "ongoing" },
): Game {
  const moves = makeMoves(seed, n);
  return {
    id,
    kind: "fixture",
    mode: "vsai",
    aiDepth: 4,
    title,
    black,
    white,
    status,
    moves,
    moveCount: moves.length,
    captures: { black: 0, white: 0 },
    createdAt: "2026-04-20T09:00:00Z",
    updatedAt: "2026-04-22T14:12:00Z",
  };
}

export const GAMES: Game[] = [
  makeGame("g1", "Opening study — Pro Pro swap", "You", "pela AI (depth 6)", 1, 28),
  makeGame("g2", "Defense demo #3 — capture race", "pela AI", "Guest", 7, 16, {
    kind: "win",
    player: "black",
  }),
  makeGame("g3", "Tournament rehearsal", "AI α", "AI β", 42, 42),
  makeGame("g4", "Double-three sandbox", "You", "You", 99, 6),
];

export function listGamesFixture(): GameSummary[] {
  return GAMES.map(summaryOf);
}

export function getGameFixture(id: string): Game | undefined {
  return GAMES.find((g) => g.id === id);
}
