// Memory information parsing from /proc/meminfo

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Memory information structure
#[derive(Debug, Clone, Copy, Default)]
pub struct MemInfo {
    /// Total physical memory in KiB
    pub mem_total: u64,
    /// Available memory in KiB (more accurate than free)
    pub mem_available: u64,
    /// Total swap space in KiB
    pub swap_total: u64,
    /// Free swap space in KiB
    pub swap_free: u64,
}

impl MemInfo {
    /// Read memory information from /proc/meminfo
    pub fn read() -> Result<Self> {
        Self::read_from_path("/proc/meminfo")
    }

    /// Read memory information from a specific path (for testing)
    fn read_from_path(path: &str) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("Failed to open {path}"))?;
        let reader = BufReader::new(file);

        let mut info = Self::default();

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.len() < 2 {
                continue;
            }

            let key = parts[0].trim_end_matches(':');
            let value: u64 = parts[1]
                .parse()
                .with_context(|| format!("Failed to parse value for {key}"))?;

            match key {
                "MemTotal" => info.mem_total = value,
                "MemAvailable" => info.mem_available = value,
                "SwapTotal" => info.swap_total = value,
                "SwapFree" => info.swap_free = value,
                _ => {}
            }
        }

        // Validate that we got all required fields
        if info.mem_total == 0 {
            anyhow::bail!("Failed to read MemTotal from {path}");
        }

        Ok(info)
    }

    /// Calculate percentage of available memory
    pub fn mem_available_percent(&self) -> f64 {
        if self.mem_total == 0 {
            return 0.0;
        }
        (self.mem_available as f64 / self.mem_total as f64) * 100.0
    }

    /// Calculate percentage of free swap
    pub fn swap_free_percent(&self) -> f64 {
        if self.swap_total == 0 {
            return 100.0; // No swap means we're not using any
        }
        (self.swap_free as f64 / self.swap_total as f64) * 100.0
    }

    /// Calculate percentage of used memory
    pub fn mem_used_percent(&self) -> f64 {
        100.0 - self.mem_available_percent()
    }

    /// Calculate percentage of used swap
    pub fn swap_used_percent(&self) -> f64 {
        100.0 - self.swap_free_percent()
    }

    /// Check if memory is below threshold (percentage)
    pub fn is_mem_below_threshold(&self, threshold_percent: f64) -> bool {
        self.mem_available_percent() < threshold_percent
    }

    /// Check if memory is below threshold (absolute KiB)
    pub const fn is_mem_below_threshold_kb(&self, threshold_kb: u64) -> bool {
        self.mem_available < threshold_kb
    }

    /// Check if swap is below threshold (percentage)
    pub fn is_swap_below_threshold(&self, threshold_percent: f64) -> bool {
        self.swap_free_percent() < threshold_percent
    }

    /// Check if swap is below threshold (absolute KiB)
    pub const fn is_swap_below_threshold_kb(&self, threshold_kb: u64) -> bool {
        self.swap_free < threshold_kb
    }

    /// Format memory size in human-readable format
    pub fn format_size(kb: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if kb >= GB {
            format!("{:.2} GiB", kb as f64 / GB as f64)
        } else if kb >= MB {
            format!("{:.2} MiB", kb as f64 / MB as f64)
        } else if kb >= KB {
            format!("{:.2} KiB", kb as f64 / KB as f64)
        } else {
            format!("{kb} KiB")
        }
    }
}

impl std::fmt::Display for MemInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Memory: {}/{} ({:.1}% available), Swap: {}/{} ({:.1}% free)",
            Self::format_size(self.mem_available),
            Self::format_size(self.mem_total),
            self.mem_available_percent(),
            Self::format_size(self.swap_free),
            Self::format_size(self.swap_total),
            self.swap_free_percent(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_percentages() {
        let info = MemInfo {
            mem_total: 16_000_000,
            mem_available: 8_000_000,
            swap_total: 8_000_000,
            swap_free: 4_000_000,
        };

        assert_eq!(info.mem_available_percent(), 50.0);
        assert_eq!(info.mem_used_percent(), 50.0);
        assert_eq!(info.swap_free_percent(), 50.0);
        assert_eq!(info.swap_used_percent(), 50.0);
    }

    #[test]
    fn test_thresholds() {
        let info = MemInfo {
            mem_total: 16_000_000,
            mem_available: 1_600_000, // 10%
            swap_total: 8_000_000,
            swap_free: 800_000, // 10%
        };

        assert!(info.is_mem_below_threshold(15.0));
        assert!(!info.is_mem_below_threshold(5.0));
        assert!(info.is_mem_below_threshold_kb(2_000_000));
        assert!(!info.is_mem_below_threshold_kb(1_000_000));

        assert!(info.is_swap_below_threshold(15.0));
        assert!(!info.is_swap_below_threshold(5.0));
        assert!(info.is_swap_below_threshold_kb(1_000_000));
        assert!(!info.is_swap_below_threshold_kb(500_000));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(MemInfo::format_size(512), "512 KiB");
        assert_eq!(MemInfo::format_size(1024), "1.00 KiB");
        assert_eq!(MemInfo::format_size(1536), "1.50 KiB");
        assert_eq!(MemInfo::format_size(1024 * 1024), "1.00 MiB");
        assert_eq!(MemInfo::format_size(1024 * 1024 * 1024), "1.00 GiB");
    }
}
