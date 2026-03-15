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

TARGET="x86_64-pc-windows-msvc"

echo "--- Building shared types ---"
cargo build -p shared_types --target wasm32-unknown-unknown --no-default-features --features typescript

echo "--- Packaging Desktop build (Windows x64) ---"
cd desktop
pnpm install

# Check if target is installed
if ! rustup target list --installed | grep -q "$TARGET"; then
  echo "--- Installing Rust target: $TARGET ---"
  rustup target add "$TARGET"
fi

# On macOS, use cargo-xwin for cross-compilation
if [[ "$OSTYPE" == "darwin"* ]]; then
  if ! command -v cargo-xwin &> /dev/null; then
    echo "--- Error: cargo-xwin is not installed. ---"
    echo "Please install it with: cargo install cargo-xwin"
    exit 1
  fi
  echo "--- Building for Windows on macOS via cargo-xwin ---"
  pnpm tauri build --target "$TARGET" --runner "cargo xwin"
else
  echo "--- Building for Windows ---"
  pnpm tauri build --target "$TARGET"
fi

# Find and upload the MSI package
echo "--- Finding MSI package ---"
cd "$PROJECT_ROOT"
MSI_FILE=$(find desktop/src-tauri/target/release/bundle/msi -name "*.msi" 2>/dev/null | head -n 1)
if [ -z "$MSI_FILE" ]; then
  echo "--- Error: No MSI package found ---"
  exit 1
fi
echo "--- Found MSI: $MSI_FILE ---"

# Upload to S3 (fail if upload fails)
echo "--- Uploading to S3 ---"
scripts/desktop/upload-package.sh "$MSI_FILE" || exit 1
echo "--- Upload complete ---"
