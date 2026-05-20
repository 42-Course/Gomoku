import type { Coord } from "@/api/types";

/** Columns A..T skipping "I" (gomoku convention, matches renju/gomocup). */
const COL_LETTERS = "ABCDEFGHJKLMNOPQRST";

export function coordLabel({ x, y }: Coord): string {
  return `${COL_LETTERS[x] ?? "?"}${y + 1}`;
}

export function formatMs(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(1)}s`;
  const m = Math.floor(s / 60);
  const rem = Math.round(s - m * 60);
  return `${m}m ${rem}s`;
}

export function relativeFromNow(iso: string, now = Date.now()): string {
  const t = new Date(iso).getTime();
  const diff = Math.max(0, now - t);
  const s = diff / 1000;
  if (s < 60) return `${Math.floor(s)}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}
