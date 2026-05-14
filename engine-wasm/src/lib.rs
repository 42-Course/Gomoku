//! # `engine-wasm` — browser bindings for the Gomoku engine
//!
//! This crate is a thin `wasm-bindgen` bridge over [`engine`]. It exposes
//! a small, stateful API the visualizer can drive:
//!
//! ```text
//!   const handle = new GameHandle();
//!   handle.play(9, 9);
//!   const result = handle.bestMove(4);
//!   const view   = handle.snapshot();
//! ```
//!
//! All boundary types are plain DTOs ([`crate::dto`]); the engine's
//! internal types never cross the FFI.
//!
//! ## Build
//!
//! ```bash
//! cd engine-wasm
//! ./build.sh           # wasm-pack build --target web --release
//! # → engine-wasm/pkg/  (drop-in npm package)
//! ```

pub mod dto;
pub mod handle;

pub use handle::GameHandle;

/// Wire panics through to the JS console with a real stack trace.
///
/// `wasm-bindgen(start)` runs this once when the module loads. Without it,
/// a Rust panic in Wasm would surface as a silent `RuntimeError`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn __wasm_init() {
    console_error_panic_hook::set_once();
}
