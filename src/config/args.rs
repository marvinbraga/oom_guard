// Command-line argument parsing

use clap::Parser;

/// OOM Guard - Memory monitor and process killer
///
/// A user-space Out-Of-Memory (OOM) killer that monitors system memory
/// and proactively terminates processes before the kernel OOM killer activates.
#[derive(Parser, Debug)]
#[command(name = "oom-guard")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Memory monitor and OOM prevention daemon", long_about = None)]
pub struct Args {
    /// Memory threshold PERCENT[,KILL_PERCENT] (default: 10,5)
    /// First value is warning threshold, second is kill threshold
    /// If only one value given, kill threshold defaults to 50% of warning
    #[arg(short = 'm', long = "mem", value_name = "PERCENT[,KILL_PERCENT]")]
    pub mem_threshold: Option<String>,

    /// Swap threshold PERCENT[,KILL_PERCENT] (default: 10,5)
    /// First value is warning threshold, second is kill threshold
    /// If only one value given, kill threshold defaults to 50% of warning
    #[arg(short = 's', long = "swap", value_name = "PERCENT[,KILL_PERCENT]")]
    pub swap_threshold: Option<String>,

    /// Memory threshold SIZE[,KILL_SIZE] in KiB (alternative to -m)
    #[arg(short = 'M', long = "mem-size", value_name = "SIZE[,KILL_SIZE]")]
    pub mem_size_kb: Option<String>,

    /// Swap threshold SIZE[,KILL_SIZE] in KiB (alternative to -s)
    #[arg(short = 'S', long = "swap-size", value_name = "SIZE[,KILL_SIZE]")]
    pub swap_size_kb: Option<String>,

    /// Memory check interval in seconds (default: 1)
    #[arg(short = 'i', long = "interval", value_name = "SECONDS")]
    pub interval: Option<u64>,

    /// Status report interval in seconds (default: 60)
    #[arg(short = 'r', long = "report", value_name = "SECONDS")]
    pub report: Option<u64>,

    /// Enable desktop notifications when killing processes
    #[arg(short = 'n', long = "notify")]
    pub notify: bool,

    /// Script to run after killing a process
    #[arg(short = 'N', long = "post-kill-script", value_name = "PATH")]
    pub post_kill_script: Option<String>,

    /// Script to run before killing a process
    #[arg(short = 'P', long = "pre-kill-script", value_name = "PATH")]
    pub pre_kill_script: Option<String>,

    /// Kill entire process group instead of just the process
    #[arg(short = 'g', long = "kill-group")]
    pub kill_group: bool,

    /// Set daemon priority (-20 to 19, lower = higher priority)
    #[arg(short = 'p', long = "set-priority", value_name = "PRIORITY")]
    pub priority: Option<i32>,

    /// Enable debug logging
    #[arg(short = 'd', long = "debug")]
    pub debug: bool,

    /// Sort processes by RSS memory usage instead of oom_score
    #[arg(long = "sort-by-rss")]
    pub sort_by_rss: bool,

    /// Prefer to kill processes matching this regex (can be used multiple times)
    #[arg(long = "prefer", value_name = "REGEX")]
    pub prefer: Vec<String>,

    /// Avoid killing processes matching this regex (can be used multiple times)
    #[arg(long = "avoid", value_name = "REGEX")]
    pub avoid: Vec<String>,

    /// Completely ignore processes matching this regex (can be used multiple times)
    #[arg(long = "ignore", value_name = "REGEX")]
    pub ignore: Vec<String>,

    /// Dry run mode - don't actually kill processes, just report what would be killed
    #[arg(long = "dryrun")]
    pub dry_run: bool,

    /// Ignore processes owned by root user
    #[arg(long = "ignore-root-user")]
    pub ignore_root_user: bool,

    /// Use syslog instead of stdout/stderr for logging
    #[arg(long = "syslog")]
    pub syslog: bool,
}

impl Args {
    /// Parse arguments from command line
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_cli() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }
}
