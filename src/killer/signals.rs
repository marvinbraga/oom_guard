// Signal management for process termination

use anyhow::Result;
use nix::sys::signal::{self, killpg, Signal};
use nix::unistd::{getpgid, Pid};
use std::thread;
use std::time::Duration;

/// Strategy for killing processes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillStrategy {
    /// Send SIGTERM first (graceful shutdown)
    Graceful,
    /// Send SIGKILL immediately (forceful termination)
    Forceful,
}

/// Result of a kill operation
#[derive(Debug)]
pub enum KillResult {
    /// Process was successfully terminated
    Success,
    /// Process was already dead
    AlreadyDead,
    /// Permission denied (typically need root)
    PermissionDenied,
    /// Process not found
    NotFound,
    /// Other error occurred
    Error(String),
}

impl KillResult {
    /// Check if the kill operation was successful
    pub fn is_success(&self) -> bool {
        matches!(self, KillResult::Success | KillResult::AlreadyDead)
    }

    /// Get a human-readable description
    pub fn description(&self) -> &str {
        match self {
            KillResult::Success => "successfully terminated",
            KillResult::AlreadyDead => "already dead",
            KillResult::PermissionDenied => "permission denied",
            KillResult::NotFound => "not found",
            KillResult::Error(msg) => msg,
        }
    }
}

/// Send a signal to a process
fn send_signal(pid: i32, signal: Signal) -> Result<KillResult> {
    let nix_pid = Pid::from_raw(pid);

    match signal::kill(nix_pid, signal) {
        Ok(()) => Ok(KillResult::Success),
        Err(nix::errno::Errno::ESRCH) => {
            // Process does not exist
            Ok(KillResult::NotFound)
        }
        Err(nix::errno::Errno::EPERM) => {
            // Permission denied
            Ok(KillResult::PermissionDenied)
        }
        Err(e) => Ok(KillResult::Error(format!("signal error: {}", e))),
    }
}

/// Check if a process is still alive
fn is_process_alive(pid: i32) -> bool {
    let nix_pid = Pid::from_raw(pid);
    // Send signal 0 to check if process exists without actually sending a signal
    signal::kill(nix_pid, None).is_ok()
}

/// Kill a single process using the specified strategy
///
/// # Arguments
/// * `pid` - Process ID to kill
/// * `strategy` - Whether to use graceful (SIGTERM) or forceful (SIGKILL) termination
/// * `kill_group` - If true, kill the entire process group instead of just the process
///
/// # Returns
/// Result containing the KillResult enum describing the outcome
pub fn kill_process(pid: i32, strategy: KillStrategy, kill_group: bool) -> Result<KillResult> {
    log::debug!(
        "Attempting to kill process {} (strategy: {:?}, group: {})",
        pid,
        strategy,
        kill_group
    );

    // Check if process exists before attempting to kill
    if !is_process_alive(pid) {
        log::debug!("Process {} is already dead", pid);
        return Ok(KillResult::AlreadyDead);
    }

    match strategy {
        KillStrategy::Graceful => kill_graceful(pid, kill_group),
        KillStrategy::Forceful => kill_forceful(pid, kill_group),
    }
}

/// Send signal to process or process group
fn send_signal_to_target(pid: i32, signal: Signal, kill_group: bool) -> Result<KillResult> {
    let nix_pid = Pid::from_raw(pid);

    if kill_group {
        // Get the process group ID and kill the entire group
        match getpgid(Some(nix_pid)) {
            Ok(pgid) => {
                log::debug!("Killing process group {} (leader pid {})", pgid, pid);
                match killpg(pgid, signal) {
                    Ok(()) => Ok(KillResult::Success),
                    Err(nix::errno::Errno::ESRCH) => Ok(KillResult::NotFound),
                    Err(nix::errno::Errno::EPERM) => Ok(KillResult::PermissionDenied),
                    Err(e) => Ok(KillResult::Error(format!("killpg error: {}", e))),
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to get process group for pid {}: {}. Falling back to single process kill.",
                    pid,
                    e
                );
                // Fall back to killing single process
                send_signal(pid, signal)
            }
        }
    } else {
        send_signal(pid, signal)
    }
}

/// Kill a process gracefully using SIGTERM
fn kill_graceful(pid: i32, kill_group: bool) -> Result<KillResult> {
    log::info!("Sending SIGTERM to process {} (group: {})", pid, kill_group);

    let result = send_signal_to_target(pid, Signal::SIGTERM, kill_group)?;

    if !result.is_success() {
        log::warn!(
            "Failed to send SIGTERM to process {}: {}",
            pid,
            result.description()
        );
        return Ok(result);
    }

    // Wait briefly to see if process terminates gracefully
    for i in 0..10 {
        thread::sleep(Duration::from_millis(100));
        if !is_process_alive(pid) {
            log::info!("Process {} terminated gracefully after {}ms", pid, i * 100);
            return Ok(KillResult::Success);
        }
    }

    // Process didn't die after SIGTERM, escalate to SIGKILL
    log::warn!(
        "Process {} did not respond to SIGTERM, escalating to SIGKILL",
        pid
    );
    kill_forceful(pid, kill_group)
}

/// Kill a process forcefully using SIGKILL
fn kill_forceful(pid: i32, kill_group: bool) -> Result<KillResult> {
    log::info!("Sending SIGKILL to process {} (group: {})", pid, kill_group);

    let result = send_signal_to_target(pid, Signal::SIGKILL, kill_group)?;

    if !result.is_success() {
        log::warn!(
            "Failed to send SIGKILL to process {}: {}",
            pid,
            result.description()
        );
        return Ok(result);
    }

    // Wait briefly to verify process termination
    for i in 0..5 {
        thread::sleep(Duration::from_millis(50));
        if !is_process_alive(pid) {
            log::info!("Process {} forcefully terminated after {}ms", pid, i * 50);
            return Ok(KillResult::Success);
        }
    }

    // Process should always die after SIGKILL, but check just in case
    if is_process_alive(pid) {
        log::error!(
            "Process {} still alive after SIGKILL - this should not happen!",
            pid
        );
        Ok(KillResult::Error("process survived SIGKILL".to_string()))
    } else {
        Ok(KillResult::Success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kill_strategy_equality() {
        assert_eq!(KillStrategy::Graceful, KillStrategy::Graceful);
        assert_eq!(KillStrategy::Forceful, KillStrategy::Forceful);
        assert_ne!(KillStrategy::Graceful, KillStrategy::Forceful);
    }

    #[test]
    fn test_kill_result_is_success() {
        assert!(KillResult::Success.is_success());
        assert!(KillResult::AlreadyDead.is_success());
        assert!(!KillResult::PermissionDenied.is_success());
        assert!(!KillResult::NotFound.is_success());
    }

    #[test]
    fn test_kill_result_description() {
        assert_eq!(KillResult::Success.description(), "successfully terminated");
        assert_eq!(KillResult::AlreadyDead.description(), "already dead");
        assert_eq!(
            KillResult::PermissionDenied.description(),
            "permission denied"
        );
        assert_eq!(KillResult::NotFound.description(), "not found");
    }

    #[test]
    fn test_kill_nonexistent_process() {
        // Process ID 999999 should not exist
        let result = kill_process(999999, KillStrategy::Forceful, false);
        assert!(result.is_ok());
        let kill_result = result.unwrap();
        assert!(matches!(
            kill_result,
            KillResult::NotFound | KillResult::AlreadyDead
        ));
    }
}
