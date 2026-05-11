//! Fixed-size transposition table used by the alpha-beta search.
//!
//! Positions are indexed by Zobrist hash and stored using simple
//! replacement-by-depth. Each entry stores:
//!
//! - the full position hash
//! - the search depth
//! - the evaluated score
//! - a bound classification (`Exact`, `Lower`, `Upper`)
//! - the best move found from the position
//!
//! The table uses a power-of-two size so indexing can use:
//!
//! ```ignore
//! hash & mask
//! ```
//!
//! instead of modulo division.

use crate::game::Pos;
/// Type of score stored in the transposition table.
///
/// TT scores may represent:
///
/// - [`Bound::Exact`] — exact minimax evaluation
/// - [`Bound::Lower`] — lower bound from a beta cutoff (fail-high)
/// - [`Bound::Upper`] — upper bound from a fail-low search
///
/// Lower/upper bounds are used to tighten alpha-beta windows during
/// future searches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

/// One cached search result stored in the transposition table.
#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub depth: i32,
    pub score: i32,
    pub flag: Bound,
    pub best_move: Option<Pos>,
}

#[allow(dead_code)]
impl TTEntry {
    /// Sentinel empty entry used to initialize the table.
    pub fn empty() -> Self {
        Self {
            key: 0,
            depth: -1,
            score: 0,
            flag: Bound::Exact,
            best_move: None,
        }
    }
}

/// Fixed-size transposition table used to cache previously searched
/// positions during alpha-beta search.
///
/// Positions are indexed by Zobrist hash using:
///
/// ```ignore
/// (hash as usize) & mask
/// ```
///
/// Because the table has a fixed size, multiple positions may map to the
/// same slot (collision). Collisions are resolved using a depth-preferred
/// replacement policy: deeper search results replace shallower ones.
pub struct TranspositionTable {
    /// Table storage.
    entries: Vec<TTEntry>,
    // size - 1, used for fast index computation via:
    // (hash as usize) & mask
    mask: usize,
}

#[allow(dead_code)]
impl TranspositionTable {
    /// Create a transposition table with `2^size_power` entries.
    ///
    /// The table size is rounded to a power of two so indexing can use
    /// fast bit masking instead of modulo division.
    pub fn new(size_power: usize) -> Self {
        let size = 1 << size_power;
        Self {
            entries: vec![TTEntry::empty(); size],
            mask: size - 1,
        }
    }
    /// Look up a position by Zobrist hash.
    ///
    /// Returns `None` if the slot is empty or contains a different hash
    /// due to collision replacement.
    pub fn get(&self, hash: u64) -> Option<&TTEntry> {
        let entry = &self.entries[(hash as usize) & self.mask];
        if entry.key == hash {
            Some(entry)
        } else {
            None
        }
    }
    /// Insert or replace a TT entry.
    ///
    /// Replacement policy: deeper searches replace shallower ones. Empty
    /// slots use `depth = -1` (set by [`TTEntry::empty`]), so any valid
    /// depth `>= 0` will always replace an empty slot under the same rule.
    pub fn insert(&mut self, hash: u64, new_entry: TTEntry) {
        let index = (hash as usize) & self.mask;
        let entry = &mut self.entries[index];

        if new_entry.depth >= entry.depth {
            *entry = new_entry;
        }
    }
}
