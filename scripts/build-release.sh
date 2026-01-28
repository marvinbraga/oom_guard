#!/bin/bash
set -e

# OOM Guard Release Build Script
# Author: Marcus Vinicius Braga (mvbraga@gmail.com)
# License: GPL-2.0
# Creates release binaries for multiple platforms

VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
RELEASE_DIR="release/v$VERSION"

echo "Building OOM Guard v$VERSION"
echo "=========================="

# Create release directory
mkdir -p "$RELEASE_DIR"

# Build for current platform (default)
echo ""
echo "Building for current platform..."
cargo build --release
cp target/release/oom_guard "$RELEASE_DIR/oom_guard-linux-$(uname -m)"
echo "Created: $RELEASE_DIR/oom_guard-linux-$(uname -m)"

# Optional: Build for other targets if cross is installed
if command -v cross &> /dev/null; then
    echo ""
    echo "Cross compilation available. Building additional targets..."

    # x86_64 musl (static)
    echo "Building x86_64-unknown-linux-musl..."
    cross build --release --target x86_64-unknown-linux-musl
    cp target/x86_64-unknown-linux-musl/release/oom_guard "$RELEASE_DIR/oom_guard-linux-x86_64-musl"

    # aarch64 (ARM64)
    echo "Building aarch64-unknown-linux-gnu..."
    cross build --release --target aarch64-unknown-linux-gnu
    cp target/aarch64-unknown-linux-gnu/release/oom_guard "$RELEASE_DIR/oom_guard-linux-aarch64"
else
    echo ""
    echo "Note: Install 'cross' for cross-compilation:"
    echo "  cargo install cross"
fi

# Create checksums
echo ""
echo "Creating checksums..."
cd "$RELEASE_DIR"
sha256sum oom_guard-* > SHA256SUMS.txt
cd - > /dev/null

echo ""
echo "Release files created in: $RELEASE_DIR/"
ls -la "$RELEASE_DIR/"

echo ""
echo "To create a release on GitHub:"
echo "  git tag v$VERSION"
echo "  git push origin v$VERSION"
echo ""
echo "GitHub Actions will automatically build and publish the release."
