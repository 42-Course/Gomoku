//! Stateful handle over [`engine::game::Game`] exposed to JS.
//!
//! The `Game` lives entirely on the Rust side; TS holds an opaque pointer
//! and drives it through the methods below. Every method that returns a
//! DTO converts it to `JsValue` via `serde-wasm-bindgen`, which is
//! significantly cheaper than going through a JSON string.
//!
//! ## Memory
//!
//! `wasm-bindgen` reference-counts handles on the JS side. The visualizer
//! must call `handle.free()` when it's done with a game, or hold a single
//! handle for the lifetime of the page.

use std::result;

use engine::ai;
use engine::constants::BOARD_SIZE;
use engine::game::Game;
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::prelude::*;

use crate::dto::{
    BestMoveDTO, CellDTO, GameStateDTO, MoveDTO, PlayResultDTO, PlayerDTO,
};

/// Default transposition-table size passed to `ai::best_move`, in powers of
/// two (so `2^20` entries). Picked to fit comfortably in a browser tab and
/// to match the size the engine's own tests use.
const TT_SIZE_POWER: usize = 20;

/// A live Gomoku game, mutated in place by [`GameHandle::play`].
#[wasm_bindgen]
pub struct GameHandle {
    game: Game,
}

/// `serde-wasm-bindgen` serializer that emits real JS objects rather than
/// `Map` instances — friendlier for the TS consumer.
fn js_serializer() -> Serializer {
    Serializer::new().serialize_maps_as_objects(true)
}

#[wasm_bindgen]
impl GameHandle {
    /// Construct an empty game (Black to move).
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { game: Game::new() }
    }

    /// Play a move for the current side and apply captures.
    ///
    /// On success returns a [`PlayResultDTO`] describing what changed
    /// (captured stones, new status, capture totals). On failure the
    /// rule error is surfaced as a JS exception.
    pub fn play(&mut self, x: u32, y: u32) -> Result<JsValue, JsError> {
        let before = self.game.history.len();

        self.game
            .play_move(x as usize, y as usize)
            .map_err(JsError::new)?;

        // Read back what the engine recorded for this move.
        let last = self
            .game
            .history
            .get(before)
            .expect("play_move succeeded so a move was pushed");
        let captured: Vec<MoveDTO> = last
            .captured
            .iter()
            .map(|pos| {
                let (x, y) = pos.to_xy();
                MoveDTO { x: x as u32, y: y as u32 }
            })
            .collect();

        let dto = PlayResultDTO {
            captured,
            status: self.game.status.clone().into(),
            captures: [self.game.captures.0, self.game.captures.1],
        };
        dto.serialize(&js_serializer()).map_err(into_js_error)
    }

    /// Undo the most recently played move, restoring captured stones.
    pub fn undo(&mut self) -> Result<(), JsError> {
        self.game.undo_move().map_err(|e| JsError::new(&e))
    }

    /// Run alpha-beta to `depth` plies and return the recommendation.
    ///
    /// The game state is unchanged on return. A fresh transposition table
    /// is allocated per call — the engine's TT lifetime is one search.
    #[wasm_bindgen(js_name = bestMove)]
    pub fn best_move(&mut self, depth: u32) -> Result<JsValue, JsError> {
        let result = ai::best_move(&mut self.game, depth, TT_SIZE_POWER);
        let dto = BestMoveDTO {
            r#move: result.best_move.map(|pos| {
                let (x, y) = pos.to_xy();
                MoveDTO { x: x as u32, y: y as u32 }
            }),
            score: result.score,
            depth_reached: result.depth_reached,
            total_nodes: result.total_nodes,
            max_ply: result.max_ply
        };
        dto.serialize(&js_serializer()).map_err(into_js_error)
    }

    /// Read-only snapshot for rendering.
    pub fn snapshot(&self) -> Result<JsValue, JsError> {
        let n = BOARD_SIZE;
        let mut board = Vec::with_capacity(n * n);
        for y in 0..n {
            for x in 0..n {
                board.push(CellDTO::from(self.game.board.cell_at_xy(x, y)));
            }
        }
        let dto = GameStateDTO {
            board,
            board_size: n as u32,
            status: self.game.status.clone().into(),
            current_player: PlayerDTO::from(self.game.current_player()),
            captures: [self.game.captures.0, self.game.captures.1],
            move_count: self.game.history.len() as u32,
        };
        dto.serialize(&js_serializer()).map_err(into_js_error)
    }
}

impl Default for GameHandle {
    fn default() -> Self {
        Self::new()
    }
}

fn into_js_error<E: std::fmt::Display>(e: E) -> JsError {
    JsError::new(&e.to_string())
}
