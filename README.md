# Gomoku

A high-performance Gomoku AI written in Rust and compiled to WebAssembly.

The project implements the complete 42 Gomoku ruleset, including captures, endgame capture validation, and double-three detection, while focusing on strong search performance, modern web technologies, and a polished user experience.

![Gource video](gource.mp4)

---

## Features

### AI Engine

* Alpha-Beta search with depth 10+ exploration
* Late Move Reductions (LMR)
* Advanced move ordering
* Zobrist hashing
* Transposition tables
* Bitwise pattern detection
* Threat-based position evaluation

### Game Rules

* Capture mechanics
* Double-three detection
* Endgame capture validation
* Five-or-more alignment victory
* Full 42 Gomoku rules compliance

### User Experience

* Human vs AI
* Human vs Human
* Move suggestions
* AI thinking timer
* Position analysis
* Game replay system
* Persistent game history

---

## Architecture

```text id="39p0f0"
┌─────────────────────┐
│     Visualizer      │
│  (Bun + Vite + TS)  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│     WebAssembly     │
│   Rust Bindings     │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│    Gomoku Engine    │
│       Rust          │
└─────────────────────┘
```

All game logic lives inside the Rust engine.

The frontend is responsible only for rendering, user interactions, and visualization.

---

## Technology Stack

### Engine

* Rust
* WebAssembly

### Frontend

* Bun
* Vite
* TypeScript
* Zustand

### Infrastructure

* Web Workers (multithreaded search)
* IndexedDB (local persistence)
* GitHub Actions (CI/CD)
* GitHub Pages (deployment)

---

## Search Architecture

### Search

* Iterative Deepening
* Alpha-Beta Pruning
* Late Move Reductions (LMR)

### Move Ordering

* Principal Variation ordering
* Transposition Table best move ordering
* Heuristic move sorting

### Evaluation

* Pattern-based evaluation
* Threat detection
* Incremental board analysis

### Performance

* Zobrist Hashing
* Transposition Tables
* Bitwise pattern scanning
* Incremental state updates

---

## Project Structure

```text id="b0x6yb"
.
├── engine/          # Core game engine
├── engine-wasm/     # WASM bindings
├── visualizer/      # Frontend application
└── docs/            # Documentation
```

---

## Development

### Run Development Environment

```bash id="92s4bn"
make dev
```

Builds the WASM package and starts the development server.

### Production Build

```bash id="tt2l40"
make build
```

### Run Tests

```bash id="ezlt6l"
make test
```

### Watch Mode

```bash id="hkkr5h"
make watch
```

---

## Maintenance

### Clean Rust Artifacts

```bash id="twe5ot"
make clean
```

### Remove All Generated Files

```bash id="vok8mx"
make fclean
```

### Rebuild Everything

```bash id="n1ejzy"
make re
```

---

## Bonus Features

### Analysis

* Post-game analysis
* Alternative move suggestions
* Position review

### Replay System

* Stored game history
* Move-by-move navigation
* Replay past games

### Multiple Game Modes

* Human vs Human
* Human vs AI

### Documentation & Deployment

* Automated documentation publishing
* GitHub Pages deployment
* GitHub Actions CI/CD pipeline
