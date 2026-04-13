# Gomoku

## Architecture

Frontend → FastAPI → Rust Engine

- FastAPI: thin routing layer (no logic)
- Rust: game state, rules, and AI

## Notes

- All game data lives in Rust (in-memory)
- Python only forwards requests
- Communication is done via JSON (stdin/stdout)

## Goal

Build a fast and correct Gomoku AI using a clean separation between API and engine.