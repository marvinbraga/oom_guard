// OOM Guard - Memory monitor and process management library

pub mod config;
pub mod daemon;
pub mod killer;
pub mod monitor;
pub mod notify;

// Re-export commonly used types
pub use config::Config;
pub use monitor::{MemInfo, ProcessInfo};

/// Sanitize a string for safe logging by removing control characters.
/// This prevents log injection attacks where malicious process names
/// could inject fake log entries or corrupt log output.
pub fn sanitize_for_log(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_control() && c != '\n' && c != '\t' {
                '?'
            } else {
                c
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_for_log_normal_string() {
        assert_eq!(sanitize_for_log("firefox"), "firefox");
        assert_eq!(sanitize_for_log("my-process"), "my-process");
    }

    #[test]
    fn test_sanitize_for_log_with_control_chars() {
        // Test with null byte
        assert_eq!(sanitize_for_log("evil\x00process"), "evil?process");
        // Test with carriage return (potential log injection)
        assert_eq!(
            sanitize_for_log("process\rFake: log entry"),
            "process?Fake: log entry"
        );
        // Test with escape sequences
        assert_eq!(sanitize_for_log("process\x1b[31mred"), "process?[31mred");
    }

    #[test]
    fn test_sanitize_for_log_preserves_newlines_and_tabs() {
        // Newlines and tabs are allowed as they are common in logs
        assert_eq!(sanitize_for_log("line1\nline2"), "line1\nline2");
        assert_eq!(sanitize_for_log("col1\tcol2"), "col1\tcol2");
    }

    #[test]
    fn test_sanitize_for_log_empty_string() {
        assert_eq!(sanitize_for_log(""), "");
    }
}
