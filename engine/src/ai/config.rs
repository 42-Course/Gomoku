/// Runtime configuration for search heuristics and limits.
pub struct SearchConfig {
    /// Enable Late Move Reductions during search.
    pub enable_lmr: bool,

    /// Maximum search time in milliseconds for iterative deepening.
    pub timeout_ms: u64,

    /// Initial depth used for iterative deepening search.
    pub iterative_start_depth: u32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            enable_lmr: false,
            timeout_ms: 100,
            iterative_start_depth: 1,
        }
    }
}