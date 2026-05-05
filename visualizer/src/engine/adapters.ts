/**
 * Adapters between `engine-wasm` DTOs and the visualizer's UI types.
 *
 * The engine speaks in flat arrays and side-to-move scores; the UI speaks
 * in nested trees and per-cell grids. These conversions are pure and live
 * here so components don't reach across the boundary themselves.
 */

import type {
  CellDTO,
  FlatNodeDTO,
  GameStateDTO,
  PlayerDTO,
  SearchTreeDTO,
} from "engine-wasm";
import type {
  Analysis,
  Board,
  CandidateMove,
  Cell,
  Coord,
  Player,
  TreeNode,
} from "@/api/types";

/** Side-to-move scores → flip on every parent edge to recover root-side scores. */
function rootSideScore(node: FlatNodeDTO, depthFromRoot: number): number {
  return depthFromRoot % 2 === 0 ? node.score : -node.score;
}

function cellToPlayer(c: CellDTO): Cell {
  if (c === "black") return "black";
  if (c === "white") return "white";
  return null;
}

function playerOf(p: PlayerDTO): Player {
  return p;
}

/** Flat row-major `CellDTO[]` → 2D `Board` indexed `[y][x]`. */
export function cellsToBoard(cells: CellDTO[], boardSize: number): Board {
  const board: Board = Array.from({ length: boardSize }, () =>
    Array.from({ length: boardSize }, () => null as Cell),
  );
  for (let i = 0; i < cells.length; i++) {
    const x = i % boardSize;
    const y = Math.floor(i / boardSize);
    board[y][x] = cellToPlayer(cells[i]);
  }
  return board;
}

export function snapshotToBoard(state: GameStateDTO): Board {
  return cellsToBoard(state.board, state.board_size);
}

/**
 * Flatten → nested, with scores normalized to the root side's perspective.
 *
 * The engine records `score` from each node's side-to-move (negamax). For
 * a UI that compares scores across siblings of mixed depth, we want them
 * all in the *root mover's* frame. We compute depth-from-root via parent
 * walk on the first pass, then flip every other level.
 */
export function flatTreeToNested(tree: SearchTreeDTO): TreeNode | null {
  if (tree.nodes.length === 0) return null;

  const depths = new Uint32Array(tree.nodes.length);
  for (let i = 1; i < tree.nodes.length; i++) {
    const parent = tree.nodes[i].parent;
    depths[i] = parent === undefined ? 0 : depths[parent] + 1;
  }

  const nested: TreeNode[] = tree.nodes.map((n, i) => ({
    id: `n${i}`,
    coord: n.move ? { x: n.move.x, y: n.move.y } : { x: -1, y: -1 },
    player: playerOf(n.player_to_move),
    score: rootSideScore(n, depths[i]),
    depth: n.depth_remaining,
    alpha: n.alpha_in,
    beta: n.beta_in,
    pruned: n.pruned,
    children: [],
  }));

  for (let i = 1; i < tree.nodes.length; i++) {
    const parentIdx = tree.nodes[i].parent!;
    nested[parentIdx].children.push(nested[i]);
  }
  return nested[0];
}

/**
 * The chain of best-scoring children from the root, in play order.
 *
 * "Best" is measured in the root mover's frame, which `flatTreeToNested`
 * has already normalized. The root itself contributes no move (no `coord`),
 * so the returned coords start at depth 1.
 */
export function principalVariation(root: TreeNode | null): Coord[] {
  if (!root) return [];
  const pv: Coord[] = [];
  let cur: TreeNode | undefined = root;
  let isRootSide = true;
  while (cur && cur.children.length > 0) {
    const children = cur.children;
    let next: TreeNode = children[0];
    for (let i = 1; i < children.length; i++) {
      const c = children[i];
      const better = isRootSide ? c.score > next.score : c.score < next.score;
      if (better) next = c;
    }
    pv.push(next.coord);
    cur = next;
    isRootSide = !isRootSide;
  }
  return pv;
}

/**
 * Count nodes in each subtree from a flat tree, in O(n).
 *
 * Iterates back-to-front: a node's count is `1 + Σ children counts`. Since
 * the flatten emits children after parents, by the time we visit index `i`
 * all of `i`'s descendants have already been counted. Indexing into this
 * array gives the subtree size of any node (including itself).
 */
function subtreeSizes(tree: SearchTreeDTO): Uint32Array {
  const sizes = new Uint32Array(tree.nodes.length);
  for (let i = tree.nodes.length - 1; i >= 0; i--) {
    sizes[i] += 1;
    const parent = tree.nodes[i].parent;
    if (parent !== undefined) sizes[parent] += sizes[i];
  }
  return sizes;
}

/**
 * Build the full Analysis payload from one verbose-search result.
 *
 * Candidates are the root's direct children, scored from the root mover's
 * perspective (= negate the child's negamax score). `subtreeNodes` is what
 * the engine spent below each candidate; pruned siblings will report fewer.
 */
export function treeToAnalysis(args: {
  id: string;
  gameId: string;
  moveIndex: number;
  depth: number;
  nodesVisited: number;
  thinkMs: number;
  chosen: Coord | null;
  tree: SearchTreeDTO;
}): Analysis {
  const root = flatTreeToNested(args.tree);
  const sizes = subtreeSizes(args.tree);

  const candidates: CandidateMove[] = [];
  for (let i = 1; i < args.tree.nodes.length; i++) {
    const n = args.tree.nodes[i];
    if (n.parent !== 0 || !n.move) continue;
    candidates.push({
      coord: { x: n.move.x, y: n.move.y },
      score: -n.score, // child negamax → root mover's frame
      subtreeNodes: sizes[i],
      pruned: n.pruned,
    });
  }
  candidates.sort((a, b) => b.score - a.score);

  return {
    id: args.id,
    gameId: args.gameId,
    moveIndex: args.moveIndex,
    chosen: args.chosen,
    rootScore: root?.score ?? 0,
    thinkMs: args.thinkMs,
    depth: args.depth,
    nodesVisited: args.nodesVisited,
    candidates,
    principalVariation: principalVariation(root),
  };
}
