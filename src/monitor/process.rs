// Process information and selection

use anyhow::Result;
use procfs::process::Process;
use std::fs;

/// Information about a process
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: i32,
    pub name: String,
    pub cmdline: String,
    pub rss_kb: u64,
    pub oom_score: i32,
    pub uid: u32,
}

impl ProcessInfo {
    /// Read information about a specific process
    pub fn read(pid: i32) -> Result<Self> {
        let process = Process::new(pid)?;
        let stat = process.stat()?;
        let status = process.status()?;
        
        // Get RSS in KiB (stat.rss is in pages, typically 4KB)
        let page_size = procfs::page_size();
        let rss_kb = (stat.rss * page_size) / 1024;
        
        // Get OOM score
        let oom_score = process.oom_score().unwrap_or(0);
        
        // Get UID
        let uid = status.ruid;
        
        // Get command line
        let cmdline = process
            .cmdline()
            .unwrap_or_default()
            .join(" ");
        
        let cmdline = if cmdline.is_empty() {
            format!("[{}]", stat.comm)
        } else {
            cmdline
        };
        
        Ok(Self {
            pid,
            name: stat.comm,
            cmdline,
            rss_kb: rss_kb as u64,
            oom_score: oom_score as i32,
            uid,
        })
    }
    
    /// Get all processes on the system
    pub fn all_processes() -> Result<Vec<Self>> {
        let mut processes = Vec::new();
        
        for entry in fs::read_dir("/proc")? {
            let entry = entry?;
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            
            // Check if directory name is a number (PID)
            if let Ok(pid) = name.parse::<i32>() {
                if let Ok(info) = Self::read(pid) {
                    processes.push(info);
                }
            }
        }
        
        Ok(processes)
    }
}

impl std::fmt::Display for ProcessInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PID {} ({}): {} KiB, OOM score {}",
            self.pid,
            self.name,
            self.rss_kb,
            self.oom_score
        )
    }
}
