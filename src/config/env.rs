// Environment variable configuration support

use super::Config;
use anyhow::Result;
use std::env;
use std::time::Duration;

/// Apply environment variable overrides to configuration
pub fn apply_env_overrides(mut config: Config) -> Result<Config> {
    // Memory thresholds (warn)
    if let Ok(val) = env::var("OOM_GUARD_MEM_WARN") {
        config.mem_threshold_warn = val.parse()?;
    }
    if let Ok(val) = env::var("OOM_GUARD_SWAP_WARN") {
        config.swap_threshold_warn = val.parse()?;
    }

    // Memory thresholds (kill)
    if let Ok(val) = env::var("OOM_GUARD_MEM_KILL") {
        config.mem_threshold_kill = val.parse()?;
    }
    if let Ok(val) = env::var("OOM_GUARD_SWAP_KILL") {
        config.swap_threshold_kill = val.parse()?;
    }

    // Memory sizes (warn)
    if let Ok(val) = env::var("OOM_GUARD_MEM_SIZE_WARN") {
        config.mem_size_warn = Some(val.parse()?);
    }
    if let Ok(val) = env::var("OOM_GUARD_SWAP_SIZE_WARN") {
        config.swap_size_warn = Some(val.parse()?);
    }

    // Memory sizes (kill)
    if let Ok(val) = env::var("OOM_GUARD_MEM_SIZE_KILL") {
        config.mem_size_kill = Some(val.parse()?);
    }
    if let Ok(val) = env::var("OOM_GUARD_SWAP_SIZE_KILL") {
        config.swap_size_kill = Some(val.parse()?);
    }

    // Monitoring intervals
    if let Ok(val) = env::var("OOM_GUARD_INTERVAL") {
        config.check_interval = Duration::from_secs(val.parse()?);
    }
    if let Ok(val) = env::var("OOM_GUARD_REPORT") {
        config.report_interval = Duration::from_secs(val.parse()?);
    }

    // Process selection
    if let Ok(val) = env::var("OOM_GUARD_SORT_BY_RSS") {
        config.sort_by_rss = parse_bool(&val)?;
    }

    // Behavior flags
    if let Ok(val) = env::var("OOM_GUARD_DRY_RUN") {
        config.dry_run = parse_bool(&val)?;
    }
    if let Ok(val) = env::var("OOM_GUARD_DEBUG") {
        config.debug = parse_bool(&val)?;
    }
    if let Ok(val) = env::var("OOM_GUARD_NOTIFY") {
        config.notify = parse_bool(&val)?;
    }
    if let Ok(val) = env::var("OOM_GUARD_IGNORE_ROOT_USER") {
        config.ignore_root_user = parse_bool(&val)?;
    }

    // Kill group
    if let Ok(val) = env::var("OOM_GUARD_KILL_GROUP") {
        config.kill_group = parse_bool(&val)?;
    }

    // Priority
    if let Ok(val) = env::var("OOM_GUARD_PRIORITY") {
        config.priority = Some(val.parse()?);
    }

    Ok(config)
}

/// Parse boolean value from string
/// Accepts: true/false, 1/0, yes/no, on/off (case-insensitive)
fn parse_bool(s: &str) -> Result<bool> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => anyhow::bail!("Invalid boolean value: {}", s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_bool("true").unwrap(), true);
        assert_eq!(parse_bool("TRUE").unwrap(), true);
        assert_eq!(parse_bool("1").unwrap(), true);
        assert_eq!(parse_bool("yes").unwrap(), true);
        assert_eq!(parse_bool("on").unwrap(), true);

        assert_eq!(parse_bool("false").unwrap(), false);
        assert_eq!(parse_bool("FALSE").unwrap(), false);
        assert_eq!(parse_bool("0").unwrap(), false);
        assert_eq!(parse_bool("no").unwrap(), false);
        assert_eq!(parse_bool("off").unwrap(), false);

        assert!(parse_bool("invalid").is_err());
    }
}
