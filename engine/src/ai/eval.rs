//! Heuristic evaluation from the side-to-move's perspective.
//!
//! Scores are positive when the position favors the player whose turn it is.
//! The negamax search calls [`evaluate`] only at non-terminal nodes; the win
//! shortcut here is just a safety net for callers outside the search.
//!
//! # Pipeline
//!
//! 1. [`Board::for_each_line`] yields every row, column, and diagonal as a
//!    pair of `(me, opp)` bitmasks plus a length.
//! 2. [`crate::patterns::count_patterns`] tallies the runs on each line by
//!    length and openness.
//! 3. [`score_from_counts`] converts the tallies to a single integer using
//!    the per-pattern weights below.
//!
//! That's substantially cheaper than the per-cell walk this module used to
//! do, especially under a deep search.
//!
//! # Sign convention
//!
//! [`evaluate`] returns the score from the perspective of
//! [`Game::current_player`]; positive means the side to move is winning.
//! This is the convention negamax expects — the caller negates between plies.
#![allow(rustdoc::private_intra_doc_links)]

use crate::board::Board;
use crate::game::{Game, GameStatus, Player};
use crate::patterns::{count_patterns, PatternCounts};

/// Score returned for a winning terminal position. Mirrors
/// [`crate::ai::search::WIN_SCORE`] so the two modules can be reasoned
/// about independently.
pub const WIN_SCORE: i32    = 1_000_000;
const OPEN_FOUR: i32        = 100_000;
const CLOSED_FOUR: i32      = 20_000;
const OPEN_THREE: i32       = 5_000;
const CLOSED_THREE: i32     = 1_000;
const OPEN_TWO: i32         = 300;
const CLOSED_TWO: i32       = 50;
const CAPTURE_PAIR: i32     = 2_000;

/// A single 5-in-a-row already wins, but we never reach this branch in
/// search — `terminal_score` has the final word at the root. Five-counts
/// here are a defensive fallback if eval is called from outside.
const FIVE_FALLBACK: i32    = WIN_SCORE;

/// Score the position from the side-to-move's perspective.
///
/// Positive means the player on move is favored, negative means the
/// opponent is. Decisive positions return `±WIN_SCORE`; everything else
/// is a weighted sum of pattern counts plus a capture-difference term.
///
/// # Examples
///
/// ```ignore
/// let game = Game::new();
/// assert_eq!(evaluate(&game), 0); // empty board is balanced
/// ```
pub fn evaluate(game: &Game) -> i32 {
    if let GameStatus::Win(winner) = game.status {
        return if winner == game.current_player() { WIN_SCORE } else { -WIN_SCORE };
    }

    let me = game.current_player();
    let my_score = score_player(&game.board, me);
    let opp_score = score_player(&game.board, me.opponent());
    let capture_score = capture_diff(game, me) * CAPTURE_PAIR;

    my_score - opp_score + capture_score
}

/// Capture pairs `me` is ahead by. Negative when `me` is behind.
fn capture_diff(game: &Game, me: Player) -> i32 {
    let (black, white) = (game.captures.0 as i32, game.captures.1 as i32);
    match me {
        Player::Black => black - white,
        Player::White => white - black,
    }
}

/// Total pattern score for one player across every line on the board.
fn score_player(board: &Board, player: Player) -> i32 {
    let mut totals = PatternCounts::default();
    // No useful pattern fits in fewer than 5 cells — skip stub diagonals.
    board.for_each_line(player, 5, |me, opp, len| {
        let line = count_patterns(me, opp, len);
        totals.add(&line);
    });

    score_from_counts(&totals)
}

/// Convert a [`PatternCounts`] tally to an integer score using the
/// per-pattern weights at the top of this module.
fn score_from_counts(c: &PatternCounts) -> i32 {
    (c.fives as i32) * FIVE_FALLBACK
        + (c.open_four as i32) * OPEN_FOUR
        + (c.closed_four as i32) * CLOSED_FOUR
        + (c.open_three as i32) * OPEN_THREE
        + (c.closed_three as i32) * CLOSED_THREE
        + (c.open_two as i32) * OPEN_TWO
        + (c.closed_two as i32) * CLOSED_TWO
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    #[test]
    fn empty_board_is_balanced() {
        let game = Game::new();
        assert_eq!(evaluate(&game), 0);
    }

    #[test]
    fn open_three_outscores_open_two() {
        // Black builds an open three: . X X X .
        let mut three = Game::new();
        three.play_move(5, 9).unwrap();   // B
        three.play_move(0, 0).unwrap();   // W (parking, far away)
        three.play_move(6, 9).unwrap();   // B
        three.play_move(0, 1).unwrap();   // W
        three.play_move(7, 9).unwrap();   // B  → it's now White to move

        // Same setup but only an open two: . X X .
        let mut two = Game::new();
        two.play_move(5, 9).unwrap();
        two.play_move(0, 0).unwrap();
        two.play_move(6, 9).unwrap();
        two.play_move(0, 1).unwrap();
        two.play_move(8, 0).unwrap();      // extra B so White is on move here too

        assert!(
            evaluate(&three) < evaluate(&two),
            "open three (={}) should be more painful than open two (={})",
            evaluate(&three),
            evaluate(&two),
        );
    }

    #[test]
    fn capture_lead_helps_the_capturer() {
        let mut game = Game::new();
        game.play_move(0, 0).unwrap();   // B
        game.play_move(1, 0).unwrap();   // W
        game.play_move(4, 0).unwrap();   // B
        game.play_move(2, 0).unwrap();   // W
        game.play_move(3, 0).unwrap();   // B → captures (1,0) and (2,0)

        assert_eq!(game.captures, (1, 0));
        let score = evaluate(&game);
        assert!(score <= -CAPTURE_PAIR, "expected capture penalty, got {score}");
    }

    #[test]
    fn winning_status_dominates() {
        let mut game = Game::new();
        game.play_move(0, 0).unwrap();
        game.play_move(0, 1).unwrap();
        game.play_move(1, 0).unwrap();
        game.play_move(1, 1).unwrap();
        game.play_move(2, 0).unwrap();
        game.play_move(2, 1).unwrap();
        game.play_move(3, 0).unwrap();
        game.play_move(3, 1).unwrap();
        game.play_move(4, 0).unwrap(); // Black wins

        // White is on move; Black has won → -WIN_SCORE from White's POV.
        assert_eq!(evaluate(&game), -WIN_SCORE);
    }
}
