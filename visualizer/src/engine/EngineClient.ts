/**
 * Main-thread proxy for the engine worker.
 *
 * Each method posts a Request, stashes a Promise resolver keyed by id, and
 * resolves it when the matching Response arrives. The UI never sees raw
 * `postMessage` traffic — it `await`s typed methods.
 *
 * The worker holds *one* `GameHandle`, so callers must coordinate state.
 * For analyzing a saved game at move N, use `replay(moves, N)` to drive
 * the handle there before calling `bestMove`.
 */

import type {
  BestMoveDTO,
  GameStateDTO,
  PlayResultDTO,
} from "engine-wasm";
import type { Request, Response } from "./messages";

type Pending = {
  resolve: (value: unknown) => void;
  reject: (err: Error) => void;
};

let nextId = 0;
const newId = () => `r${++nextId}`;

export class EngineClient {
  private worker: Worker;
  private pending = new Map<string, Pending>();

  constructor() {
    this.worker = new Worker(new URL("./EngineWorker.ts", import.meta.url), {
      type: "module",
    });
    this.worker.addEventListener("message", (ev: MessageEvent<Response>) => {
      const msg = ev.data;
      const p = this.pending.get(msg.id);
      if (!p) return;
      this.pending.delete(msg.id);
      if (msg.ok) p.resolve(msg);
      else p.reject(new Error(msg.error));
    });
  }

  private send<R extends Response & { ok: true }>(req: Request): Promise<R> {
    return new Promise<R>((resolve, reject) => {
      this.pending.set(req.id, { resolve: resolve as (v: unknown) => void, reject });
      this.worker.postMessage(req);
    });
  }

  play(x: number, y: number): Promise<PlayResultDTO> {
    return this.send<Extract<Response, { kind: "play"; ok: true }>>({
      id: newId(),
      kind: "play",
      x,
      y,
    }).then((r) => r.result);
  }

  undo(): Promise<void> {
    return this.send<Extract<Response, { kind: "undo"; ok: true }>>({
      id: newId(),
      kind: "undo",
    }).then(() => undefined);
  }

  snapshot(): Promise<GameStateDTO> {
    return this.send<Extract<Response, { kind: "snapshot"; ok: true }>>({
      id: newId(),
      kind: "snapshot",
    }).then((r) => r.state);
  }

  reset(): Promise<void> {
    return this.send<Extract<Response, { kind: "reset"; ok: true }>>({
      id: newId(),
      kind: "reset",
    }).then(() => undefined);
  }

  bestMove(depth: number): Promise<{ result: BestMoveDTO; thinkMs: number }> {
    return this.send<Extract<Response, { kind: "bestMove"; ok: true }>>({
      id: newId(),
      kind: "bestMove",
      depth,
    }).then((r) => ({ result: r.result, thinkMs: r.thinkMs }));
  }

  /**
   * Drive the engine to the position after `upToIndex` moves of `moves`.
   *
   * Resets first, then replays sequentially. Make/unmake is cheap, so this
   * is fine for review-mode position-jumping. `upToIndex` is inclusive;
   * `-1` leaves an empty board.
   */
  async replay(
    moves: ReadonlyArray<{ x: number; y: number }>,
    upToIndex: number,
  ): Promise<void> {
    await this.reset();
    const limit = Math.min(upToIndex, moves.length - 1);
    for (let i = 0; i <= limit; i++) {
      const m = moves[i];
      await this.play(m.x, m.y);
    }
  }

  dispose() {
    this.worker.terminate();
    this.pending.clear();
  }
}

let singleton: EngineClient | null = null;

/** Lazy-init shared client. Most of the app should use this, not `new`. */
export function getEngine(): EngineClient {
  if (!singleton) singleton = new EngineClient();
  return singleton;
}
