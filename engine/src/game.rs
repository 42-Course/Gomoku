pub type Board = [[u8; 19]; 19];

#[derive(Copy, Clone, PartialEq, Debug)]
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

#[derive(Clone)]
pub struct Move {
    pub x: usize,
    pub y: usize,
    pub player: Player,
    pub captured: Vec<(usize, usize)> // coordinates of stones captured
}

pub struct Game {
    pub board: Board,
    pub current_player: Player,
    pub status: GameStatus,
    pub history: Vec<Move>,
    pub captures: (u8, u8), // (black, white)
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
            captures: (0, 0),
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

        if self.count_free_threes(x, y) >= 2 {
            self.board[y][x] = 0;
            return Err("Double three is forbidden".to_string());
        }
        
        // apply captures
        let captured = self.apply_captures(x, y);

        // update capture count
        let captured_paris = captured.len() / 2;
        match self.current_player {
            Player::Black => self.captures.0 += captured_paris as u8,
            Player::White => self.captures.1 += captured_paris as u8,
        }

        self.history.push(Move {
            x,
            y,
            player: self.current_player,
            captured,
        });

        match self.current_player {
            Player::Black if self.captures.0 >= 5 => {
                self.status = GameStatus::Win(Player::Black);
            }
            Player::White if self.captures.1 >= 5 => {
                self.status = GameStatus::Win(Player::White);
            }
            _ => {
                if self.check_win(x, y) {
                    // mark game as finished
                    self.status = GameStatus::Win(self.current_player)
                } else if self.is_board_full() {
                    self.status = GameStatus::Draw;
                } else {
                    self.current_player = self.current_player.opponent();
                }
            }
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

    fn count_free_threes(&self, x: usize, y: usize) -> u32 {
        let mut count = 0;

        let directions = [(1,0), (0,1), (1,1), (1,-1)];

        for (dx, dy) in directions {
            if self.is_free_three(x, y, dx, dy) {
                count += 1;
            }
        }

        count
    }

    fn is_free_three(&self, x: usize, y: usize, dx: isize, dy: isize) -> bool {
        let player = self.board[y][x];

        let mut line = Vec::new();

        for i in -4..=4 {
            let cx = x as isize + i * dx;
            let cy = y as isize + i * dy;

            if cx < 0 || cy < 0 || cx >= 19 || cy >= 19 {
                line.push(3); //push three for outside board
            } else {
                line.push(self.board[cy as usize][cx as usize]);
            }
        }

        for i in 0..=line.len() - 6 {
            if line[i] != 0 { continue; }

            // scan for pattern: . X X X . .
            if line[i + 1] == player
                && line[i + 2] == player
                && line[i + 3] == player
                && line[i + 4] == 0
                && line[i + 5] == 0
            {
                return true;
            }

            // scan for pattern: . . X X X .
            if line[i + 1] == 0
                && line[i + 2] == player
                && line[i + 3] == player
                && line[i + 4] == player
                && line[i + 5] == 0
            {
                return true;
            }

            // scan for pattern: . X X . X .
            if line[i + 1] == player
                && line[i + 2] == player
                && line[i + 3] == 0
                && line[i + 4] == player
                && line[i + 5] == 0
            {
                return true;
            }

            // scan for pattern: . X . X X .
            if line[i + 1] == player
                && line[i + 2] == 0
                && line[i + 3] == player
                && line[i + 4] == player
                && line[i + 5] == 0
            {
                return true;
            }
        }
        false
    }

    fn count_direction(&self, x: usize, y: usize, dx: isize, dy: isize) -> u32 {
        let mut count = 0;
        let player = self.board[y][x];

        let mut cx = x as isize;
        let mut cy = y as isize;

        loop {
            cx += dx;
            cy += dy;

            if cx < 0 || cy < 0 || cx >= 19 || cy >= 19 {
                break;
            }

            if self.board[cy as usize][cx as usize] != player {
                break;
            }

            count += 1;
        }
        count
    }

    pub fn check_win(&self, x: usize, y: usize) -> bool {
        let directions = [
            (1, 0),  // horizontal
            (0, 1),  // vertical
            (1, 1),  // diagonal \
            (1, -1), // diagonal /
        ];

        for (dx, dy) in directions {
            // check the row length
            let count = 1
                + self.count_direction(x, y, dx, dy)
                + self.count_direction(x, y, -dx, -dy);

            if count >= 5 {
                return true;
            }
        }
        false

    }

    fn apply_captures(&mut self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut captured = Vec::new();
        let player = self.current_player.to_u8();
        let opponent = self.current_player.opponent().to_u8();

        let directions = [(1, 0), (0, 1), (1, 1), (1, -1)];

        for (dx, dy) in directions {
            for &(sx, sy) in &[(dx, dy), (-dx, -dy)] {
                let x1 = x as isize + sx;
                let y1 = y as isize + sy;

                let x2 = x as isize + 2 * sx;
                let y2 = y as isize + 2 * sy;

                let x3 = x as isize + 3 * sx;
                let y3 = y as isize + 3 * sy;

                if x3 < 0 || y3 < 0 || x3 >= 19 || y3 >= 19 {
                    continue;
                }

                let (x1, y1) = (x1 as usize, y1 as usize);
                let (x2, y2) = (x2 as usize, y2 as usize);
                let (x3, y3) = (x3 as usize, y3 as usize);

                if self.board[y1][x1] == opponent
                    && self.board[y2][x2] == opponent
                    && self.board[y3][x3] == player
                {
                    //capture
                    self.board[y1][x1] = 0;
                    self.board[y2][x2] = 0;

                    captured.push((x1, y1));
                    captured.push((x2, y2));
                }
            }
        }
        captured
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

#[cfg(test)]
mod tests {
    use super::*;
}

#[test]
fn test_horizontal_win() {
    let mut game = Game::new();

    game.play_move(0, 0).unwrap();
    game.play_move(0, 1).unwrap();
    game.play_move(1, 0).unwrap();
    game.play_move(1, 1).unwrap();
    game.play_move(2, 0).unwrap();
    game.play_move(2, 1).unwrap();
    game.play_move(3, 0).unwrap();
    game.play_move(3, 1).unwrap();
    game.play_move(4, 0).unwrap();

    match game.status {
        GameStatus::Win(Player::Black) => {}
        _ => panic!("Expected Black to win"),
    }
}

#[test]
fn test_capture_simple() {
    let mut game = Game::new();

    game.play_move(0, 0).unwrap(); // X
    game.play_move(1, 0).unwrap(); // O
    game.play_move(4, 0).unwrap(); // X
    game.play_move(2, 0).unwrap(); // O

    game.play_move(3, 0).unwrap(); // X → capture

    assert_eq!(game.board[0][1], 0);
    assert_eq!(game.board[0][2], 0);
}

#[test]
fn test_double_capture() {
    let mut game = Game::new();

    // setup: X O O X O O X
    game.play_move(0, 0).unwrap(); // X
    game.play_move(1, 0).unwrap(); // O
    game.play_move(0, 1).unwrap(); // X
    game.play_move(2, 0).unwrap(); // O

    game.play_move(0, 4).unwrap(); // X
    game.play_move(4, 0).unwrap(); // O
    game.play_move(6, 0).unwrap(); // X
    game.play_move(5, 0).unwrap(); // O

    game.play_move(3, 0).unwrap(); // X → should capture both pairs

    assert_eq!(game.board[0][1], 0);
    assert_eq!(game.board[0][2], 0);
    assert_eq!(game.board[0][4], 0);
    assert_eq!(game.board[0][5], 0);
}

#[test]
fn test_double_three_forbidden() {
    let mut game = Game::new();

    // Setup shape:
    //    0 1 2 3 4 5 6 7
    //   0. . . . . . . .
    //   1. X . . . . . .
    //   2. . X . . . . .
    //   3. . . . . . . .
    //   4. . . . . X X .
    //   5. . . . . . . .
    //
    // Playing center creates two open-threes

    game.play_move(1, 1).unwrap(); // X
    game.play_move(10, 18).unwrap(); // O
    game.play_move(2, 2).unwrap(); // X
    game.play_move(12, 15).unwrap(); // O
    game.play_move(5, 4).unwrap(); // X
    game.play_move(0, 2).unwrap(); // O
    game.play_move(6, 4).unwrap(); // X
    game.play_move(0, 3).unwrap(); // O

    // This should be forbidden (double three)
    let result = game.play_move(4, 4);

    assert!(result.is_err());
}

#[test]
fn test_not_double_three_border() {
    let mut game = Game::new();

    // Setup shape:
    //    0 1 2 3 4 5 6 7
    //   0. . X . . . . .
    //   1. . X . . . . .
    //   2. . . X X . . .
    //   3. . . . . . . .
    //   4. . . . . . . .
    //   5. . . . . . . .
    //
    // Playing center creates two open-threes

    game.play_move(2, 0).unwrap(); // X
    game.play_move(10, 18).unwrap(); // O
    game.play_move(2, 1).unwrap(); // X
    game.play_move(12, 15).unwrap(); // O
    game.play_move(3, 2).unwrap(); // X
    game.play_move(0, 5).unwrap(); // O
    game.play_move(4, 2).unwrap(); // X
    game.play_move(0, 3).unwrap(); // O

    let result = game.play_move(2, 2);

    assert!(result.is_ok());
}