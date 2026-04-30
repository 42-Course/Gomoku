//! AI: search + heuristic evaluation.
//!
//! The two submodules are intentionally small and orthogonal:
//!
//! - [`search`] — negamax with alpha-beta pruning and a generic observer
//!   hook for the visualizer.
//! - [`eval`] — heuristic scoring of non-terminal positions, built on top
//!   of the line-pattern primitives in [`crate::patterns`].
//!
//! ## Public entry points
//!
//! - [`best_move`] for the fast path (no tree recording).
//! - [`best_move_verbose`] for the same search plus a recorded
//!   [`SearchNode`] tree.
//! - [`evaluate`] if you want the leaf score for a position directly.

pub mod eval;
pub mod search;

#[allow(unused_imports)]
pub use eval::evaluate;
#[allow(unused_imports)]
pub use search::{best_move, best_move_verbose, SearchNode, SearchResult};
