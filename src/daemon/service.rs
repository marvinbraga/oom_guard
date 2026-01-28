// Main daemon service implementation

use crate::config::Config;
use crate::killer::{kill_process, KillInfo, KillStrategy};
use crate::monitor::{MemInfo, ProcessInfo};
use crate::notify::NotificationManager;
use crate::sanitize_for_log;
use anyhow::{anyhow, Context, Result};
use nix::libc::{setpriority, PRIO_PROCESS};
use std::fs;
use std::io::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Set daemon priority using the configured value
fn set_daemon_priority(priority: i32) -> Result<()> {
    // SAFETY: setpriority is a standard POSIX function. We pass valid arguments:
    // - PRIO_PROCESS: adjust priority of current process
    // - 0: target the calling process
    // - priority: validated to be in range -20..=19 by Config::validate()
    #[allow(unsafe_code)]
    let result = unsafe { setpriority(PRIO_PROCESS, 0, priority) };

    if result != 0 {
        let err = Error::last_os_error();
        log::warn!("Failed to set niceness to {priority}: {err}. May need root privileges.");
    } else {
        log::info!("Set daemon niceness to {priority} (priority)");
    }

    // Set oom_score_adj to -100 (protect from OOM killer)
    match fs::write("/proc/self/oom_score_adj", "-100") {
        Ok(()) => log::info!("Set oom_score_adj to -100 (protected from OOM killer)"),
        Err(e) => log::warn!(
            "Failed to set oom_score_adj: {e}. Daemon may be killed under extreme memory pressure."
        ),
    }

    Ok(())
}

/// Daemon service that monitors memory and kills processes
pub struct DaemonService {
    config: Config,
    notification_manager: NotificationManager,
    last_report: Instant,
    last_kill: Option<Instant>,
    running: Arc<AtomicBool>,
}

impl DaemonService {
    /// Create a new daemon service
    pub fn new(config: Config) -> Self {
        let notification_manager = NotificationManager::new(
            config.notify_dbus,
            config.pre_kill_script.clone(),
            config.post_kill_script.clone(),
        );
        Self {
            config,
            notification_manager,
            last_report: Instant::now(),
            last_kill: None,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the running flag for signal handling
    pub fn running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }

    /// Main run loop
    pub fn run(&mut self) -> Result<()> {
        // Set daemon priority if requested
        if let Some(priority) = self.config.priority {
            if let Err(e) = set_daemon_priority(priority) {
                log::error!("Failed to set daemon priority: {e}");
            }
        }

        // Print startup information
        self.print_startup_info()?;

        self.running.store(true, Ordering::SeqCst);
        self.last_report = Instant::now();

        // Setup signal handlers
        self.setup_signal_handlers()?;

        while self.running.load(Ordering::SeqCst) {
            // Read memory info once per iteration
            let meminfo = match MemInfo::read() {
                Ok(m) => m,
                Err(e) => {
                    log::error!("Failed to read memory info: {e}");
                    std::thread::sleep(self.config.check_interval);
                    continue;
                }
            };

            // Check memory and act if needed
            if let Err(e) = self.check_and_act_with_meminfo(&meminfo) {
                log::error!("Error in main loop: {e}");
            }

            // Periodic status report
            if self.last_report.elapsed() >= self.config.report_interval {
                self.report_status()?;
                self.last_report = Instant::now();
            }

            // Use adaptive sleep or fixed interval based on configuration
            let sleep_duration = if self.config.adaptive_sleep {
                self.calculate_adaptive_sleep(&meminfo)
            } else {
                self.config.check_interval
            };
            log::trace!("Sleeping for {}ms", sleep_duration.as_millis());
            std::thread::sleep(sleep_duration);
        }

        log::info!("OOM Guard daemon shutting down gracefully");
        Ok(())
    }

    /// Setup signal handlers for graceful shutdown
    fn setup_signal_handlers(&self) -> Result<()> {
        let running = Arc::clone(&self.running);

        // Handle SIGTERM and SIGINT
        let r = running;
        ctrlc::set_handler(move || {
            log::info!("Received shutdown signal");
            r.store(false, Ordering::SeqCst);
        })
        .map_err(|e| anyhow!("Failed to set signal handler: {e}"))?;

        Ok(())
    }

    /// Print startup information
    #[allow(clippy::cognitive_complexity)]
    fn print_startup_info(&self) -> Result<()> {
        let meminfo = MemInfo::read()?;

        log::info!("=== OOM Guard v{} starting ===", env!("CARGO_PKG_VERSION"));
        log::info!(
            "Memory total: {} MiB, available: {} MiB ({:.1}%)",
            meminfo.mem_total / 1024,
            meminfo.mem_available / 1024,
            meminfo.mem_available_percent()
        );
        log::info!(
            "Swap total: {} MiB, free: {} MiB ({:.1}%)",
            meminfo.swap_total / 1024,
            meminfo.swap_free / 1024,
            meminfo.swap_free_percent()
        );

        log::info!("Thresholds:");

        // Display thresholds based on configuration
        if self.config.mem_size_warn.is_some() {
            log::info!(
                "  SIGTERM when mem <= {} KiB AND swap <= {} KiB",
                self.config.mem_size_warn.unwrap_or(0),
                self.config.swap_size_warn.unwrap_or(0)
            );
            log::info!(
                "  SIGKILL when mem <= {} KiB AND swap <= {} KiB",
                self.config.mem_size_kill.unwrap_or(0),
                self.config.swap_size_kill.unwrap_or(0)
            );
        } else {
            log::info!(
                "  SIGTERM when mem <= {:.1}% AND swap <= {:.1}%",
                self.config.mem_threshold_warn,
                self.config.swap_threshold_warn
            );
            log::info!(
                "  SIGKILL when mem <= {:.1}% AND swap <= {:.1}%",
                self.config.mem_threshold_kill,
                self.config.swap_threshold_kill
            );
        }

        if !self.config.prefer.is_empty() {
            log::info!("Prefer killing: {} pattern(s)", self.config.prefer.len());
        }
        if !self.config.avoid.is_empty() {
            log::info!("Avoid killing: {} pattern(s)", self.config.avoid.len());
        }
        if !self.config.ignore.is_empty() {
            log::info!("Ignore processes: {} pattern(s)", self.config.ignore.len());
        }

        if self.config.dry_run {
            log::warn!("DRY RUN MODE - will not actually kill processes");
        }

        if self.config.kill_group {
            log::info!("Kill process groups enabled");
        }

        if let Some(priority) = self.config.priority {
            log::info!("Daemon priority: {priority}");
        }

        if self.config.adaptive_sleep {
            log::info!(
                "Monitoring: adaptive sleep (100-1000ms), report interval: {}s",
                self.config.report_interval.as_secs()
            );
        } else {
            log::info!(
                "Monitoring interval: {}s, report interval: {}s",
                self.config.check_interval.as_secs(),
                self.config.report_interval.as_secs()
            );
        }
        log::info!("==========================================");

        Ok(())
    }

    /// Check memory and take action if thresholds are exceeded
    fn check_and_act_with_meminfo(&mut self, meminfo: &MemInfo) -> Result<()> {
        log::debug!("Current memory status: {meminfo}");

        // Check if we're in cooldown period after a recent kill
        if let Some(last_kill_time) = self.last_kill {
            let cooldown = Duration::from_secs(10); // 10 second cooldown
            let elapsed = last_kill_time.elapsed();
            if elapsed < cooldown {
                let remaining = cooldown.saturating_sub(elapsed);
                log::debug!(
                    "In cooldown period ({:.1}s remaining)",
                    remaining.as_secs_f64()
                );
                return Ok(());
            }
        }

        // Determine if we need to kill and what strategy to use
        let kill_strategy = self.determine_kill_strategy(meminfo)?;

        if let Some(strategy) = kill_strategy {
            log::warn!("Memory threshold exceeded - using {strategy:?} strategy");

            // Select victim process
            if let Some(victim) = self.select_victim()? {
                self.kill_victim(victim, strategy)?;
                self.last_kill = Some(Instant::now());
            } else {
                log::warn!("No suitable victim process found");
            }
        }

        Ok(())
    }

    /// Calculate adaptive sleep duration based on memory headroom
    ///
    /// Returns Duration between 100ms and 1000ms based on how far we are
    /// from the warning thresholds. When memory is low (close to threshold),
    /// we check more frequently. When memory is plentiful, we check less often.
    ///
    /// Adaptive sleep algorithm (inspired by earlyoom):
    /// - headroom <= 0: 100ms (critical, check frequently)
    /// - headroom >= 20: 1000ms (safe, check less often)
    /// - Linear interpolation between these values
    fn calculate_adaptive_sleep(&self, meminfo: &MemInfo) -> Duration {
        // Constants for adaptive sleep
        const MIN_SLEEP_MS: u64 = 100; // Minimum sleep when critical
        const MAX_SLEEP_MS: u64 = 1000; // Maximum sleep when safe
        const MAX_HEADROOM: f64 = 20.0; // Headroom at which we use max sleep

        // Calculate headroom (how far we are from thresholds)
        let mem_headroom = meminfo.mem_available_percent() - self.config.mem_threshold_warn;
        let swap_headroom = meminfo.swap_free_percent() - self.config.swap_threshold_warn;

        // Use the smaller headroom (most critical resource)
        let headroom = mem_headroom.min(swap_headroom);

        // Map headroom to sleep duration:
        // headroom <= 0: MIN_SLEEP_MS (critical, check frequently)
        // headroom >= MAX_HEADROOM: MAX_SLEEP_MS (safe, check less often)
        let sleep_ms = if headroom <= 0.0 {
            MIN_SLEEP_MS
        } else if headroom >= MAX_HEADROOM {
            MAX_SLEEP_MS
        } else {
            // Linear interpolation: MIN_SLEEP_MS to MAX_SLEEP_MS
            MIN_SLEEP_MS + ((headroom / MAX_HEADROOM) * (MAX_SLEEP_MS - MIN_SLEEP_MS) as f64) as u64
        };

        Duration::from_millis(sleep_ms)
    }

    /// Determine if we need to kill a process and what strategy to use
    fn determine_kill_strategy(&self, meminfo: &MemInfo) -> Result<Option<KillStrategy>> {
        // Check kill threshold first (more aggressive - SIGKILL)
        let mem_critical = if let Some(kb) = self.config.mem_size_kill {
            meminfo.is_mem_below_threshold_kb(kb)
        } else {
            meminfo.is_mem_below_threshold(self.config.mem_threshold_kill)
        };

        let swap_critical = if let Some(kb) = self.config.swap_size_kill {
            meminfo.is_swap_below_threshold_kb(kb)
        } else {
            meminfo.is_swap_below_threshold(self.config.swap_threshold_kill)
        };

        if mem_critical && swap_critical {
            log::warn!(
                "Critical thresholds exceeded: mem={:.1}%, swap={:.1}%",
                meminfo.mem_available_percent(),
                meminfo.swap_free_percent()
            );
            return Ok(Some(KillStrategy::Forceful));
        }

        // Check warn threshold (less aggressive - SIGTERM)
        let mem_low = if let Some(kb) = self.config.mem_size_warn {
            meminfo.is_mem_below_threshold_kb(kb)
        } else {
            meminfo.is_mem_below_threshold(self.config.mem_threshold_warn)
        };

        let swap_low = if let Some(kb) = self.config.swap_size_warn {
            meminfo.is_swap_below_threshold_kb(kb)
        } else {
            meminfo.is_swap_below_threshold(self.config.swap_threshold_warn)
        };

        if mem_low && swap_low {
            log::warn!(
                "Warning thresholds exceeded: mem={:.1}%, swap={:.1}%",
                meminfo.mem_available_percent(),
                meminfo.swap_free_percent()
            );
            return Ok(Some(KillStrategy::Graceful));
        }

        Ok(None)
    }

    /// Select a victim process to kill
    fn select_victim(&self) -> Result<Option<ProcessInfo>> {
        let mut processes = ProcessInfo::all_processes().context("Failed to get process list")?;

        // Filter out processes based on ignore patterns
        processes.retain(|p| !self.should_ignore(p));

        // Filter out root processes if configured
        if self.config.ignore_root_user {
            processes.retain(|p| p.uid != 0);
        }

        // Apply avoid patterns with lower priority
        let (avoided, mut candidates): (Vec<_>, Vec<_>) =
            processes.into_iter().partition(|p| self.should_avoid(p));

        // Apply prefer patterns
        let mut preferred: Vec<_> = candidates
            .iter()
            .filter(|p| self.should_prefer(p))
            .cloned()
            .collect();

        // Sort by selection criteria
        if self.config.sort_by_rss {
            preferred.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));
            candidates.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));
        } else {
            preferred.sort_by(|a, b| b.oom_score.cmp(&a.oom_score));
            candidates.sort_by(|a, b| b.oom_score.cmp(&a.oom_score));
        }

        // Select from preferred first, then candidates, then avoided
        if let Some(victim) = preferred.first() {
            log::info!("Selected preferred victim: {victim}");
            return Ok(Some(victim.clone()));
        }

        if let Some(victim) = candidates.first() {
            log::info!("Selected candidate victim: {victim}");
            return Ok(Some(victim.clone()));
        }

        if let Some(victim) = avoided.first() {
            log::warn!("No candidates available, selecting from avoided: {victim}");
            return Ok(Some(victim.clone()));
        }

        Ok(None)
    }

    /// Check if a process should be ignored completely
    fn should_ignore(&self, process: &ProcessInfo) -> bool {
        // Always ignore our own process
        if process.pid == std::process::id() as i32 {
            return true;
        }

        // Always ignore PID 1 (init)
        if process.pid == 1 {
            return true;
        }

        // Never kill protected processes (oom_score_adj = -1000)
        if process.oom_score_adj == -1000 {
            log::debug!(
                "Ignoring process {} (protected with oom_score_adj=-1000)",
                process.pid
            );
            return true;
        }

        // Never kill zombie processes (already dead)
        if process.is_zombie {
            log::debug!("Ignoring process {} (zombie)", process.pid);
            return true;
        }

        // Check ignore patterns
        for pattern in &self.config.ignore {
            if pattern.is_match(&process.cmdline) || pattern.is_match(&process.name) {
                log::debug!("Ignoring process {} (matches ignore pattern)", process.pid);
                return true;
            }
        }

        false
    }

    /// Check if a process should be avoided (but can be killed if necessary)
    fn should_avoid(&self, process: &ProcessInfo) -> bool {
        for pattern in &self.config.avoid {
            if pattern.is_match(&process.cmdline) || pattern.is_match(&process.name) {
                log::debug!("Avoiding process {} (matches avoid pattern)", process.pid);
                return true;
            }
        }
        false
    }

    /// Check if a process should be preferred for killing
    fn should_prefer(&self, process: &ProcessInfo) -> bool {
        for pattern in &self.config.prefer {
            if pattern.is_match(&process.cmdline) || pattern.is_match(&process.name) {
                log::debug!(
                    "Preferring process {} (matches prefer pattern)",
                    process.pid
                );
                return true;
            }
        }
        false
    }

    /// Kill the selected victim process
    fn kill_victim(&self, victim: ProcessInfo, strategy: KillStrategy) -> Result<()> {
        // Double-check: re-verify memory situation before killing
        let meminfo = MemInfo::read()?;
        let still_critical = self.determine_kill_strategy(&meminfo)?;

        if still_critical.is_none() {
            log::info!(
                "Memory situation improved, skipping kill of {} ({})",
                victim.pid,
                sanitize_for_log(&victim.name)
            );
            return Ok(());
        }

        log::warn!(
            "Killing process {} ({}) - RSS: {} KiB, Strategy: {:?}",
            victim.pid,
            sanitize_for_log(&victim.name),
            victim.rss_kb,
            strategy
        );

        if self.config.dry_run {
            log::info!(
                "DRY RUN: Would kill process {} ({})",
                victim.pid,
                sanitize_for_log(&victim.name)
            );
            return Ok(());
        }

        let result = kill_process(victim.pid, strategy, self.config.kill_group)
            .context("Failed to kill process")?;

        let kill_info = KillInfo::new(
            victim.pid,
            victim.name.clone(),
            victim.cmdline.clone(),
            victim.uid,
            victim.rss_kb,
            victim.oom_score,
            strategy,
            &result,
        );

        if result.is_success() {
            log::info!(
                "Successfully killed process {} ({}): {}",
                victim.pid,
                sanitize_for_log(&victim.name),
                result.description()
            );

            if self.config.notify {
                self.send_notification(&kill_info)?;
            }
        } else {
            log::error!(
                "Failed to kill process {} ({}): {}",
                victim.pid,
                sanitize_for_log(&victim.name),
                result.description()
            );
        }

        Ok(())
    }

    /// Send notification about killed process via scripts and D-Bus
    fn send_notification(&self, kill_info: &KillInfo) -> Result<()> {
        self.notification_manager.send_post_kill_notification(
            kill_info.pid,
            &kill_info.name,
            &kill_info.cmdline,
            kill_info.uid,
            kill_info.rss_kb,
            kill_info.oom_score,
        )
    }

    /// Report current status
    fn report_status(&self) -> Result<()> {
        let meminfo = MemInfo::read().context("Failed to read memory info")?;

        log::info!("Status Report: {meminfo}");

        if let Some(last_kill_time) = self.last_kill {
            log::info!(
                "Last kill: {:.1}s ago",
                last_kill_time.elapsed().as_secs_f64()
            );
        } else {
            log::info!("No kills yet");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn create_test_meminfo(mem_available_percent: f64, swap_free_percent: f64) -> MemInfo {
        // Create meminfo with specific percentages
        // mem_available / mem_total = mem_available_percent / 100
        let mem_total = 16_000_000; // 16 GB in KiB
        let mem_available = (mem_total as f64 * mem_available_percent / 100.0) as u64;

        let swap_total = 8_000_000; // 8 GB in KiB
        let swap_free = (swap_total as f64 * swap_free_percent / 100.0) as u64;

        MemInfo {
            mem_total,
            mem_available,
            swap_total,
            swap_free,
        }
    }

    #[test]
    fn test_adaptive_sleep_critical() {
        // When memory is critical (below threshold), sleep should be minimum (100ms)
        let config = Config::default(); // warn threshold = 10%
        let service = DaemonService::new(config);

        // Memory at 5% available (below 10% warn threshold)
        let meminfo = create_test_meminfo(5.0, 5.0);
        let duration = service.calculate_adaptive_sleep(&meminfo);

        assert_eq!(duration, Duration::from_millis(100));
    }

    #[test]
    fn test_adaptive_sleep_safe() {
        // When memory is safe (well above threshold), sleep should be maximum (1000ms)
        let config = Config::default(); // warn threshold = 10%
        let service = DaemonService::new(config);

        // Memory at 50% available (40% headroom above 10% threshold, > 20% max)
        let meminfo = create_test_meminfo(50.0, 50.0);
        let duration = service.calculate_adaptive_sleep(&meminfo);

        assert_eq!(duration, Duration::from_millis(1000));
    }

    #[test]
    fn test_adaptive_sleep_interpolation() {
        // When memory is in the middle, sleep should be interpolated
        let config = Config::default(); // warn threshold = 10%
        let service = DaemonService::new(config);

        // Memory at 20% available (10% headroom above 10% threshold)
        // 10% headroom / 20% max headroom = 50% of the way
        // 100 + (0.5 * 900) = 550ms
        let meminfo = create_test_meminfo(20.0, 20.0);
        let duration = service.calculate_adaptive_sleep(&meminfo);

        assert_eq!(duration, Duration::from_millis(550));
    }

    #[test]
    fn test_adaptive_sleep_uses_minimum_headroom() {
        // Should use the smaller headroom (memory or swap)
        let config = Config::default(); // warn threshold = 10%
        let service = DaemonService::new(config);

        // Memory at 50% but swap at 12% (only 2% headroom)
        // 2% headroom / 20% max headroom = 10% of the way
        // 100 + (0.1 * 900) = 190ms
        let meminfo = create_test_meminfo(50.0, 12.0);
        let duration = service.calculate_adaptive_sleep(&meminfo);

        assert_eq!(duration, Duration::from_millis(190));
    }

    #[test]
    fn test_adaptive_sleep_exactly_at_threshold() {
        // When exactly at threshold (headroom = 0), should be minimum
        let config = Config::default(); // warn threshold = 10%
        let service = DaemonService::new(config);

        // Memory exactly at 10% threshold
        let meminfo = create_test_meminfo(10.0, 10.0);
        let duration = service.calculate_adaptive_sleep(&meminfo);

        assert_eq!(duration, Duration::from_millis(100));
    }

    #[test]
    fn test_adaptive_sleep_at_max_headroom() {
        // When headroom is exactly at max (20%), should be maximum sleep
        let config = Config::default(); // warn threshold = 10%
        let service = DaemonService::new(config);

        // Memory at 30% (20% headroom above 10% threshold)
        let meminfo = create_test_meminfo(30.0, 30.0);
        let duration = service.calculate_adaptive_sleep(&meminfo);

        assert_eq!(duration, Duration::from_millis(1000));
    }
}
