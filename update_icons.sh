#!/bin/bash

# Ensure we are in the script's directory (tools root)
cd "$(dirname "$0")"

ICON_PATH="./icon.png"
TAURI_DIR="./iriebook-tauri-ui"

if [ ! -f "$ICON_PATH" ]; then
    echo "Error: $ICON_PATH not found."
    exit 1
fi

if [ ! -d "$TAURI_DIR" ]; then
    echo "Error: $TAURI_DIR directory not found."
    exit 1
fi

echo "Updating Tauri icons using $ICON_PATH..."
cd "$TAURI_DIR"

# Run the tauri icon generation command
# We use 'npm run tauri --' to pass arguments to the tauri CLI
npm run tauri -- icon "../$ICON_PATH"

if [ $? -eq 0 ]; then
    echo "Icons updated successfully!"
else
    echo "Error updating icons."
    exit 1
fi
