use crate::game::{ Game, Pos };



#[derive(Debug, Clone, Copy)]
pub struct ScoredMove {
    pub mv: Pos,
    pub score: i32,
}

const TT_MOVE_BONUS: i32 = 1_000_000;
const WINNING_MOVE_BONUS: i32 = 500_000;
const CAPTURE_BONUS: i32 = 100_000;
const THREAT_BONUS: i32 = 50_000;
const CENTER_BONUS: i32 = 10;

fn is_winning_move(game: &mut Game, pos: Pos) -> bool {
    let current_player = game.current_player();
    

    game.play_move(pos.to_xy()).unwrap();

    let is_win = game.check_win(current_player);

    game.undo_move().unwrap();

    is_win
}

pub fn order_moves(
    game: &Game,
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

            if is_winning_move(game, mv) {
                score += WINNING_MOVE_BONUS;
            }

            let captures = count_captures(game, mv);
            score += captures as i32 * CAPTURE_BONUS;

            score += evaluate_threat(game, mv) * THREAT_BONUS;

            score += center_score(mv);

            ScoredMove { mv, score }
        })
        .collect();

    scored_moves.sort_by(|a, b| b.score.cmp(&a.score));
    scored_moves.into_iter().map(|s| s.mv).collect()
}