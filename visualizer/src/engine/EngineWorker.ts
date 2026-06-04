/**
 * Web Worker that owns one `GameHandle` for the lifetime of the page.
 *
 * The main thread never touches Wasm directly — it sends typed Requests
 * (see `messages.ts`) and receives Responses. Keeping the engine off the
 * main thread is what lets a depth-6 search run without freezing the UI.
 */

import init, { GameHandle } from "engine-wasm";
import type { Request, Response } from "./messages";

let handle: GameHandle | null = null;
const ready = init().then(() => {
  handle = new GameHandle();
});

function reply(msg: Response) {
  (self as unknown as Worker).postMessage(msg);
}

self.addEventListener("message", async (ev: MessageEvent<Request>) => {
  await ready;
  const req = ev.data;
  if (!handle) {
    reply({ id: req.id, ok: false, error: "engine not initialized" });
    return;
  }

  try {
    switch (req.kind) {
      case "play": {
        const result = handle.play(req.x, req.y);
        reply({ id: req.id, ok: true, kind: "play", result });
        return;
      }
      case "undo": {
        handle.undo();
        reply({ id: req.id, ok: true, kind: "undo" });
        return;
      }
      case "snapshot": {
        const state = handle.snapshot();
        reply({ id: req.id, ok: true, kind: "snapshot", state });
        return;
      }
      case "reset": {
        // GameHandle has no in-place reset; drop the old one and rebuild.
        handle.free();
        handle = new GameHandle();
        reply({ id: req.id, ok: true, kind: "reset" });
        return;
      }
      case "bestMove": {
        const t0 = performance.now();
        const result = handle.bestMove(req.depth, req.timeoutMs);
        const thinkMs = performance.now() - t0;
        reply({ id: req.id, ok: true, kind: "bestMove", result, thinkMs });
        return;
      }
    }
  } catch (e) {
    reply({ id: req.id, ok: false, error: e instanceof Error ? e.message : String(e) });
  }
});
