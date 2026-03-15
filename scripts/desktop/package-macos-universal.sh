#!/bin/bash
set -euo pipefail

# Change to project root
cd "$(dirname "$0")/../.."
PROJECT_ROOT=$(pwd)

CERT_ZIP="scripts/desktop/macos-certs.zip"
CERT_DIR="scripts/desktop/certs_unzipped"
CERT_PWD="MyBytover123!"

# Cleanup any previous runs
rm -rf "$CERT_DIR"

# Source production environment variables if they exist
if [ -f "scripts/desktop/prod-env.env" ]; then
  echo "--- Loading production environment variables ---"
  set -a
  source scripts/desktop/prod-env.env
  set +a
fi

# Unzip certificates folder
if [ -f "$CERT_ZIP" ]; then
  echo "--- Unzipping certificates folder ---"
  unzip -P "$CERT_PWD" "$CERT_ZIP" -d "$CERT_DIR" > /dev/null
  
  # Export the folder path for reference (absolute path)
  export CERT_FOLDER_PATH="$PROJECT_ROOT/$CERT_DIR"
  
  # Source any environment variables from the unzipped folder
  if [ -f "$CERT_DIR/signing.env" ]; then
    echo "--- Loading signing environment variables from certs ---"
    set -a
    source "$CERT_DIR/signing.env"
    set +a
  fi
else
  echo "--- Warning: $CERT_ZIP not found. Signing may fail. ---"
fi

echo "--- Building shared types ---"
cargo build -p shared_types --target wasm32-unknown-unknown --no-default-features --features typescript

echo "--- Packaging Desktop build (macOS Universal) ---"
cd desktop
pnpm install

if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
  echo "--- Signing enabled with identity: $APPLE_SIGNING_IDENTITY ---"
fi

if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_PASSWORD:-}" ]; then
  echo "--- Notarization enabled ---"
fi

echo "--- Running Tauri build for universal binary ---"
pnpm tauri build --target universal-apple-darwin

# Cleanup
echo "--- Cleaning up unzipped certificates ---"
cd "$PROJECT_ROOT"
rm -rf "$CERT_DIR"

# Find and upload the DMG package
echo "--- Finding DMG package ---"
DMG_FILE=$(find desktop/src-tauri/target/release/bundle/dmg -name "*.dmg" 2>/dev/null | head -n 1)
if [ -z "$DMG_FILE" ]; then
  echo "--- Error: No DMG package found ---"
  exit 1
fi
echo "--- Found DMG: $DMG_FILE ---"

# Upload to S3 (fail if upload fails)
echo "--- Uploading to S3 ---"
scripts/desktop/upload-package.sh "$DMG_FILE" || exit 1
echo "--- Upload complete ---"
