//! Negamax + alpha-beta search with transposition table support.
//!
//! # Conventions
//!
//! - Scores are from the side-to-move's perspective (negamax).
//! - Terminal wins return `-(WIN_SCORE + depth)` so faster mates outrank
//!   slower ones.
//! - The search uses make/unmake on a single [`Game`]; on return the game
//!   is in exactly the state it was on entry.
//!
//! # Examples
//!
//! ```ignore
//! let mut game = Game::new();
//! let mut tt = TranspositionTable::new(20);
//! let result = best_move(&mut game, 4, &mut tt);
//! if let Some((x, y)) = result.best_move {
//!     game.play_move(x, y).unwrap();
//! }
//! ```

#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};

use crate::ai::SearchConfig;
use crate::ai::eval::evaluate;
use crate::ai::iterative_deepening::iterative_deepening;
use crate::ai::move_ordering::order_moves;
use crate::constants::BOARD_SIZE;
use crate::game::{Game, GameStatus, Pos};
use crate::transpose::{Bound, TTEntry, TranspositionTable};

/// Score returned for a decisive terminal position. Large enough to dominate
/// any heuristic evaluation, small enough that `score + depth` can't overflow.
pub const WIN_SCORE: i32 = 1_000_000;

/// What the search returns to the caller.
pub struct SearchResult {
    /// Final chosen move.
    pub best_move: Option<Pos>,

    /// Final evaluation score.
    pub score: i32,

    /// Deepest fully completed iterative depth.
    pub depth_reached: u32,

    /// Total nodes searched across all iterations.
    pub total_nodes: u64,

    /// Deepest ply explored overall.
    pub max_ply: u32,
}

/// If the position is terminal, return its score from the side-to-move's
/// perspective. A player can never be on-move in a position they already won,
/// so any `Win(_)` encountered here is a loss for the side to move.
///
/// The `+ depth as i32` bonus makes the search prefer *faster* wins and
/// *slower* losses: a mate-in-1 scores higher than a mate-in-3.
fn terminal_score(game: &Game, depth: u32) -> Option<i32> {
    match game.status {
        GameStatus::Win(_) => Some(-(WIN_SCORE + depth as i32)),
        GameStatus::Draw => Some(0),
        GameStatus::Ongoing => None,
    }
}

/// Negamax with alpha-beta pruning and transposition table support.
///
/// The transposition table stores previously searched positions keyed by
/// Zobrist hash. Entries may contain:
///
/// - [`Bound::Exact`] — exact minimax score for this position
/// - [`Bound::Lower`] — lower bound (fail-high / beta cutoff)
/// - [`Bound::Upper`] — upper bound (fail-low)
///
/// TT entries are reused to:
///
/// - return exact evaluations immediately
/// - tighten the alpha-beta search window
/// - improve move ordering by searching the previous best move first
///
/// Only entries searched to at least the current depth are trusted for
/// pruning and ordering.
///
/// Returns `(score, best_move_at_this_node)`.
#[allow(clippy::too_many_arguments)]
pub fn negamax(
    game: &mut Game,
    depth: u32,
    mut alpha: i32,
    mut beta: i32,
    tt: &mut TranspositionTable,
    nodes: &mut u64,
    max_ply: &mut u32,
    ply: u32,
    config: &SearchConfig
) -> (i32, Option<Pos>) {
    *nodes += 1;
    *max_ply = (*max_ply).max(ply);

    // Look in the transposition table, if the board has been generated
    let original_alpha = alpha;
    let original_beta = beta;
    // Probe TT before searching children. Exact scores may terminate the
    // search immediately; lower/upper bounds may tighten the search window.
    let tt_entry = tt.get(game.hash());
    if let Some(entry) = tt_entry {
        if entry.depth >= depth as i32 {
            match entry.flag {
                Bound::Exact => {
                    return (entry.score, entry.best_move);
                }
                Bound::Lower => {
                    alpha = alpha.max(entry.score);
                }
                Bound::Upper => {
                    beta = beta.min(entry.score);
                }
            }
            if alpha >= beta {
                return (entry.score, entry.best_move);
            }
        }
    }

    if let Some(score) = terminal_score(game, depth) {
        return (score, None);
    }

    if depth == 0 {
        return (evaluate(game), None);
    }

    let tt_move = tt_entry.and_then(|entry| entry.best_move);
    let moves = game.generate_moves();

    //sort the moves
    let ordered_moves = order_moves(game, moves, tt_move);

    if ordered_moves.is_empty() {
        // No legal continuations but game isn't flagged terminal — treat as
        // a quiet position and hand off to the evaluator.
        return (evaluate(game), None);
    }

    let mut best_score = i32::MIN + 1;
    let mut best_mv: Option<Pos> = None;

    for (i, mv) in ordered_moves.into_iter().enumerate() {
        // generate_moves already filters illegal placements, but play_move
        // is still the source of truth — skip defensively if it rejects.
        if game.play_pos(mv).is_err() {
            continue;
        }

        let reduction =
            if !config.enable_lmr {
                0
            } else if depth >= 6 && i >= 3 {
                2
            } else if depth >= 3 && i >= 3 {
                1
            } else {
                0
            };

        let search_depth = depth - 1 - reduction;

        let (child_score, _) = negamax(
            game,
            search_depth,
            -beta,
            -alpha,
            tt,
            nodes,
            max_ply,
            ply + 1,
            config
        );

        let mut score = -child_score;

        // Re-search surprising reduced moves.
        if reduction > 0 && score > alpha {
            let (full_score, _) =
                negamax(
                    game,
                    depth - 1,
                    -beta,
                    -alpha,
                    tt,
                    nodes,
                    max_ply,
                    ply + 1,
                    config
                );

            score = -full_score;
        }

        game.undo_move().expect("undo_move must succeed after a successful play_move");

        if score > best_score {
            best_score = score;
            best_mv = Some(mv);
        }
        if best_score > alpha {
            alpha = best_score;
        }
        if alpha >= beta {
            break;
        }
    }

    // Classify the stored result relative to the original search window.
    let flag = if best_score <= original_alpha {
        Bound::Upper
    } else if best_score >= original_beta {
        Bound::Lower
    } else {
        Bound::Exact
    };

    tt.insert(game.hash(), TTEntry {
        key: game.hash(),
        depth: depth as i32,
        score: best_score,
        flag,
        best_move: best_mv,
    });

    (best_score, best_mv)
}

/// Run alpha-beta and return the best move + score.
///
/// The game is left untouched (every played move is undone).
///
/// # Examples
///
/// ```ignore
/// let mut game = Game::new();
/// let result = best_move(&mut game, 4, 20);
/// assert_eq!(result.best_move, Some((9, 9))); // center on empty board
/// ```
pub fn best_move(game: &mut Game, depth: u32, tt_size: usize) -> SearchResult {
    // On the empty board every cell evaluates identically, so a normal
    // search just picks the lowest-indexed candidate. Open from a random
    // cell inside the central 3×3 instead — varied enough to make
    // human-vs-AI games feel different without straying off the star points.
    if game.history.is_empty() {
        return SearchResult {
            best_move: Some(random_central_opening()),
            score: 0,
            depth_reached: 0,
            total_nodes: 0,
            max_ply: 0
        };
    }

    if depth == 0 {
        return SearchResult {
            best_move: None,
            score: evaluate(game),
            depth_reached: 0,
            total_nodes: 1,
            max_ply: 0,
        };
    }

    let config = SearchConfig::default();

    let res = iterative_deepening(game, depth, tt_size, &config);
    SearchResult {
        best_move: res.result.best_move,
        score: res.result.score,
        depth_reached: res.depth_reached,
        total_nodes: res.total_nodes,
        max_ply: res.result.max_ply
    }
}

/// Pick a Pos uniformly from the 3×3 block centred on the board centre.
///
/// Uses a static LCG seeded once at startup: deterministic per process but
/// stepped on every call, so consecutive openings differ. The engine has
/// no other source of entropy (the `rand` crate isn't a dep and the wasm
/// target makes pulling one in nontrivial), which is fine here — we only
/// need varied output, not cryptographic randomness.
fn random_central_opening() -> Pos {
    static STATE: AtomicU64 = AtomicU64::new(0xCAFE_F00D_DEAD_BEEF);

    // Numerical Recipes LCG constants — full 64-bit period.
    let prev = STATE.load(Ordering::Relaxed);
    let next = prev
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    STATE.store(next, Ordering::Relaxed);

    let pick = (next >> 32) % 9;
    let dx = (pick % 3) as isize - 1;
    let dy = (pick / 3) as isize - 1;
    let centre = (BOARD_SIZE / 2) as isize;
    Pos::from_xy((centre + dx) as usize, (centre + dy) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, Player};

    #[test]
    fn depth_zero_returns_eval_and_no_move() {
        // Empty-board calls now short-circuit; play one move so we hit the
        // depth-0 leaf branch in negamax.
        let mut game = Game::new();
        game.play_move(9, 9).unwrap();

        let result = best_move(&mut game, 0, 0);
        assert_eq!(result.best_move, None);
        assert_eq!(result.total_nodes, 1);
    }

    #[test]
    fn returns_a_legal_move_on_empty_board() {
        let mut game = Game::new();
        let result = best_move(&mut game, 1, 0);
        // The empty-board opening is randomised within the central 3×3 of
        // the board, so the specific cell varies between runs — but it must
        // be inside that block and the search reports zero nodes (short-cut).
        let centre = (BOARD_SIZE / 2) as isize;
        let mv = result.best_move.expect("opening must return a move");
        let (x, y) = mv.to_xy();
        let dx = (x as isize - centre).abs();
        let dy = (y as isize - centre).abs();
        assert!(dx <= 1 && dy <= 1, "opening {:?} must be within the central 3x3", (x, y));
        assert_eq!(result.total_nodes, 0, "empty-board opening should bypass search");
    }

    #[test]
    fn search_leaves_game_unchanged() {
        let mut game = Game::new();
        game.play_move(9, 9).unwrap();
        game.play_move(10, 9).unwrap();
        let history_before = game.history.len();

        let _ = best_move(&mut game, 2, 0);

        assert_eq!(game.history.len(), history_before);
        assert_eq!(game.board.cell_at_xy(9, 9), Some(Player::Black));
        assert_eq!(game.board.cell_at_xy(10, 9), Some(Player::White));
    }

    #[test]
    fn blocks_an_immediate_win() {
        // Black has four stones in a row against the left edge: cols 0..=3 on
        // row 9. The only way Black can extend to five is at (5, 9). If White
        // doesn't take it, Black wins on the next ply. White is on move.
        let mut game = Game::new();
        game.play_move(0, 9).unwrap();    // B
        game.play_move(0, 0).unwrap();    // W (parking)
        game.play_move(1, 9).unwrap();    // B
        game.play_move(0, 1).unwrap();    // W
        game.play_move(2, 9).unwrap();    // B
        game.play_move(0, 2).unwrap();    // W
        game.play_move(3, 9).unwrap();    // B
        // White to move.

        let result = best_move(&mut game, 2, 0);
        assert_eq!(
            result.best_move,
            Some(Pos::from_xy(4, 9)),
            "White must block the only winning square",
        );
    }

    fn assert_deterministic(game: &Game, depth: u32) {
        let mut g1 = game.clone();
        let mut g2 = game.clone();

        let r1 = best_move(&mut g1, depth, 20);
        let r2 = best_move(&mut g2, depth, 20);

        assert_eq!(r1.best_move, r2.best_move, "best move differs");
        assert_eq!(r1.score, r2.score, "score differs");
        assert_eq!(
            r1.total_nodes,
            r2.total_nodes,
            "node count differs"
        );
    }

    fn midgame_position() -> Game {
        let mut g = Game::new();

        let moves = [
            (9, 9), (10, 9),
            (9, 10), (10, 10),
            (8, 9), (11, 9),
            (8, 10), (11, 10),
            (9, 8), (10, 8),
        ];

        for (x, y) in moves {
            g.play_move(x, y).unwrap();
        }

        g
    }

    #[test]
    fn deterministic_midgame() {
        let game = midgame_position();

        assert_deterministic(&game, 4);
    }

    #[test]
    fn deterministic_two_moves() {
        let mut game = Game::new();

        game.play_move(9, 9).unwrap();
        game.play_move(10, 9).unwrap();

        assert_deterministic(&game, 3);
    }

    #[test]
    fn deterministic_block_position() {
        let mut game = Game::new();

        game.play_move(0, 9).unwrap();
        game.play_move(0, 0).unwrap();
        game.play_move(1, 9).unwrap();
        game.play_move(0, 1).unwrap();
        game.play_move(2, 9).unwrap();
        game.play_move(0, 2).unwrap();
        game.play_move(3, 9).unwrap();

        assert_deterministic(&game, 3);
    }

    #[test]
    fn tt_does_not_change_best_move() {
        // Empty boards now short-circuit search, so the TT branch is
        // never exercised from there. Seed a midgame position instead.
        let mut g1 = midgame_position();
        let mut g2 = g1.clone();

        let r1 = best_move(&mut g1, 4, 0);
        let r2 = best_move(&mut g2, 4, 20);

        assert_eq!(r1.best_move, r2.best_move);
        assert_eq!(r1.score, r2.score);
    }

    #[test]
    fn tt_reduces_node_count() {
        let mut g1 = midgame_position();
        let mut g2 = g1.clone();

        let r1 = best_move(&mut g1, 6, 0);
        let r2 = best_move(&mut g2, 6, 20);

        println!("no TT nodes: {}", r1.total_nodes);
        println!("TT nodes: {}", r2.total_nodes);

        assert!(r2.total_nodes < r1.total_nodes);
    }

    #[test]
    fn tt_reduces_node_count_midgame() {
        let mut g1 = midgame_position();
        let mut g2 = g1.clone();

        let r1 = best_move(&mut g1, 4, 0);
        let r2 = best_move(&mut g2, 4, 20);

        println!("no TT best move: {}", r1.best_move.unwrap());
        println!("TT best move: {}", r2.best_move.unwrap());

        assert_eq!(r1.best_move, r2.best_move, "best move differs with TT");
        assert_eq!(r1.score, r2.score, "score differs with TT");

        println!("no TT nodes: {}", r1.total_nodes);
        println!("TT nodes: {}", r2.total_nodes);

        assert!(r2.total_nodes < r1.total_nodes);
    }

    #[test]
    fn depth_10_benchmark() {
        use std::time::Instant;

        let mut game = midgame_position();

        let start = Instant::now();

        let result = best_move(
            &mut game,
            10,
            20,
        );

        let elapsed = start.elapsed();

        println!();
        println!("=== Depth 10 Benchmark ===");
        println!("best move: {:?}", result.best_move);
        println!("depth reached: {}", result.depth_reached);
        println!("max_ply: {}", result.max_ply);
        println!("score: {}", result.score);
        println!("total_nodes: {}", result.total_nodes);
        println!("time: {:?}", elapsed);

        let ebf = (result.total_nodes as f64)
            .powf(1.0 / result.depth_reached as f64);

        println!("effective branching factor: {:.2}", ebf);
    }

    #[test]
    fn depth_10_benchmark() {
        use std::time::Instant;

        let mut game = midgame_position();

        let start = Instant::now();

        let result = best_move(
            &mut game,
            10,
            20,
        );

        let elapsed = start.elapsed();

        println!();
        println!("=== Depth 10 Benchmark ===");
        println!("best move: {:?}", result.best_move);
        println!("score: {}", result.score);
        println!("nodes: {}", result.nodes_visited);
        println!("time: {:?}", elapsed);
    }
}
