#!/bin/bash
set -e

APP_NAME="memory-mcp"
VERSION=$(grep '^version =' Cargo.toml | head -n1 | cut -d '"' -f 2)
TARGET_DIR="target/release-artifacts"

echo "🚀 Building version $VERSION..."
mkdir -p $TARGET_DIR

# 1. Build GNU (Standard Linux) for Dockerfile.fast
echo "🔨 Building GNU binary (for local Dockerfile.fast)..."
cargo build --profile fast --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/fast/$APP_NAME $TARGET_DIR/$APP_NAME-gnu
chmod +x $TARGET_DIR/$APP_NAME-gnu

# 2. Build MUSL (Static Linux) for Alpine/Release
if rustup target list --installed | grep -q "x86_64-unknown-linux-musl"; then
    echo "🔨 Building MUSL binary (Static for Alpine)..."
    if cargo build --release --target x86_64-unknown-linux-musl; then
        BINARY_PATH="target/x86_64-unknown-linux-musl/release/$APP_NAME"
        ARCHIVE_NAME="$APP_NAME-v$VERSION-x86_64-unknown-linux-musl.tar.gz"

        echo "📦 Packaging MUSL binary into $ARCHIVE_NAME..."
        tar -czf $TARGET_DIR/$ARCHIVE_NAME -C $(dirname $BINARY_PATH) $APP_NAME
        echo "   MUSL Archive (Public): $TARGET_DIR/$ARCHIVE_NAME"
    else
        echo "⚠️  MUSL build failed (likely missing C++ tools). Skipping Alpine artifact."
    fi
else
    echo "⚠️  MUSL target not installed. Skipping Alpine artifact."
fi

# Output results
echo "✅ Build complete!"
echo "   GNU Binary (Local): $TARGET_DIR/$APP_NAME-gnu"
echo "   MUSL Archive (Public): $TARGET_DIR/$ARCHIVE_NAME"
