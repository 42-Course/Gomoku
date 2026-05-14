//! AI: search + heuristic evaluation.
//!
//! The two submodules are intentionally small and orthogonal:
//!
//! - [`search`] — negamax with alpha-beta pruning and transposition table.
//! - [`eval`] — heuristic scoring of non-terminal positions, built on top
//!   of the line-pattern primitives in [`crate::patterns`].
//!
//! ## Public entry points
//!
//! - [`best_move`] runs the search and returns the chosen move + score.
//! - [`evaluate`] if you want the leaf score for a position directly.

pub mod eval;
pub mod search;
pub mod move_ordering;

#[allow(unused_imports)]
pub use eval::evaluate;
#[allow(unused_imports)]
pub use search::{best_move, SearchResult};
