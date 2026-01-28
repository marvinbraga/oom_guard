pub mod hooks;

use anyhow::Result;
use log::{error, info};
use std::process::Command;

#[cfg(feature = "dbus-notify")]
use notify_rust::{Notification, Timeout};

/// Sanitize a string for safe use in environment variables and shell scripts
fn sanitize_env_value(s: &str) -> String {
    // Remove or replace potentially dangerous characters
    s.chars()
        .map(|c| match c {
            // Allow alphanumeric, spaces, dots, hyphens, underscores
            c if c.is_alphanumeric() => c,
            ' ' | '.' | '-' | '_' | '/' => c,
            // Replace control characters and shell metacharacters
            _ => '_',
        })
        .take(256) // Limit length
        .collect()
}

pub struct NotificationManager {
    enable_dbus: bool,
    pre_kill_script: Option<String>,
    post_kill_script: Option<String>,
}

impl NotificationManager {
    pub fn new(
        enable_dbus: bool,
        pre_kill_script: Option<String>,
        post_kill_script: Option<String>,
    ) -> Self {
        Self {
            enable_dbus,
            pre_kill_script,
            post_kill_script,
        }
    }

    pub fn send_pre_kill_notification(
        &self,
        pid: i32,
        name: &str,
        rss_kb: u64,
        score: i32,
    ) -> Result<()> {
        if let Some(script) = &self.pre_kill_script {
            info!(
                "Executing pre-kill script: {} for process {} ({})",
                script, pid, name
            );
            if let Err(e) = self.execute_script(script, pid, name, rss_kb, score) {
                error!("Failed to execute pre-kill script: {}", e);
            }
        }
        Ok(())
    }

    pub fn send_post_kill_notification(
        &self,
        pid: i32,
        name: &str,
        rss_kb: u64,
        score: i32,
    ) -> Result<()> {
        // Execute post-kill script
        if let Some(script) = &self.post_kill_script {
            info!(
                "Executing post-kill script: {} for process {} ({})",
                script, pid, name
            );
            if let Err(e) = self.execute_script(script, pid, name, rss_kb, score) {
                error!("Failed to execute post-kill script: {}", e);
            }
        }

        // Send D-Bus notification
        #[cfg(feature = "dbus-notify")]
        if self.enable_dbus {
            if let Err(e) = self.send_dbus_notification(pid, name, rss_kb) {
                error!("Failed to send D-Bus notification: {}", e);
            }
        }

        #[cfg(not(feature = "dbus-notify"))]
        if self.enable_dbus {
            error!("D-Bus notifications enabled but feature 'dbus-notify' not compiled in");
        }

        Ok(())
    }

    fn execute_script(
        &self,
        script_path: &str,
        pid: i32,
        name: &str,
        rss_kb: u64,
        score: i32,
    ) -> Result<()> {
        let safe_name = sanitize_env_value(name);

        let output = Command::new(script_path)
            .env("OOM_GUARD_PID", pid.to_string())
            .env("OOM_GUARD_NAME", &safe_name)
            .env("OOM_GUARD_RSS", rss_kb.to_string())
            .env("OOM_GUARD_SCORE", score.to_string())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(
                "Script {} failed with status {}: {}",
                script_path,
                output.status,
                stderr.trim()
            );
        } else {
            info!("Script {} executed successfully", script_path);
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.is_empty() {
                info!("Script output: {}", stdout.trim());
            }
        }

        Ok(())
    }

    #[cfg(feature = "dbus-notify")]
    fn send_dbus_notification(&self, pid: i32, name: &str, rss_kb: u64) -> Result<()> {
        let rss_mb = rss_kb / 1024;
        let message = format!(
            "OOM Guard killed process:\nPID: {}\nName: {}\nRSS: {} MB",
            pid, name, rss_mb
        );

        Notification::new()
            .summary("OOM Guard: Process Killed")
            .body(&message)
            .icon("dialog-warning")
            .timeout(Timeout::Milliseconds(6000))
            .show()?;

        info!("D-Bus notification sent for process {} ({})", pid, name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_manager_creation() {
        let manager = NotificationManager::new(false, None, None);
        assert!(!manager.enable_dbus);
        assert!(manager.pre_kill_script.is_none());
        assert!(manager.post_kill_script.is_none());
    }

    #[test]
    fn test_notification_with_scripts() {
        let manager = NotificationManager::new(
            false,
            Some("/tmp/pre.sh".to_string()),
            Some("/tmp/post.sh".to_string()),
        );
        assert!(manager.pre_kill_script.is_some());
        assert!(manager.post_kill_script.is_some());
    }

    #[test]
    fn test_sanitize_env_value_normal() {
        assert_eq!(sanitize_env_value("firefox"), "firefox");
        assert_eq!(sanitize_env_value("my-app"), "my-app");
        assert_eq!(sanitize_env_value("app_v1.2"), "app_v1.2");
        assert_eq!(sanitize_env_value("/usr/bin/app"), "/usr/bin/app");
    }

    #[test]
    fn test_sanitize_env_value_shell_metacharacters() {
        // Shell metacharacters should be replaced with underscore
        assert_eq!(sanitize_env_value("$(whoami)"), "__whoami_");
        assert_eq!(sanitize_env_value("`id`"), "_id_");
        assert_eq!(sanitize_env_value("a;b"), "a_b");
        assert_eq!(sanitize_env_value("a|b"), "a_b");
        assert_eq!(sanitize_env_value("a&b"), "a_b");
        assert_eq!(sanitize_env_value("a>b"), "a_b");
        assert_eq!(sanitize_env_value("a<b"), "a_b");
        assert_eq!(sanitize_env_value("a'b"), "a_b");
        assert_eq!(sanitize_env_value("a\"b"), "a_b");
        assert_eq!(sanitize_env_value("a\\nb"), "a_nb");
    }

    #[test]
    fn test_sanitize_env_value_length_limit() {
        let long_name = "a".repeat(500);
        let sanitized = sanitize_env_value(&long_name);
        assert_eq!(sanitized.len(), 256);
    }

    #[test]
    fn test_sanitize_env_value_control_characters() {
        assert_eq!(sanitize_env_value("a\nb"), "a_b");
        assert_eq!(sanitize_env_value("a\tb"), "a_b");
        assert_eq!(sanitize_env_value("a\0b"), "a_b");
    }
}
