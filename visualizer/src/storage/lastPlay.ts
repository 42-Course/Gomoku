/**
 * Live-game session persistence for the /play page.
 *
 * The user's in-progress game is stashed to localStorage on every move so
 * that closing the tab, reloading, or navigating away doesn't lose work.
 * Home reads it to show a "Resume" tile, and /play replays it through the
 * engine on mount.
 *
 * This is intentionally small (no captures, no signatures) — it's the
 * minimum needed to reconstruct a Game by replaying the moves through the
 * engine. The full saved-game record only lands in IndexedDB when the
 * user actually presses Save.
 */

import type { GameMode, GameStatus, Move, Player } from "@/api/types";

const KEY = "gomoku.lastPlay";

export interface LastPlay {
  mode: GameMode;
  aiDepth?: number;
  aiSide?: Player;
  moves: Move[];
  status: GameStatus;
  captures: { black: number; white: number };
  /** ISO timestamp of the most recent change. */
  updatedAt: string;
  /** True once the user clicked "Start" (or moved at all in hot-seat). */
  started: boolean;
  /** Linked saved-game id if the user has saved this session. */
  savedId?: string;
}

export function readLastPlay(): LastPlay | null {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return null;
    return JSON.parse(raw) as LastPlay;
  } catch {
    return null;
  }
}

export function writeLastPlay(p: LastPlay): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(p));
  } catch {
    // localStorage might be disabled / full; the in-memory state stays
    // correct, the user just won't get resume-on-reload.
  }
}

export function clearLastPlay(): void {
  try {
    localStorage.removeItem(KEY);
  } catch {
    /* ignore */
  }
}
