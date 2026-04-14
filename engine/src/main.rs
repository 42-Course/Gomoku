mod game;

use game::Game;

fn main() {
    let mut game = Game::new();

    game.play_move(9, 9).unwrap();
    game.play_move(10, 9).unwrap();

    game.print_board();
}
