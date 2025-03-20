#!/bin/bash

# Description: Script to build once and run app on multiple simulators
# Run on terminal: /bin/sh buildAndRunMultipleSimulators.sh (uncomment all UDIDs on simulators section of this script)
# Author: Rob Enriquez
# Last Update: 8/31/2024

# Example project
# project name = BitBridge
# app name = BitBridge
# scheme = BitBridge
# bundle identifier = com.robe.games.ios.BitBridge

# Define your project and scheme
project_name="BitBridge"
scheme="BitBridge"
app_name="BitBridge"
bundle_identifier="com.devlog.bitbridge.BitBridge"
sdk="iphonesimulator"
configuration="Debug"
project_path="$(dirname "$0")/${app_name}.xcodeproj"  # Relative path to the Xcode project

# Define simulators
# open Run Destinations in Xcode Cmd+Shift+2 to check the device name and identifier
# or in terminal: xcrun simctl list devices

simulators=(
    "iPhone 16"
    "iPhone 16 Pro"
)

# Build the Xcode project
echo "Building the Xcode project..."
xcodebuild -project "$project_path" -scheme "$scheme" -sdk "$sdk" -configuration "$configuration" build

# Find the .app file
app_path=$(find ~/Library/Developer/Xcode/DerivedData/${project_name}-*/Build/Products/Debug-iphonesimulator -name "${app_name}.app" | head -n 1)

if [ -z "$app_path" ]; then
    echo "Error: .app file not found."
    exit 1
fi

echo "App found at: $app_path"

# Install and launch the app on each simulator
for simulator in "${simulators[@]}"; do
    # Check if the simulator is already booted
    if ! xcrun simctl list booted | grep -q "$simulator"; then
        echo "Booting simulator $simulator..."
        xcrun simctl boot "$simulator"
    else
        echo "Simulator $simulator is already booted."
    fi

    echo "Installing app on simulator $simulator..."
    xcrun simctl install "$simulator" "$app_path"

    echo "Launching app on simulator $simulator..."
    xcrun simctl launch "$simulator" "$bundle_identifier"
done

echo "All operations completed."
