// Configuration module

mod args;
mod env;

pub use args::Args;
use anyhow::{bail, Context, Result};
use regex::{Regex, RegexBuilder};
use std::time::Duration;

/// Maximum allowed length for regex patterns to prevent ReDoS attacks
const MAX_REGEX_PATTERN_LENGTH: usize = 256;

/// Maximum compiled regex size in bytes (10MB) to prevent memory exhaustion
const REGEX_SIZE_LIMIT: usize = 10 * (1 << 20);

/// Compile a regex pattern with safety limits to prevent ReDoS attacks.
///
/// This function applies the following protections:
/// - Limits pattern length to MAX_REGEX_PATTERN_LENGTH characters
/// - Sets a compiled size limit to prevent memory exhaustion
/// - Uses RegexBuilder with size_limit for additional protection
///
/// # Arguments
/// * `pattern` - The regex pattern to compile
///
/// # Returns
/// * `Ok(Regex)` - A safely compiled regex
/// * `Err` - If the pattern is too long, invalid, or potentially dangerous
fn compile_safe_regex(pattern: &str) -> Result<Regex> {
    // Validate pattern length
    if pattern.len() > MAX_REGEX_PATTERN_LENGTH {
        bail!(
            "Regex pattern too long (max {} chars): {}...",
            MAX_REGEX_PATTERN_LENGTH,
            &pattern[..50.min(pattern.len())]
        );
    }

    // Compile with size limit to prevent ReDoS
    RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .build()
        .context(format!("Invalid regex pattern: {}", pattern))
}

/// Parse threshold pair from string "WARN" or "WARN,KILL"
/// Returns (warn_threshold, kill_threshold)
fn parse_threshold_pair(s: &str, default_kill_ratio: f64) -> Result<(f64, f64)> {
    let parts: Vec<&str> = s.split(',').collect();
    let warn: f64 = parts[0]
        .trim()
        .parse()
        .context("Invalid threshold value")?;

    let kill: f64 = if parts.len() > 1 {
        parts[1].trim().parse().context("Invalid kill threshold")?
    } else {
        warn * default_kill_ratio // Default to ratio of warn
    };

    Ok((warn, kill))
}

/// Parse size pair from string "SIZE" or "SIZE,KILL_SIZE" (in KiB)
/// Returns (warn_size, kill_size)
fn parse_size_pair(s: &str, default_kill_ratio: f64) -> Result<(u64, u64)> {
    let parts: Vec<&str> = s.split(',').collect();
    let warn: u64 = parts[0]
        .trim()
        .parse()
        .context("Invalid size value")?;

    let kill: u64 = if parts.len() > 1 {
        parts[1].trim().parse().context("Invalid kill size")?
    } else {
        (warn as f64 * default_kill_ratio) as u64 // Default to ratio of warn
    };

    Ok((warn, kill))
}

/// Main configuration struct for OOM Guard
#[derive(Debug, Clone)]
pub struct Config {
    // Memory thresholds (percentages)
    pub mem_threshold_warn: f64,  // Warning threshold
    pub mem_threshold_kill: f64,  // Kill threshold
    pub swap_threshold_warn: f64, // Warning threshold
    pub swap_threshold_kill: f64, // Kill threshold

    // Memory thresholds (absolute KiB)
    pub mem_size_warn: Option<u64>,  // Warning size in KiB
    pub mem_size_kill: Option<u64>,  // Kill size in KiB
    pub swap_size_warn: Option<u64>, // Warning size in KiB
    pub swap_size_kill: Option<u64>, // Kill size in KiB

    // Monitoring intervals
    pub check_interval: Duration,  // How often to check memory
    pub report_interval: Duration, // How often to report status

    // Process selection
    pub sort_by_rss: bool,    // Sort by RSS instead of oom_score
    pub prefer: Vec<Regex>,   // Regex patterns for preferred victims
    pub avoid: Vec<Regex>,    // Regex patterns to avoid killing
    pub ignore: Vec<Regex>,   // Regex patterns to completely ignore

    // Behavior flags
    pub dry_run: bool,        // Don't actually kill processes
    pub debug: bool,          // Enable debug logging
    pub notify: bool,         // Send notifications when killing

    // System interaction
    pub ignore_root_user: bool,  // Ignore processes owned by root

    // Notification options
    pub notify_dbus: bool,                 // Enable D-Bus notifications
    pub pre_kill_script: Option<String>,   // Script to run before killing
    pub post_kill_script: Option<String>,  // Script to run after killing

    // Process group killing
    pub kill_group: bool,     // Kill entire process group

    // Priority setting
    pub priority: Option<i32>, // Daemon priority
}

impl Config {
    /// Create configuration from command-line arguments
    pub fn from_args(args: Args) -> Result<Self> {
        let mut config = Self::default();

        // Parse memory thresholds (percentages)
        if let Some(mem_threshold_str) = args.mem_threshold {
            let (warn, kill) = parse_threshold_pair(&mem_threshold_str, 0.5)?;
            config.mem_threshold_warn = warn;
            config.mem_threshold_kill = kill;
        }

        if let Some(swap_threshold_str) = args.swap_threshold {
            let (warn, kill) = parse_threshold_pair(&swap_threshold_str, 0.5)?;
            config.swap_threshold_warn = warn;
            config.swap_threshold_kill = kill;
        }

        // Parse memory thresholds (absolute KiB)
        if let Some(mem_size_str) = args.mem_size_kb {
            let (warn, kill) = parse_size_pair(&mem_size_str, 0.5)?;
            config.mem_size_warn = Some(warn);
            config.mem_size_kill = Some(kill);
        }

        if let Some(swap_size_str) = args.swap_size_kb {
            let (warn, kill) = parse_size_pair(&swap_size_str, 0.5)?;
            config.swap_size_warn = Some(warn);
            config.swap_size_kill = Some(kill);
        }

        // Monitoring intervals
        if let Some(interval) = args.interval {
            config.check_interval = Duration::from_secs(interval);
        }
        if let Some(report) = args.report {
            config.report_interval = Duration::from_secs(report);
        }

        // Process selection
        config.sort_by_rss = args.sort_by_rss;

        // Compile regex patterns with safety limits (ReDoS protection)
        for pattern in args.prefer {
            config.prefer.push(compile_safe_regex(&pattern)?);
        }
        for pattern in args.avoid {
            config.avoid.push(compile_safe_regex(&pattern)?);
        }
        for pattern in args.ignore {
            config.ignore.push(compile_safe_regex(&pattern)?);
        }

        // Behavior flags
        config.dry_run = args.dry_run;
        config.debug = args.debug;
        config.notify = args.notify;
        config.ignore_root_user = args.ignore_root_user;

        // Scripts
        config.pre_kill_script = args.pre_kill_script;
        config.post_kill_script = args.post_kill_script;

        // Process group killing
        config.kill_group = args.kill_group;

        // Priority
        config.priority = args.priority;

        // Apply environment variable overrides
        config = env::apply_env_overrides(config)?;

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate configuration values
    fn validate(&self) -> Result<()> {
        // Validate percentage ranges
        if self.mem_threshold_warn < 0.0 || self.mem_threshold_warn > 100.0 {
            anyhow::bail!("mem_threshold_warn must be between 0 and 100");
        }
        if self.mem_threshold_kill < 0.0 || self.mem_threshold_kill > 100.0 {
            anyhow::bail!("mem_threshold_kill must be between 0 and 100");
        }
        if self.swap_threshold_warn < 0.0 || self.swap_threshold_warn > 100.0 {
            anyhow::bail!("swap_threshold_warn must be between 0 and 100");
        }
        if self.swap_threshold_kill < 0.0 || self.swap_threshold_kill > 100.0 {
            anyhow::bail!("swap_threshold_kill must be between 0 and 100");
        }

        // Validate that kill threshold is less than or equal to warn threshold
        if self.mem_threshold_kill > self.mem_threshold_warn {
            log::warn!(
                "mem_threshold_kill ({}) is greater than mem_threshold_warn ({})",
                self.mem_threshold_kill,
                self.mem_threshold_warn
            );
        }
        if self.swap_threshold_kill > self.swap_threshold_warn {
            log::warn!(
                "swap_threshold_kill ({}) is greater than swap_threshold_warn ({})",
                self.swap_threshold_kill,
                self.swap_threshold_warn
            );
        }

        // Check that either percentage or absolute values are set
        if self.mem_size_warn.is_some() && self.mem_threshold_warn != 10.0 {
            log::warn!("Both -m and -M set, using -M (absolute size)");
        }
        if self.swap_size_warn.is_some() && self.swap_threshold_warn != 10.0 {
            log::warn!("Both -s and -S set, using -S (absolute size)");
        }

        // Validate priority range
        if let Some(priority) = self.priority {
            if !(-20..=19).contains(&priority) {
                anyhow::bail!("priority must be between -20 and 19");
            }
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mem_threshold_warn: 10.0,  // 10% warning
            mem_threshold_kill: 5.0,   // 5% kill
            swap_threshold_warn: 10.0, // 10% warning
            swap_threshold_kill: 5.0,  // 5% kill
            mem_size_warn: None,
            mem_size_kill: None,
            swap_size_warn: None,
            swap_size_kill: None,
            check_interval: Duration::from_secs(1),   // Check every second
            report_interval: Duration::from_secs(60), // Report every minute
            sort_by_rss: false,                       // Use oom_score by default
            prefer: Vec::new(),
            avoid: Vec::new(),
            ignore: Vec::new(),
            dry_run: false,
            debug: false,
            notify: false,
            ignore_root_user: false,
            notify_dbus: false,
            pre_kill_script: None,
            post_kill_script: None,
            kill_group: false,
            priority: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_safe_regex_valid_pattern() {
        let regex = compile_safe_regex("^firefox$").unwrap();
        assert!(regex.is_match("firefox"));
        assert!(!regex.is_match("firefox-esr"));
    }

    #[test]
    fn test_compile_safe_regex_complex_but_safe_pattern() {
        let regex = compile_safe_regex(r"chrome|chromium|google-chrome").unwrap();
        assert!(regex.is_match("chrome"));
        assert!(regex.is_match("chromium"));
        assert!(regex.is_match("google-chrome"));
    }

    #[test]
    fn test_compile_safe_regex_pattern_too_long() {
        let long_pattern = "a".repeat(MAX_REGEX_PATTERN_LENGTH + 1);
        let result = compile_safe_regex(&long_pattern);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("too long"));
    }

    #[test]
    fn test_compile_safe_regex_invalid_pattern() {
        let result = compile_safe_regex("[invalid");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid regex pattern"));
    }

    #[test]
    fn test_compile_safe_regex_max_length_boundary() {
        // Pattern exactly at the limit should work
        let pattern = "a".repeat(MAX_REGEX_PATTERN_LENGTH);
        let result = compile_safe_regex(&pattern);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_threshold_pair_single_value() {
        let (warn, kill) = parse_threshold_pair("10", 0.5).unwrap();
        assert_eq!(warn, 10.0);
        assert_eq!(kill, 5.0); // 50% of 10
    }

    #[test]
    fn test_parse_threshold_pair_both_values() {
        let (warn, kill) = parse_threshold_pair("10,5", 0.5).unwrap();
        assert_eq!(warn, 10.0);
        assert_eq!(kill, 5.0);
    }

    #[test]
    fn test_parse_threshold_pair_custom_ratio() {
        let (warn, kill) = parse_threshold_pair("20", 0.5).unwrap();
        assert_eq!(warn, 20.0);
        assert_eq!(kill, 10.0);
    }

    #[test]
    fn test_parse_size_pair_single_value() {
        let (warn, kill) = parse_size_pair("1048576", 0.5).unwrap();
        assert_eq!(warn, 1048576);
        assert_eq!(kill, 524288); // 50% of 1048576
    }

    #[test]
    fn test_parse_size_pair_both_values() {
        let (warn, kill) = parse_size_pair("1048576,262144", 0.5).unwrap();
        assert_eq!(warn, 1048576);
        assert_eq!(kill, 262144);
    }

    #[test]
    fn test_config_default_thresholds() {
        let config = Config::default();
        assert_eq!(config.mem_threshold_warn, 10.0);
        assert_eq!(config.mem_threshold_kill, 5.0);
        assert_eq!(config.swap_threshold_warn, 10.0);
        assert_eq!(config.swap_threshold_kill, 5.0);
    }
}
