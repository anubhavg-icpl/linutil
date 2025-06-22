#!/bin/bash

echo "🐧 Starting Linutil Desktop (eGUI)..."

cd "$(dirname "$0")/egui-desktop"

if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found. Make sure you're in the right directory."
    exit 1
fi

echo "Building and running the desktop application..."
cargo run --release

echo "Desktop application closed."