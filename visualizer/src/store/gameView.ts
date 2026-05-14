import { create } from "zustand";
import { persist } from "zustand/middleware";

type ToggleKey =
  | "showLastMove"
  | "autoAnalyze"
  | "showCoordinates"
  | "showCrosshair"
  | "showHoverGhost"
  | "showEvalBar";

interface GameViewState {
  /** Index of the move currently being displayed (−1 = empty board). */
  selectedMoveIndex: number;
  showLastMove: boolean;
  /** Render A–T / 1–19 column & row labels around the board edges. */
  showCoordinates: boolean;
  /** Vertical + horizontal guide lines following the cursor. */
  showCrosshair: boolean;
  /** Translucent ghost stone at the hovered intersection. */
  showHoverGhost: boolean;
  /** Lichess-style eval bar (driven by current analysis, if any). */
  showEvalBar: boolean;
  /** When true, every move you land on runs a search automatically. */
  autoAnalyze: boolean;

  setSelected: (i: number) => void;
  step: (delta: number, max: number) => void;
  toggle: (key: ToggleKey) => void;
}

/**
 * View-state store. Display toggles persist to localStorage so that a
 * preference set on /play follows the user to /games and across reloads.
 * Ephemeral fields (`selectedMoveIndex`) are stripped from the persisted
 * slice via `partialize`.
 */
export const useGameView = create<GameViewState>()(
  persist(
    (set) => ({
      selectedMoveIndex: -1,
      showLastMove: true,
      showCoordinates: true,
      showCrosshair: true,
      showHoverGhost: true,
      showEvalBar: true,
      autoAnalyze: false,

      setSelected: (i) => set({ selectedMoveIndex: i }),
      step: (delta, max) =>
        set((s) => ({
          selectedMoveIndex: Math.min(max, Math.max(-1, s.selectedMoveIndex + delta)),
        })),
      toggle: (key) =>
        set((s) => ({ [key]: !s[key] } as Partial<GameViewState>)),
    }),
    {
      name: "gomoku.gameView",
      partialize: (s) => ({
        showLastMove: s.showLastMove,
        showCoordinates: s.showCoordinates,
        showCrosshair: s.showCrosshair,
        showHoverGhost: s.showHoverGhost,
        showEvalBar: s.showEvalBar,
        autoAnalyze: s.autoAnalyze,
      }),
    },
  ),
);
