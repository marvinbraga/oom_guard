# OOM Guard

<div align="center">

[![CI](https://github.com/marvinbraga/oom_guard/actions/workflows/ci.yml/badge.svg)](https://github.com/marvinbraga/oom_guard/actions/workflows/ci.yml)
[![Release](https://github.com/marvinbraga/oom_guard/actions/workflows/release.yml/badge.svg)](https://github.com/marvinbraga/oom_guard/actions/workflows/release.yml)
[![License: GPL-2.0](https://img.shields.io/badge/License-GPL%202.0-blue.svg)](LICENSE)
[![GitHub release](https://img.shields.io/github/v/release/marvinbraga/oom_guard)](https://github.com/marvinbraga/oom_guard/releases)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

**A memory monitoring daemon that prevents system freezes by killing memory-hogging processes before the kernel's OOM killer activates.**

[Features](#features) •
[Installation](#installation) •
[Usage](#usage) •
[Documentation](#how-it-works) •
[Contributing](#contributing)

</div>

---

## 🚀 Quick Start

```bash
# Download and run installer
curl -L https://raw.githubusercontent.com/marvinbraga/oom_guard/main/install.sh | sudo bash

# Or clone and install
git clone https://github.com/marvinbraga/oom_guard.git
cd oom_guard
sudo ./install.sh
```

The installer will:
- ✅ Auto-install Rust if needed, or
- ✅ Download pre-compiled binary from GitHub Releases
- ✅ Configure systemd service
- ✅ Start monitoring immediately

---

## ✨ Features

<table>
<tr>
<td width="50%">

**Core Functionality**
- 🎯 Real-time RAM and swap monitoring
- ⚡ Configurable memory thresholds (% or absolute)
- 🎲 Process selection by OOM score or RSS
- 🔍 Regex-based filtering (prefer/avoid/ignore)
- 🔒 Memory locking to prevent daemon swapping
- ⏱️ Adaptive sleep (100ms-1000ms based on headroom)

</td>
<td width="50%">

**Advanced Features**
- 📜 Pre/post-kill script hooks
- 🔔 D-Bus desktop notifications
- 🧪 Dry-run mode for testing
- 🐧 Systemd integration with hardening
- 👥 Process group killing support
- 🚀 pidfd + process_mrelease (Linux 5.3+/5.14+)

</td>
</tr>
</table>

### Why OOM Guard?

| Problem | Solution |
|---------|----------|
| 💥 System freezes when RAM is full | Proactive monitoring kills processes **before** freeze |
| 🐌 Kernel OOM killer acts too late | Configurable thresholds (10% default, customizable) |
| 🎯 Wrong process killed | Smart selection by OOM score, RSS, or regex filters |
| 🔧 Manual intervention needed | Automated daemon with systemd integration |

---

## 📦 Installation

### Option 1: Quick Install (Recommended)

The installation script handles everything automatically:

```bash
git clone https://github.com/marvinbraga/oom_guard.git
cd oom_guard
sudo ./install.sh
```

**What it does:**
1. Detects if Rust is installed
2. Offers to install Rust via rustup, OR
3. Downloads pre-compiled binary from GitHub Releases
4. Installs to `/usr/local/bin/oom_guard`
5. Sets up systemd service
6. Creates config at `/etc/default/oom_guard`

### Option 2: Download Pre-compiled Binary

Pre-compiled binaries available for:

```bash
# x86_64 Linux (Ubuntu, Debian, Fedora, RHEL)
curl -L -o oom_guard https://github.com/marvinbraga/oom_guard/releases/latest/download/oom_guard-linux-x86_64
chmod +x oom_guard
sudo mv oom_guard /usr/local/bin/

# x86_64 Static (Alpine, minimal containers)
curl -L -o oom_guard https://github.com/marvinbraga/oom_guard/releases/latest/download/oom_guard-linux-x86_64-musl

# ARM64 (Raspberry Pi 4, AWS Graviton, Apple Silicon)
curl -L -o oom_guard https://github.com/marvinbraga/oom_guard/releases/latest/download/oom_guard-linux-aarch64
```

**Supported Platforms:**

| Platform | Binary | Tested On |
|----------|--------|-----------|
| x86_64 (glibc) | `oom_guard-linux-x86_64` | Ubuntu 20.04+, Debian 10+, Fedora 33+ |
| x86_64 (musl) | `oom_guard-linux-x86_64-musl` | Alpine Linux, Minimal containers |
| ARM64 | `oom_guard-linux-aarch64` | Raspberry Pi 4, AWS Graviton2/3 |

### Option 3: Build from Source

**Prerequisites:**
- Rust toolchain 1.70+
- Linux with `/proc` filesystem

```bash
git clone https://github.com/marvinbraga/oom_guard.git
cd oom_guard
cargo build --release

# Binary at: target/release/oom_guard
```

---

## 🎮 Usage

### Command Line Interface

```bash
oom_guard [OPTIONS]
```

#### Memory Thresholds

| Flag | Description | Example |
|------|-------------|---------|
| `-m PERCENT[,KILL]` | Memory threshold (warn, kill) | `-m 15,5` → warn 15%, kill 5% |
| `-s PERCENT[,KILL]` | Swap threshold (warn, kill) | `-s 20,10` |
| `-M SIZE[,KILL_SIZE]` | Memory in KiB (absolute) | `-M 1048576,524288` → 1GB/512MB |
| `-S SIZE[,KILL_SIZE]` | Swap in KiB (absolute) | `-S 524288,262144` |

#### Process Selection

| Flag | Description | Example |
|------|-------------|---------|
| `--prefer REGEX` | Prefer killing these | `--prefer "chrome\|firefox"` |
| `--avoid REGEX` | Avoid killing these | `--avoid "ssh\|tmux"` |
| `--ignore REGEX` | Never kill these | `--ignore "^systemd$"` |
| `--sort-by-rss` | Sort by RSS instead of oom_score | `--sort-by-rss` |
| `--ignore-root-user` | Never kill root processes | `--ignore-root-user` |

#### Behavior

| Flag | Description |
|------|-------------|
| `-g` | Kill entire process group |
| `-r SECONDS` | Report interval (default: 60s) |
| `-p PRIORITY` | Set daemon priority (-20 to 19) |
| `--dryrun` | Test mode (don't actually kill) |
| `-n` | Enable D-Bus notifications |
| `-N /path/script` | Post-kill script |
| `-P /path/script` | Pre-kill script |
| `-d` | Debug output |
| `--syslog` | Use syslog instead of stdout (requires feature) |

### Example Commands

```bash
# Basic usage with defaults (10% memory, 10% swap)
sudo oom_guard -m 10,5 -s 10,5

# Conservative thresholds with notifications
sudo oom_guard -m 15,10 -s 20,10 -n -r 3600

# Aggressive settings for production servers
sudo oom_guard -m 5,2 -s 5,2 -g -p -20

# Prefer killing browsers, avoid critical services
sudo oom_guard -m 10,5 \
  --prefer "(chrome|firefox|chromium)" \
  --avoid "(ssh|systemd|postgres|nginx)"

# Test without killing anything
sudo oom_guard --dryrun -m 20 -d

# Using absolute memory values (2GB warn, 1GB kill)
sudo oom_guard -M 2097152,1048576 -S 1048576,524288
```

### Hook Scripts

Scripts receive these environment variables:

```bash
OOM_GUARD_PID      # Process ID
OOM_GUARD_NAME     # Process name
OOM_GUARD_CMDLINE  # Full command line
OOM_GUARD_UID      # User ID of process owner
OOM_GUARD_RSS      # Memory usage in KiB
OOM_GUARD_SCORE    # OOM score
```

**Example post-kill script:**

```bash
#!/bin/bash
# /usr/local/bin/oom-notify.sh

# Log to file
echo "$(date): Killed $OOM_GUARD_NAME (PID $OOM_GUARD_PID, RSS ${OOM_GUARD_RSS}KB)" \
  >> /var/log/oom_guard_kills.log

# Send to Slack
curl -X POST https://hooks.slack.com/services/YOUR/WEBHOOK/URL \
  -H 'Content-Type: application/json' \
  -d "{\"text\":\"⚠️ OOM Guard killed $OOM_GUARD_NAME (${OOM_GUARD_RSS}KB)\"}"

# Make executable
chmod +x /usr/local/bin/oom-notify.sh

# Use with: oom_guard -m 10,5 -N /usr/local/bin/oom-notify.sh
```

---

## 🔧 Configuration

### Via Systemd Environment File

Edit `/etc/default/oom_guard`:

```bash
# Memory thresholds (warn,kill)
OOM_GUARD_MEM_THRESHOLD=10,5
OOM_GUARD_SWAP_THRESHOLD=10,5

# Filters
OOM_GUARD_PREFER="(chrome|firefox)"
OOM_GUARD_AVOID="(ssh|systemd)"

# Scripts
OOM_GUARD_POST_KILL_SCRIPT=/usr/local/bin/oom-notify.sh

# Options
OOM_GUARD_NOTIFY=true
OOM_GUARD_REPORT_INTERVAL=3600
```

Then restart:
```bash
sudo systemctl restart oom_guard
```

### Systemd Service Management

```bash
# Start/Stop
sudo systemctl start oom_guard
sudo systemctl stop oom_guard

# Enable/Disable at boot
sudo systemctl enable oom_guard
sudo systemctl disable oom_guard

# Status and logs
sudo systemctl status oom_guard
sudo journalctl -u oom_guard -f
sudo journalctl -u oom_guard -n 100 --no-pager
```

---

## 📊 How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                    OOM Guard Flow                           │
└─────────────────────────────────────────────────────────────┘

  ┌──────────────┐
  │  Read        │   Every 1 second (configurable)
  │  /proc/meminfo│
  └──────┬───────┘
         │
         ▼
  ┌──────────────────────────┐
  │  Check Thresholds        │
  │  - MemAvailable < 10%?   │   ◄── SIGTERM threshold
  │  - SwapFree < 10%?       │
  └──────┬──────────┬────────┘
         │ NO       │ YES
         │          ▼
         │   ┌──────────────────┐
         │   │  Select Victim   │
         │   │  1. Filter (ignore/avoid)
         │   │  2. Apply prefer patterns
         │   │  3. Sort by oom_score/RSS
         │   └──────┬───────────┘
         │          │
         │          ▼
         │   ┌──────────────────┐
         │   │  Execute hooks   │
         │   │  - Pre-kill (-P) │
         │   └──────┬───────────┘
         │          │
         │          ▼
         │   ┌──────────────────┐
         │   │  Send SIGTERM    │   ◄── Graceful
         │   │  Wait 1 second   │
         │   └──────┬───────────┘
         │          │ Still alive?
         │          ▼
         │   ┌──────────────────┐
         │   │  Send SIGKILL    │   ◄── Forceful
         │   └──────┬───────────┘
         │          │
         │          ▼
         │   ┌──────────────────┐
         │   │  Execute hooks   │
         │   │  - Post-kill (-N)│
         │   │  - D-Bus notify  │
         │   └──────┬───────────┘
         │          │
         │          ▼
         │   ┌──────────────────┐
         │   │  Cooldown 10s    │   Prevent rapid kills
         │   └──────────────────┘
         │
         └──► Continue monitoring
```

### Key Design Principles

1. **Two-tier thresholds**: Warn threshold (SIGTERM) and kill threshold (SIGKILL)
2. **Smart selection**: Prefers high oom_score processes, applies user filters
3. **Graceful first**: Always try SIGTERM before SIGKILL
4. **Self-protection**: Memory locked, high priority, protected from OOM
5. **Cooldown period**: Prevents rapid consecutive kills

---

## 🧪 Testing

### Simulate Memory Pressure

```bash
# Install stress-ng
sudo apt-get install stress-ng

# Run in dry-run mode first
sudo oom_guard --dryrun -m 20 -s 20 -d

# In another terminal, stress memory
stress-ng --vm 4 --vm-bytes 90% --timeout 60s

# Check logs
sudo journalctl -u oom_guard -f
```

### Unit Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture --test-threads=1

# Run specific test
cargo test test_process_selection
```

---

## ⚡ Performance

Designed for minimal system impact:

| Metric | Value |
|--------|-------|
| Memory usage | < 5 MB RSS |
| CPU usage (idle) | < 0.1% |
| CPU usage (monitoring) | < 1% |
| Monitoring latency | < 1ms |
| Binary size | ~2.5 MB (stripped) |

**Benchmarks** (on 16GB RAM system):
- Memory scan: ~0.5ms for 200 processes
- Threshold check: ~0.1ms
- Process selection: ~2ms for 200 processes

---

## 🔒 Security

### Security Audit

OOM Guard underwent a comprehensive security audit with the following protections:

| Protection | Description |
|------------|-------------|
| **ReDoS Prevention** | Regex patterns limited to 256 chars with compiled size limits |
| **Log Injection** | Process names sanitized before logging |
| **Script Security** | Symlink detection and ownership validation for hooks |
| **Env Var Sanitization** | Shell metacharacters removed from hook environment |
| **Memory Safety** | Rust's ownership model prevents buffer overflows |

### Systemd Hardening

The service file includes security measures:

```ini
CapabilityBoundingSet=CAP_KILL CAP_DAC_OVERRIDE CAP_SYS_NICE CAP_SYS_PTRACE
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/proc
CPUQuota=10%
MemoryMax=50M
```

### Best Practices

✅ **DO:**
- Review hook scripts before execution
- Use absolute paths for scripts
- Test in `--dryrun` mode first
- Monitor logs after deployment
- Set appropriate thresholds for your workload

❌ **DON'T:**
- Run with overly aggressive thresholds (< 5%)
- Ignore processes critical to your services
- Use untrusted scripts in hooks
- Disable memory locking

**Report security issues:** mvbraga@gmail.com

---

## 🐛 Troubleshooting

<details>
<summary><b>Service won't start</b></summary>

```bash
# Check status
sudo systemctl status oom_guard

# View detailed logs
sudo journalctl -u oom_guard -n 50

# Common causes:
# - Permission denied → Need root/sudo
# - Binary not found → Run install.sh again
# - Config error → Check /etc/default/oom_guard
```
</details>

<details>
<summary><b>Processes not being killed</b></summary>

```bash
# Enable debug mode
sudo oom_guard -m 10,5 -d

# Check:
# 1. Are thresholds actually exceeded?
# 2. Are processes protected by --ignore/--avoid?
# 3. Is dry-run mode enabled?
# 4. Check oom_score_adj of processes (protected if -1000)
```
</details>

<details>
<summary><b>Wrong process killed</b></summary>

```bash
# View current process rankings
ps aux --sort=-rss | head -20

# Adjust filters
sudo oom_guard -m 10,5 \
  --prefer "high-memory-app" \
  --avoid "critical-service"

# Or use RSS-based selection
sudo oom_guard -m 10,5 --sort-by-rss
```
</details>

<details>
<summary><b>Hook scripts not executing</b></summary>

```bash
# Check permissions
ls -l /usr/local/bin/your-script.sh
chmod +x /usr/local/bin/your-script.sh

# Test script manually
bash -x /usr/local/bin/your-script.sh

# Check logs for errors
sudo journalctl -u oom_guard | grep -i "hook\|script"
```
</details>

<details>
<summary><b>High CPU usage</b></summary>

```bash
# Increase monitoring interval
sudo oom_guard -m 10,5 -i 5  # Check every 5 seconds

# Check for runaway logging
sudo journalctl -u oom_guard --disk-usage
```
</details>

---

## 📚 FAQ

<details>
<summary><b>How is this different from the kernel OOM killer?</b></summary>

The kernel OOM killer acts as a last resort when memory is **completely exhausted**, often causing:
- System freezes/unresponsiveness
- Long delays before action
- Less predictable victim selection

OOM Guard acts **proactively** at configurable thresholds (e.g., 10% free) to:
- Prevent freezes before they happen
- Give controlled, predictable behavior
- Allow custom selection logic
</details>

<details>
<summary><b>What thresholds should I use?</b></summary>

**Recommended defaults:**
- Desktop/Laptop: `-m 10,5 -s 10,5`
- Server (stable load): `-m 15,10 -s 15,10`
- Server (bursty load): `-m 20,15 -s 20,15`
- Container/VM: `-m 5,2 -s 5,2`

Adjust based on:
- Available RAM (more RAM → lower %)
- Workload predictability
- Importance of uptime vs. process preservation
</details>

<details>
<summary><b>Can I protect specific processes?</b></summary>

Yes, multiple ways:

```bash
# Never kill (ignore completely)
--ignore "postgres|nginx|ssh"

# Avoid unless necessary
--avoid "important-app"

# Set oom_score_adj to -1000 (kernel-level protection)
echo -1000 | sudo tee /proc/$(pidof critical-app)/oom_score_adj
```
</details>

<details>
<summary><b>Does it work with containers?</b></summary>

Yes, with considerations:

- **Docker/Podman:** Mount `/proc` and run privileged
- **Kubernetes:** Use DaemonSet with hostPID
- **Systemd-nspawn:** Should work out of the box

Example Docker:
```bash
docker run --privileged --pid=host \
  -v /proc:/proc \
  oom_guard -m 10,5
```
</details>

---

## 🗺️ Roadmap

- [x] Core memory monitoring
- [x] Process selection and killing
- [x] Systemd integration
- [x] Script hooks
- [x] Multi-platform binaries (x86_64, ARM64, musl)
- [x] Security audit and hardening
- [x] Comprehensive Clippy linting
- [x] Full earlyoom compatibility
- [x] Adaptive sleep (100ms-1000ms based on memory headroom)
- [x] Advanced kernel features (pidfd_open, process_mrelease)
- [x] Syslog support (optional feature)
- [x] Process protection detection (oom_score_adj=-1000)
- [x] Zombie process detection and skipping
- [x] Double-check pattern before killing
- [ ] Prometheus metrics exporter
- [ ] Web dashboard
- [ ] Machine learning-based prediction
- [ ] Cgroup v2 integration
- [ ] Windows support (via WSL2)

---

## 🛠️ Code Quality

This project enforces strict code quality through comprehensive linting:

### Clippy Lints

```bash
# Run linter (same as CI)
cargo clippy

# All warnings treated as errors in CI
cargo clippy -- -D warnings
```

**Enforced Rules:**
- `clippy::all` - All default lints
- `clippy::pedantic` - Extra pedantic checks
- `clippy::nursery` - Experimental improvements

**Security Lints (DENY - Build fails):**
- `unwrap_used` - Prevents panics from unwrap()
- `expect_used` - Prevents panics from expect()
- `panic` - Explicit panics not allowed
- `unimplemented` - No unfinished code

### Code Style

```bash
# Format code
cargo fmt

# Check formatting (CI)
cargo fmt -- --check
```

---

## 🤝 Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes with tests
4. Run tests and linters:
   ```bash
   cargo test
   cargo fmt
   cargo clippy -- -D warnings
   ```
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

---

## 📄 License

This project is licensed under the [GNU General Public License v2.0](LICENSE).

```
OOM Guard - Memory Monitor Daemon
Copyright (C) 2024 Marcus Vinicius Braga

This program is free software; you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation; either version 2 of the License, or
(at your option) any later version.
```

---

## 🙏 Acknowledgments

- Inspired by system administrators dealing with OOM situations
- Built with the amazing Rust ecosystem
- Community feedback and testing

---

## 📞 Support

- **Issues:** [GitHub Issues](https://github.com/marvinbraga/oom_guard/issues)
- **Discussions:** [GitHub Discussions](https://github.com/marvinbraga/oom_guard/discussions)
- **Email:** mvbraga@gmail.com

---

<div align="center">

**Made with ❤️ and Rust**

[⬆ Back to Top](#oom-guard)

</div>
