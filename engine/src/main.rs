//! # Gomoku engine
//!
//! A Gomoku (Five-in-a-Row, with captures and the no-double-three rule)
//! engine: rules, search, and heuristic evaluation. The binary is currently
//! a stub — every interesting piece is a library module and the public API
//! lives behind [`ai::best_move`] and [`ai::best_move_verbose`].
//!
//! ## Module map
//!
//! | Module           | Responsibility                                           |
//! | ---------------- | -------------------------------------------------------- |
//! | [`constants`]    | Board geometry constants (size as `usize` and `isize`).  |
//! | [`board`]        | Bit-packed storage + line-walking primitives.            |
//! | [`patterns`]     | Pure bit-twiddling pattern detection on packed lines.    |
//! | [`game`]         | Rules: turns, captures, win, draw, double-three.         |
//! | [`ai`]           | Negamax + alpha-beta with a heuristic evaluator.         |
//!
//! ## Bitmap-driven design
//!
//! Stones live in two per-player [`board::BitBoard`]s. To recognize patterns
//! ("five in a row", "open four", "free three", …), the board is sliced into
//! lines (rows, columns, diagonals, anti-diagonals); each line is packed
//! into two `u32` masks (`me`, `opp`); and recognition is plain shift+mask.
//!
//! Off-the-board cells sit at zero in *both* masks, which makes them act as
//! walls — patterns that need an empty endpoint cannot match against an
//! edge, so edge handling falls out for free. See [`patterns`] for the full
//! treatment, and `engine/REFACTOR.md` for the historical context.
//!
//! ## Search
//!
//! [`ai::search`] runs negamax with alpha-beta pruning. Two entry points
//! share one recursive core through a generic `Observer`: the fast path
//! ([`ai::best_move`]) uses a no-op observer that the compiler erases, while
//! the verbose path ([`ai::best_move_verbose`]) builds a full search tree
//! for the visualizer. The evaluator ([`ai::evaluate`]) scores positions
//! from the side-to-move's perspective using the pattern tallies above.

mod ai;
mod zobrist;
mod transpose;
mod board;
mod constants;
mod game;
mod patterns;

fn main() {}
