/// Runtime configuration for search heuristics and limits.
pub struct SearchConfig {
    /// Enable Late Move Reductions during search.
    pub enable_lmr: bool,

    /// Maximum search time in milliseconds for iterative deepening.
    pub timeout_ms: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            enable_lmr: true,
            timeout_ms: 100,
        }
    }
}