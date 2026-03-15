#!/bin/bash
set -euo pipefail

# Change to project root
cd "$(dirname "$0")/../.."
PROJECT_ROOT=$(pwd)

# Source production environment variables if they exist
if [ -f "scripts/desktop/prod-env.env" ]; then
  echo "--- Loading production environment variables ---"
  set -a
  source scripts/desktop/prod-env.env
  set +a
fi

TARGET="x86_64-unknown-linux-gnu"

echo "--- Building shared types ---"
cargo build -p shared_types --target wasm32-unknown-unknown --no-default-features --features typescript

echo "--- Packaging Desktop build (Linux x64) ---"
cd desktop
pnpm install

# Check if target is installed
if ! rustup target list --installed | grep -q "$TARGET"; then
  echo "--- Installing Rust target: $TARGET ---"
  rustup target add "$TARGET"
fi

# On macOS, use cargo-zigbuild for cross-compilation
if [[ "$OSTYPE" == "darwin"* ]]; then
  if ! command -v zig &> /dev/null; then
    echo "--- Error: zig is not installed. ---"
    echo "Please install it with: brew install zig"
    exit 1
  fi
  if ! command -v cargo-zigbuild &> /dev/null; then
    echo "--- Error: cargo-zigbuild is not installed. ---"
    echo "Please install it with: cargo install cargo-zigbuild"
    exit 1
  fi
  echo "--- Building for Linux on macOS via cargo-zigbuild ---"
  pnpm tauri build --target "$TARGET" --runner "cargo zigbuild"
else
  echo "--- Building for Linux ---"
  pnpm tauri build --target "$TARGET"
fi

# Find and upload Linux packages (AppImage and deb)
echo "--- Finding Linux packages ---"
cd "$PROJECT_ROOT"

# Upload AppImage if exists
APPIMAGE_FILE=$(find desktop/src-tauri/target/release/bundle/appimage -name "*.AppImage" 2>/dev/null | head -n 1)
if [ -n "$APPIMAGE_FILE" ]; then
  echo "--- Found AppImage: $APPIMAGE_FILE ---"
  echo "--- Uploading AppImage to S3 ---"
  scripts/desktop/upload-package.sh "$APPIMAGE_FILE" || exit 1
  echo "--- AppImage upload complete ---"
else
  echo "--- Error: No AppImage package found ---"
  exit 1
fi

# Upload deb if exists
DEB_FILE=$(find desktop/src-tauri/target/release/bundle/deb -name "*.deb" 2>/dev/null | head -n 1)
if [ -n "$DEB_FILE" ]; then
  echo "--- Found deb: $DEB_FILE ---"
  echo "--- Uploading deb to S3 ---"
  scripts/desktop/upload-package.sh "$DEB_FILE" || exit 1
  echo "--- deb upload complete ---"
else
  echo "--- Warning: No deb package found, skipping ---"
fi
