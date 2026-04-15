pub type Board = [[u8; 19]; 19];

#[derive(Copy, Clone, PartialEq)]
pub enum Player {
    Black,
    White,
}

impl Player {
    pub fn opponent(self) -> Player {
        match self {
            Player::Black => Player::White,
            Player::White => Player::Black,
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            Player::Black => 1,
            Player::White => 2,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Move {
    pub x: usize,
    pub y: usize,
    pub player: Player,
}

pub struct Game {
    pub board: Board,
    pub current_player: Player,
    pub status: GameStatus,
    pub history: Vec<Move>,
}

pub enum GameStatus {
    Ongoing,
    Win(Player),
    Draw,
}

impl Game {
    pub fn new() -> Self {
        Self {
            board: [[0; 19]; 19],
            current_player: Player::Black,
            status: GameStatus::Ongoing,
            history: Vec::new(),
        }
    }

    pub fn play_move(&mut self, x: usize, y: usize) -> Result<(), String> {
        if x >= 19 || y >= 19 {
            return Err("Out of bounds".to_string());
        }

        if self.board[y][x] != 0 {
            return Err("Cell already occupied".to_string());
        }

        if !matches!(self.status, GameStatus::Ongoing) {
            return Err("Game already finished".to_string());
        }

        self.board[y][x] = self.current_player.to_u8();
        self.history.push(Move {
            x,
            y,
            player: self.current_player,
        });

        if self.check_win(x, y) {
            // mark game as finished
            self.status = GameStatus::Win(self.current_player)
        } else if self.is_board_full() {
            self.status = GameStatus::Draw;
        } else {
            self.current_player = self.current_player.opponent();
        }

        Ok(())
    }

    pub fn print_board(&self) {
        for row in self.board.iter() {
            for cell in row.iter() {
                let symbol = match cell {
                    0 => ".",
                    1 => "X",
                    2 => "O",
                    _ => "?",
                };
                print!("{} ", symbol);
            }
            println!();
        }
    }

    pub fn check_win(&self, x: usize, y: usize) -> bool {
        //TODO:implement check win
        false

    }

    pub fn is_board_full(&self) -> bool {
        for row in self.board.iter() {
            for cell in row.iter() {
                if *cell == 0 {
                    return false;
                }
            }
        }
        true
    }
}