use crate::game::{Player, Cell};
use crate::constants::BOARD_SIZE;

const CELL_COUNT: usize = BOARD_SIZE * BOARD_SIZE;
const WORD_COUNT: usize = CELL_COUNT.div_ceil(64);

#[allow(dead_code)]
const LAST_WORDS_BITS: usize = CELL_COUNT % 64;

pub struct BitBoard {
    words: [u64; 6],
}

impl BitBoard {
    pub fn new() -> Self {
        Self {
            words: [0; 6]
        }
    }
    fn set(&mut self, idx: usize) {
        let w = idx / 64;
        let b = idx % 64;
        self.words[w] |= 1u64 << b;
    }

    fn clear(&mut self, idx: usize) {
        let w = idx / 64;
        let b = idx % 64;
        self.words[w] &= !(1u64 << b);
    }

    fn get(&self, idx: usize) -> bool {
        let w = idx / 64;
        let b = idx % 64;
        (self.words[w] >> b) & 1 == 1
    }

    fn index(x: usize, y: usize) -> usize {
        y * BOARD_SIZE + x
    }

    pub fn place_stone(&mut self, x: usize, y: usize) {
        let idx = Self::index(x, y);
        self.set(idx);
    }

    pub fn remove_stone(&mut self, x: usize, y: usize) {
        let idx = Self::index(x, y);
        self.clear(idx);
    }

    pub fn is_occupied(&self, x: usize, y: usize) -> bool {
        let idx = Self::index(x, y);
        self.get(idx)
    }

    fn or(&self, other: &Self) -> Self {
        let mut out = Self::new();

        for i in 0..WORD_COUNT {
            out.words[i] = self.words[i] | other.words[i];
        }

        out
    }
}

pub struct Board {
    boards: [BitBoard; 2],
}

impl Board {
    pub fn new() -> Self {
        Self {
            boards: [BitBoard::new(), BitBoard::new()]
        }
    }

    pub fn place_stone(&mut self, x: usize, y: usize, player: Player) {
        self.boards[player.idx()].place_stone(x, y);
    }

    pub fn remove_stone(&mut self, x: usize, y: usize, player: Player) {
        self.boards[player.idx()].remove_stone(x, y);
    }

    #[allow(dead_code)]
    pub fn has(&mut self, x: usize, y: usize, player: Player) {
        self.boards[player.idx()].is_occupied(x, y);
    }

    pub fn is_empty(&self, x: usize, y: usize) -> bool {
        !self.boards[Player::Black.idx()].is_occupied(x, y) &&
        !self.boards[Player::White.idx()].is_occupied(x, y)
    }

    pub fn empty_check(&self, x: usize, y: usize) -> Result<(), &'static str> {
        if x >= BOARD_SIZE || y >= BOARD_SIZE {
            return Err("Out of bounds");
        }

        if !self.is_empty(x, y) {
            return Err("Cell already occupied");
        }
        Ok(())
    }

    pub fn cell_at(&self, x: usize, y: usize) -> Cell {
        if self.boards[Player::Black.idx()].is_occupied(x, y) {
            Some(Player::Black)
        } else if self.boards[Player::White.idx()].is_occupied(x, y) {
            Some(Player::White)
        } else {
            None
        }
    }

    pub fn is_full(&self) -> bool {
        let occupied = self.boards[Player::Black.idx()].or(&self.boards[Player::White.idx()]);
        let w = BOARD_SIZE / 64;
        let b = BOARD_SIZE % 64;

        for i in 0..w {
            if occupied.words[i] != u64::MAX {
                return false;
            }
        }

        let last_mast = (1u64 << b) - 1;

        occupied.words[w] == last_mast
    }

    pub fn print_board(&self) {
        let height = BOARD_SIZE;
        let width = BOARD_SIZE;
        let row_digits = height.to_string().len();

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

        print!("{}", " ".repeat(row_digits + 3));

        for col in 0..width {
            print!("{} ", col % 10);
        }

        println!();
        println!("{}{}", " ".repeat(row_digits + 3), "-".repeat(width * 2));

        for y in 0..height {
            let mut row_str = String::new();

            for x in 0..width {
                let symbol = match self.cell_at(x, y) {
                    None => ".",
                    Some(Player::Black) => "X",
                    Some(Player::White) => "O",
                };

                row_str.push_str(symbol);
                row_str.push(' ');
            }

            println!("{:2} | {}", y, row_str.trim_end());
        }
    }
}