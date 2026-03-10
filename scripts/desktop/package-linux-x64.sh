#!/bin/bash
set -euo pipefail

# Change to project root
cd "$(dirname "$0")/../.."

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
