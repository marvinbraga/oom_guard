// Signal management for process termination

use anyhow::Result;
use nix::sys::signal::{self, killpg, Signal};
use nix::unistd::{getpgid, Pid};
use std::thread;
use std::time::Duration;

// Syscall numbers for pidfd_open and process_mrelease
// These vary by architecture
#[cfg(target_arch = "x86_64")]
mod syscall_numbers {
    pub const SYS_PIDFD_OPEN: i64 = 434;
    pub const SYS_PROCESS_MRELEASE: i64 = 448;
}

#[cfg(target_arch = "aarch64")]
mod syscall_numbers {
    pub const SYS_PIDFD_OPEN: i64 = 438;
    pub const SYS_PROCESS_MRELEASE: i64 = 452;
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
mod syscall_numbers {
    // Fallback - these syscalls won't work but we fail gracefully
    pub const SYS_PIDFD_OPEN: i64 = -1;
    pub const SYS_PROCESS_MRELEASE: i64 = -1;
}

use syscall_numbers::{SYS_PIDFD_OPEN, SYS_PROCESS_MRELEASE};

/// PIDFD_NONBLOCK flag for pidfd_open (0x800 = O_NONBLOCK)
const PIDFD_NONBLOCK: u32 = 0x800;

/// Try to open a pidfd for the process (Linux 5.3+)
/// Returns None if the syscall is not available or fails
#[cfg(target_os = "linux")]
fn try_pidfd_open(pid: i32) -> Option<i32> {
    if SYS_PIDFD_OPEN < 0 {
        return None;
    }

    // SAFETY: syscall is a standard Linux system call interface.
    // We pass valid arguments: pid (process ID) and flags (PIDFD_NONBLOCK).
    // The syscall returns a file descriptor on success, or -1 on error.
    #[allow(unsafe_code)]
    let result = unsafe { libc::syscall(SYS_PIDFD_OPEN, pid, PIDFD_NONBLOCK as i32) };

    if result >= 0 {
        log::trace!("pidfd_open({pid}) = {result}");
        Some(result as i32)
    } else {
        log::trace!(
            "pidfd_open({pid}) failed: {}",
            std::io::Error::last_os_error()
        );
        None
    }
}

#[cfg(not(target_os = "linux"))]
fn try_pidfd_open(_pid: i32) -> Option<i32> {
    None
}

/// Try to release memory from a killed process faster (Linux 5.14+)
/// This syscall helps free memory pages more quickly after a process is killed
#[cfg(target_os = "linux")]
fn try_process_mrelease(pidfd: i32) {
    if SYS_PROCESS_MRELEASE < 0 {
        return;
    }

    // SAFETY: syscall is a standard Linux system call interface.
    // We pass the pidfd obtained from pidfd_open and flags (0).
    // This syscall releases memory associated with the dying process.
    #[allow(unsafe_code)]
    let result = unsafe { libc::syscall(SYS_PROCESS_MRELEASE, pidfd, 0) };

    if result >= 0 {
        log::debug!("process_mrelease({pidfd}) succeeded - memory release accelerated");
    } else {
        // This is expected to fail if:
        // - Kernel doesn't support it (< 5.14)
        // - Process already reaped
        // - Not enough privileges
        log::trace!(
            "process_mrelease({pidfd}) failed: {}",
            std::io::Error::last_os_error()
        );
    }
}

#[cfg(not(target_os = "linux"))]
fn try_process_mrelease(_pidfd: i32) {
    // No-op on non-Linux systems
}

/// Close a file descriptor safely
#[cfg(target_os = "linux")]
fn close_fd(fd: i32) {
    // SAFETY: close is a standard POSIX function that closes a file descriptor.
    // We only call this with valid file descriptors obtained from pidfd_open.
    #[allow(unsafe_code)]
    unsafe {
        libc::close(fd);
    }
}

#[cfg(not(target_os = "linux"))]
fn close_fd(_fd: i32) {
    // No-op on non-Linux systems
}

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
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::AlreadyDead)
    }

    /// Get a human-readable description
    pub fn description(&self) -> &str {
        match self {
            Self::Success => "successfully terminated",
            Self::AlreadyDead => "already dead",
            Self::PermissionDenied => "permission denied",
            Self::NotFound => "not found",
            Self::Error(msg) => msg,
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
        Err(e) => Ok(KillResult::Error(format!("signal error: {e}"))),
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
///
/// This function uses modern Linux kernel features when available:
/// - `pidfd_open()` (Linux 5.3+) for race-free process tracking
/// - `process_mrelease()` (Linux 5.14+) for faster memory reclamation
pub fn kill_process(pid: i32, strategy: KillStrategy, kill_group: bool) -> Result<KillResult> {
    log::debug!("Attempting to kill process {pid} (strategy: {strategy:?}, group: {kill_group})");

    // Try to get pidfd for safer process tracking (Linux 5.3+)
    // This prevents race conditions where the PID might be reused
    let pidfd = try_pidfd_open(pid);
    if pidfd.is_some() {
        log::trace!("Using pidfd for process {pid} tracking");
    }

    // Check if process exists before attempting to kill
    if !is_process_alive(pid) {
        log::debug!("Process {pid} is already dead");
        // Clean up pidfd if we opened one
        if let Some(fd) = pidfd {
            close_fd(fd);
        }
        return Ok(KillResult::AlreadyDead);
    }

    let result = match strategy {
        KillStrategy::Graceful => kill_graceful(pid, kill_group),
        KillStrategy::Forceful => kill_forceful(pid, kill_group),
    };

    // After kill attempt, try to release memory faster using process_mrelease (Linux 5.14+)
    // This syscall helps the kernel reclaim memory pages more quickly
    if let Some(fd) = pidfd {
        if result.as_ref().is_ok_and(KillResult::is_success) {
            try_process_mrelease(fd);
        }
        close_fd(fd);
    }

    result
}

/// Send signal to process or process group
fn send_signal_to_target(pid: i32, signal: Signal, kill_group: bool) -> Result<KillResult> {
    let nix_pid = Pid::from_raw(pid);

    if kill_group {
        // Get the process group ID and kill the entire group
        match getpgid(Some(nix_pid)) {
            Ok(pgid) => {
                log::debug!("Killing process group {pgid} (leader pid {pid})");
                match killpg(pgid, signal) {
                    Ok(()) => Ok(KillResult::Success),
                    Err(nix::errno::Errno::ESRCH) => Ok(KillResult::NotFound),
                    Err(nix::errno::Errno::EPERM) => Ok(KillResult::PermissionDenied),
                    Err(e) => Ok(KillResult::Error(format!("killpg error: {e}"))),
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to get process group for pid {pid}: {e}. Falling back to single process kill."
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
    log::info!("Sending SIGTERM to process {pid} (group: {kill_group})");

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
    log::warn!("Process {pid} did not respond to SIGTERM, escalating to SIGKILL");
    kill_forceful(pid, kill_group)
}

/// Kill a process forcefully using SIGKILL
fn kill_forceful(pid: i32, kill_group: bool) -> Result<KillResult> {
    log::info!("Sending SIGKILL to process {pid} (group: {kill_group})");

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
        log::error!("Process {pid} still alive after SIGKILL - this should not happen!");
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
