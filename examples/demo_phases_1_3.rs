// Demonstration of Phases 1-3 implementation
//
// Run with: cargo run --example demo_phases_1_3

use anyhow::Result;
use oom_guard::config::{Args, Config};
use oom_guard::killer::ProcessSelector;
use oom_guard::monitor::{MemInfo, ProcessInfo};

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("=== OOM Guard - Phases 1-3 Demonstration ===\n");

    // PHASE 1: Configuration
    println!("PHASE 1: Configuration");
    println!("{}", "-".repeat(50));

    // Parse args (will use defaults if run without args)
    let args = Args::parse_args();
    let config = Config::from_args(args)?;

    println!("Configuration loaded:");
    println!(
        "  Memory threshold (warn): {:.1}%",
        config.mem_threshold_warn
    );
    println!(
        "  Memory threshold (kill): {:.1}%",
        config.mem_threshold_kill
    );
    println!(
        "  Swap threshold (warn): {:.1}%",
        config.swap_threshold_warn
    );
    println!(
        "  Swap threshold (kill): {:.1}%",
        config.swap_threshold_kill
    );
    println!("  Check interval: {}s", config.check_interval.as_secs());
    println!("  Sort by RSS: {}", config.sort_by_rss);
    println!("  Dry run: {}", config.dry_run);
    println!();

    // PHASE 2: Memory Monitor
    println!("PHASE 2: Memory Monitor");
    println!("{}", "-".repeat(50));

    // Read memory information
    let mem_info = MemInfo::read()?;
    println!("System Memory Information:");
    println!("  Total: {}", MemInfo::format_size(mem_info.mem_total));
    println!(
        "  Available: {} ({:.1}%)",
        MemInfo::format_size(mem_info.mem_available),
        mem_info.mem_available_percent()
    );
    println!(
        "  Swap Total: {}",
        MemInfo::format_size(mem_info.swap_total)
    );
    println!(
        "  Swap Free: {} ({:.1}%)",
        MemInfo::format_size(mem_info.swap_free),
        mem_info.swap_free_percent()
    );
    println!();

    // Check thresholds
    let mem_below_warn = mem_info.is_mem_below_threshold(config.mem_threshold_warn);
    let mem_below_kill = mem_info.is_mem_below_threshold(config.mem_threshold_kill);
    let swap_below_warn = mem_info.is_swap_below_threshold(config.swap_threshold_warn);
    let swap_below_kill = mem_info.is_swap_below_threshold(config.swap_threshold_kill);

    println!("Threshold Status:");
    println!("  Memory below warn threshold: {}", mem_below_warn);
    println!("  Memory below kill threshold: {}", mem_below_kill);
    println!("  Swap below warn threshold: {}", swap_below_warn);
    println!("  Swap below kill threshold: {}", swap_below_kill);
    println!();

    // Read process information
    println!("Reading process information...");
    let processes = ProcessInfo::all_processes()?;
    println!("Found {} processes", processes.len());
    println!();

    // PHASE 3: Process Selector
    println!("PHASE 3: Process Selector");
    println!("{}", "-".repeat(50));

    // Create process selector
    let selector = ProcessSelector::new(config.clone());

    // Get statistics
    let stats = selector.get_statistics(&processes);
    println!("Process Statistics:");
    println!("  Total processes: {}", stats.total);
    println!("  Killable processes: {}", stats.killable);
    println!("  Preferred processes: {}", stats.preferred);
    println!("  Avoided processes: {}", stats.avoided);
    println!("  Ignored processes: {}", stats.ignored);
    println!();

    // Select a victim (for demonstration)
    if let Some(victim) = selector.select_victim(processes.clone()) {
        println!("Selected Victim Process:");
        println!("  PID: {}", victim.pid);
        println!("  Name: {}", victim.name);
        println!("  Command: {}", victim.cmdline);
        println!("  RSS: {}", MemInfo::format_size(victim.rss_kb));
        println!("  OOM Score: {}", victim.oom_score);
        println!("  UID: {}", victim.uid);
        println!();
    } else {
        println!("No suitable victim found (all processes are protected)");
        println!();
    }

    // Show top 10 processes by memory usage
    println!("Top 10 Processes by RSS:");
    println!("{}", "-".repeat(50));
    let mut sorted = processes.clone();
    sorted.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));

    for (i, proc) in sorted.iter().take(10).enumerate() {
        println!(
            "{}. {} (PID {}): {} - OOM score: {}",
            i + 1,
            proc.name,
            proc.pid,
            MemInfo::format_size(proc.rss_kb),
            proc.oom_score
        );
    }
    println!();

    // Show top 10 Processes by OOM score
    println!("Top 10 Processes by OOM Score:");
    println!("{}", "-".repeat(50));
    let mut by_oom = processes;
    by_oom.sort_by(|a, b| b.oom_score.cmp(&a.oom_score));

    for (i, proc) in by_oom.iter().take(10).enumerate() {
        println!(
            "{}. {} (PID {}): OOM={}, RSS={}",
            i + 1,
            proc.name,
            proc.pid,
            proc.oom_score,
            MemInfo::format_size(proc.rss_kb)
        );
    }
    println!();

    println!("=== Demonstration Complete ===");

    Ok(())
}
