//! Bit-packed board storage and line iteration.
//!
//! Stones are stored in two per-player [`BitBoard`]s rather than a 2D array
//! of [`Cell`]s. The board exposes two complementary views:
//!
//! - **Cell-level** ([`Board::cell_at`], [`Board::is_empty`], …) for rule
//!   logic that thinks one square at a time.
//! - **Line-level** ([`Board::pack_line`], [`Board::for_each_line`]) for
//!   pattern detection that thinks one row/column/diagonal at a time.
//!
//! The line view packs up to 19 cells of one direction into a pair of
//! `u32` bitmasks `(me, opp)`. Off-the-board positions are zero in *both*
//! masks, which makes them walls - see the [`crate::patterns`] module for
//! how that interacts with pattern recognition.

use crate::constants::{BOARD_SIZE, BOARD_SIZE_I, CELL_COUNT};
use crate::game::{Cell, Player, Pos};
const WORD_COUNT: usize = CELL_COUNT.div_ceil(64);

/// One player's stones, stored as a bitmap with one bit per cell.
///
/// Cells are flattened in row-major order: bit `y * BOARD_SIZE + x` is set
/// when that cell holds a stone. Six `u64` words is enough for 19×19 = 361
/// cells with room to spare.
#[derive(Clone)]
pub struct BitBoard {
    words: [u64; WORD_COUNT],
}

impl BitBoard {
    /// Create an empty bitboard.
    pub fn new() -> Self {
        Self {
            words: [0; 6]
        }
    }
    /// Set the bit at `idx`.
    #[inline]
    fn set(&mut self, idx: usize) {
        let w = idx / 64;
        let b = idx % 64;
        self.words[w] |= 1u64 << b;
    }

    /// Clear the bit at `idx`.
    #[inline]
    fn clear(&mut self, idx: usize) {
        let w = idx / 64;
        let b = idx % 64;
        self.words[w] &= !(1u64 << b);
    }

    /// Returns whether the bit at `idx` is set.
    #[inline]
    fn get(&self, idx: usize) -> bool {
        let w = idx / 64;
        let b = idx % 64;
        (self.words[w] >> b) & 1 == 1
    }

    /// Map `(x, y)` to a flat bit index in row-major order.
    #[allow(dead_code)]
    #[inline]
    fn index(x: usize, y: usize) -> usize {
        y * BOARD_SIZE + x
    }

    /// Mark cell `(x, y)` as containing a stone.
    #[inline]
    pub fn place_stone(&mut self, pos: Pos) {
        self.set(pos.idx());
    }

    /// Clear the stone at cell `(x, y)`.
    #[inline]
    pub fn remove_stone(&mut self, pos: Pos) {
        self.clear(pos.idx());
    }

    /// Whether cell `(x, y)` holds a stone.
    #[inline]
    pub fn is_occupied(&self, pos: Pos) -> bool {
        self.get(pos.idx())
    }

    /// Bitwise OR with another bitboard. Used to compute "any stone here".
    fn or(&self, other: &Self) -> Self {
        let mut out = Self::new();

        for i in 0..WORD_COUNT {
            out.words[i] = self.words[i] | other.words[i];
        }

        out
    }
}

/// The full game board: one [`BitBoard`] per player.
///
/// Indexed `boards[player.idx()]`; black is index 0 and white is index 1.
#[derive(Clone)]
pub struct Board {
    boards: [BitBoard; 2],
}

impl Board {
    /// Create an empty board.
    pub fn new() -> Self {
        Self {
            boards: [BitBoard::new(), BitBoard::new()]
        }
    }

    /// Place a stone for `player` at `(x, y)`.
    ///
    /// No bounds or occupancy check is performed - callers should go
    /// through [`Board::empty_check`] first.
    #[inline]
    pub fn place_stone(&mut self, pos: Pos, player: Player) {
        self.boards[player.idx()].place_stone(pos);
    }

    /// Remove `player`'s stone at `(x, y)`. Used by undo and capture.
    #[inline]
    pub fn remove_stone(&mut self, pos: Pos, player: Player) {
        self.boards[player.idx()].remove_stone(pos);
    }

    /// Check if the given coordinate contains the player
    #[inline]
    pub fn has(&self, pos: Pos, player: Player) -> bool {
        self.boards[player.idx()].is_occupied(pos)
    }

    /// Whether `(x, y)` is empty for *both* players.
    #[inline]
    pub fn is_empty(&self, pos: Pos) -> bool {
        !self.has(pos, Player::Black) && !self.has(pos, Player::White)
    }

    /// Validate that `(x, y)` is a legal placement target.
    ///
    /// # Errors
    ///
    /// - `"Out of bounds"` if `x` or `y` is outside `0..BOARD_SIZE`.
    /// - `"Cell already occupied"` if either player has a stone there.
    pub fn empty_check(&self, pos: Pos) -> Result<(), &'static str> {
        if pos.idx() >= CELL_COUNT {
            return Err("Out of bounds");
        }

        if !self.is_empty(pos) {
            return Err("Cell already occupied");
        }

        Ok(())
    }

    /// Look up the contents of cell `Pos`.
    ///
    /// Returns `Some(Player)` if a stone is present, `None` if empty.
    pub fn cell_at(&self, pos: Pos) -> Cell {
        if self.boards[Player::Black.idx()].is_occupied(pos) {
            Some(Player::Black)
        } else if self.boards[Player::White.idx()].is_occupied(pos) {
            Some(Player::White)
        } else {
            None
        }
    }

    /// Look up the contents of cell `(x, y)`.
    ///
    /// Returns `Some(Player)` if a stone is present, `None` if empty.
    #[allow(dead_code)]
    pub fn cell_at_xy(&self, x: usize, y: usize) -> Cell {
        let pos = Pos::from_xy(x, y);
        if self.boards[Player::Black.idx()].is_occupied(pos) {
            Some(Player::Black)
        } else if self.boards[Player::White.idx()].is_occupied(pos) {
            Some(Player::White)
        } else {
            None
        }
    }

    /// Whether every cell on the board is occupied (used to flag draws).
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
    /// stopping at the board edge. Returns `(me, opp, len)` where:
    ///
    /// - bit `i` of `me`  is set when the i-th cell holds `player`,
    /// - bit `i` of `opp` is set when it holds the opponent,
    /// - `len` is the number of cells actually visited.
    ///
    /// `len <= 19 < 32`, so a `u32` is always wide enough.
    ///
    /// Cells beyond the line (off-board, or past `len`) sit at zero in
    /// both masks - pattern matchers that need an *empty* cell at an
    /// endpoint won't see them as empty, so the board edge is a wall.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Pack the row containing y = 9 from left to right, for Black.
    /// let (me, opp, len) = board.pack_line(0, 9, 1, 0, BOARD_SIZE as u32, Player::Black);
    /// ```
    pub fn pack_line(
        &self,
        x0: isize,
        y0: isize,
        dx: isize,
        dy: isize,
        max_len: u32,
        player: Player,
    ) -> (u32, u32, u32) {
        let me_board = &self.boards[player.idx()];
        let opp_board =
            &self.boards[player.opponent().idx()];

        let mut me = 0u32;
        let mut opp = 0u32;
        let mut len = 0u32;

        let mut x = x0;
        let mut y = y0;

        while len < max_len
            && x >= 0
            && y >= 0
            && x < BOARD_SIZE_I
            && y < BOARD_SIZE_I
        {
            let idx =
                (y as usize) * BOARD_SIZE
                + (x as usize);

            if me_board.get(idx) {
                me |= 1 << len;
            } else if opp_board.get(idx) {
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
    /// Each line is packed via [`Board::pack_line`]. Lines shorter than
    /// `min_len` are skipped (no useful pattern fits in fewer cells, and
    /// skipping cuts the diagonal count almost in half).
    ///
    /// # Coverage
    ///
    /// - All `BOARD_SIZE` rows and `BOARD_SIZE` columns.
    /// - Both diagonal families, starting from cells where the previous
    ///   cell is off-board.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut totals = PatternCounts::default();
    /// board.for_each_line(Player::Black, 5, |me, opp, len| {
    ///     totals.add(&count_patterns(me, opp, len));
    /// });
    /// ```
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

    /// Print the board to stdout in human-readable form.
    ///
    /// Black is shown as `X`, White as `O`, and empty cells as `.`. Used
    /// for debugging and the [`crate::play!`] macro.
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
                let pos = Pos::from_xy(x, y);
                let symbol = match self.cell_at(pos) {
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
