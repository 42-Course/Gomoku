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

## Backend

```
cd backend
python3 -m venv venv
source venv/bin/activate
pip install fastapi uvicorn
uvicorn main:app --reload
```

Open:
http://127.0.0.1:8000/docs

## Rust Engine

### Development
```
cargo run
```

### (Later) optimized build
```
cargo build --release
```