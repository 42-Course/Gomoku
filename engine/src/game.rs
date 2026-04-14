pub type Board = [[u8; 19]; 19];

pub struct Game {
    pub board: Board,
    pub current_player: u8,
}

impl Game {
    pub fn new() -> Self {
        Self {
            board: [[0; 19]; 19],
            current_player: 1,
        }
    }

    pub fn play_move(&mut self, x: usize, y: usize) -> Result<(), String> {
        if x >= 19 || y >= 19 {
            return Err("Out of bounds".to_string());
        }

        if self.board[y][x] != 0 {
            return Err("Cell already occupied".to_string());
        }

        self.board[y][x] = self.current_player;

        self.current_player = if self.current_player == 1 { 2 } else { 1 };

        Ok(())
    }

    pub fn print_board(&self) {
        for row in self.board.iter() {
            for cell in row.iter() {
                let symbol = match cell {
                    0 => ".",
                    1 => "X",
                    2 => "0",
                    _ => "?",
                };
                print!("{} ", symbol);
            }
            println!();
        }
    }
}