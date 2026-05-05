//! Flat representation of the search tree for the visualizer.
//!
//! The engine's [`engine::ai::SearchNode`] is a recursive type. Marshalling
//! it across the FFI as a nested object pays per-level allocation cost
//! that adds up fast — a depth-4 search can have thousands of nodes.
//!
//! Instead we *flatten*: every visited node lives in a single `Vec` and
//! parent links are stored as indices. The root is at index 0 (when the
//! tree is non-empty). A child's index is always greater than its
//! parent's, so consumers can iterate in `nodes` order to do a top-down
//! traversal without recursion.

use crate::dto::{MoveDTO, PlayerDTO};
use engine::ai::SearchNode;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// One node in the flattened search tree.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct FlatNodeDTO {
    /// Index of this node's parent in the same tree, or `None` for the root.
    pub parent: Option<u32>,
    /// The move that led to this node (`None` at the root).
    pub r#move: Option<MoveDTO>,
    pub player_to_move: PlayerDTO,
    pub depth_remaining: u32,
    pub alpha_in: i32,
    pub beta_in: i32,
    pub score: i32,
    pub pruned: bool,
}

/// The flattened tree handed back by `bestMoveVerbose`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct SearchTreeDTO {
    pub nodes: Vec<FlatNodeDTO>,
}

/// Convert a recursive [`SearchNode`] into a flat tree.
///
/// Walks pre-order so a node's children always appear after it.
pub fn flatten(root: &SearchNode) -> SearchTreeDTO {
    let mut out = SearchTreeDTO::default();
    push(&mut out, root, None);
    out
}

fn push(tree: &mut SearchTreeDTO, node: &SearchNode, parent: Option<u32>) {
    let idx = tree.nodes.len() as u32;
    tree.nodes.push(FlatNodeDTO {
        parent,
        r#move: node.mv.map(|(x, y)| MoveDTO { x: x as u32, y: y as u32 }),
        player_to_move: node.player_to_move.into(),
        depth_remaining: node.depth_remaining,
        alpha_in: node.alpha_in,
        beta_in: node.beta_in,
        score: node.score,
        pruned: node.pruned,
    });
    for child in &node.children {
        push(tree, child, Some(idx));
    }
}
