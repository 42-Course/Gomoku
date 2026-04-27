use crate::constants::{BOARD_SIZE, BOARD_SIZE_I};
use crate::game::{Cell, Player};

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

    /// Pack one straight line of cells into two bitmasks.
    ///
    /// Walks from `(x0, y0)` along `(dx, dy)` for at most `max_len` steps,
    /// stopping at the board edge. Returns `(me, opp, len)` where bit `i`
    /// of `me` is set if the i-th cell holds `player`, bit `i` of `opp` is
    /// set if it holds the opponent, and `len` is the number of cells we
    /// walked. `len <= 19 < 32`, so a `u32` is always wide enough.
    ///
    /// Cells beyond the line (off-board, or past `len`) sit at zero in
    /// both masks — pattern matchers that need an *empty* cell at an
    /// endpoint won't see them as empty, so the board edge is a wall.
    pub fn pack_line(
        &self,
        x0: isize,
        y0: isize,
        dx: isize,
        dy: isize,
        max_len: u32,
        player: Player,
    ) -> (u32, u32, u32) {
        let opp_idx = player.opponent().idx();
        let me_idx = player.idx();
        let mut me = 0u32;
        let mut opp = 0u32;
        let mut len = 0u32;
        let mut x = x0;
        let mut y = y0;
        while len < max_len && (0..BOARD_SIZE_I).contains(&x) && (0..BOARD_SIZE_I).contains(&y) {
            let (ux, uy) = (x as usize, y as usize);
            if self.boards[me_idx].is_occupied(ux, uy) {
                me |= 1 << len;
            } else if self.boards[opp_idx].is_occupied(ux, uy) {
                opp |= 1 << len;
            }
            x += dx;
            y += dy;
            len += 1;
        }
        (me, opp, len)
    }

    /// Visit every distinct horizontal/vertical/diagonal/anti-diagonal line
    /// on the board for `player`, calling `f(me, opp, len)` for each.
    ///
    /// Lines shorter than `min_len` are skipped (no useful pattern fits in
    /// fewer cells, and skipping cuts the diagonal count almost in half).
    pub fn for_each_line<F>(&self, player: Player, min_len: u32, mut f: F)
    where
        F: FnMut(u32, u32, u32),
    {
        let n = BOARD_SIZE_I;

        // Rows, then columns: always full board-sized lines.
        for y in 0..n {
            let (me, opp, len) = self.pack_line(0, y, 1, 0, BOARD_SIZE as u32, player);
            if len >= min_len {
                f(me, opp, len);
            }
        }
        for x in 0..n {
            let (me, opp, len) = self.pack_line(x, 0, 0, 1, BOARD_SIZE as u32, player);
            if len >= min_len {
                f(me, opp, len);
            }
        }

        // Diagonals (dx=1, dy=1): start where the previous cell is off-board,
        // i.e. x == 0 or y == 0.
        for x in 0..n {
            let (me, opp, len) = self.pack_line(x, 0, 1, 1, BOARD_SIZE as u32, player);
            if len >= min_len {
                f(me, opp, len);
            }
        }
        for y in 1..n {
            let (me, opp, len) = self.pack_line(0, y, 1, 1, BOARD_SIZE as u32, player);
            if len >= min_len {
                f(me, opp, len);
            }
        }

        // Anti-diagonals (dx=1, dy=-1): start where x == 0 or y == n-1.
        for x in 0..n {
            let (me, opp, len) = self.pack_line(x, n - 1, 1, -1, BOARD_SIZE as u32, player);
            if len >= min_len {
                f(me, opp, len);
            }
        }
        for y in 0..(n - 1) {
            let (me, opp, len) = self.pack_line(0, y, 1, -1, BOARD_SIZE as u32, player);
            if len >= min_len {
                f(me, opp, len);
            }
        }
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