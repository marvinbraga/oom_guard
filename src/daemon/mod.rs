// Daemon module - main monitoring loop and service

mod service;

pub use service::DaemonService;

use crate::config::Config;
use anyhow::Result;

/// Run the OOM Guard daemon with the given configuration
pub fn run(config: Config) -> Result<()> {
    // Initialize logger if not already initialized
    if env_logger::try_init().is_err() {
        log::warn!("Logger already initialized");
    }

    // Set log level based on configuration
    let log_level = if config.debug {
        "debug"
    } else {
        "info"
    };

    std::env::set_var("RUST_LOG", log_level);

    // Create and run the daemon service
    let mut service = DaemonService::new(config);
    service.run()
}
