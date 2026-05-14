//! Move ordering heuristics for negamax alpha-beta search.
//!
//! Candidate moves are scored before expansion to improve alpha-beta
//! pruning efficiency and reduce the explored search tree.
//!
//! Current ordering heuristics include:
//!
//! - transposition-table best move
//! - immediate winning moves
//! - captures
//! - local tactical threats
//! - center proximity
//!
//! # Examples
//!
//! ```ignore
//! let moves = game.generate_moves();
//!
//! let tt_move = tt
//!     .get(game.hash())
//!     .and_then(|entry| entry.best_move);
//!
//! let ordered = order_moves(
//!     &mut game,
//!     moves,
//!     tt_move,
//! );
//! ```
use crate::game::{ Direction, Player, Game, GameStatus, Pos };
use crate::board::Board;
use crate::patterns::count_patterns;

/// A move paired with its ordering heuristic score.
#[derive(Debug, Clone, Copy)]
pub struct ScoredMove {
    /// Candidate move position.
    pub mv: Pos,
    /// Ordering score used for move sorting.
    pub score: i32,
}

/// Large bonus forcing the TT best move to be searched first.
const TT_MOVE_BONUS: i32 = 1_000_000;
/// Bonus for moves that immediately win the game.
const WINNING_MOVE_BONUS: i32 = 500_000;
/// Bonus applied per captured pair.
const CAPTURE_BONUS: i32 = 100_000;

/// Pack a small local line around `pos` into bit patterns.
///
/// Returns `(me, opp, len)` for pattern evaluation.
fn pack_local_line(
    board: &Board,
    pos: Pos,
    dir: Direction,
    radius: isize,
    player: Player,
) -> (u16, u16, u32) {
    let mut me = 0u16;
    let mut opp = 0u16;
    let mut len = 0;

    for step in -radius..=radius {
        let bit = 1 << len;

        if let Some(p) = pos.offset(dir, step) {
            match board.cell_at(p) {
                Some(stone) if stone == player => me |= bit,
                Some(_) => opp |= bit,
                None => {}
            }

            len += 1;
        }
    }

    (me, opp, len)
}

/// Estimate the tactical strength of playing at `pos`.
///
/// Used only for move ordering heuristics.
fn evaluate_threat(game: &Game, pos: Pos) -> i32 {
    let player = game.current_player();

    let mut score = 0;

    for dir in Direction::all() {
        let (me, opp, len) =
            pack_local_line(&game.board, pos, dir, 4, player);//I'm not sure if the radius should be 4 or 5

        let patterns = count_patterns(me as u32, opp as u32, len);
        score += patterns.fives as i32 * 100_000;
        score += patterns.open_four as i32 * 10_000;
        score += patterns.closed_four as i32 * 2_000;
        score += patterns.open_three as i32 * 500;
    }

    score
}

/// Small positional bonus favoring central moves.
fn center_score(pos: Pos) -> i32 {
    let (x, y) = pos.to_xy();

    let dx = (x as i32 - 9).abs();
    let dy = (y as i32 - 9).abs();

    -(dx + dy)
}

/// Score and sort candidate moves for alpha-beta search.
///
/// Moves are ordered using lightweight tactical heuristics:
/// - transposition-table best move
/// - immediate wins
/// - captures
/// - local threats
/// - center proximity
///
/// Better move ordering improves pruning efficiency and reduces
/// the explored search tree.
pub fn order_moves(
    game: &mut Game,
    moves: Vec<Pos>,
    tt_move: Option<Pos>,
) -> Vec<Pos> {
    let mut scored_moves: Vec<ScoredMove> = moves
        .into_iter()
        .map(|mv| {
            let mut score = 0;

            if Some(mv) == tt_move {
                score += TT_MOVE_BONUS;
            }

            let current_player = game.current_player();
            let before_capture = game.capture_count(current_player);

            // place the stone
            game.play_pos(mv).unwrap();

            // check for win
            let is_win = game.status == GameStatus::Win(current_player);
            let captures = game.capture_count(current_player) - before_capture;

            score += evaluate_threat(game, mv);

            score += center_score(mv);

            // add scores
            if is_win {
                score += WINNING_MOVE_BONUS;
            }
            score += captures as i32 * CAPTURE_BONUS;
            // remove the stone
            game.undo_move().unwrap();

            ScoredMove { mv, score }
        })
        .collect();

    #[allow(clippy::unnecessary_sort_by)]
    scored_moves.sort_by(|a, b| b.score.cmp(&a.score));
    scored_moves.into_iter().map(|s| s.mv).collect()
}