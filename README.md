# OOM Guard

A memory monitoring daemon written in Rust that prevents system freezes by killing memory-hogging processes before the kernel's Out-Of-Memory (OOM) killer is triggered.

**Author:** Marcus Vinicius Braga (mvbraga@gmail.com)
**Repository:** https://github.com/marvinbraga/oom_guard
**License:** GPL-2.0

## Features

- Real-time RAM and swap monitoring
- Configurable memory thresholds (percentage or absolute values)
- Process selection based on OOM score or RSS
- Regex-based process filtering (prefer/avoid/ignore)
- Pre-kill and post-kill script hooks
- D-Bus desktop notifications (optional)
- Dry-run mode for testing
- Systemd integration
- Low resource overhead
- Written in safe Rust

## Installation

### Quick Install (Recommended)

The installation script handles everything automatically:

```bash
# Clone and install
git clone https://github.com/marvinbraga/oom_guard.git
cd oom_guard
sudo ./install.sh
```

The script will:
- **Auto-detect** if Rust is installed
- **Offer to install Rust** automatically if not found
- **Or download** a pre-compiled binary from GitHub Releases
- Install the binary to `/usr/local/bin/`
- Setup the systemd service
- Create default configuration

### Download Pre-compiled Binary

Pre-compiled binaries are available for Linux:

```bash
# Download latest release (x86_64)
curl -L -o oom_guard https://github.com/marvinbraga/oom_guard/releases/latest/download/oom_guard-linux-x86_64
chmod +x oom_guard
sudo mv oom_guard /usr/local/bin/

# For ARM64/aarch64
curl -L -o oom_guard https://github.com/marvinbraga/oom_guard/releases/latest/download/oom_guard-linux-aarch64
chmod +x oom_guard
sudo mv oom_guard /usr/local/bin/
```

Available binaries:
- `oom_guard-linux-x86_64` - Standard Linux (glibc)
- `oom_guard-linux-x86_64-musl` - Static binary (Alpine, minimal distros)
- `oom_guard-linux-aarch64` - ARM64 (Raspberry Pi 4, AWS Graviton)

### Building from Source

Prerequisites:
- Rust toolchain (1.70+)
- Linux system with /proc filesystem

```bash
# Clone the repository
git clone https://github.com/marvinbraga/oom_guard.git
cd oom_guard

# Build in release mode
cargo build --release

# The binary will be at target/release/oom_guard
```

### Manual Installation

```bash
# Build and copy binary
cargo build --release
sudo cp target/release/oom_guard /usr/local/bin/

# Install systemd service
sudo cp systemd/oom_guard.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable oom_guard
sudo systemctl start oom_guard
```

## Usage

### Command Line Options

```bash
oom_guard [OPTIONS]
```

#### Memory Thresholds

- `-m PERCENT[,KILL_PERCENT]` - Set memory threshold (default: 10,5)
  - First value: send SIGTERM when free memory drops below this percentage
  - Second value: send SIGKILL when free memory drops below this percentage
  - Example: `-m 15,5` - warn at 15%, kill at 5%

- `-s PERCENT[,KILL_PERCENT]` - Set swap threshold (default: 10,5)
  - Same format as memory threshold
  - Example: `-s 20,10`

- `-M SIZE[,KILL_SIZE]` - Set memory threshold in KiB (absolute)
  - Example: `-M 1048576,524288` - warn at 1GB, kill at 512MB free

- `-S SIZE[,KILL_SIZE]` - Set swap threshold in KiB (absolute)
  - Example: `-S 524288,262144`

#### Process Selection

- `--prefer REGEX` - Prefer to kill processes matching regex
  - Example: `--prefer "(chrome|firefox)"`

- `--avoid REGEX` - Avoid killing processes matching regex
  - Example: `--avoid "(ssh|tmux|systemd)"`

- `--ignore REGEX` - Completely ignore processes matching regex
  - Example: `--ignore "^(init|systemd)$"`

- `--sort-by-rss` - Sort processes by RSS instead of OOM score

#### Behavior Options

- `-g` - Kill entire process group instead of just the process
- `-r INTERVAL` - Report memory status every INTERVAL seconds
- `-p` - Increase priority of oom_guard itself (set niceness and oom_score_adj)
- `--dryrun` - Don't actually kill processes, just log what would be killed

#### Notifications

- `-n` - Enable D-Bus desktop notifications (requires dbus-notify feature)
- `-N SCRIPT` - Execute script after killing a process (post-kill hook)
- `-P SCRIPT` - Execute script before killing a process (pre-kill hook)

#### Debug Options

- `-d` - Enable debug output
- `-v, --version` - Show version information
- `-h, --help` - Show help message

### Examples

#### Basic usage with default thresholds
```bash
sudo oom_guard -m 10,5 -s 10,5
```

#### With notifications and reporting
```bash
sudo oom_guard -m 15,10 -s 20,10 -n -r 3600
```

#### Test mode (dry-run)
```bash
sudo oom_guard --dryrun -m 20 -d
```

#### With custom scripts
```bash
sudo oom_guard -m 10,5 \
  -P /usr/local/bin/pre-kill.sh \
  -N /usr/local/bin/post-kill.sh
```

#### Prefer killing browsers, avoid system processes
```bash
sudo oom_guard -m 10,5 \
  --prefer "(chrome|firefox|chromium)" \
  --avoid "(ssh|systemd|dbus)"
```

#### Using absolute memory values
```bash
sudo oom_guard -M 2097152,1048576 -S 1048576,524288
```

### Hook Scripts

Hook scripts receive the following environment variables:

- `OOM_GUARD_PID` - Process ID of the killed process
- `OOM_GUARD_NAME` - Name of the killed process
- `OOM_GUARD_RSS` - Resident Set Size in KiB
- `OOM_GUARD_SCORE` - OOM score of the process

Example post-kill script:

```bash
#!/bin/bash
# /usr/local/bin/post-kill.sh

echo "$(date): Killed process $OOM_GUARD_NAME (PID: $OOM_GUARD_PID, RSS: $OOM_GUARD_RSS KB)" \
  >> /var/log/oom_guard_kills.log
```

Make scripts executable:
```bash
chmod +x /usr/local/bin/post-kill.sh
```

## Configuration

### Via Command Line

Pass options directly when starting the daemon:
```bash
sudo oom_guard -m 15,10 -s 20,10 -n -r 3600 -p
```

### Via Environment Variables

Set environment variables (useful for systemd):

```bash
export OOM_GUARD_MEM_THRESHOLD="15,10"
export OOM_GUARD_SWAP_THRESHOLD="20,10"
export OOM_GUARD_NOTIFY="true"
export OOM_GUARD_REPORT_INTERVAL="3600"
export OOM_GUARD_PREFER="(chrome|firefox)"
export OOM_GUARD_AVOID="(ssh|systemd)"
export OOM_GUARD_DRY_RUN="false"
```

### Via Systemd Environment File

Edit `/etc/default/oom_guard`:

```bash
sudo nano /etc/default/oom_guard
```

Then uncomment and modify the desired options:

```bash
OOM_GUARD_MEM_THRESHOLD=10,5
OOM_GUARD_SWAP_THRESHOLD=10,5
OOM_GUARD_NOTIFY=true
OOM_GUARD_REPORT_INTERVAL=3600
```

Update the service file to use the environment file:

```ini
[Service]
EnvironmentFile=/etc/default/oom_guard
ExecStart=/usr/local/bin/oom_guard
```

## Systemd Service

### Managing the Service

```bash
# Start the service
sudo systemctl start oom_guard

# Stop the service
sudo systemctl stop oom_guard

# Restart the service
sudo systemctl restart oom_guard

# Enable at boot
sudo systemctl enable oom_guard

# Disable at boot
sudo systemctl disable oom_guard

# Check status
sudo systemctl status oom_guard

# View logs
sudo journalctl -u oom_guard -f

# View recent logs
sudo journalctl -u oom_guard -n 100
```

### Customizing the Service

Edit the service file:

```bash
sudo systemctl edit oom_guard
```

Or edit the main service file:

```bash
sudo nano /etc/systemd/system/oom_guard.service
```

After editing, reload systemd:

```bash
sudo systemctl daemon-reload
sudo systemctl restart oom_guard
```

## Testing

### Simulate Memory Pressure

Use `stress-ng` to test the daemon:

```bash
# Install stress-ng
sudo apt-get install stress-ng

# Run in dry-run mode first
sudo oom_guard --dryrun -m 20 -s 20 -d

# In another terminal, create memory pressure
stress-ng --vm 4 --vm-bytes 90% --timeout 60s
```

### Unit Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## How It Works

OOM Guard monitors system memory at regular intervals and takes action when memory falls below configured thresholds:

1. **Monitoring**: Reads `/proc/meminfo` to get current memory and swap usage
2. **Threshold Check**: Compares available memory against configured thresholds
3. **Process Selection**: When threshold is exceeded:
   - Scans `/proc/[pid]/` for all processes
   - Applies regex filters (prefer/avoid/ignore)
   - Ranks processes by OOM score or RSS
4. **Action**: Sends appropriate signal:
   - SIGTERM for first threshold (graceful)
   - SIGKILL for second threshold (forced)
5. **Notification**: Executes hooks and sends notifications
6. **Cooldown**: Waits before next evaluation to avoid rapid kills

## Troubleshooting

### Service Won't Start

Check the service status and logs:
```bash
sudo systemctl status oom_guard
sudo journalctl -u oom_guard -n 50
```

### Permission Denied Errors

OOM Guard requires root privileges to:
- Read `/proc/[pid]/` information
- Send kill signals to processes
- Adjust its own priority

Run with `sudo` or as a systemd service.

### Hook Scripts Not Executing

Verify script permissions:
```bash
ls -l /usr/local/bin/pre-kill.sh
chmod +x /usr/local/bin/pre-kill.sh
```

Check script syntax:
```bash
bash -n /usr/local/bin/pre-kill.sh
```

### D-Bus Notifications Not Working

Ensure the `dbus-notify` feature is enabled:
```bash
cargo build --release --features dbus-notify
```

Check if D-Bus is running:
```bash
systemctl status dbus
```

## Performance

OOM Guard is designed to have minimal system impact:

- Memory usage: < 5 MB RSS
- CPU usage: < 1% on average
- Monitoring overhead: Negligible
- Written in Rust for memory safety and efficiency

## Technical Details

| Aspect | Details |
|--------|---------|
| Language | Rust |
| Memory Safety | Guaranteed by Rust |
| Systemd Support | Yes |
| D-Bus Notifications | Yes (optional) |
| Hook Scripts | Yes |
| Memory Locking | Yes (mlockall) |
| Process Group Killing | Yes (-g flag) |

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Run `cargo fmt` and `cargo clippy`
5. Submit a pull request

## License

This project is licensed under the [GNU General Public License v2.0](LICENSE).

## Security

OOM Guard runs with elevated privileges. Always:
- Review hook scripts before execution
- Use absolute paths for scripts
- Validate regex patterns
- Test in dry-run mode first

Report security issues to: mvbraga@gmail.com

## Resources

- [Linux OOM Killer](https://www.kernel.org/doc/gorman/html/understand/understand016.html)
- [Rust procfs library](https://docs.rs/procfs/)
