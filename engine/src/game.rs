#[allow(dead_code)]
pub const DEBUG_PRINT: bool = true;

#[macro_export]
macro_rules! play {
    ($game:expr, $x:expr, $y:expr) => {{
        let result = $game.play_move($x, $y);
        if $crate::game::DEBUG_PRINT {
            $game.print_board_with(true, true);
        }
        result
    }};
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Player {
    Black,
    White,
}

pub type Cell = Option<Player>;
pub type Board = [[Cell; 19]; 19];

#[derive(Copy, Clone, Debug)]
pub enum Direction {
    Horizontal,
    Vertical,
    Diagonal,
    AntiDiagonal,
}

impl Direction {
    pub fn delta(self) -> (isize, isize) {
        match self {
            Direction::Horizontal => (1, 0),
            Direction::Vertical => (0, 1),
            Direction::Diagonal => (1, 1),
            Direction::AntiDiagonal => (1, -1),
        }
    }

    pub fn all_directions() -> [(isize, isize); 4] {
        [
            Direction::Horizontal.delta(),   // (1, 0)
            Direction::Vertical.delta(),     // (0, 1)
            Direction::Diagonal.delta(),     // (1, 1)
            Direction::AntiDiagonal.delta(), // (1, -1)
        ]
    }
}


impl Player {
    pub fn opponent(self) -> Player {
        match self {
            Player::Black => Player::White,
            Player::White => Player::Black,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct Move {
    pub x: usize,
    pub y: usize,
    pub captured: Vec<(usize, usize)> // coordinates of stones captured
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "[{}, {}]", self.x, self.y)
    }
}

pub struct Game {
    pub board: Board,
    pub status: GameStatus,
    pub history: Vec<Move>,
    pub captures: (u8, u8), // (black, white)
}

#[allow(dead_code)]
pub enum GameStatus {
    Ongoing,
    Win(Player),
    Draw,
}

#[allow(dead_code)]
impl Game {
    pub fn new() -> Self {
        Self {
            board: [[None; 19]; 19],
            status: GameStatus::Ongoing,
            history: Vec::new(), // The index of the history represents the move number, starting from 0 (player)
            captures: (0, 0),
        }
    }

    pub fn player_at(&self, move_index: usize) -> Player {
        if move_index.is_multiple_of(2) {
            Player::Black
        } else {
            Player::White
        }
    }

    pub fn current_player(&self) -> Player {
        self.player_at(self.history.len())
    }

    pub fn get_player_at(&self, index: usize) -> Player {
        self.player_at(index)
    }

    pub fn play_move(&mut self, x: usize, y: usize) -> Result<(), String> {
        if x >= 19 || y >= 19 {
            return Err("Out of bounds".to_string());
        }

        if self.board[y][x].is_some() {
            return Err("Cell already occupied".to_string());
        }

        if !matches!(self.status, GameStatus::Ongoing) {
            return Err("Game already finished".to_string());
        }

        let player = self.current_player();
        self.board[y][x] = Some(player);

        if self.count_free_threes(x, y) >= 2 {
            self.board[y][x] = None;
            return Err("Double three is forbidden".to_string());
        }

        // apply captures
        let captured = self.apply_captures(x, y);

        // update capture count
        let captured_pairs = captured.len() / 2;
        match player {
            Player::Black => self.captures.0 += captured_pairs as u8,
            Player::White => self.captures.1 += captured_pairs as u8,
        }

        self.history.push(Move {
            x,
            y,
            captured,
        });

        match player {
            Player::Black if self.captures.0 >= 5 => {
                self.status = GameStatus::Win(Player::Black);
            }
            Player::White if self.captures.1 >= 5 => {
                self.status = GameStatus::Win(Player::White);
            }
            _ => {
                if self.check_win(x, y) {
                    self.status = GameStatus::Win(player)
                } else if self.is_board_full() {
                    self.status = GameStatus::Draw;
                }
            }

        }

        Ok(())
    }

    pub fn print_board_with(&self, show_coords: bool, print_turn: bool) {
        if print_turn {
            println!("\n-------\nTurn {:2} | Move {}", self.history.len(), self.history.last().unwrap());
        }

        let height = self.board.len();
        let width = self.board.first().map(|r| r.len()).unwrap_or(0);
        let row_digits = height.to_string().len();

        if show_coords {
            // Tens row
            print!("{}", " ".repeat(row_digits + 3));
            for col in 0..width {
                let tens = col / 10;
                if tens > 0 {
                    print!("{} ", tens);
                } else {
                    print!("  ");
                }
            }
            println!();

            // Ones row
            print!("{}", " ".repeat(row_digits + 3));
            for col in 0..width {
                print!("{} ", col % 10);
            }
            println!();

            // Separator
            println!("{}{}", " ".repeat(row_digits + 3), "-".repeat(width * 2));
        }

        for (index, row) in self.board.iter().enumerate() {
            let mut row_str = String::new();
            for cell in row.iter() {
                let symbol = match cell {
                    None => ".",
                    Some(Player::Black) => "X",
                    Some(Player::White) => "O",
                };
                row_str.push_str(symbol);
                row_str.push(' ');
            }

            if show_coords {
                println!("{:2} | {}", index, row_str.trim_end());
            } else {
                println!("{}", row_str.trim_end());
            }
        }
    }

    pub fn print_board(&self) {
        self.print_board_with(false, false);
    }

    fn count_direction(&self, x: usize, y: usize, dx: isize, dy: isize) -> u8 {
        let player = self.board[y][x];
        let mut count = 0;

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
        for (dx, dy) in Direction::all_directions() {
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
        let player = self.current_player();
        let opponent = player.opponent();

        for (dx, dy) in Direction::all_directions() {
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

                if self.board[y1][x1] == Some(opponent)
                    && self.board[y2][x2] == Some(opponent)
                    && self.board[y3][x3] == Some(player)
                {
                    self.board[y1][x1] = None;
                    self.board[y2][x2] = None;

                    captured.push((x1, y1));
                    captured.push((x2, y2));
                }
            }
        }
        captured
    }

        fn count_free_threes(&self, x: usize, y: usize) -> u32 {
        let mut count = 0;

        for (dx, dy) in Direction::all_directions() {
            if self.is_free_three(x, y, dx, dy) {
                count += 1;
            }
        }

        count
    }

    fn is_free_three(&self, x: usize, y: usize, dx: isize, dy: isize) -> bool {
        let player = self.board[y][x].unwrap();

        // let mut line = Vec::new();
        let mut line: Vec<Cell> = Vec::new(); // Cell = Option<Player>

        for i in -4..=4 {
            let cx = x as isize + i * dx;
            let cy = y as isize + i * dy;

            if cx < 0 || cy < 0 || cx >= 19 || cy >= 19 {
                continue;
                // line.push(None); //push three for outside board
            } else {
                line.push(self.board[cy as usize][cx as usize]);
            }
        }

        if line.len() < 6 { return false; }
        for i in 0..=line.len() - 6 {
            if line[i].is_some() { continue; }

            // scan for pattern: . X X X . .
            if line[i + 1] == Some(player)
                && line[i + 2] == Some(player)
                && line[i + 3] == Some(player)
                && line[i + 4].is_none()
                && line[i + 5].is_none()
            {
                return true;
            }

            // scan for pattern: . . X X X .
            if line[i + 1].is_none()
                && line[i + 2] == Some(player)
                && line[i + 3] == Some(player)
                && line[i + 4] == Some(player)
                && line[i + 5].is_none()
            {
                return true;
            }

            // scan for pattern: . X X . X .
            if line[i + 1] == Some(player)
                && line[i + 2] == Some(player)
                && line[i + 3].is_none()
                && line[i + 4] == Some(player)
                && line[i + 5].is_none()
            {
                return true;
            }

            // scan for pattern: . X . X X .
            if line[i + 1] == Some(player)
                && line[i + 2].is_none()
                && line[i + 3] == Some(player)
                && line[i + 4] == Some(player)
                && line[i + 5].is_none()
            {
                return true;
            }
        }
        false
    }

    pub fn is_board_full(&self) -> bool {
        self.board.iter().all(|row| row.iter().all(|c| c.is_some()))
    }
}

#[allow(unused_imports)]
#[cfg(test)]
mod tests {
    use super::*;
}

#[test]
fn test_horizontal_win() {
    let mut game = Game::new();

    play!(game, 0, 0).unwrap();
    play!(game, 0, 1).unwrap();
    play!(game, 1, 0).unwrap();
    play!(game, 1, 1).unwrap();
    play!(game, 2, 0).unwrap();
    play!(game, 2, 1).unwrap();
    play!(game, 3, 0).unwrap();
    play!(game, 3, 1).unwrap();
    play!(game, 4, 0).unwrap();

    match game.status {
        GameStatus::Win(Player::Black) => {}
        _ => panic!("Expected Black to win"),
    }
}

#[test]
fn test_capture_simple() {
    let mut game = Game::new();

    play!(game, 0, 0).unwrap(); // X
    play!(game, 1, 0).unwrap(); // O
    play!(game, 4, 0).unwrap(); // X
    play!(game, 2, 0).unwrap(); // O

    play!(game, 3, 0).unwrap(); // X → capture

    assert_eq!(game.board[0][1], None);
    assert_eq!(game.board[0][2], None);
}

#[test]
fn test_double_capture() {
    let mut game = Game::new();

    // setup: X O O X O O X
    play!(game, 0, 0).unwrap(); // X
    play!(game, 1, 0).unwrap(); // O
    play!(game, 0, 1).unwrap(); // X
    play!(game, 2, 0).unwrap(); // O

    play!(game, 0, 4).unwrap(); // X
    play!(game, 4, 0).unwrap(); // O
    play!(game, 6, 0).unwrap(); // X
    play!(game, 5, 0).unwrap(); // O

    play!(game, 3, 0).unwrap(); // X → should capture both pairs

    assert_eq!(game.board[0][1], None);
    assert_eq!(game.board[0][2], None);
    assert_eq!(game.board[0][4], None);
    assert_eq!(game.board[0][5], None);
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

    play!(game, 1, 1).unwrap(); // X
    play!(game, 10, 18).unwrap(); // O
    play!(game, 2, 2).unwrap(); // X
    play!(game, 12, 15).unwrap(); // O
    play!(game, 5, 4).unwrap(); // X
    play!(game, 0, 2).unwrap(); // O
    play!(game, 6, 4).unwrap(); // X
    play!(game, 0, 3).unwrap(); // O

    // This should be forbidden (double three)
    let result = play!(game, 4, 4);

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

    play!(game, 2, 0).unwrap(); // X
    play!(game, 10, 18).unwrap(); // O
    play!(game, 2, 1).unwrap(); // X
    play!(game, 12, 15).unwrap(); // O
    play!(game, 3, 2).unwrap(); // X
    play!(game, 0, 5).unwrap(); // O
    play!(game, 4, 2).unwrap(); // X
    play!(game, 0, 3).unwrap(); // O

    let result = play!(game, 2, 2);

    assert!(result.is_ok());
}