// OOM Guard - Memory monitor and process management library

pub mod config;
pub mod monitor;
pub mod killer;
pub mod daemon;
pub mod notify;

// Re-export commonly used types
pub use config::Config;
pub use monitor::{MemInfo, ProcessInfo};
