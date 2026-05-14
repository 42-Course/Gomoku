# engine-wasm

WebAssembly bindings for the Gomoku [`engine`](../engine).

## What this crate is

A thin `wasm-bindgen` wrapper that exposes a stateful `GameHandle` plus a
small set of DTOs. The engine itself stays plain Rust — no `wasm-bindgen`
attributes leak into it — so native callers and the browser share one
implementation without one constraining the other.

## Build

```bash
cd engine-wasm
./build.sh              # equivalent to: ./build.sh web release
./build.sh web dev      # faster iteration, larger artifact
./build.sh bundler      # for Next.js / Webpack consumers
```

Output lands in `engine-wasm/pkg/` as a drop-in npm package:

```
pkg/
├── engine_wasm.js
├── engine_wasm_bg.wasm
├── engine_wasm.d.ts
└── package.json
```

Install `wasm-pack` once if you don't have it:

```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

## Use from TypeScript

```ts
import init, { GameHandle } from "engine-wasm";

await init();                       // fetch + instantiate the .wasm
const handle = new GameHandle();

handle.play(9, 9);
const result = handle.bestMove(4);  // { move, score, nodesVisited }
const view   = handle.snapshot();   // GameStateDTO

handle.free();                      // when done — releases the Wasm memory
```

For the visualizer, do the search on a Web Worker. A depth-4+ search will
jank the main thread otherwise.

## API

| Method                          | Returns               | Notes                                              |
| ------------------------------- | --------------------- | -------------------------------------------------- |
| `new GameHandle()`              | `GameHandle`          | Empty board, Black to move.                        |
| `handle.play(x, y)`             | `PlayResultDTO`       | Throws on rule violations (e.g. double three).     |
| `handle.undo()`                 | `void`                | Throws if there's nothing to undo.                 |
| `handle.bestMove(depth)`        | `BestMoveDTO`         | Game state unchanged on return.                    |
| `handle.snapshot()`             | `GameStateDTO`        | Read-only view for rendering.                      |

DTOs are derived via `tsify`, so the generated `.d.ts` has real
`interface` definitions (no `any`).

## Layout

```
src/
├── lib.rs        — module wiring, panic hook
├── handle.rs     — GameHandle and #[wasm_bindgen] entry points
└── dto.rs        — DTOs that cross the FFI boundary
```
