# OOM Guard Usage Manual

Complete guide to using OOM Guard, a memory monitoring daemon that prevents system freezes.

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Configuration](#configuration)
- [Command Line Options](#command-line-options)
- [Systemd Service](#systemd-service)
- [Environment Variables](#environment-variables)
- [Hook Scripts](#hook-scripts)
- [Process Selection](#process-selection)
- [Testing](#testing)
- [Troubleshooting](#troubleshooting)
- [Advanced Usage](#advanced-usage)
- [Examples](#examples)

## Quick Start

Install and start monitoring in 3 commands:

```bash
# Install
sudo ./install.sh

# Check status
sudo systemctl status oom_guard

# View logs
sudo journalctl -u oom_guard -f
```

Default configuration:
- **SIGTERM** when memory ≤ 10% AND swap ≤ 10%
- **SIGKILL** when memory ≤ 5% AND swap ≤ 5%

## Installation

### Automated Installation

```bash
git clone https://github.com/marvinbraga/oom_guard.git
cd oom_guard
sudo ./install.sh
```

The installer will:
1. Detect if Rust is installed
2. Offer to install Rust via rustup, OR download pre-compiled binary
3. Install to `/usr/local/bin/oom_guard`
4. Set up systemd service
5. Start monitoring automatically

### Manual Installation

```bash
# Build from source
cargo build --release

# Install binary
sudo cp target/release/oom_guard /usr/local/bin/
sudo chmod +x /usr/local/bin/oom_guard

# Install systemd service
sudo cp systemd/oom_guard.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable oom_guard
sudo systemctl start oom_guard
```

### Download Pre-compiled Binary

```bash
# x86_64 Linux
curl -L -o oom_guard https://github.com/marvinbraga/oom_guard/releases/latest/download/oom_guard-linux-x86_64
chmod +x oom_guard
sudo mv oom_guard /usr/local/bin/

# ARM64
curl -L -o oom_guard https://github.com/marvinbraga/oom_guard/releases/latest/download/oom_guard-linux-aarch64
chmod +x oom_guard
sudo mv oom_guard /usr/local/bin/
```

## Configuration

OOM Guard can be configured via:
1. Command line arguments
2. Environment variables
3. Systemd service file

### Configuration Priority

Command line arguments override environment variables:

```bash
# This will use -m 15,10 (CLI overrides env var)
export OOM_GUARD_MEM_WARN=20
oom_guard -m 15,10
```

## Command Line Options

### Memory Thresholds

```bash
# Percentage-based (recommended for most use cases)
-m, --mem <PERCENT[,KILL_PERCENT]>
    Set memory thresholds as percentages
    Examples:
      -m 10      # Warn at 10%, kill at 5% (50% of warn)
      -m 15,10   # Warn at 15%, kill at 10%

-s, --swap <PERCENT[,KILL_PERCENT]>
    Set swap thresholds as percentages
    Examples:
      -s 10      # Warn at 10%, kill at 5%
      -s 20,10   # Warn at 20%, kill at 10%

# Absolute size-based (for specific memory requirements)
-M, --mem-size <SIZE[,KILL_SIZE]>
    Set memory thresholds in KiB
    Examples:
      -M 1048576           # Warn at 1GB remaining
      -M 2097152,1048576   # Warn at 2GB, kill at 1GB

-S, --swap-size <SIZE[,KILL_SIZE]>
    Set swap thresholds in KiB
    Examples:
      -S 524288            # Warn at 512MB remaining
      -S 1048576,524288    # Warn at 1GB, kill at 512MB
```

### Monitoring Intervals

```bash
-i, --interval <SECONDS>
    Memory check interval (default: 1 second)
    Examples:
      -i 2    # Check every 2 seconds
      -i 5    # Check every 5 seconds (less CPU usage)

-r, --report <SECONDS>
    Status report interval (default: 60 seconds)
    Examples:
      -r 300   # Report every 5 minutes
      -r 3600  # Report every hour
      -r 0     # Disable periodic reports
```

### Process Selection

```bash
--prefer <REGEX>
    Prefer killing processes matching regex (boosts oom_score by 1000)
    Can be used multiple times
    Examples:
      --prefer "chrome"
      --prefer "(firefox|chromium)"
      --prefer "memory-hog.*"

--avoid <REGEX>
    Avoid killing processes matching regex (reduces oom_score by 1000)
    Can be used multiple times
    Examples:
      --avoid "ssh"
      --avoid "(systemd|sshd|postgres)"
      --avoid "critical-service"

--ignore <REGEX>
    Completely ignore processes matching regex (never kill)
    Can be used multiple times
    Examples:
      --ignore "^systemd$"
      --ignore "(sshd|nginx|postgres)"
      --ignore "backup-.*"

--sort-by-rss
    Sort processes by RSS memory usage instead of oom_score
    Useful when you want to kill the largest memory consumer

--ignore-root-user
    Never kill processes owned by root user
    Protects system services
```

### Notifications & Hooks

```bash
-n, --notify
    Enable D-Bus desktop notifications when killing processes
    Requires 'dbus-notify' feature at compile time

-N, --post-kill-script <PATH>
    Execute script after killing a process
    Script receives environment variables with process info
    Example:
      -N /usr/local/bin/notify-slack.sh

-P, --pre-kill-script <PATH>
    Execute script before killing a process
    Useful for logging or alerts
    Example:
      -P /usr/local/bin/pre-kill-log.sh
```

### Behavior Options

```bash
-g, --kill-group
    Kill entire process group instead of just the process
    Ensures child processes are also terminated

-p, --set-priority <PRIORITY>
    Set daemon priority (-20 to 19, lower = higher priority)
    Examples:
      --set-priority=-20  # Maximum priority (recommended)
      --set-priority=0    # Normal priority
      --set-priority=10   # Lower priority

-d, --debug
    Enable debug logging
    Shows detailed memory checks and process selections

--dryrun
    Test mode - don't actually kill processes
    Just report what would be killed
    Useful for testing configuration

--syslog
    Use syslog instead of stdout/stderr
    Requires 'syslog' feature at compile time
```

### Help & Version

```bash
-h, --help
    Display help information

-V, --version
    Display version information
```

## Systemd Service

### Service Management

```bash
# Start service
sudo systemctl start oom_guard

# Stop service
sudo systemctl stop oom_guard

# Restart service
sudo systemctl restart oom_guard

# Enable at boot
sudo systemctl enable oom_guard

# Disable at boot
sudo systemctl disable oom_guard

# Check status
sudo systemctl status oom_guard

# View logs
sudo journalctl -u oom_guard -f

# View last 100 lines
sudo journalctl -u oom_guard -n 100 --no-pager
```

### Service Configuration

Edit `/etc/systemd/system/oom_guard.service`:

```ini
[Unit]
Description=OOM Guard - Memory Monitor Daemon
Documentation=https://github.com/marvinbraga/oom_guard
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/oom_guard -m 10,5 -s 10,5 -n -r 3600 --set-priority=-20
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security hardening
CapabilityBoundingSet=CAP_KILL CAP_DAC_OVERRIDE CAP_SYS_NICE CAP_SYS_PTRACE
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/proc

# Resource limits
CPUQuota=10%
MemoryMax=50M

[Install]
WantedBy=multi-user.target
```

After editing, reload:

```bash
sudo systemctl daemon-reload
sudo systemctl restart oom_guard
```

## Environment Variables

Configure OOM Guard via environment variables in `/etc/default/oom_guard`:

### Threshold Variables

```bash
# Memory thresholds (percentage)
OOM_GUARD_MEM_WARN=10      # Warning threshold %
OOM_GUARD_MEM_KILL=5       # Kill threshold %
OOM_GUARD_SWAP_WARN=10     # Swap warning %
OOM_GUARD_SWAP_KILL=5      # Swap kill %

# Memory thresholds (absolute KiB)
OOM_GUARD_MEM_SIZE_WARN=2097152   # 2GB
OOM_GUARD_MEM_SIZE_KILL=1048576   # 1GB
OOM_GUARD_SWAP_SIZE_WARN=1048576  # 1GB
OOM_GUARD_SWAP_SIZE_KILL=524288   # 512MB
```

### Monitoring Variables

```bash
OOM_GUARD_INTERVAL=1       # Check interval (seconds)
OOM_GUARD_REPORT=60        # Report interval (seconds)
```

### Behavior Variables

```bash
OOM_GUARD_SORT_BY_RSS=false        # Sort by RSS (true/false)
OOM_GUARD_DRY_RUN=false            # Dry run mode (true/false)
OOM_GUARD_DEBUG=false              # Debug logging (true/false)
OOM_GUARD_NOTIFY=false             # D-Bus notifications (true/false)
OOM_GUARD_IGNORE_ROOT_USER=false   # Ignore root processes (true/false)
OOM_GUARD_KILL_GROUP=false         # Kill process groups (true/false)
OOM_GUARD_PRIORITY=-20             # Daemon priority
```

### Filter Variables

```bash
OOM_GUARD_PREFER="(chrome|firefox)"     # Prefer killing these
OOM_GUARD_AVOID="(ssh|systemd)"         # Avoid killing these
OOM_GUARD_IGNORE="^critical-service$"   # Never kill these
```

### Hook Variables

```bash
OOM_GUARD_PRE_KILL_SCRIPT=/usr/local/bin/pre-kill.sh
OOM_GUARD_POST_KILL_SCRIPT=/usr/local/bin/post-kill.sh
```

### Using Environment File

Create `/etc/default/oom_guard`:

```bash
# Memory thresholds
OOM_GUARD_MEM_WARN=15
OOM_GUARD_MEM_KILL=10
OOM_GUARD_SWAP_WARN=20
OOM_GUARD_SWAP_KILL=10

# Process filters
OOM_GUARD_PREFER="(chrome|firefox)"
OOM_GUARD_AVOID="(ssh|systemd|postgres)"

# Notifications
OOM_GUARD_NOTIFY=true
OOM_GUARD_POST_KILL_SCRIPT=/usr/local/bin/notify-slack.sh

# Behavior
OOM_GUARD_REPORT=3600
OOM_GUARD_PRIORITY=-20
```

Update systemd service to use environment file:

```ini
[Service]
EnvironmentFile=/etc/default/oom_guard
ExecStart=/usr/local/bin/oom_guard
```

## Hook Scripts

Hook scripts receive environment variables with process information.

### Available Environment Variables

```bash
OOM_GUARD_PID       # Process ID
OOM_GUARD_NAME      # Process name
OOM_GUARD_CMDLINE   # Full command line
OOM_GUARD_UID       # User ID of process owner
OOM_GUARD_RSS       # Memory usage in KiB
OOM_GUARD_SCORE     # OOM score
```

### Example: Post-Kill Notification Script

`/usr/local/bin/oom-notify.sh`:

```bash
#!/bin/bash

# Log to file
LOG_FILE="/var/log/oom_guard_kills.log"
echo "$(date): Killed $OOM_GUARD_NAME (PID $OOM_GUARD_PID, RSS ${OOM_GUARD_RSS}KB, UID $OOM_GUARD_UID)" >> "$LOG_FILE"

# Send to Slack
SLACK_WEBHOOK="https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
curl -X POST "$SLACK_WEBHOOK" \
  -H 'Content-Type: application/json' \
  -d "{
    \"text\": \"⚠️ OOM Guard killed process\",
    \"attachments\": [{
      \"color\": \"danger\",
      \"fields\": [
        {\"title\": \"Process\", \"value\": \"$OOM_GUARD_NAME\", \"short\": true},
        {\"title\": \"PID\", \"value\": \"$OOM_GUARD_PID\", \"short\": true},
        {\"title\": \"Memory\", \"value\": \"${OOM_GUARD_RSS}KB\", \"short\": true},
        {\"title\": \"Score\", \"value\": \"$OOM_GUARD_SCORE\", \"short\": true}
      ]
    }]
  }"
```

Make executable:

```bash
sudo chmod +x /usr/local/bin/oom-notify.sh
```

Use with:

```bash
oom_guard -m 10,5 -N /usr/local/bin/oom-notify.sh
```

### Example: Pre-Kill Script

`/usr/local/bin/pre-kill-alert.sh`:

```bash
#!/bin/bash

# Log warning
logger -t oom_guard "About to kill $OOM_GUARD_NAME (PID $OOM_GUARD_PID)"

# Send email alert
echo "OOM Guard is about to kill $OOM_GUARD_NAME (PID $OOM_GUARD_PID, RSS ${OOM_GUARD_RSS}KB)" | \
  mail -s "OOM Guard Alert" admin@example.com

# Give process chance to cleanup
sleep 2
```

### Script Security

OOM Guard sanitizes environment variables:
- Shell metacharacters are replaced with underscores
- Length limited to 256 characters
- Symlinks are detected and rejected
- Script ownership is validated

## Process Selection

OOM Guard selects victim processes using this algorithm:

### Selection Criteria

1. **Filter out protected processes:**
   - PID 1 (init)
   - Kernel threads (name in brackets `[...]`)
   - Processes with `oom_score_adj = -1000`
   - Zombie processes (state 'Z')
   - Root processes (if `--ignore-root-user`)
   - Processes matching `--ignore` patterns

2. **Apply regex filters:**
   - `--prefer`: Add 1000 to oom_score
   - `--avoid`: Subtract 1000 from oom_score

3. **Sort processes:**
   - By `oom_score` (default) - kernel's OOM score
   - By `RSS` (with `--sort-by-rss`) - memory usage

4. **Select highest score/RSS**

### Understanding OOM Score

The kernel assigns each process an `oom_score` based on:
- Memory usage (RSS)
- Process age
- Process priority
- `oom_score_adj` setting

View process scores:

```bash
# Show top 10 processes by OOM score
ps aux --sort=-oom_score | head -11

# Check specific process
cat /proc/$(pidof firefox)/oom_score
```

### Protecting Processes

#### Method 1: Using --ignore flag

```bash
oom_guard --ignore "postgres|nginx|ssh"
```

#### Method 2: Using oom_score_adj

```bash
# Protect permanently (requires root)
echo -1000 | sudo tee /proc/$(pidof critical-app)/oom_score_adj

# Make it persistent (add to systemd service)
[Service]
OOMScoreAdjust=-1000
```

#### Method 3: Using --avoid flag

```bash
# Less strict than --ignore (avoid but allow if necessary)
oom_guard --avoid "important-service"
```

## Testing

### Dry Run Mode

Test configuration without killing processes:

```bash
sudo oom_guard --dryrun -m 20 -s 20 -d
```

Output shows what would be killed:

```
[DRY RUN] Would kill process: firefox (PID 12345, RSS 2048MB, Score 856)
```

### Simulate Memory Pressure

Install stress-ng:

```bash
sudo apt-get install stress-ng
```

Test with high thresholds (won't kill with normal memory):

```bash
# Terminal 1: Start OOM Guard
sudo oom_guard --dryrun -m 20 -s 20 -d

# Terminal 2: Stress memory (use 90% of RAM for 60 seconds)
stress-ng --vm 4 --vm-bytes 90% --timeout 60s

# Terminal 3: Watch logs
sudo journalctl -u oom_guard -f
```

### Test Process Selection

View process rankings:

```bash
# By OOM score
ps aux --sort=-oom_score | head -20

# By RSS memory
ps aux --sort=-rss | head -20

# Test filters
sudo oom_guard --dryrun -m 95 --prefer "chrome" --avoid "ssh" -d
```

## Troubleshooting

### Service Won't Start

**Check status:**
```bash
sudo systemctl status oom_guard
```

**View detailed logs:**
```bash
sudo journalctl -u oom_guard -n 50
```

**Common causes:**
- **Permission denied**: Need root/sudo
- **Binary not found**: Check `/usr/local/bin/oom_guard` exists
- **Config error**: Check flag syntax in service file
- **Invalid regex**: Test regex patterns

### Processes Not Being Killed

**Enable debug mode:**
```bash
sudo oom_guard -m 10,5 -d
```

**Check:**
1. Are thresholds actually exceeded?
   ```bash
   free -h
   ```
2. Are processes protected?
   ```bash
   cat /proc/$(pidof process)/oom_score_adj
   ```
3. Is dry-run enabled?
   ```bash
   systemctl cat oom_guard | grep dryrun
   ```

### Wrong Process Killed

**View current rankings:**
```bash
ps aux --sort=-rss | head -20
```

**Adjust filters:**
```bash
sudo oom_guard -m 10,5 \
  --prefer "high-memory-app" \
  --avoid "critical-service"
```

**Or use RSS sorting:**
```bash
sudo oom_guard -m 10,5 --sort-by-rss
```

### Hook Scripts Not Executing

**Check permissions:**
```bash
ls -l /usr/local/bin/your-script.sh
sudo chmod +x /usr/local/bin/your-script.sh
```

**Test script manually:**
```bash
export OOM_GUARD_PID=12345
export OOM_GUARD_NAME=test
export OOM_GUARD_RSS=1024
bash -x /usr/local/bin/your-script.sh
```

**Check logs:**
```bash
sudo journalctl -u oom_guard | grep -i "hook\|script"
```

### High CPU Usage

**Increase check interval:**
```bash
sudo oom_guard -m 10,5 -i 5  # Check every 5 seconds
```

**Check for runaway logging:**
```bash
sudo journalctl -u oom_guard --disk-usage
```

**Reduce debug output:**
```bash
# Remove -d flag from service
sudo systemctl edit oom_guard
```

### Memory Not Being Freed After Kill

This may indicate:
1. Memory leaks in kernel
2. Cached memory (normal, will be freed when needed)
3. Process had memory locked

**Check memory details:**
```bash
cat /proc/meminfo
```

**Force cache drop (safe):**
```bash
sudo sync
echo 3 | sudo tee /proc/sys/vm/drop_caches
```

## Advanced Usage

### Multi-Tier Thresholds

Use different thresholds for memory and swap:

```bash
# Conservative memory, aggressive swap
oom_guard -m 20,15 -s 10,5
```

### Custom Process Priority

Run specific workloads with adjusted OOM protection:

```bash
# Start important process with low OOM score
systemd-run --scope -p OOMScoreAdjust=-500 ./my-important-app

# Start low-priority process with high OOM score
systemd-run --scope -p OOMScoreAdjust=500 ./batch-job
```

### Monitoring Multiple Metrics

Combine with other monitoring tools:

```bash
# Log memory stats every 60 seconds
while true; do
  free -h >> /var/log/memory-stats.log
  sleep 60
done &

# Run OOM Guard
sudo oom_guard -m 10,5 -r 60
```

### Container Integration

#### Docker

```dockerfile
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y curl
RUN curl -L -o /usr/local/bin/oom_guard \
  https://github.com/marvinbraga/oom_guard/releases/latest/download/oom_guard-linux-x86_64
RUN chmod +x /usr/local/bin/oom_guard
CMD ["/usr/local/bin/oom_guard", "-m", "10,5", "-s", "10,5"]
```

Run with host PID namespace:

```bash
docker run --privileged --pid=host \
  -v /proc:/proc:ro \
  oom_guard
```

#### Kubernetes DaemonSet

```yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: oom-guard
spec:
  selector:
    matchLabels:
      app: oom-guard
  template:
    metadata:
      labels:
        app: oom-guard
    spec:
      hostPID: true
      containers:
      - name: oom-guard
        image: oom-guard:latest
        securityContext:
          privileged: true
        volumeMounts:
        - name: proc
          mountPath: /proc
          readOnly: true
      volumes:
      - name: proc
        hostPath:
          path: /proc
```

### Performance Tuning

#### For High-Memory Systems (128GB+)

```bash
# More conservative thresholds
oom_guard -m 5,2 -s 5,2 -i 2
```

#### For Low-Memory Systems (< 4GB)

```bash
# More aggressive, faster response
oom_guard -m 20,15 -s 20,15 -i 1
```

#### For Desktop/Laptop

```bash
# Balanced, with notifications
oom_guard -m 15,10 -s 15,10 -n \
  --avoid "(firefox|chrome|code|terminal)"
```

#### For Server (Stable Load)

```bash
# Conservative, with monitoring
oom_guard -m 10,5 -s 10,5 -r 3600 \
  --avoid "(postgres|nginx|redis|mysql)" \
  --prefer "(java|node)"
```

#### For Server (Bursty Load)

```bash
# Aggressive, quick response
oom_guard -m 5,2 -s 5,2 -i 1 \
  --sort-by-rss
```

## Examples

### Basic Usage

```bash
# Default settings (10% warn, 5% kill)
sudo oom_guard -m 10,5 -s 10,5

# Conservative (more memory before action)
sudo oom_guard -m 20,15 -s 20,15

# Aggressive (quick response)
sudo oom_guard -m 5,2 -s 5,2
```

### With Notifications

```bash
# Desktop notifications
sudo oom_guard -m 10,5 -n

# Email notifications via hook
sudo oom_guard -m 10,5 \
  -N /usr/local/bin/email-notify.sh

# Slack notifications via hook
sudo oom_guard -m 10,5 \
  -N /usr/local/bin/slack-notify.sh
```

### Protecting Critical Services

```bash
# Web server protection
sudo oom_guard -m 10,5 \
  --avoid "(nginx|apache2)" \
  --prefer "(chrome|firefox)"

# Database server protection
sudo oom_guard -m 10,5 \
  --ignore "(postgres|mysql|redis)" \
  --prefer "(java|node)"

# SSH and system services
sudo oom_guard -m 10,5 \
  --ignore "(sshd|systemd)" \
  --ignore-root-user
```

### Development Environment

```bash
# Protect IDE and terminal
sudo oom_guard -m 15,10 \
  --avoid "(code|terminal|tmux|vim)" \
  --prefer "test.*"
```

### Production Server

```bash
# Kill process groups, high priority
sudo oom_guard -m 10,5 -s 10,5 \
  -g \
  --set-priority=-20 \
  --avoid "(nginx|postgres|redis)" \
  --prefer "(worker|job)" \
  -N /usr/local/bin/pagerduty-alert.sh
```

### Testing Configuration

```bash
# Dry run with debug
sudo oom_guard --dryrun -m 20 -d \
  --prefer "firefox" \
  --avoid "ssh"

# Short test (60 seconds)
sudo timeout 60 oom_guard --dryrun -m 50 -d
```

---

## Summary

**Key Points:**

1. **Start simple**: Default settings work well for most systems
2. **Test first**: Use `--dryrun` to validate configuration
3. **Protect critical services**: Use `--ignore` or `--avoid`
4. **Monitor logs**: Check `journalctl -u oom_guard -f`
5. **Adjust thresholds**: Based on your memory usage patterns
6. **Use hooks**: For custom notifications and logging

**Recommended Configurations:**

| System Type | Command |
|-------------|---------|
| Desktop | `oom_guard -m 15,10 -s 15,10 -n` |
| Web Server | `oom_guard -m 10,5 -s 10,5 --avoid "(nginx\|postgres)"` |
| Container Host | `oom_guard -m 5,2 -s 5,2 -g` |
| Low Memory | `oom_guard -m 20,15 -s 20,15` |

For additional help:
- GitHub Issues: https://github.com/marvinbraga/oom_guard/issues
- README: https://github.com/marvinbraga/oom_guard/blob/main/README.md
