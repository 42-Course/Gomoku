//! Iterative deepening wrapper for negamax alpha-beta search.
//!
//! The search progressively increases depth from `1..=max_depth` while
//! reusing the same transposition table across iterations.
//!
//! Earlier iterations improve move ordering for deeper searches through
//! TT reuse, reducing the explored search tree and improving pruning
//! efficiency at larger depths.
use std::time::{Duration, Instant};
use crate::game::{ Game, Pos };
use crate::transpose::{ TranspositionTable };
use crate::ai::negamax;

// Set max to 1 / 5 of the actual time limit
const TIMEOUT_MS : u64 = 100;

/// What the search returns to the caller.
#[derive(Debug, Clone)]
pub struct SearchIterationResult {
    /// The chosen move, or `None` at depth 0 / when no legal moves exist.
    pub best_move: Option<Pos>,
    /// Score from the root side-to-move's perspective.
    pub score: i32,
    /// Total nodes (including leaves) visited during the search.
    pub nodes_visited: u64,
    /// Deepest ply explored during the search.
    pub max_ply: u32,
}

/// Run a single fixed-depth negamax search iteration using
/// alpha-beta pruning and the shared transposition table.
pub fn search_iteration(
    game: &mut Game,
    depth: u32,
    tt: &mut TranspositionTable,
) -> SearchIterationResult {
    let mut nodes = 0u64;
    let mut max_ply = 0;
    let (score, mv) = negamax(
        game,
        depth,
        i32::MIN + 1,
        i32::MAX - 1,
        tt,
        &mut nodes,
        &mut max_ply,
        0
    );

    SearchIterationResult {
        best_move: mv,
        score,
        nodes_visited: nodes,
        max_ply
    }
}

/// Result returned by iterative deepening search.
///
/// Contains the best result from the deepest completed iteration.
#[allow(dead_code)]
pub struct IterativeResult {
    /// Best move search result from the final iteration.
    pub result: SearchIterationResult,
    /// Deepest fully completed search depth.
    pub depth_reached: u32,
    /// Total nodes seached.
    pub total_nodes: u64,
}

/// Run iterative deepening negamax search up to `max_depth`.
///
/// The search progressively explores increasing depths while reusing
/// the same transposition table across iterations to improve move
/// ordering and alpha-beta pruning efficiency.
///
/// Iterations stop early once the cumulative search time exceeds
/// `TIMEOUT_MS`, preventing the next depth from exploding
/// exponentially in cost.
///
/// Returns the best result from the deepest completed iteration.
#[allow(dead_code)]
pub fn iterative_deepening(
    game: &mut Game,
    max_depth: u32,
    tt_size: usize,
) -> IterativeResult {
    let mut best = None;
    let mut tt = TranspositionTable::new(tt_size);
    let mut total_nodes = 0;
    let start = Instant::now();

    for depth in 1..=max_depth {
        // Avoid starting an iteration that is
        // likely to explode exponentially.
        let elapsed = start.elapsed();

        if elapsed > Duration::from_millis(TIMEOUT_MS) {
            break;
        }

        let result = search_iteration(game, depth, &mut tt);
        println!("i: {}, nodes: {}", depth, result.nodes_visited);
        total_nodes += result.nodes_visited;
        best = Some((depth, result));
    }

    let (depth_reached, result) =
        best.expect("max_depth >= 1");

    IterativeResult {
        result,
        depth_reached,
        total_nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Pos;
    use crate::ai::search::best_move;

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
    fn iterative_does_not_modify_game_state() {
        use crate::constants::BOARD_SIZE;

        let mut game = midgame_position();

        let before = game.clone();

        iterative_deepening(
            &mut game,
            5,
            20,
        );

        assert_eq!(game.hash(), before.hash());

        assert_eq!(
            game.history.len(),
            before.history.len(),
        );

        assert_eq!(
            game.captures,
            before.captures,
        );

        assert_eq!(
            game.status,
            before.status,
        );

        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                assert_eq!(
                    game.board.cell_at_xy(x, y),
                    before.board.cell_at_xy(x, y),
                );
            }
        }
    }

    #[test]
    fn iterative_finds_forced_block() {
        let mut game = Game::new();

        game.play_move(0, 9).unwrap();
        game.play_move(0, 0).unwrap();

        game.play_move(1, 9).unwrap();
        game.play_move(0, 1).unwrap();

        game.play_move(2, 9).unwrap();
        game.play_move(0, 2).unwrap();

        game.play_move(3, 9).unwrap();

        let result = iterative_deepening(
            &mut game,
            4,
            20,
        );

        assert_eq!(
            result.result.best_move,
            Some(Pos::from_xy(4, 9)),
        );
    }

    #[test]
    fn iterative_matches_direct_search() {
        use std::time::Instant;
        let mut g1 = midgame_position();
        let mut g2 = g1.clone();

        let depth = 6;

        let start = Instant::now();

        let direct = best_move(&mut g1, depth, 20);
        let direct_time = start.elapsed();

        let start = Instant::now();
        let iterative = iterative_deepening(&mut g2, depth, 20);
        let iterative_time = start.elapsed();

        println!();
        println!("=== Direct Search ===");
        println!("best move: {:?}", direct.best_move);
        println!("score: {}", direct.score);
        println!("nodes: {}", direct.total_nodes);
        println!("time: {:?}", direct_time);

        println!();
        println!("=== Iterative Deepening ===");
        println!("best move: {:?}", iterative.result.best_move);
        println!("score: {}", iterative.result.score);
        println!("nodes: {}", iterative.result.nodes_visited);
        println!("time: {:?}", iterative_time);
        assert_eq!(
            direct.best_move,
            iterative.result.best_move,
        );

        assert_eq!(
            direct.score,
            iterative.result.score,
        );
    }

    #[test]
    fn iterative_stops_after_slow_iteration() {
        use std::time::{Duration, Instant};

        let mut game = midgame_position();
        let start = Instant::now();

        let result = iterative_deepening(
            &mut game,
            20,
            20,
        );

        let elapsed = start.elapsed();

        println!();
        println!("depth reached: {}", result.depth_reached);
        println!("elapsed: {:?}", elapsed);

        // We should stop before reaching max depth.
        assert!(result.depth_reached < 20);

        // Sanity check that the search remains bounded.
        assert!(elapsed < Duration::from_secs(2));
    }
}
