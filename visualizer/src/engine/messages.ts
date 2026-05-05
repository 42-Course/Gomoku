/**
 * Wire types for the EngineClient ↔ EngineWorker channel.
 *
 * Every Request carries an `id` the worker echoes back on its Response so
 * the client can resolve the matching Promise. The DTO shapes come straight
 * from `engine-wasm` — we don't re-model them here, we just route them.
 */

import type {
  BestMoveDTO,
  GameStateDTO,
  PlayResultDTO,
  SearchTreeDTO,
} from "engine-wasm";

export type Request =
  | { id: string; kind: "play"; x: number; y: number }
  | { id: string; kind: "undo" }
  | { id: string; kind: "snapshot" }
  | { id: string; kind: "reset" }
  | { id: string; kind: "bestMove"; depth: number }
  | { id: string; kind: "bestMoveVerbose"; depth: number };

export type Response =
  | { id: string; ok: true; kind: "play"; result: PlayResultDTO }
  | { id: string; ok: true; kind: "undo" }
  | { id: string; ok: true; kind: "snapshot"; state: GameStateDTO }
  | { id: string; ok: true; kind: "reset" }
  | { id: string; ok: true; kind: "bestMove"; result: BestMoveDTO; thinkMs: number }
  | {
      id: string;
      ok: true;
      kind: "bestMoveVerbose";
      result: BestMoveDTO;
      tree: SearchTreeDTO;
      thinkMs: number;
    }
  | { id: string; ok: false; error: string };
