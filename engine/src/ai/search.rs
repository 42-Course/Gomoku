//! Negamax + alpha-beta search.
//!
//! Two entry points share one recursive core through a generic [`Observer`]:
//!
//! - [`best_move`]         — fast path, uses [`NoopObserver`] (inlined away).
//! - [`best_move_verbose`] — records the full search tree for the visualizer.
//!
//! Keeping the observer generic means the hot path pays nothing for the
//! visualizer plumbing; the verbose path builds a [`SearchNode`] tree.
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
//! let result = best_move(&mut game, 4);
//! if let Some((x, y)) = result.best_move {
//!     game.play_move(x, y).unwrap();
//! }
//! ```

#![allow(dead_code)]

use crate::ai::eval::evaluate;
use crate::game::{Game, GameStatus, Player};
use crate::transpose::{Bound, TTEntry, TranspositionTable};

/// Score returned for a decisive terminal position. Large enough to dominate
/// any heuristic evaluation, small enough that `score + depth` can't overflow.
pub const WIN_SCORE: i32 = 1_000_000;

/// What the search returns to the caller.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The chosen move, or `None` at depth 0 / when no legal moves exist.
    pub best_move: Option<(usize, usize)>,
    /// Score from the root side-to-move's perspective.
    pub score: i32,
    /// Total nodes (including leaves) visited during the search.
    pub nodes_visited: u64,
}

/// One node in the recorded search tree (verbose mode only).
///
/// `score` is from the side-to-move's perspective at this node (negamax
/// convention). `pruned` is true when an alpha cutoff stopped exploration
/// before all children were visited.
#[derive(Debug, Clone)]
pub struct SearchNode {
    /// The move that led to this node. `None` at the root.
    pub mv: Option<(usize, usize)>,
    /// Whose turn it is at this node.
    pub player_to_move: Player,
    /// Plies still to expand below this node.
    pub depth_remaining: u32,
    /// Alpha bound on entry.
    pub alpha_in: i32,
    /// Beta bound on entry.
    pub beta_in: i32,
    /// Final score returned by this node, side-to-move's perspective.
    pub score: i32,
    /// `true` if an alpha-beta cutoff stopped expansion early.
    pub pruned: bool,
    /// Recorded children, in the order they were visited.
    pub children: Vec<SearchNode>,
}

/// Hook points the search calls on entering/leaving every node.
///
/// Every recursive call into [`negamax`] fires `enter` once on the way
/// down and `leave` once on the way up. Implementations must be cheap —
/// they're on the hot path.
pub trait Observer {
    /// Called once before exploring a node.
    fn enter(
        &mut self,
        mv: Option<(usize, usize)>,
        player: Player,
        depth: u32,
        alpha: i32,
        beta: i32,
    );

    /// Called once after a node returns. `pruned` is `true` if exploration
    /// stopped on an alpha-beta cutoff.
    fn leave(&mut self, score: i32, pruned: bool);
}

/// Zero-overhead observer. Monomorphization + `#[inline]` lets the compiler
/// erase all calls in release builds.
pub struct NoopObserver;

impl Observer for NoopObserver {
    #[inline(always)]
    fn enter(&mut self, _: Option<(usize, usize)>, _: Player, _: u32, _: i32, _: i32) {}
    #[inline(always)]
    fn leave(&mut self, _: i32, _: bool) {}
}

/// Builds a tree of every visited node for the visualizer.
///
/// The observer maintains a stack mirroring the recursion: `enter` pushes
/// a fresh node, `leave` pops the current node and attaches it to its
/// parent (or stores it as the root when the stack empties).
pub struct TreeObserver {
    stack: Vec<SearchNode>,
    root: Option<SearchNode>,
}

impl TreeObserver {
    /// Create an empty observer ready to record a search.
    pub fn new() -> Self {
        Self { stack: Vec::new(), root: None }
    }

    /// Consume the observer and return the recorded root (if any).
    pub fn into_tree(self) -> Option<SearchNode> {
        self.root
    }
}

impl Default for TreeObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl Observer for TreeObserver {
    fn enter(
        &mut self,
        mv: Option<(usize, usize)>,
        player: Player,
        depth: u32,
        alpha: i32,
        beta: i32,
    ) {
        self.stack.push(SearchNode {
            mv,
            player_to_move: player,
            depth_remaining: depth,
            alpha_in: alpha,
            beta_in: beta,
            score: 0,
            pruned: false,
            children: Vec::new(),
        });
    }

    fn leave(&mut self, score: i32, pruned: bool) {
        let mut node = self
            .stack
            .pop()
            .expect("Observer::leave without matching enter");
        node.score = score;
        node.pruned = pruned;
        match self.stack.last_mut() {
            Some(parent) => parent.children.push(node),
            None => self.root = Some(node),
        }
    }
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
fn negamax<O: Observer>(
    game: &mut Game,
    depth: u32,
    mut alpha: i32,
    mut beta: i32,
    tt: &mut TranspositionTable,
    observer: &mut O,
    incoming_mv: Option<(usize, usize)>,
    nodes: &mut u64,
) -> (i32, Option<(usize, usize)>) {
    *nodes += 1;
    let player = game.current_player();
    observer.enter(incoming_mv, player, depth, alpha, beta);

    // Look in the transposition table, if the board has been generated
    let original_alpha = alpha;
    let original_beta = beta;
    // Probe TT before searching children. Exact scores may terminate the
    // search immediately; lower/upper bounds may tighten the search window.
    let tt_entry = tt.get(game.hash);
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
                    if alpha >= beta {
                        observer.leave(entry.score, true);
                        return (entry.score, entry.best_move);
                    }
                }
            }
            if alpha >= beta {
                observer.leave(entry.score, true);
                return (entry.score, entry.best_move);
            }
        }
    }

    if let Some(score) = terminal_score(game, depth) {
        observer.leave(score, false);
        return (score, None);
    }

    if depth == 0 {
        let score = evaluate(game);
        observer.leave(score, false);
        return (score, None);
    }

    let mut moves = game.generate_moves();

    if moves.is_empty() {
        // No legal continuations but game isn't flagged terminal — treat as
        // a quiet position and hand off to the evaluator.
        let score = evaluate(game);
        observer.leave(score, false);
        return (score, None);
    }

    if let Some(entry) = tt_entry {
        if entry.depth >= depth as i32 {
            if let Some(tt_mv) = entry.best_move {
                if let Some(pos) = moves.iter().position(|&m| m == tt_mv) {
                    // Search the previous best move first to maximize alpha-beta cutoffs.
                    moves.swap(0, pos);
                }
            }
        }
    }

    let mut best_score = i32::MIN + 1;
    let mut best_mv: Option<(usize, usize)> = None;
    let mut pruned = false;

    for (x, y) in moves {
        // generate_moves already filters illegal placements, but play_move
        // is still the source of truth — skip defensively if it rejects.
        if game.play_move(x, y).is_err() {
            continue;
        }

        let (child_score, _) =
            negamax(game, depth - 1, -beta, -alpha, tt, observer, Some((x, y)), nodes);
        let score = -child_score;

        game.undo_move().expect("undo_move must succeed after a successful play_move");

        if score > best_score {
            best_score = score;
            best_mv = Some((x, y));
        }
        if best_score > alpha {
            alpha = best_score;
        }
        if alpha >= beta {
            pruned = true;
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

    tt.insert(game.hash, TTEntry {
        key: game.hash,
        depth: depth as i32,
        score: best_score,
        flag,
        best_move: best_mv,
    });

    observer.leave(best_score, pruned);
    (best_score, best_mv)
}

/// Fast path: run alpha-beta and return the best move + score.
///
/// The game is left untouched (every played move is undone).
///
/// # Examples
///
/// ```ignore
/// let mut game = Game::new();
/// let result = best_move(&mut game, 4);
/// assert_eq!(result.best_move, Some((9, 9))); // center on empty board
/// ```
pub fn best_move(game: &mut Game, depth: u32, tt: &mut TranspositionTable) -> SearchResult {
    let mut obs = NoopObserver;
    let mut nodes = 0u64;
    let (score, mv) = negamax(
        game,
        depth,
        i32::MIN + 1,
        i32::MAX - 1,
        tt,
        &mut obs,
        None,
        &mut nodes,
    );
    SearchResult { best_move: mv, score, nodes_visited: nodes }
}

/// Verbose path: same search, plus a recorded tree for the visualizer.
///
/// Returns the same [`SearchResult`] as [`best_move`] alongside the root
/// [`SearchNode`] of the explored tree (if any nodes were visited).
pub fn best_move_verbose(
    game: &mut Game,
    depth: u32,
    tt: &mut TranspositionTable
) -> (SearchResult, Option<SearchNode>) {
    let mut obs = TreeObserver::new();
    let mut nodes = 0u64;
    let (score, mv) = negamax(
        game,
        depth,
        i32::MIN + 1,
        i32::MAX - 1,
        tt,
        &mut obs,
        None,
        &mut nodes,
    );
    let result = SearchResult { best_move: mv, score, nodes_visited: nodes };
    (result, obs.into_tree())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    #[test]
    fn depth_zero_returns_eval_and_no_move() {
        let mut game = Game::new();
        let mut tt = TranspositionTable::new(0);
        let result = best_move(&mut game, 0, &mut tt);
        assert_eq!(result.best_move, None);
        assert_eq!(result.nodes_visited, 1);
    }

    #[test]
    fn returns_a_legal_move_on_empty_board() {
        let mut game = Game::new();
        let mut tt = TranspositionTable::new(0);
        let result = best_move(&mut game, 1, &mut tt);
        // On an empty board generate_moves yields exactly (9, 9).
        assert_eq!(result.best_move, Some((9, 9)));
    }

    #[test]
    fn search_leaves_game_unchanged() {
        let mut game = Game::new();
        game.play_move(9, 9).unwrap();
        game.play_move(10, 9).unwrap();
        let history_before = game.history.len();

        let mut tt = TranspositionTable::new(0);
        let _ = best_move(&mut game, 2, &mut tt);

        assert_eq!(game.history.len(), history_before);
        assert_eq!(game.board.cell_at(9, 9), Some(Player::Black));
        assert_eq!(game.board.cell_at(10, 9), Some(Player::White));
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

        let mut tt = TranspositionTable::new(0);
        let result = best_move(&mut game, 2, &mut tt);
        assert_eq!(
            result.best_move,
            Some((4, 9)),
            "White must block the only winning square",
        );
    }

    #[test]
    fn verbose_records_a_tree() {
        let mut game = Game::new();
        game.play_move(9, 9).unwrap();

        let mut tt = TranspositionTable::new(0);
        let (result, tree) = best_move_verbose(&mut game, 2, &mut tt);
        let root = tree.expect("verbose mode should produce a root node");

        assert_eq!(root.depth_remaining, 2);
        assert_eq!(root.mv, None);
        assert_eq!(root.score, result.score);
        assert!(!root.children.is_empty());
    }

    fn assert_deterministic(game: &Game, depth: u32) {
        let mut g1 = game.clone();
        let mut g2 = game.clone();
        let mut tt1 = TranspositionTable::new(20);
        let mut tt2 = TranspositionTable::new(20);

        let r1 = best_move(&mut g1, depth, &mut tt1);
        let r2 = best_move(&mut g2, depth, &mut tt2);

        assert_eq!(r1.best_move, r2.best_move, "best move differs");
        assert_eq!(r1.score, r2.score, "score differs");
        assert_eq!(
            r1.nodes_visited,
            r2.nodes_visited,
            "node count differs"
        );
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
        let mut g1 = Game::new();
        let mut g2 = g1.clone();

        let mut tt_empty = TranspositionTable::new(0);
        let mut tt = TranspositionTable::new(20);

        let r1 = best_move(&mut g1, 4, &mut tt_empty); // no TT version
        let r2 = best_move(&mut g2, 4, &mut tt);

        assert_eq!(r1.best_move, r2.best_move);
        assert_eq!(r1.score, r2.score);
    }

    #[test]
    fn tt_reduces_node_count() {
        let mut g1 = Game::new();
        let mut g2 = g1.clone();

        let mut tt_empty = TranspositionTable::new(0);
        let mut tt = TranspositionTable::new(20);

        let r1 = best_move(&mut g1, 6, &mut tt_empty);
        let r2 = best_move(&mut g2, 6, &mut tt);

        println!("no TT nodes: {}", r1.nodes_visited);
        println!("TT nodes: {}", r2.nodes_visited);

        assert!(r2.nodes_visited < r1.nodes_visited);
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
    fn tt_reduces_node_count_midgame() {
        let mut g1 = midgame_position();
        let mut g2 = g1.clone();

        let mut tt_empty = TranspositionTable::new(0);
        let mut tt = TranspositionTable::new(20);

        let r1 = best_move(&mut g1, 4, &mut tt_empty);
        let r2 = best_move(&mut g2, 4, &mut tt);

        println!("no TT best move: {}, {}", r1.best_move.unwrap().0, r1.best_move.unwrap().1);
        println!("TT best move: {}, {}", r2.best_move.unwrap().0, r2.best_move.unwrap().1);

        assert_eq!(r1.best_move, r2.best_move, "best move differs with TT");
        assert_eq!(r1.score, r2.score, "score differs with TT");

        println!("no TT nodes: {}", r1.nodes_visited);
        println!("TT nodes: {}", r2.nodes_visited);

        assert!(r2.nodes_visited < r1.nodes_visited);
    }
}
