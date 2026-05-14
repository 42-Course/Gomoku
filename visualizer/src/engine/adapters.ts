/**
 * Adapters between `engine-wasm` DTOs and the visualizer's UI types.
 *
 * The engine speaks in flat arrays of cells; the UI speaks in 2D grids.
 * These conversions are pure and live here so components don't reach
 * across the boundary themselves.
 */

import type { CellDTO, GameStateDTO } from "engine-wasm";
import type { Board, Cell } from "@/api/types";

function cellToPlayer(c: CellDTO): Cell {
  if (c === "black") return "black";
  if (c === "white") return "white";
  return null;
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
