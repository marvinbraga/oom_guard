// OOM Guard - Main entry point

use nix::sys::mman::{mlockall, MlockAllFlags};
use oom_guard::config::{Args, Config};
use oom_guard::daemon;
use std::process;

fn main() {
    // Parse command-line arguments
    let args = Args::parse_args();

    // Initialize logging based on debug flag
    let log_level = if args.debug {
        "debug"
    } else {
        "info"
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .format_timestamp_secs()
        .init();

    // Lock all current and future memory pages to prevent swapping
    // This ensures the daemon stays responsive even under memory pressure
    match mlockall(MlockAllFlags::MCL_CURRENT | MlockAllFlags::MCL_FUTURE) {
        Ok(_) => log::info!("Memory locked successfully - daemon will not be swapped"),
        Err(e) => log::warn!("Failed to lock memory: {}. Daemon may be slow under memory pressure.", e),
    }

    // Create configuration from arguments
    let config = match Config::from_args(args) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            eprintln!("Use --help for usage information");
            process::exit(1);
        }
    };

    // Run the daemon
    if let Err(e) = daemon::run(config) {
        eprintln!("Fatal error: {}", e);
        process::exit(1);
    }
}
