//! Board geometry constants.
//!
//! Gomoku is played on a 19×19 board. Two flavors of the same value are
//! exposed because the rest of the engine indexes the board in two
//! contexts:
//!
//! - `usize` for array indexing (board cells, bit positions).
//! - `isize` for direction walks where intermediate coordinates can go
//!   negative before they're bounds-checked.

/// Number of cells on each edge of the board, as a `usize`.
///
/// Use this for indexing into per-cell arrays and bitmaps.
pub const BOARD_SIZE: usize = 19;

/// Number of cells on each edge of the board, as an `isize`.
///
/// Use this in line walks where coordinates may transiently be negative
/// (e.g. stepping `(x - 1, y - 1)` along a diagonal).
pub const BOARD_SIZE_I: isize = 19;
