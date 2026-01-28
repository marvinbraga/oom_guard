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
    pub cmdline: String,
    pub uid: u32,
    pub rss_kb: u64,
    pub oom_score: i32,
    pub strategy: KillStrategy,
    pub result: String,
}

impl KillInfo {
    /// Create a new KillInfo
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pid: i32,
        name: String,
        cmdline: String,
        uid: u32,
        rss_kb: u64,
        oom_score: i32,
        strategy: KillStrategy,
        result: &KillResult,
    ) -> Self {
        Self {
            pid,
            name,
            cmdline,
            uid,
            rss_kb,
            oom_score,
            strategy,
            result: result.description().to_string(),
        }
    }
}
