// Process killer module

mod selector;
pub mod signals;

pub use selector::ProcessSelector;
pub use signals::{kill_process, KillResult, KillStrategy};

/// Information about a killed process
#[derive(Debug, Clone)]
pub struct KillInfo {
    pub pid: i32,
    pub name: String,
    pub rss_kb: u64,
    pub strategy: KillStrategy,
    pub result: String,
}

impl KillInfo {
    /// Create a new KillInfo
    pub fn new(
        pid: i32,
        name: String,
        rss_kb: u64,
        strategy: KillStrategy,
        result: &KillResult,
    ) -> Self {
        Self {
            pid,
            name,
            rss_kb,
            strategy,
            result: result.description().to_string(),
        }
    }
}
