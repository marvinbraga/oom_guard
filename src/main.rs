// OOM Guard - Main entry point

use nix::sys::mman::{mlockall, MlockAllFlags};
use oom_guard::config::{Args, Config};
use oom_guard::daemon;
use std::process;

/// Setup logging based on configuration
fn setup_logging(debug: bool, use_syslog: bool) {
    let log_level = if debug { "debug" } else { "info" };

    if use_syslog {
        #[cfg(feature = "syslog")]
        {
            use syslog::{BasicLogger, Facility, Formatter3164};
            let formatter = Formatter3164 {
                facility: Facility::LOG_DAEMON,
                hostname: None,
                process: "oom_guard".into(),
                pid: std::process::id(),
            };

            match syslog::unix(formatter) {
                Ok(logger) => {
                    let level = if debug {
                        log::LevelFilter::Debug
                    } else {
                        log::LevelFilter::Info
                    };
                    if log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
                        .map(|()| log::set_max_level(level))
                        .is_ok()
                    {
                        return;
                    }
                }
                Err(e) => eprintln!("Failed to connect to syslog: {e}"),
            }
        }

        #[cfg(not(feature = "syslog"))]
        eprintln!("Warning: --syslog requires the 'syslog' feature to be enabled");
    }

    // Fallback to env_logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .format_timestamp_secs()
        .init();
}

fn main() {
    // Parse command-line arguments
    let args = Args::parse_args();

    // Initialize logging based on debug flag and syslog option
    setup_logging(args.debug, args.syslog);

    // Lock all current and future memory pages to prevent swapping
    // This ensures the daemon stays responsive even under memory pressure
    match mlockall(MlockAllFlags::MCL_CURRENT | MlockAllFlags::MCL_FUTURE) {
        Ok(()) => log::info!("Memory locked successfully - daemon will not be swapped"),
        Err(e) => {
            log::warn!("Failed to lock memory: {e}. Daemon may be slow under memory pressure.");
        }
    }

    // Create configuration from arguments
    let config = match Config::from_args(args) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Configuration error: {e}");
            eprintln!("Use --help for usage information");
            process::exit(1);
        }
    };

    // Run the daemon
    if let Err(e) = daemon::run(config) {
        eprintln!("Fatal error: {e}");
        process::exit(1);
    }
}
