# Gomoku Visualizer — Design & Development Guide

> **Status:** living document. Update it as the visualizer evolves; treat
> deviations as a signal to either change the doc or change the code.

This is the design contract for the Gomoku **visualizer** — the
TypeScript app in `visualizer/` that consumes
[`engine-wasm`](../engine-wasm) and presents the game to users. It
covers the philosophy, the architecture, the storage model, the play
modes, the analysis features, and a concrete development roadmap.

---

## 1. Philosophy

The visualizer is **offline-first** and **local-driven**. Open the page,
play immediately. No login, no server round-trip, no permissions wall.

Three principles drive every design choice:

1. **Local works fully on its own.** Every feature except multiplayer
   must work with the network unplugged. The Wasm engine, the storage
   layer, the AI, the board editor, the analyzer — all client-side.
2. **Online is an opt-in upgrade, not a precondition.** When a user
   wants identity continuity across devices, leaderboards, or live
   multiplayer, they log in to a server account. Until then, everything
   they do is theirs and lives on their device.
3. **We don't fight cheaters.** Local data is hackable by design. We
   make tampering inconvenient (signed records, server-side
   reconciliation when online), not impossible. Stats from a purely
   offline player are *not* authoritative; that's a feature, not a bug.

---

## 2. Architecture overview

```
┌──────────────────────────────────────────────────────────────────┐
│                         Browser                                  │
│                                                                  │
│  ┌──────────────────┐    ┌──────────────────────────────────┐   │
│  │  React UI (main  │◄──►│  AppState (Zustand or context)   │   │
│  │  thread)         │    │   — game, settings, identity     │   │
│  └──────────────────┘    └──────────────────────────────────┘   │
│           ▲                              ▲                       │
│           │                              │                       │
│           ▼                              ▼                       │
│  ┌──────────────────┐    ┌──────────────────────────────────┐   │
│  │  Engine Worker   │    │  Storage layer                   │   │
│  │  (Web Worker)    │    │   — IndexedDB (games, friends)   │   │
│  │   loads          │    │   — localStorage (prefs, ident)  │   │
│  │   engine-wasm    │    │   — signed records               │   │
│  └──────────────────┘    └──────────────────────────────────┘   │
│           ▲                              ▲                       │
│           │                              │                       │
│           ▼                              ▼                       │
│  ┌──────────────────┐    ┌──────────────────────────────────┐   │
│  │  Net layer       │    │  Identity layer                  │   │
│  │  (lazy)          │    │   — local UUID + signing key     │   │
│  │  WebSocket /     │    │   — optional server account      │   │
│  │  WebRTC          │    │     bound to the local key       │   │
│  └──────────────────┘    └──────────────────────────────────┘   │
│           ▲                                                      │
└───────────┼──────────────────────────────────────────────────────┘
            │
            ▼
       Optional
       multiplayer
       server
```

Every box on the left edge (UI, engine worker, net) is **stateless about
identity**. They all read identity through the storage layer, which is
the single point of truth.

---

## 3. Module layout

```
visualizer/src/
├── main.tsx                    — Vite entry
├── app/                        — top-level routing & layout
│   ├── App.tsx
│   ├── routes.tsx              — /play, /editor, /stats, /friends, /settings
│   └── shell/                  — sidebar, topbar, mobile nav
│
├── engine/                     — wraps engine-wasm via Web Worker
│   ├── EngineWorker.ts         — worker script that owns one GameHandle
│   ├── EngineClient.ts         — main-thread proxy with typed methods
│   └── messages.ts             — request/response message types
│
├── game/                       — game state on the main thread
│   ├── useGame.ts              — hook over the active GameHandle
│   ├── modes/                  — one file per play mode
│   │   ├── HotSeat.ts          — two humans, one device
│   │   ├── VsAI.ts             — human vs engine
│   │   ├── LocalNetwork.ts     — WebRTC/LAN discovery
│   │   ├── OnlineMatch.ts      — server-mediated multiplayer
│   │   └── BoardEditor.ts      — free-form setup, no rules enforced
│   └── analysis/
│       ├── Heatmap.ts          — score overlay across all empty cells
│       ├── BestMoveTracker.ts  — "was your move in the AI's top-N?"
│       └── SearchTreeView.ts   — visualizer for SearchTreeDTO
│
├── storage/                    — IndexedDB + localStorage wrappers
│   ├── db.ts                   — open() the database, version migrations
│   ├── games.ts                — saved games, replays
│   ├── stats.ts                — aggregate user statistics
│   ├── friends.ts              — local roster of opponents played
│   ├── settings.ts             — UI prefs (theme, board style)
│   └── records.ts              — signed-record helpers (see §6)
│
├── identity/
│   ├── localKey.ts             — Ed25519 keypair generation + storage
│   ├── account.ts              — optional online account binding
│   └── signing.ts              — sign / verify game records
│
├── net/                        — only loaded when needed
│   ├── socket.ts               — WebSocket client for the matchmaking server
│   ├── webrtc.ts               — LAN/peer-to-peer discovery
│   └── sync.ts                 — push local records → server reconciliation
│
└── ui/                         — board, pieces, controls
    ├── Board.tsx               — canvas-based renderer
    ├── Cell.tsx
    ├── PieceTray.tsx           — for the editor
    ├── HeatmapOverlay.tsx
    ├── SearchTreePanel.tsx
    └── theme/
```

> Use this map as the directory layout when bootstrapping. Don't create
> all of it on day one; lay down `app/`, `engine/`, `game/modes/HotSeat`,
> `storage/db`, and `ui/Board` first, then grow.

---

## 4. Engine integration

The Wasm engine is **always** behind a Web Worker. The main thread must
stay responsive even during a depth-6 search.

### Loading

```ts
// engine/EngineClient.ts
import init, { GameHandle } from "engine-wasm";
// ...inside the worker:
await init();
const handle = new GameHandle();
```

### Message protocol

The worker speaks one request type:

```ts
type Request =
  | { id: string; kind: "play"; x: number; y: number }
  | { id: string; kind: "undo" }
  | { id: string; kind: "bestMove"; depth: number }
  | { id: string; kind: "bestMoveVerbose"; depth: number }
  | { id: string; kind: "snapshot" }
  | { id: string; kind: "reset" };
```

Responses echo the `id`. The main thread calls into a typed `EngineClient`
that returns Promises. Don't sprinkle `worker.postMessage` across the UI.

### Why one handle

The engine is stateful. Sharing one `GameHandle` for one game means
make/unmake stays cheap. If you ever need a second game (e.g. board
editor preview), spawn a second worker — don't try to share state.

---

## 5. Play modes

Each mode is a **driver** that owns a game session. The board UI doesn't
know what mode it's in; it reads the snapshot, dispatches user moves to
the active driver, and renders whatever comes back.

| Mode                | Local-only? | Notes                                                          |
| ------------------- | ----------- | -------------------------------------------------------------- |
| Hot-seat            | yes         | Two humans on one device. Detect mobile → touch board layout.  |
| Vs AI               | yes         | Human vs `engine-wasm`. Difficulty = search depth.             |
| Local network       | yes (LAN)   | WebRTC over mDNS or a one-time pairing code. No server.        |
| Online multiplayer  | no          | Server-mediated. Requires online account.                      |

### Mobile detection

On viewport `< 640px` *and* coarse pointer (touch), the board switches
to a touch-optimized layout:

- Larger cells, no hover states.
- "Tap to select cell, tap confirm" two-stage placement (no fat-finger
  misclicks).
- Captures the bottom nav for the move history.

Don't gate on user-agent strings; gate on `matchMedia('(pointer: coarse)')`
plus viewport width.

### Local network

Two devices on the same LAN:

1. Host clicks "Play on local network" → app generates a 6-character
   pairing code and starts a WebRTC peer.
2. Guest enters the code on their device.
3. Either via signaling server (light, just relays the SDP) or QR code
   for fully serverless pairing.

This is **online-grade** in feel but requires no account.

### Online multiplayer

Requires identity (§6). The matchmaking server is a thin WebSocket
service: it pairs two accounts, relays moves, declares the winner,
and posts the result to both players' authoritative stats.

**Important**: online games use **server-validated** rules. The client
still runs its own engine for prediction/UX, but the server is the
source of truth. Local engines disagreeing means the cheating client
loses the game.

---

## 6. Identity and the "decentralized first, sync later" model

### Day-zero (no internet ever)

On first load:

1. Generate an Ed25519 keypair locally. Store the private key in
   IndexedDB under a key the user can't accidentally clobber.
2. Generate a UUID-shaped local ID. Display it in Settings as "Your
   local ID".
3. Every game record is **signed** with this key before being stored.

Stats, friends, and replays all live in IndexedDB, signed.

### Day-N (user goes online)

When a user creates an online account:

1. They prove ownership of the local keypair to the server (sign a
   challenge). Server binds `account_id ↔ public_key`.
2. The client uploads signed records. The server **reconciles**:
   - Records that violate engine rules → rejected (the client cheated).
   - Records signed with the *same* key → accepted, deduped by record ID.
   - Records signed with a *different* key → not this account's; ignored.

The server publishes the user's "online stats" derived from records it
verified. The client's "local stats" stay separate; the UI shows both,
clearly labeled.

### Tampering

We expect users to edit their IndexedDB. Local stats are visibly
labeled "(local)"; nothing is ever pushed to leaderboards from local
storage without server validation. A user who wants to cheat their
*own* offline numbers gets to. We don't care.

We make the tampering **inconvenient** but not impossible:

- All records are signed; deleting/forging entries breaks the chain.
- Records reference their parent's hash (Merkle-style). Editing the
  middle of the history requires resigning everything after it, which
  the server will notice during sync if the edits don't match its copy.
- We don't obfuscate or DRM. That's an arms race we'd lose.

### Files involved

| File                          | Purpose                                          |
| ----------------------------- | ------------------------------------------------ |
| `identity/localKey.ts`        | Generate, store, retrieve the Ed25519 key.       |
| `identity/account.ts`         | Optional server-account binding flow.            |
| `identity/signing.ts`         | `sign(record)` / `verify(record, publicKey)`.    |
| `storage/records.ts`          | Wrap raw IndexedDB writes with signing.          |

---

## 7. Storage model

Two backends, used for different things:

### `localStorage`

For small, synchronous, plain-text settings:

- UI theme, board style, board size preview.
- Last-used play mode.
- Whether sound is on.

Never stores anything signed or auth-relevant.

### IndexedDB

For everything that's actually data. One database, multiple object stores:

| Object store     | Schema                                                                    |
| ---------------- | ------------------------------------------------------------------------- |
| `identity`       | `{ id, publicKey, encryptedPrivateKey, createdAt, accountBinding? }`      |
| `games`          | `{ id, mode, players: [...], moves: [...], result, signature, parentHash }` |
| `replays`        | derivable from `games` — keep as a view, not a separate store.            |
| `stats`          | `{ wins, losses, draws, byMode, byOpponent, currentStreak, ... }`         |
| `friends`        | `{ id, displayName, lastPlayed, gamesPlayed, accountId? }`                |
| `puzzles`        | `{ id, name, position: BoardSnapshot, sideToMove, tags[] }`               |

Every write goes through `storage/records.ts`, which signs and chains.

### Schema migrations

Every migration is a numbered upgrade in `storage/db.ts`:

```ts
db.onupgradeneeded = (e) => {
  const db = e.target.result;
  if (e.oldVersion < 1) { /* initial schema */ }
  if (e.oldVersion < 2) { /* added `puzzles` */ }
};
```

Never destructively modify in place; always provide a forward path.

---

## 8. Analysis features

### Heatmap overlay

Analize and make sure to reuse existing code

### Best-move tracker

After every human move:

1. Call `bestMove(depth)` *as if the move had not been played*.
2. Record whether the human's move was the engine's top pick, in the
   top-3, or worse.
3. Show a small badge: "Top move ✓", "Top-3", "Engine preferred (X, Y)".

Surfaces the user's improvement over time without nagging.

### Search-tree visualization

Already supported by `engine-wasm`'s `bestMoveVerbose` — returns a
flattened `SearchTreeDTO`. Render it as:

- A collapsible tree panel for explore-mode users.
- Per-move arrows on the board for the principal variation.
- Pruning markers for the alpha-beta cutoffs.

---

## 9. Statistics

Local-first (always). The user sees:

- **Per-mode**: wins, losses, draws, current streak, best streak.
- **Vs AI**: by depth tier (Easy 2, Medium 4, Hard 6+).
- **Vs friends**: leaderboard *of people you've played*.
- **Move quality**: average "best-move agreement" rate over the last
  20 games. Trend line.

Stored as the `stats` object store. Recomputed lazily from the `games`
store when changes invalidate the cached aggregates.

When online, the server publishes its own "verified stats" — UI shows
both:

```
Wins (local):  47
Wins (online): 12   ← only games the server verified
```

---

## 10. Friends

Local-only by default. Every time you finish a hot-seat / LAN / online
game, prompt: *"Add Player B as a friend?"*

The friend record stores:

- A user-set display name.
- The opponent's public key (if known).
- A history of games against them.
- A `accountId?` once you've both linked online accounts.

When two friends are both online, the friends list shows their status
and lets you start a match directly.

---

## 11. Roadmap (build in this order)

### Phase 1 — Walking skeleton (1 week)

- [ ] `engine/EngineWorker.ts` + `EngineClient.ts` plumbing.
- [ ] Hot-seat mode end-to-end: place stones, captures, win detection.
- [ ] Basic `Board.tsx` (desktop only, no mobile yet).
- [ ] Settings page with theme toggle.

**Done when**: two users can finish a Gomoku game on one machine.

### Phase 2 — AI and local persistence (1 week)

- [ ] Vs AI mode (depth selector).
- [ ] IndexedDB schema + signed records.
- [ ] Local identity: keygen, signing, the user's local ID in Settings.
- [ ] Save completed games, basic stats page.
- [ ] Mobile board layout.

**Done when**: refreshing the page preserves stats; Vs AI is playable on phone.

### Phase 3 — Analysis (1 week)

- [ ] Heatmap overlay (requires new engine endpoint).
- [ ] Best-move tracker badge after each human move.
- [ ] Search-tree side panel for verbose mode.
- [ ] "Play from here" replay scrubber.

**Done when**: user can review any past game move-by-move with full overlay.

### Phase 4 — Board editor (1 week)

- [ ] Free placement, no rules.
- [ ] Save/load puzzles.
- [ ] "Analyze this position" → hand off to the analysis layer.
- [ ] Keyboard shortcuts.

**Done when**: user can build a position from scratch and run AI analysis.

### Phase 5 — LAN multiplayer (1 week)

- [ ] WebRTC pairing via QR code or pairing code.
- [ ] Light signaling server (out of scope for visualizer/, lives in `backend/`).
- [ ] Friends prompt after the game.

**Done when**: two phones on the same WiFi can play each other.

### Phase 6 — Online multiplayer (2+ weeks)

- [ ] Account creation flow.
- [ ] Server-validated games.
- [ ] Online stats sync.
- [ ] Live opponent list.

**Done when**: login → matchmaking → ranked game → updated leaderboard.

---

## 12. Conventions

- **TypeScript strict.** No `any` outside boundary code.
- **No `useEffect` for game logic.** Game state lives in the AppState,
  driven by mode drivers; React effects are for DOM-side concerns only.
- **The engine is the source of truth.** Rules, win detection,
  captures, double-three — *always* via `engine-wasm`. Never reimplement
  in TS, even "for speed". Wrong engines drift.
- **Stateless components, stateful stores.** Components read from the
  store, dispatch actions. No prop-drilling deeper than two levels.
- **Tests live next to the code** (`Foo.test.ts`).

### What not to do

- Don't put rule logic in the UI.
- Don't share `GameHandle` between two games — spawn another worker.
- Don't push to the server from any code path that the user can edit
  (i.e. anything in `storage/`).
- Don't let a feature ship if it doesn't degrade gracefully offline.

---

## 13. Open questions

These are deferred decisions, not blockers:

- **Storage encryption.** Do we encrypt the private key with a
  user-supplied passphrase, or accept that a stolen device = stolen
  key? Probably the latter for v1; reconsider when accounts ship.
- **Replay format.** Plain JSON of `moves[]` is fine for now; switch to
  a binary format (or PGN-style) if file size becomes a concern.
- **Server protocol.** WebSocket message shapes — define when phase 6
  starts, not before.
- **Mobile app.** Capacitor/PWA wrap of the same codebase. After web is
  solid.

---

## 14. References

- [`engine-wasm/README.md`](../engine-wasm/README.md) — the API the
  visualizer consumes.
- [`engine/REFACTOR.md`](../engine/REFACTOR.md) — bitmap pattern
  detection design.
- [`engine/PR.md`](../engine/PR.md) — current branch summary.
