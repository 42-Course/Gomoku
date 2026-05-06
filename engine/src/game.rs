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
    ($game:expr, $x:expr, $y:expr) => {{
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
    /// The unit `(dx, dy)` step for this direction.
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
    /// Column where the stone was placed.
    pub x: usize,
    /// Row where the stone was placed.
    pub y: usize,
    /// Coordinates of opponent stones captured by this move.
    pub captured: Vec<(usize, usize)>,
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
    hash: u64,
}

/// Whether the game is still being played, has been won, or is drawn.
#[allow(dead_code)]
#[derive(Clone)]
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
#[derive(Copy, Clone)]
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
    pub fn player_at_move(&self, move_index: usize) -> Player {
        if move_index.is_multiple_of(2) {
            Player::Black
        } else {
            Player::White
        }
    }

    /// Whose turn it currently is. Derived from `history.len()`.
    pub fn current_player(&self) -> Player {
        self.player_at_move(self.history.len())
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
    pub fn play_move(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        self.board.empty_check(x, y)?;
        if !matches!(self.status, GameStatus::Ongoing) {
            return Err("Game already finished");
        }
        let pos = Pos(y * BOARD_SIZE + x);
        let player = self.current_player();

        self.board.place_stone(x, y, player);

        if self.count_free_threes(x, y) >= 2 {
            self.board.remove_stone(x, y, player);
            return Err("Double three is forbidden");
        }

        // hash when the move is confirmed
        self.hash_place(pos, player);
        self.hash_side(player);
        self.hash_side(player.opponent());

        // apply captures
        let captured = self.apply_captures(x, y);

        for &(cx, cy) in &captured {
            let pos = Pos(cy * BOARD_SIZE + cx);
            self.hash_remove(pos, player.opponent());
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
        let last_opponent = self.current_player().opponent();

        let pos = Pos(last.y * BOARD_SIZE + last.x);
        self.board.remove_stone(last.x, last.y, last_player);

        // undo hash
        self.hash_remove(pos, last_player);
        self.hash_side(last_opponent);
        self.hash_side(last_player);

        for &(cx, cy) in &last.captured {
            self.board.place_stone(cx, cy, last_opponent);
            let pos = Pos(cy * BOARD_SIZE + cx);
            self.hash_place(pos, last_opponent);
        }

        let pairs = last.captured.len() / 2;

        match last_player {
            Player::Black => {
                // hash the previous count out, and the new one in
                self.hash_capture(last_player, self.captures.0);
                self.captures.0 -= pairs as u8;
                self.hash_capture(last_player, self.captures.0);
            },
            Player::White => {
                // hash the previous count out, and the new one in
                self.hash_capture(last_player, self.captures.1);
                self.captures.1 -= pairs as u8;
                self.hash_capture(last_player, self.captures.1);
            },
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
                print!(" | Move {}", last_move);
            }

            println!();
        }
        self.board.print_board();
    }

    /// Whether the stone at `(x, y)` participates in a 5+ run along any
    /// of the four directions.
    ///
    /// Packs the 9-cell window centered on the stone into bitmasks and
    /// hands the detection off to [`crate::patterns::count_patterns`].
    /// Off-board cells fall outside the packed window, so the board edge
    /// acts as a wall — same convention used by [`Game::is_free_three`].
    pub fn check_win(&self, x: usize, y: usize) -> bool {
        let player = match self.board.cell_at(x, y) {
            Some(p) => p,
            None => return false,
        };

        for (dx, dy) in Direction::all_directions() {
            // Walk back up to 4 cells along (-dx, -dy) so a 5-run that
            // ends at (x, y) still fits in the packed line.
            let mut x0 = x as isize;
            let mut y0 = y as isize;
            for _ in 0..4 {
                let nx = x0 - dx;
                let ny = y0 - dy;
                if !(0..BOARD_SIZE_I).contains(&nx) || !(0..BOARD_SIZE_I).contains(&ny) {
                    break;
                }
                x0 = nx;
                y0 = ny;
            }

            let (me, opp, len) = self.board.pack_line(x0, y0, dx, dy, 9, player);
            if crate::patterns::count_patterns(me, opp, len).fives > 0 {
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

                if x3 < 0 || y3 < 0 || x3 >= BOARD_SIZE_I || y3 >= BOARD_SIZE_I {
                    continue;
                }

                let (x1, y1) = (x1 as usize, y1 as usize);
                let (x2, y2) = (x2 as usize, y2 as usize);
                let (x3, y3) = (x3 as usize, y3 as usize);

                if self.board.cell_at(x1, y1) == Some(opponent)
                    && self.board.cell_at(x2, y2) == Some(opponent)
                    && self.board.cell_at(x3, y3) == Some(player)
                {
                    self.board.remove_stone(x1, y1, opponent);
                    self.board.remove_stone(x2, y2, opponent);

                    captured.push((x1, y1));
                    captured.push((x2, y2));
                }
            }
        }
        captured
    }

    /// How many of the four directions show a free three through `(x, y)`.
    /// Used to enforce the no-double-three rule.
    fn count_free_threes(&self, x: usize, y: usize) -> u32 {
        let mut count = 0;

        for (dx, dy) in Direction::all_directions() {
            if self.is_free_three(x, y, dx, dy) {
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
    fn is_free_three(&self, x: usize, y: usize, dx: isize, dy: isize) -> bool {
        let player = match self.board.cell_at(x, y) {
            Some(p) => p,
            None => return false,
        };

        let mut me = 0u32;
        let mut opp = 0u32;
        let mut len = 0u32;
        for i in -4..=4 {
            let cx = x as isize + i * dx;
            let cy = y as isize + i * dy;
            if cx < 0 || cy < 0 || cx >= BOARD_SIZE_I || cy >= BOARD_SIZE_I {
                continue;
            }
            match self.board.cell_at(cx as usize, cy as usize) {
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
    fn is_valid_move(&mut self, x: usize, y:usize) -> bool {
        if !self.board.is_empty(x, y) { return false; }

        let player = self.current_player();
        self.board.place_stone(x, y, player);

        let valid = self.count_free_threes(x, y) < 2;
        self.board.remove_stone(x, y, player);
        valid
    }

    /// Candidate moves the search should consider, in unspecified order.
    ///
    /// On the empty board, returns the single move `(9, 9)` to seed the
    /// game at the center. Otherwise returns every empty cell within
    /// [`MOVE_GEN_RADIUS`] of an existing stone that passes
    /// [`Game::is_valid_move`] (i.e. doesn't create a double three).
    pub fn generate_moves(&mut self) -> Vec<(usize, usize)> {
        use std::collections::HashSet;

        if self.history.is_empty() {
            return vec![(9, 9)];
        }

        let mut candidates = HashSet::new();

        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                if self.board.is_empty(x, y) {
                    continue;
                }

                for dy in -MOVE_GEN_RADIUS..=MOVE_GEN_RADIUS {
                    for dx in -MOVE_GEN_RADIUS..=MOVE_GEN_RADIUS {
                        if dx == 0 && dy == 0 {
                            continue;
                        }

                        let nx = x as isize + dx;
                        let ny = y as isize + dy;

                        if nx < 0 || ny < 0 || nx >= BOARD_SIZE_I || ny >= BOARD_SIZE_I {
                            continue;
                        }

                        let nx = nx as usize;
                        let ny = ny as usize;

                        if !self.board.is_empty(nx, ny) {
                            continue;
                        }

                        candidates.insert((nx, ny));
                    }
                }
            }
        }
        let mut moves = Vec::new();

        for (x, y) in candidates {
            if self.is_valid_move(x, y) {
                moves.push((x, y));
            }
        }
        moves.sort_unstable();
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

    assert_eq!(game.board.cell_at(1, 0), None);
    assert_eq!(game.board.cell_at(2, 0), None);
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

    assert_eq!(game.board.cell_at(1, 0), None);
    assert_eq!(game.board.cell_at(2, 0), None);
    assert_eq!(game.board.cell_at(4, 0), None);
    assert_eq!(game.board.cell_at(5, 0), None);
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

    assert_eq!(moves, vec![(9, 9)]);
}

#[test]
fn test_generate_moves_near_single_stone() {
    let mut game = Game::new();

    play!(game, 9, 9).unwrap();

    let moves = game.generate_moves();

    assert!(moves.contains(&(8, 8)));
    assert!(moves.contains(&(9, 8)));
    assert!(moves.contains(&(10, 10)));
    assert!(!moves.contains(&(0, 0)));
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

    assert_eq!(game.board.cell_at(9, 9), None);
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
    assert_eq!(game.board.cell_at(1, 0), Some(Player::White));
    assert_eq!(game.board.cell_at(2, 0), Some(Player::White));
}

#[test]
fn test_undo_simple_move_hash() {
    let mut game = Game::new();

    let h = game.hash;

    play!(game, 9, 9).unwrap();
    game.undo_move().unwrap();

    assert_eq!(game.hash, h);
    assert_eq!(game.board.cell_at(9, 9), None);
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
    assert_eq!(game.board.cell_at(1, 0), Some(Player::White));
    assert_eq!(game.board.cell_at(2, 0), Some(Player::White));
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