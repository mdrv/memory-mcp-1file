#!/bin/bash
set -e

APP_NAME="memory-mcp"
VERSION=$(grep '^version =' Cargo.toml | head -n1 | cut -d '"' -f 2)
TARGET_DIR="target/release-artifacts"

echo "🚀 Building version $VERSION..."
mkdir -p $TARGET_DIR

# 1. Build GNU (Standard Linux) for Dockerfile.fast
echo "🔨 Building GNU binary (for local Dockerfile.fast)..."
cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/$APP_NAME $TARGET_DIR/$APP_NAME-gnu
chmod +x $TARGET_DIR/$APP_NAME-gnu

# 2. Build MUSL (Static Linux) for Alpine/Release
# Requires: rustup target add x86_64-unknown-linux-musl
if ! rustup target list --installed | grep -q "x86_64-unknown-linux-musl"; then
    echo "📦 Installing musl target..."
    rustup target add x86_64-unknown-linux-musl
fi

echo "🔨 Building MUSL binary (Static for Alpine)..."
# We use standard release profile for musl compatibility, or max-perf if tested
cargo build --release --target x86_64-unknown-linux-musl

BINARY_PATH="target/x86_64-unknown-linux-musl/release/$APP_NAME"
ARCHIVE_NAME="$APP_NAME-v$VERSION-x86_64-unknown-linux-musl.tar.gz"

echo "📦 Packaging MUSL binary into $ARCHIVE_NAME..."
tar -czf $TARGET_DIR/$ARCHIVE_NAME -C $(dirname $BINARY_PATH) $APP_NAME

# Output results
echo "✅ Build complete!"
echo "   GNU Binary (Local): $TARGET_DIR/$APP_NAME-gnu"
echo "   MUSL Archive (Public): $TARGET_DIR/$ARCHIVE_NAME"
