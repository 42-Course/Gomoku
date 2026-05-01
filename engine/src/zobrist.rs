//! Zobrist hashing for fast game state identification.
//!
//! The game state is represented as a 64-bit hash composed from independent
//! components. This allows the hash to be updated incrementally during
//! move generation instead of recomputing it from scratch.
//!
//! Two complementary views are used:
//!
//! - **State view**: the hash uniquely represents the current position
//!   (board, captures, side-to-move).
//! - **Incremental view**: each move updates only the affected parts
//!   of the hash in O(1) time.
//!
//! This is essential for efficient search (e.g. transposition tables),
//! where many positions are revisited.use crate::constants::{ CELL_COUNT };

use crate::constants::CELL_COUNT;
use once_cell::sync::Lazy;

const MAX_CAPTURES: usize = 5;
const ZOBRIST_SEED: u64 = 0x9E3779B97F4A7C15;

const MIX_MULT: u64 = 0x2545F4914F6CDD1D;
const SHIFT1: u32 = 12;
const SHIFT2: u32 = 25;
const SHIFT3: u32 = 27;

/// Global Zobrist table, initialized once and shared across the engine.
pub static ZOBRIST: Lazy<Zobrist> = Lazy::new(Zobrist::new);

/// Pseudo-random 64-bit generator (SplitMix-like), used for key generation.
fn rand_u64(seed: &mut u64) -> u64 {
    *seed ^= *seed >> SHIFT1;
    *seed ^= *seed << SHIFT2;
    *seed ^= *seed >> SHIFT3;
    (*seed).wrapping_mul(MIX_MULT)
}

/// Zobrist key table for hashing game state.
pub struct Zobrist {
    /// Keys for each (position, player).
    pub board: [[u64; 2]; CELL_COUNT],

    /// Keys for (player, capture count).
    pub capture: [[u64; MAX_CAPTURES + 1]; 2],

    /// Keys for side-to-move.
    pub side: [u64; 2],
}

impl Zobrist {
    /// Initialize all Zobrist keys using a fixed seed.
    ///
    /// Deterministic initialization ensures reproducible hashes.
    pub fn new() -> Self {
        let mut seed = ZOBRIST_SEED;
        let mut zob = Zobrist {
            board: [[0; 2]; CELL_COUNT],
            capture: [[0; MAX_CAPTURES + 1]; 2],
            side: [0; 2],
        };

        for i in 0..CELL_COUNT {
            for p in 0..2 {
                zob.board[i][p] = rand_u64(&mut seed);
            }
        }

        for p in 0..2 {
            for c in 0..=MAX_CAPTURES {
                zob.capture[p][c] = rand_u64(&mut seed);
            }
        }

        for p in 0..2 {
            zob.side[p] = rand_u64(&mut seed);
        }
        zob
    }
}
