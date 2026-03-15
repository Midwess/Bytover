#!/bin/bash
set -euo pipefail

# Change to project root
cd "$(dirname "$0")/../.."

echo "=============================================="
echo "  Building and uploading all desktop packages"
echo "=============================================="
echo ""

# Track overall success
FAILED=0

# Function to run a packaging script
run_package() {
    local script="$1"
    local name="$2"

    echo ""
    echo "=============================================="
    echo "  Building: $name"
    echo "=============================================="

    if scripts/desktop/"$script"; then
        echo "--- $name: SUCCESS ---"
    else
        echo "--- $name: FAILED ---"
        FAILED=1
    fi
    echo ""
}

# Build and upload for all platforms
run_package "package-macos-universal.sh" "macOS Universal"
run_package "package-linux-x64.sh" "Linux x64"
run_package "package-windows-x64.sh" "Windows x64"

echo "=============================================="
echo "  Summary"
echo "=============================================="

if [ $FAILED -eq 0 ]; then
    echo "All platforms built and uploaded successfully!"
    exit 0
else
    echo "Some platforms failed. Check logs above."
    exit 1
fi
