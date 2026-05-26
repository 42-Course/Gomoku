//! Data Transfer Objects that cross the FFI boundary.
//!
//! Each DTO derives both [`serde`] traits and [`tsify::Tsify`], so it
//! serializes through `serde-wasm-bindgen` *and* shows up as a real TS
//! `interface` in the generated `.d.ts`. The engine's internal types
//! (bitmaps, pattern counts, observer trees) never cross — only these.

use engine::game::{GameStatus, Player};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// One cell on the board for the visualizer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "lowercase")]
pub enum CellDTO {
    Empty,
    Black,
    White,
}

impl From<engine::game::Cell> for CellDTO {
    fn from(cell: engine::game::Cell) -> Self {
        match cell {
            None => CellDTO::Empty,
            Some(Player::Black) => CellDTO::Black,
            Some(Player::White) => CellDTO::White,
        }
    }
}

/// Whose turn it is, or who won, in a TS-friendly tagged form.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "lowercase", tag = "kind", content = "player")]
pub enum StatusDTO {
    Ongoing,
    Win(PlayerDTO),
    Draw,
}

impl From<GameStatus> for StatusDTO {
    fn from(s: GameStatus) -> Self {
        match s {
            GameStatus::Ongoing => StatusDTO::Ongoing,
            GameStatus::Win(p) => StatusDTO::Win(p.into()),
            GameStatus::Draw => StatusDTO::Draw,
        }
    }
}

/// Renamed copy of [`engine::game::Player`] so TS sees a stable shape
/// independent of the internal enum.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "lowercase")]
pub enum PlayerDTO {
    Black,
    White,
}

impl From<Player> for PlayerDTO {
    fn from(p: Player) -> Self {
        match p {
            Player::Black => PlayerDTO::Black,
            Player::White => PlayerDTO::White,
        }
    }
}

/// Move coordinate in board space.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct MoveDTO {
    pub x: u32,
    pub y: u32,
}

/// Result of a successful [`crate::GameHandle::play`].
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct PlayResultDTO {
    /// Stones removed by capture, in `(x, y)` form.
    pub captured: Vec<MoveDTO>,
    /// New game status after the move.
    pub status: StatusDTO,
    /// `(black_pairs, white_pairs)` capture totals after the move.
    pub captures: [u8; 2],
}

/// Output of [`crate::GameHandle::best_move`].
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct BestMoveDTO {
    /// Recommended move, or `None` if the engine couldn't find one
    /// (depth 0, terminal position, or no legal moves).
    pub r#move: Option<MoveDTO>,
    /// Score from the side-to-move's perspective.
    pub score: i32,
    /// Deepest fully completed iterative depth.
    pub depth_reached: u32,
    /// Total nodes (incl. leaves) the search visited.
    pub total_nodes: u64,
    /// Deepest ply explored overall.
    pub max_ply: u32,
}

/// Read-only snapshot of the game for rendering.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct GameStateDTO {
    /// Row-major `BOARD_SIZE * BOARD_SIZE` cells.
    pub board: Vec<CellDTO>,
    pub board_size: u32,
    pub status: StatusDTO,
    /// Whose turn it currently is (only meaningful when `status == Ongoing`).
    pub current_player: PlayerDTO,
    /// `(black_pairs, white_pairs)`.
    pub captures: [u8; 2],
    pub move_count: u32,
}
