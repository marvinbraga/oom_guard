#!/bin/bash
set -e

# OOM Guard Installation Script
# Author: Marcus Vinicius Braga (mvbraga@gmail.com)
# Repository: https://github.com/marvinbraga/oom_guard
# License: GPL-2.0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BINARY_NAME="oom_guard"
INSTALL_DIR="/usr/local/bin"
SERVICE_FILE="systemd/oom_guard.service"
SYSTEMD_DIR="/etc/systemd/system"
ENV_FILE="/etc/default/oom_guard"
GITHUB_REPO="marvinbraga/oom_guard"

# Detect architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        ARCH_NAME="x86_64"
        ;;
    aarch64)
        ARCH_NAME="aarch64"
        ;;
    *)
        ARCH_NAME="unknown"
        ;;
esac

print_header() {
    echo -e "${GREEN}"
    echo "╔══════════════════════════════════════════╗"
    echo "║         OOM Guard Installer              ║"
    echo "║    Memory Monitor Daemon for Linux       ║"
    echo "╚══════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Function to install Rust via rustup
install_rust() {
    echo -e "${YELLOW}Installing Rust via rustup...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}Rust installed successfully!${NC}"
}

# Function to check and install Rust if needed
check_rust() {
    if command -v cargo &> /dev/null; then
        CARGO_VERSION=$(cargo --version)
        echo -e "${GREEN}Found: $CARGO_VERSION${NC}"
        return 0
    else
        echo -e "${YELLOW}Rust/Cargo not found on this system.${NC}"
        echo ""
        echo "Options:"
        echo "  1) Install Rust automatically (recommended)"
        echo "  2) Download pre-compiled binary from GitHub Releases"
        echo "  3) Exit and install Rust manually"
        echo ""
        read -p "Choose an option [1/2/3]: " choice

        case $choice in
            1)
                install_rust
                return 0
                ;;
            2)
                download_binary
                return 1  # Skip build step
                ;;
            3)
                echo -e "${YELLOW}Please install Rust from https://rustup.rs and run this script again.${NC}"
                exit 0
                ;;
            *)
                echo -e "${RED}Invalid option. Exiting.${NC}"
                exit 1
                ;;
        esac
    fi
}

# Function to download pre-compiled binary
download_binary() {
    echo -e "${YELLOW}Downloading pre-compiled binary...${NC}"

    # Get latest release from GitHub
    LATEST_RELEASE=$(curl -s "https://api.github.com/repos/$GITHUB_REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

    if [ -z "$LATEST_RELEASE" ]; then
        echo -e "${RED}Error: Could not fetch latest release from GitHub.${NC}"
        echo -e "${YELLOW}Please check: https://github.com/$GITHUB_REPO/releases${NC}"
        exit 1
    fi

    echo -e "${BLUE}Latest version: $LATEST_RELEASE${NC}"

    DOWNLOAD_URL="https://github.com/$GITHUB_REPO/releases/download/$LATEST_RELEASE/oom_guard-linux-$ARCH_NAME"

    echo -e "${YELLOW}Downloading from: $DOWNLOAD_URL${NC}"

    # Create temp directory
    TMP_DIR=$(mktemp -d)

    if curl -L -o "$TMP_DIR/$BINARY_NAME" "$DOWNLOAD_URL"; then
        chmod +x "$TMP_DIR/$BINARY_NAME"

        # Verify binary works
        if "$TMP_DIR/$BINARY_NAME" --version &> /dev/null; then
            # Move to target directory (needs root)
            mkdir -p "target/release"
            mv "$TMP_DIR/$BINARY_NAME" "target/release/$BINARY_NAME"
            echo -e "${GREEN}Binary downloaded successfully!${NC}"
        else
            echo -e "${RED}Error: Downloaded binary is not valid for this system.${NC}"
            rm -rf "$TMP_DIR"
            exit 1
        fi
    else
        echo -e "${RED}Error: Failed to download binary.${NC}"
        echo -e "${YELLOW}Please check: https://github.com/$GITHUB_REPO/releases${NC}"
        rm -rf "$TMP_DIR"
        exit 1
    fi

    rm -rf "$TMP_DIR"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        echo -e "${RED}Error: This script must be run as root${NC}"
        echo "Please run: sudo $0"
        exit 1
    fi
}

# Build from source
build_from_source() {
    echo -e "\n${YELLOW}Building OOM Guard from source...${NC}"
    cargo build --release
    echo -e "${GREEN}Build successful!${NC}"
}

# Install binary
install_binary() {
    echo -e "\n${YELLOW}Installing binary to $INSTALL_DIR...${NC}"

    if [ ! -f "target/release/$BINARY_NAME" ]; then
        echo -e "${RED}Error: Binary not found at target/release/$BINARY_NAME${NC}"
        exit 1
    fi

    cp "target/release/$BINARY_NAME" "$INSTALL_DIR/"
    chmod 755 "$INSTALL_DIR/$BINARY_NAME"
    echo -e "${GREEN}Binary installed: $INSTALL_DIR/$BINARY_NAME${NC}"

    # Show version
    "$INSTALL_DIR/$BINARY_NAME" --version
}

# Install systemd service
install_service() {
    echo -e "\n${YELLOW}Installing systemd service...${NC}"

    if [ -f "$SERVICE_FILE" ]; then
        cp "$SERVICE_FILE" "$SYSTEMD_DIR/"
        chmod 644 "$SYSTEMD_DIR/$(basename $SERVICE_FILE)"
        echo -e "${GREEN}Service file installed: $SYSTEMD_DIR/oom_guard.service${NC}"
    else
        # Create service file if not found (for binary-only installs)
        echo -e "${YELLOW}Creating systemd service file...${NC}"
        cat > "$SYSTEMD_DIR/oom_guard.service" << 'EOF'
[Unit]
Description=OOM Guard - Memory Monitor Daemon
Documentation=https://github.com/marvinbraga/oom_guard
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/oom_guard -m 10,5 -s 10,5 -r 3600
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

CapabilityBoundingSet=CAP_KILL CAP_DAC_OVERRIDE CAP_SYS_NICE CAP_SYS_PTRACE
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/proc

CPUQuota=10%
MemoryMax=50M

[Install]
WantedBy=multi-user.target
EOF
        chmod 644 "$SYSTEMD_DIR/oom_guard.service"
        echo -e "${GREEN}Service file created${NC}"
    fi
}

# Create default configuration
create_config() {
    echo -e "\n${YELLOW}Creating default configuration...${NC}"

    if [ ! -f "$ENV_FILE" ]; then
        cat > "$ENV_FILE" << 'EOF'
# OOM Guard Configuration
# Repository: https://github.com/marvinbraga/oom_guard
# Uncomment and modify as needed

# Memory threshold percentages (warn,kill)
#OOM_GUARD_MEM_THRESHOLD=10,5

# Swap threshold percentages (warn,kill)
#OOM_GUARD_SWAP_THRESHOLD=10,5

# Enable D-Bus notifications
#OOM_GUARD_NOTIFY=true

# Report interval in seconds
#OOM_GUARD_REPORT_INTERVAL=3600

# Prefer certain processes (regex)
#OOM_GUARD_PREFER="(chrome|firefox)"

# Avoid certain processes (regex)
#OOM_GUARD_AVOID="(ssh|systemd)"

# Pre-kill script
#OOM_GUARD_PRE_KILL_SCRIPT=/usr/local/bin/pre-kill.sh

# Post-kill script
#OOM_GUARD_POST_KILL_SCRIPT=/usr/local/bin/post-kill.sh

# Dry run mode (test without killing)
#OOM_GUARD_DRY_RUN=false
EOF
        chmod 644 "$ENV_FILE"
        echo -e "${GREEN}Configuration file created: $ENV_FILE${NC}"
    else
        echo -e "${BLUE}Configuration file already exists: $ENV_FILE${NC}"
    fi
}

# Reload systemd and optionally start service
finalize_install() {
    echo -e "\n${YELLOW}Reloading systemd daemon...${NC}"
    systemctl daemon-reload
    echo -e "${GREEN}Systemd daemon reloaded${NC}"

    echo ""
    echo -e "${YELLOW}Do you want to enable and start the OOM Guard service now? [y/N]${NC}"
    read -r response

    if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
        echo -e "\n${YELLOW}Enabling and starting service...${NC}"
        systemctl enable oom_guard
        systemctl start oom_guard
        echo -e "${GREEN}Service enabled and started!${NC}"

        echo -e "\n${YELLOW}Service status:${NC}"
        systemctl status oom_guard --no-pager || true
    else
        echo -e "\n${YELLOW}Service not started. Start manually with:${NC}"
        echo "  sudo systemctl enable oom_guard"
        echo "  sudo systemctl start oom_guard"
    fi
}

# Print installation summary
print_summary() {
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║       Installation Complete!             ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${BLUE}Installed files:${NC}"
    echo "  Binary:  $INSTALL_DIR/$BINARY_NAME"
    echo "  Service: $SYSTEMD_DIR/oom_guard.service"
    echo "  Config:  $ENV_FILE"
    echo ""
    echo -e "${BLUE}Useful commands:${NC}"
    echo "  Check status:  sudo systemctl status oom_guard"
    echo "  View logs:     sudo journalctl -u oom_guard -f"
    echo "  Stop service:  sudo systemctl stop oom_guard"
    echo "  Restart:       sudo systemctl restart oom_guard"
    echo "  Edit config:   sudo nano $ENV_FILE"
    echo ""
    echo -e "${BLUE}Test with dry-run:${NC}"
    echo "  sudo oom_guard --dryrun -m 20 -s 20 -d"
    echo ""
    echo -e "${GREEN}Repository: https://github.com/marvinbraga/oom_guard${NC}"
}

# Main installation flow
main() {
    print_header
    check_root

    echo -e "${BLUE}System: $(uname -s) $(uname -r) ($ARCH)${NC}"
    echo ""

    # Check Rust and build or download
    if check_rust; then
        build_from_source
    fi

    # Install components
    install_binary
    install_service
    create_config
    finalize_install
    print_summary
}

# Run main
main "$@"
