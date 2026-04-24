use crate::game::Game;
use crate::game::Player;

const WIN_SCORE: i32        = 1_000_000;
const OPEN_FOUR: i32       = 100_000;
const CLOSED_FOUR: i32     = 20_000;
const OPEN_THREE: i32      = 5_000;
const CLOSED_THREE: i32    = 1_000;
const OPEN_TWO: i32        = 300;
const CAPTURE_VALUE: i32   = 2_000;

pub fn evaluate(game: &Game) -> i32 {
    if game.is_win_for(game.current_player()) {
        return WIN_SCORE;
    }

    if game.is_win_for(game.current_player().opponent()) {
        return -WIN_SCORE;
    }

    let me = game.current_player();
    let opp = me.opponent();

    let my_score = evaluate_player(game, me);
    let opp_score = evaluate_player(game, opp);

    let capture_score = evaluate_captures(game, me, opp);

    my_score - opp_score + capture_score
}

fn evaluate_player(game: &Game, player: Player) -> i32 {
    let mut score = 0;

    //TODO: evaluate player score

    score
}

fn score_pattern(run: usize, open_ends: usize) -> i32 {
    let mut score = 0;
    match (run, open_ends) {
        //TODO: match patterns and return score
    }
    score
}

fn evaluate_captures(game: &Game, me: Player, opp: Player) -> i32 {
    let mut score = 0;
    //TODO: evaluate scores based on captures
    score
}