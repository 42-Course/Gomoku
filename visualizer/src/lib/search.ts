/**
 * Shared search-strength helpers.
 *
 * A search is described by a depth bound and a wall-clock budget. The engine
 * runs iterative deepening up to `depth`, stopping early once `timeoutMs` is
 * exceeded — so a fixed depth runs to completion under a generous safety cap,
 * while "any depth" runs as deep as the budget allows.
 */

/** Sentinel depth meaning "search as deep as the time budget allows". */
export const ANY_DEPTH = 99;

/** Highest fixed depth the strength sliders expose. */
export const MAX_SLIDER_DEPTH = 10;

/** Safety cap so a fixed-depth search can't hang the worker indefinitely. */
export const FIXED_DEPTH_BUDGET_MS = 10_000;

/** Default wall-clock budget for an "any depth" search. */
export const DEFAULT_ANY_BUDGET_MS = 2_000;

/** Budget choices (ms) offered for "any depth" play. */
export const ANY_BUDGET_CHOICES_MS = [500, 1_000, 2_000, 4_000, 8_000] as const;

/**
 * Progressive auto-analysis budgets. Each pass doubles the wall-clock budget
 * (so the result deepens) starting from `AUTO_START_BUDGET_MS` and capping at
 * `AUTO_MAX_BUDGET_MS`. The cap also bounds how long a queued move waits behind
 * an in-flight analysis search, keeping the board responsive.
 */
export const AUTO_START_BUDGET_MS = 120;
export const AUTO_MAX_BUDGET_MS = 2_000;

export function isAnyDepth(depth: number): boolean {
  return depth >= ANY_DEPTH;
}

/** Human label for a depth value (numbers as-is, the sentinel as "Any"). */
export function depthLabel(depth: number): string {
  return isAnyDepth(depth) ? "Any" : String(depth);
}

/**
 * Resolve a depth value into the `(depth, timeoutMs)` pair the engine wants.
 *
 * Fixed depths get a generous safety cap; "any depth" gets the supplied
 * wall-clock budget, which is what actually bounds the search.
 */
export function searchBudget(
  depth: number,
  anyBudgetMs: number = DEFAULT_ANY_BUDGET_MS,
): { depth: number; timeoutMs: number } {
  return isAnyDepth(depth)
    ? { depth: ANY_DEPTH, timeoutMs: anyBudgetMs }
    : { depth, timeoutMs: FIXED_DEPTH_BUDGET_MS };
}
