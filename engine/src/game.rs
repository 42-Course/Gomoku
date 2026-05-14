//! Gomoku rules: turns, captures, win/draw detection, and the no-double-three
//! restriction.
//!
//! The [`Game`] type owns a [`Board`] plus the small amount of state Gomoku
//! needs around it: whose move it is (derived from `history.len()`), how
//! many capture pairs each player has, and a redo-able move history.
//!
//! ## Rules implemented
//!
//! - **Five in a row** wins (any direction). See [`Game::check_win`].
//! - **Captures**: flanking two opponent stones with your own stones along
//!   any of the eight directions removes them. Five capture pairs also
//!   wins. See [`Game::apply_captures`].
//! - **Double three forbidden**: a move that creates two simultaneous free
//!   threes is rejected. See [`Game::is_free_three`], which delegates the
//!   actual recognition to [`crate::patterns::has_free_three`].
//!
//! ## Move generation
//!
//! [`Game::generate_moves`] yields every empty cell within
//! [`MOVE_GEN_RADIUS`] of an existing stone (so the search doesn't waste
//! time on isolated points). The search calls this directly.

use crate::constants::BOARD_SIZE;
use crate::constants::BOARD_SIZE_I;
use crate::board::Board;
use crate::constants::CELL_COUNT;
use crate::zobrist::ZOBRIST;

/// Chebyshev radius used by [`Game::generate_moves`] to grow candidate
/// moves out from existing stones. `1` means "the 8 neighbours".
pub const MOVE_GEN_RADIUS: isize = 1;

/// Convenience wrapper around [`Game::play_move`] for tests.
///
/// Plays the given coordinate and prints the resulting board.
/// Evaluates to the same `Result` that
/// [`Game::play_move`] returns.
///
/// # Examples
///
/// ```ignore
/// use engine::play;
/// let mut game = Game::new();
/// play!(game, 9, 9).unwrap();
/// ```
#[macro_export]
macro_rules! play {
    ($game:expr, $x:expr, $y: expr) => {{
        let result = $game.play_move($x, $y);
        $game.print_board(true);
        result
    }};
}

/// Which side a stone belongs to. Black moves first (index `0`).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Player {
    /// First player. Conventionally drawn as `X`.
    Black,
    /// Second player. Conventionally drawn as `O`.
    White,
}

/// Optional stone at a board cell: `Some(player)` if occupied, `None` if empty.
pub type Cell = Option<Player>;

/// One of the four directions a Gomoku line can run in.
///
/// Captures and win checks iterate all four. The "anti-diagonal" runs from
/// upper-left to lower-right of the screen, i.e. `(dx, dy) = (1, -1)` in
/// the y-axis-down convention used throughout the engine.
#[derive(Copy, Clone, Debug)]
pub enum Direction {
    /// `(1, 0)` — left to right.
    Horizontal,
    /// `(0, 1)` — top to bottom.
    Vertical,
    /// `(1, 1)` — top-left to bottom-right.
    Diagonal,
    /// `(1, -1)` — bottom-left to top-right.
    AntiDiagonal,
}

impl Direction {
    #[inline]
    pub fn offset(self) -> isize {
        match self {
            Direction::Horizontal => 1,
            Direction::Vertical => BOARD_SIZE as isize,
            Direction::Diagonal => BOARD_SIZE as isize + 1,
            Direction::AntiDiagonal => BOARD_SIZE as isize - 1,
        }
    }

    #[inline]
    pub fn all() -> [Direction; 4] {
        [
            Direction::Horizontal,
            Direction::Vertical,
            Direction::Diagonal,
            Direction::AntiDiagonal,
        ]
    }

    /// The unit `(dx, dy)` step for this direction.
    #[inline]
    pub fn delta(self) -> (isize, isize) {
        match self {
            Direction::Horizontal => (1, 0),
            Direction::Vertical => (0, 1),
            Direction::Diagonal => (1, 1),
            Direction::AntiDiagonal => (1, -1),
        }
    }

    /// All four direction deltas, in a fixed order.
    ///
    /// Used by win and capture checks that need to scan every line through
    /// a given cell.
    #[inline]
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
    /// Index into `[BitBoard; 2]` arrays — Black is `0`, White is `1`.
    pub fn idx(self) -> usize {
        match self {
            Player::Black => 0,
            Player::White => 1,
        }
    }

    /// The other player.
    pub fn opponent(self) -> Player {
        match self {
            Player::Black => Player::White,
            Player::White => Player::Black,
        }
    }
}

/// A single move in the game history.
///
/// `captured` records every opponent stone removed by this move so that
/// [`Game::undo_move`] can put them back.
#[allow(dead_code)]
#[derive(Clone)]
pub struct Move {
    /// Position where the stone was placed.
    pub pos: Pos,
    /// Coordinates of opponent stones captured by this move.
    pub captured: Vec<Pos>,
}

/// The complete state of a Gomoku game.
///
/// All public fields are mutated in place by [`Game::play_move`] and
/// [`Game::undo_move`]; nothing is reference-counted or cloned. The search
/// uses make/unmake against a single `Game` rather than copying.
#[derive(Clone)]
pub struct Game {
    /// Stone placements.
    pub board: Board,
    /// `Ongoing`, `Win(player)`, or `Draw`.
    pub status: GameStatus,
    /// Played moves in order. Length determines whose move it is.
    pub history: Vec<Move>,
    /// Capture pair counts as `(black, white)`. Five pairs wins.
    pub captures: (u8, u8),
    /// Zobrist hash of the current game state (board, captures, side-to-move), updated incrementally for fast position lookup in search.
    pub hash: u64,
}

/// Whether the game is still being played, has been won, or is drawn.
#[allow(dead_code)]
#[derive(Clone, PartialEq)]
pub enum GameStatus {
    /// More moves are legal.
    Ongoing,
    /// `player` has won, either by five-in-a-row or by five capture pairs.
    Win(Player),
    /// Board is full with no winner.
    Draw,
}

/// Compact board position as a linear index (y * 19 + x).
/// Used for fast indexing and hashing without passing (x, y) pairs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Pos(pub usize);

impl Pos {
    /// Returns the underlying index.
    #[inline]
    pub fn idx(self) -> usize {
        self.0
    }

    /// Converts the index to (x, y).
    #[allow(dead_code)]
    #[inline]
    pub fn to_xy(self) -> (usize, usize) {
        (self.0 % BOARD_SIZE, self.0 / BOARD_SIZE)
    }

    #[inline]
    pub fn from_xy(x: usize, y: usize) -> Self {
        Self(y * BOARD_SIZE + x)
    }

    #[inline]
    pub fn offset(self, dir: Direction, step: isize) -> Option<Pos> {
        let next = self.idx() as isize + dir.offset() * step;

        if !(0..(CELL_COUNT) as isize).contains(&next) {
            return None;
        }

        let next = Pos(next as usize);

        if !self.is_valid_step(next, dir) {
            return None;
        }

        Some(next)
    }

    fn is_valid_step(self, next: Pos, dir: Direction) -> bool {
        let x1 = self.idx() % BOARD_SIZE;
        let y1 = self.idx() / BOARD_SIZE;

        let x2 = next.idx() % BOARD_SIZE;
        let y2 = next.idx() / BOARD_SIZE;

        match dir {
            Direction::Horizontal => y1 == y2,
            Direction::Vertical => x1 == x2,
            Direction::Diagonal => {
                x1.abs_diff(x2) == y1.abs_diff(y2)
            }
            Direction::AntiDiagonal => {
                x1.abs_diff(x2) == y1.abs_diff(y2)
            }
        }
    }
}

impl std::fmt::Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let x = self.0 % BOARD_SIZE;
        let y = self.0 / BOARD_SIZE;

        write!(f, "[{}, {}]", x, y)
    }
}

#[allow(dead_code)]
impl Game {
    /// Start a new game with an empty board, Black to move.
    ///
    /// Initializes:
    /// - capture counts to 0–0
    /// - side-to-move to Black
    /// - Zobrist hash consistent with the initial state
    pub fn new() -> Self {
        let mut hash = 0u64;

        // captures start at 0–0
        hash ^= ZOBRIST.capture[Player::Black.idx()][0];
        hash ^= ZOBRIST.capture[Player::White.idx()][0];

        // Black to move
        hash ^= ZOBRIST.side[Player::Black.idx()];
        Self {
            board: Board::new(),
            status: GameStatus::Ongoing,
            history: Vec::new(), // The index of the history represents the move number, starting from 0 (player)
            captures: (0, 0),
            hash
        }
    }

    /// Toggles hash for placing/removing a stone at `pos` for `player`.
    #[inline]
    fn hash_place(&mut self, pos: Pos, player: Player) {
        self.hash ^= ZOBRIST.board[pos.idx()][player as usize];
    }

    /// Same as `hash_place` (XOR is symmetric).
    #[inline]
    fn hash_remove(&mut self, pos: Pos, player: Player) {
        self.hash ^= ZOBRIST.board[pos.idx()][player as usize];
    }

    /// Toggles hash for a given capture count of `player`.
    #[inline]
    fn hash_capture(&mut self, player: Player, capture: u8) {
        self.hash ^= ZOBRIST.capture[player as usize][capture as usize];
    }

    /// Toggles side-to-move in the hash.
    #[inline]
    fn hash_side(&mut self, player: Player) {
        self.hash ^= ZOBRIST.side[player as usize];
    }

    /// Whose move is the `move_index`-th move (zero-indexed).
    ///
    /// Even indices are Black, odd indices are White.
    #[inline]
    pub fn player_at_move(&self, move_index: usize) -> Player {
        if move_index.is_multiple_of(2) {
            Player::Black
        } else {
            Player::White
        }
    }

    /// Whose turn it currently is. Derived from `history.len()`.
    #[inline]
    pub fn current_player(&self) -> Player {
        self.player_at_move(self.history.len())
    }

    /// Number of capture pairs taken by `player`.
    ///
    /// Reaching 5 captured pairs wins the game.
    #[inline]
    pub fn capture_count(&self, player: Player) -> u8 {
        match player {
            Player::Black => self.captures.0,
            Player::White => self.captures.1,
        }
    }

    /// Place the current player's stone at `(x, y)`.
    ///
    /// This is a public coordinate-based wrapper around [`Game::play_pos`]
    /// that validates bounds before converting to [`Pos`].
    ///
    /// # Errors
    ///
    /// Returns `"Out of bounds"` if `(x, y)` lies outside the board,
    /// or any error propagated from [`Game::play_pos`].
    pub fn play_move(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if x >= BOARD_SIZE || y >= BOARD_SIZE {
            return Err("Out of bounds");
        }
        let pos = Pos::from_xy(x, y);
        self.play_pos(pos)
    }

    /// Place the current player's stone at `Pos` and update the game state.
    /// Zobrist hash of the current game state.
    #[inline]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Place the current player's stone at `(x, y)` and update the game state.
    ///
    /// Applies captures, checks for the double-three rule, and updates
    /// [`Game::status`] if the move ends the game.
    ///
    /// # Hashing
    ///
    /// This function updates the Zobrist hash incrementally:
    /// - stone placement
    /// - captured stones removal
    /// - capture count changes
    /// - side-to-move switch
    ///
    /// The order of these updates must be mirrored exactly in [`Game::undo_move`].
    ///
    /// # Errors
    ///
    /// - `"Out of bounds"` or `"Cell already occupied"` from the underlying
    ///   [`Board::empty_check`].
    /// - `"Game already finished"` when called after a win or draw.
    /// - `"Double three is forbidden"` when the move would create two
    ///   simultaneous free threes.
    pub fn play_pos(&mut self, pos: Pos) -> Result<(), &'static str> {
        self.board.empty_check(pos)?;
        if !matches!(self.status, GameStatus::Ongoing) {
            return Err("Game already finished");
        }
        let player= self.current_player();

        self.board.place_stone(pos, player);

        if self.count_free_threes(pos) >= 2 {
            self.board.remove_stone(pos, player);
            return Err("Double three is forbidden");
        }

        // hash when the move is confirmed
        self.hash_place(pos, player);
        self.hash_side(player);
        self.hash_side(player.opponent());

        // apply captures
        let captured = self.apply_captures(pos);

        for &capture in &captured {
            self.hash_remove(capture, player.opponent());
        }

        // update capture count
        let captured_pairs = captured.len() / 2;
        match player {
            Player::Black => {
                // hash the previous count out, and the new one in
                self.hash_capture(player, self.captures.0);
                self.captures.0 += captured_pairs as u8;
                self.hash_capture(player, self.captures.0);
            },
            Player::White => {
                // hash the previous count out, and the new one in
                self.hash_capture(player, self.captures.1);
                self.captures.1 += captured_pairs as u8;
                self.hash_capture(player, self.captures.1);
            },
        }

        self.history.push(Move {
            pos,
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
                if self.check_win(pos) {
                    self.status = GameStatus::Win(player)
                } else if self.board.is_full() {
                    self.status = GameStatus::Draw;
                }
            }

        }

        Ok(())
    }

    /// Reverse the most recent move, restoring captured stones and capture
    /// counts. Resets [`Game::status`] to [`GameStatus::Ongoing`].
    ///
    /// # Hashing
    ///
    /// This function is the exact inverse of [`Game::play_move`].
    /// All Zobrist hash updates are reverted in reverse order, ensuring:
    ///
    /// ```text
    /// play_move → undo_move ⇒ identical hash and state
    /// ```
    ///
    /// # Errors
    ///
    /// - `"No moves to undo"` if the history is empty.
    pub fn undo_move(&mut self) -> Result<(), String> {
        let last = self.history.pop().ok_or("No moves to undo")?;
        let last_player = self.current_player();
        let last_opponent = last_player.opponent();

        self.board.remove_stone(last.pos, last_player);

        // undo hash
        self.hash_remove(last.pos, last_player);

        self.hash_side(last_opponent);
        self.hash_side(last_player);

        for &captured_pos in &last.captured {
            self.board.place_stone(captured_pos, last_opponent);

            self.hash_place(captured_pos, last_opponent);
        }

        let pairs = last.captured.len() / 2;

        match last_player {
            Player::Black => {
                self.hash_capture(last_player, self.captures.0);
                self.captures.0 -= pairs as u8;
                self.hash_capture(last_player, self.captures.0);
            }

            Player::White => {
                self.hash_capture(last_player, self.captures.1);
                self.captures.1 -= pairs as u8;
                self.hash_capture(last_player, self.captures.1);
            }
        }

        self.status = GameStatus::Ongoing;

        Ok(())
    }

    /// Print the board (and optionally a header showing the move number
    /// and the last move played). Used by [`crate::play!`] and tests.
    pub fn print_board(&self, print_turn: bool) {
        if print_turn {
            print!("\n-------\nTurn {:2}", self.history.len());

            if let Some(last_move) = self.history.last() {
                print!(" | Move {}", last_move.pos);
            }

            println!();
        }
        self.board.print_board();
    }

    fn count_direction(
        &self,
        start: Pos,
        dir: Direction,
        sign: isize,
        player: Player,
    ) -> usize {
        let mut count = 0;
        let mut current = start;

        while let Some(next) = current.offset(dir, sign) {
            if self.board.cell_at(next) != Some(player) {
                break;
            }

            count += 1;
            current = next;
        }

        count
    }

    /// Whether the stone at `(x, y)` participates in a 5+ run along any
    /// of the four directions.
    ///
    /// Packs the 9-cell window centered on the stone into bitmasks and
    /// hands the detection off to [`crate::patterns::count_patterns`].
    /// Off-board cells fall outside the packed window, so the board edge
    /// acts as a wall — same convention used by [`Game::is_free_three`].
    pub fn check_win(&self, pos: Pos) -> bool {
        let player = match self.board.cell_at(pos) {
            Some(p) => p,
            None => return false,
        };

        for dir in Direction::all() {
            let mut count = 1;

            count += self.count_direction(pos, dir, 1, player);
            count += self.count_direction(pos, dir, -1, player);

            if count >= 5 {
                return true;
            }
        }

        false
    }

    /// Resolve captures triggered by a stone just played at `(x, y)`.
    ///
    /// For each of the eight directions, if the pattern is
    /// `played, opp, opp, played`, the two opponent stones are removed.
    /// Returns the coordinates of the removed stones so the move can be
    /// undone later.
    fn apply_captures(&mut self, pos: Pos) -> Vec<Pos> {
        let mut captured = Vec::new();

        let player = self.current_player();
        let opponent = player.opponent();

        for dir in Direction::all() {
            for step in [-1, 1] {
                let p1 = pos.offset(dir, step);
                let p2 = pos.offset(dir, step * 2);
                let p3 = pos.offset(dir, step * 3);

                let (Some(p1), Some(p2), Some(p3)) = (p1, p2, p3)
                else {
                    continue;
                };

                if self.board.cell_at(p1) == Some(opponent)
                    && self.board.cell_at(p2) == Some(opponent)
                    && self.board.cell_at(p3) == Some(player)
                {
                    self.board.remove_stone(p1, opponent);
                    self.board.remove_stone(p2, opponent);

                    captured.push(p1);
                    captured.push(p2);
                }
            }
        }

        captured
    }

    /// How many of the four directions show a free three through `(x, y)`.
    /// Used to enforce the no-double-three rule.
    fn count_free_threes(&self, pos: Pos) -> u32 {
        let mut count = 0;

        for (dx, dy) in Direction::all_directions() {
            if self.is_free_three(pos, dx, dy) {
                count += 1;
            }
        }

        count
    }

    /// Does the just-placed stone at `(x, y)` create a free-three along
    /// `(dx, dy)`?
    ///
    /// Packs the 9-cell window centered on the stone into bitmasks and
    /// hands the detection off to [`crate::patterns::has_free_three`].
    /// Off-board cells are dropped from the window — they act as walls,
    /// so a 6-cell pattern that requires an empty endpoint won't match
    /// against the board edge.
    fn is_free_three(&self, pos: Pos, dx: isize, dy: isize) -> bool {
        let player = match self.board.cell_at(pos) {
            Some(p) => p,
            None => return false,
        };

        let mut me = 0u32;
        let mut opp = 0u32;
        let mut len = 0u32;
        let (x, y) = pos.to_xy();
        for i in -4..=4 {
            let cx = x as isize + i * dx;
            let cy = y as isize + i * dy;
            if cx < 0 || cy < 0 || cx >= BOARD_SIZE_I || cy >= BOARD_SIZE_I {
                continue;
            }
            let check_pos = Pos::from_xy(cx as usize, cy as usize);
            match self.board.cell_at(check_pos) {
                Some(p) if p == player => me |= 1 << len,
                Some(_) => opp |= 1 << len,
                None => {}
            }
            len += 1;
        }

        crate::patterns::has_free_three(me, opp, len)
    }

    /// Whether the current player could legally play at `(x, y)` *right now*.
    ///
    /// Probes by temporarily placing the stone, counting the free threes
    /// it creates, and undoing the placement. Used as the legality filter
    /// in [`Game::generate_moves`].
    fn is_valid_move(&mut self, pos: Pos) -> bool {
        if !self.board.is_empty(pos) {
            return false;
        }

        let player = self.current_player();

        self.board.place_stone(pos, player);

        let valid = self.count_free_threes(pos) < 2;

        self.board.remove_stone(pos, player);

        valid
    }

    /// Candidate moves the search should consider, in unspecified order.
    ///
    /// On the empty board, returns the single move `(9, 9)` to seed the
    /// game at the center. Otherwise returns every empty cell within
    /// [`MOVE_GEN_RADIUS`] of an existing stone that passes
    /// [`Game::is_valid_move`] (i.e. doesn't create a double three).
    pub fn generate_moves(&mut self) -> Vec<Pos> {
        use std::collections::HashSet;

        if self.history.is_empty() {
            return vec![Pos::from_xy(9, 9)];
        }

        let mut candidates = HashSet::new();

        for idx in 0..CELL_COUNT {
            let pos = Pos(idx);

            if self.board.is_empty(pos) {
                continue;
            }

            let (x, y) = pos.to_xy();

            let x = x as isize;
            let y = y as isize;

            for dy in -MOVE_GEN_RADIUS..=MOVE_GEN_RADIUS {
                for dx in -MOVE_GEN_RADIUS..=MOVE_GEN_RADIUS {
                    if dx == 0 && dy == 0 {
                        continue;
                    }

                    let nx = x + dx;
                    let ny = y + dy;

                    if nx < 0
                        || ny < 0
                        || nx >= BOARD_SIZE_I
                        || ny >= BOARD_SIZE_I
                    {
                        continue;
                    }

                    let candidate =
                        Pos::from_xy(nx as usize, ny as usize);

                    if !self.board.is_empty(candidate) {
                        continue;
                    }

                    candidates.insert(candidate);
                }
            }
        }

        let mut moves = Vec::new();

        for pos in candidates {
            if self.is_valid_move(pos) {
                moves.push(pos);
            }
        }

        moves.sort_unstable_by_key(|p| p.idx());
        moves
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

    assert_eq!(game.board.cell_at_xy(1, 0), None);
    assert_eq!(game.board.cell_at_xy(2, 0), None);
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

    assert_eq!(game.board.cell_at_xy(1, 0), None);
    assert_eq!(game.board.cell_at_xy(2, 0), None);
    assert_eq!(game.board.cell_at_xy(4, 0), None);
    assert_eq!(game.board.cell_at_xy(5, 0), None);
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

#[test]
fn test_generate_moves_empty_board() {
    let mut game = Game::new();

    let moves = game.generate_moves();

    assert_eq!(moves, vec![Pos::from_xy(9, 9)]);
}

#[test]
fn test_generate_moves_near_single_stone() {
    let mut game = Game::new();

    play!(game, 9, 9).unwrap();

    let moves = game.generate_moves();

    assert!(moves.contains(&Pos::from_xy(8, 8)));
    assert!(moves.contains(&Pos::from_xy(9, 8)));
    assert!(moves.contains(&Pos::from_xy(10, 10)));
    assert!(!moves.contains(&Pos::from_xy(0, 0)));
}

#[test]
fn test_generate_moves_no_duplicates() {
    let mut game = Game::new();

    play!(game, 9, 9).unwrap();
    play!(game, 10, 9).unwrap();

    let moves = game.generate_moves();

    let len = moves.len();
    let unique: std::collections::HashSet<_> = moves.iter().cloned().collect();

    assert_eq!(len, unique.len());
}

#[test]
fn test_undo_simple_move() {
    let mut game = Game::new();
    play!(game, 9, 9).unwrap();
    game.undo_move().unwrap();
    game.print_board(true);

    assert_eq!(game.board.cell_at_xy(9, 9), None);
    assert_eq!(game.history.len(), 0);
}

#[test]
fn test_undo_capture() {
    let mut game = Game::new();

    play!(game, 0, 0).unwrap(); // X
    play!(game, 1, 0).unwrap(); // O
    play!(game, 4, 0).unwrap(); // X
    play!(game, 2, 0).unwrap(); // O

    play!(game, 3, 0).unwrap(); // X capture

    game.undo_move().unwrap();
    game.print_board(true);

    // stones should be restored
    assert_eq!(game.board.cell_at_xy(1, 0), Some(Player::White));
    assert_eq!(game.board.cell_at_xy(2, 0), Some(Player::White));
}

#[test]
fn test_undo_simple_move_hash() {
    let mut game = Game::new();

    let h = game.hash;

    play!(game, 9, 9).unwrap();
    game.undo_move().unwrap();

    assert_eq!(game.hash, h);
    assert_eq!(game.board.cell_at_xy(9, 9), None);
    assert_eq!(game.history.len(), 0);
}

#[test]
fn test_undo_capture_hash() {
    let mut game = Game::new();

    play!(game, 0, 0).unwrap(); // X
    play!(game, 1, 0).unwrap(); // O
    play!(game, 4, 0).unwrap(); // X
    play!(game, 2, 0).unwrap(); // O

    let h = game.hash;

    play!(game, 3, 0).unwrap(); // X capture
    game.undo_move().unwrap();

    assert_eq!(game.hash, h);

    // stones restored
    assert_eq!(game.board.cell_at_xy(1, 0), Some(Player::White));
    assert_eq!(game.board.cell_at_xy(2, 0), Some(Player::White));
}

#[test]
fn test_undo_sequence_hash_stepwise() {
    let mut game = Game::new();

    let mut hashes = Vec::new();

    let moves = [(9, 9), (10, 9), (9, 10), (10, 10)];

    // store hash BEFORE each move
    for &(x, y) in &moves {
        let prev = game.hash;
        hashes.push(prev);

        play!(game, x, y).unwrap();

        // ensure hash actually changed
        assert_ne!(game.hash, prev);
    }

    // undo and compare step-by-step
    for expected_hash in hashes.iter().rev() {
        let before_undo = game.hash;

        game.undo_move().unwrap();

        // ensure undo also changes hash
        assert_ne!(game.hash, before_undo);

        // ensure correctness
        assert_eq!(game.hash, *expected_hash);
    }

    assert_eq!(game.history.len(), 0);
}