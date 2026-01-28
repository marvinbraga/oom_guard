// Process selection logic

use crate::config::Config;
use crate::monitor::ProcessInfo;
use regex::Regex;

/// Process selector that applies filters and selects victims
pub struct ProcessSelector {
    config: Config,
}

impl ProcessSelector {
    /// Create a new process selector with the given configuration
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Select a victim process to kill based on configuration
    pub fn select_victim(&self, processes: Vec<ProcessInfo>) -> Option<ProcessInfo> {
        // Filter processes based on configuration
        let candidates = self.filter_processes(processes);

        if candidates.is_empty() {
            log::debug!("No killable processes found after filtering");
            return None;
        }

        // Select the best victim
        self.select_best_victim(candidates)
    }

    /// Filter processes based on configuration rules
    fn filter_processes(&self, processes: Vec<ProcessInfo>) -> Vec<ProcessInfo> {
        processes
            .into_iter()
            .filter(|p| self.is_killable(p))
            .collect()
    }

    /// Check if a process is killable based on configuration
    fn is_killable(&self, process: &ProcessInfo) -> bool {
        // Never kill pid 1 (init)
        if process.pid == 1 {
            log::trace!("Skipping PID 1 (init)");
            return false;
        }

        // Never kill kernel threads (processes with pid <= max kernel thread pid)
        // Kernel threads typically have ppid = 2 or pid = 2, but we use a safer check
        if self.is_kernel_thread(process) {
            log::trace!("Skipping kernel thread: {}", process.name);
            return false;
        }

        // Check ignore patterns (highest priority - completely skip)
        if self.matches_patterns(&self.config.ignore, process) {
            log::trace!("Process {} matches ignore pattern", process.name);
            return false;
        }

        // Check root user filter
        if self.config.ignore_root_user && process.uid == 0 {
            log::trace!("Skipping root-owned process: {}", process.name);
            return false;
        }

        // Process passed all filters
        true
    }

    /// Check if a process is a kernel thread
    fn is_kernel_thread(&self, process: &ProcessInfo) -> bool {
        // Kernel threads have no command line (just name in brackets)
        process.cmdline.starts_with('[') && process.cmdline.ends_with(']')
    }

    /// Check if process matches any of the given patterns
    fn matches_patterns(&self, patterns: &[Regex], process: &ProcessInfo) -> bool {
        for pattern in patterns {
            if pattern.is_match(&process.name) || pattern.is_match(&process.cmdline) {
                return true;
            }
        }
        false
    }

    /// Select the best victim from filtered candidates
    fn select_best_victim(&self, candidates: Vec<ProcessInfo>) -> Option<ProcessInfo> {
        if candidates.is_empty() {
            return None;
        }

        // Apply prefer patterns by boosting their scores
        let prefer_boost = 1000; // Add to oom_score for preferred processes

        // Create a scoring vector
        let mut scored: Vec<(ProcessInfo, i64)> = candidates
            .into_iter()
            .map(|p| {
                let mut score = if self.config.sort_by_rss {
                    // Use RSS as score (higher RSS = higher score)
                    p.rss_kb as i64
                } else {
                    // Use OOM score (higher score = more likely to kill)
                    p.oom_score as i64
                };

                // Boost score for preferred processes
                if self.matches_patterns(&self.config.prefer, &p) {
                    // Only boost if not avoiding this process
                    if !self.matches_patterns(&self.config.avoid, &p) {
                        log::debug!("Boosting score for preferred process: {}", p.name);
                        score += prefer_boost;
                    }
                }

                // Penalize avoided processes (but don't exclude them completely)
                if self.matches_patterns(&self.config.avoid, &p) {
                    log::debug!("Penalizing score for avoided process: {}", p.name);
                    score = score.saturating_sub(prefer_boost);
                }

                (p, score)
            })
            .collect();

        // Sort by score (descending - highest score first)
        scored.sort_by(|a, b| b.1.cmp(&a.1));

        // Log top candidates
        if log::log_enabled!(log::Level::Debug) {
            log::debug!("Top candidates for killing:");
            for (i, (proc, score)) in scored.iter().take(5).enumerate() {
                log::debug!(
                    "  {}. {} (PID {}): score={}, RSS={} KiB, OOM={}",
                    i + 1,
                    proc.name,
                    proc.pid,
                    score,
                    proc.rss_kb,
                    proc.oom_score
                );
            }
        }

        // Return the highest scored process
        scored.into_iter().next().map(|(p, _)| p)
    }

    /// Get statistics about filtered processes
    pub fn get_statistics(&self, processes: &[ProcessInfo]) -> ProcessStatistics {
        let total = processes.len();
        let killable = processes.iter().filter(|p| self.is_killable(p)).count();
        let preferred = processes
            .iter()
            .filter(|p| self.matches_patterns(&self.config.prefer, p))
            .count();
        let avoided = processes
            .iter()
            .filter(|p| self.matches_patterns(&self.config.avoid, p))
            .count();
        let ignored = processes
            .iter()
            .filter(|p| self.matches_patterns(&self.config.ignore, p))
            .count();

        ProcessStatistics {
            total,
            killable,
            preferred,
            avoided,
            ignored,
        }
    }
}

/// Statistics about process filtering
#[derive(Debug, Clone)]
pub struct ProcessStatistics {
    pub total: usize,
    pub killable: usize,
    pub preferred: usize,
    pub avoided: usize,
    pub ignored: usize,
}

impl std::fmt::Display for ProcessStatistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Processes: {} total, {} killable, {} preferred, {} avoided, {} ignored",
            self.total, self.killable, self.preferred, self.avoided, self.ignored
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_process(pid: i32, name: &str, cmdline: &str, rss_kb: u64, oom_score: i32) -> ProcessInfo {
        ProcessInfo {
            pid,
            name: name.to_string(),
            cmdline: cmdline.to_string(),
            rss_kb,
            oom_score,
            uid: 1000,
        }
    }

    #[test]
    fn test_kernel_thread_detection() {
        let config = Config::default();
        let selector = ProcessSelector::new(config);

        let kernel_thread = create_test_process(2, "kthreadd", "[kthreadd]", 0, 0);
        assert!(selector.is_kernel_thread(&kernel_thread));

        let user_process = create_test_process(1234, "firefox", "/usr/bin/firefox", 1000000, 100);
        assert!(!selector.is_kernel_thread(&user_process));
    }

    #[test]
    fn test_pid1_protection() {
        let config = Config::default();
        let selector = ProcessSelector::new(config);

        let init = create_test_process(1, "systemd", "/sbin/init", 10000, 0);
        assert!(!selector.is_killable(&init));
    }

    #[test]
    fn test_ignore_pattern() {
        let mut config = Config::default();
        config.ignore.push(Regex::new("^firefox$").unwrap());

        let selector = ProcessSelector::new(config);

        let firefox = create_test_process(1234, "firefox", "/usr/bin/firefox", 1000000, 100);
        assert!(!selector.is_killable(&firefox));

        let chrome = create_test_process(1235, "chrome", "/usr/bin/chrome", 1000000, 100);
        assert!(selector.is_killable(&chrome));
    }

    #[test]
    fn test_prefer_pattern() {
        let mut config = Config::default();
        config.prefer.push(Regex::new("chrome").unwrap());

        let selector = ProcessSelector::new(config);

        let chrome = create_test_process(1234, "chrome", "/usr/bin/chrome", 100000, 10);
        let firefox = create_test_process(1235, "firefox", "/usr/bin/firefox", 200000, 20);

        let candidates = vec![chrome.clone(), firefox.clone()];
        let victim = selector.select_best_victim(candidates);

        assert!(victim.is_some());
        // Chrome should be selected even though it has lower OOM score,
        // because it matches the prefer pattern
        assert_eq!(victim.unwrap().pid, 1234);
    }

    #[test]
    fn test_avoid_pattern() {
        let mut config = Config::default();
        config.avoid.push(Regex::new("important").unwrap());

        let selector = ProcessSelector::new(config);

        let important = create_test_process(1234, "important-app", "/usr/bin/important-app", 500000, 100);
        let regular = create_test_process(1235, "regular-app", "/usr/bin/regular-app", 100000, 50);

        let candidates = vec![important.clone(), regular.clone()];
        let victim = selector.select_best_victim(candidates);

        assert!(victim.is_some());
        // Regular app should be selected even though important has higher score,
        // because important matches avoid pattern
        assert_eq!(victim.unwrap().pid, 1235);
    }

    #[test]
    fn test_sort_by_rss() {
        let mut config = Config::default();
        config.sort_by_rss = true;

        let selector = ProcessSelector::new(config);

        let small = create_test_process(1234, "small", "/usr/bin/small", 10000, 100);
        let large = create_test_process(1235, "large", "/usr/bin/large", 1000000, 10);

        let candidates = vec![small.clone(), large.clone()];
        let victim = selector.select_best_victim(candidates);

        assert!(victim.is_some());
        // Large should be selected because it has more RSS
        assert_eq!(victim.unwrap().pid, 1235);
    }

    #[test]
    fn test_root_user_filter() {
        let mut config = Config::default();
        config.ignore_root_user = true;

        let selector = ProcessSelector::new(config);

        let mut root_process = create_test_process(1234, "root-daemon", "/usr/sbin/daemon", 100000, 50);
        root_process.uid = 0;

        assert!(!selector.is_killable(&root_process));

        let user_process = create_test_process(1235, "user-app", "/usr/bin/app", 100000, 50);
        assert!(selector.is_killable(&user_process));
    }
}
