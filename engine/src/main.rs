mod game;

use game::Game;
use game::GameStatus;

fn main() {
    let mut game = Game::new();

    game.play_move(9, 9).unwrap();
    game.play_move(10, 9).unwrap();

    game.play_move(0, 0).unwrap();
    game.play_move(0, 1).unwrap();
    game.play_move(1, 0).unwrap();
    game.play_move(1, 1).unwrap();
    game.play_move(2, 0).unwrap();
    game.play_move(2, 1).unwrap();
    game.play_move(3, 0).unwrap();
    game.play_move(3, 1).unwrap();
    game.play_move(4, 0).unwrap(); // should win

    game.print_board();

    match game.status {
        GameStatus::Win(player) => {
            println!("Winner: {:?}", player);
        }
        GameStatus::Draw => {
            println!("Draw");
        }
        GameStatus::Ongoing => {
            println!("Game still ongoing");
        }
    }
}
