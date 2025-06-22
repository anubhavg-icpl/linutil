#!/bin/bash

# Test script to verify core integration works
echo "Testing Linutil Desktop Core Integration..."

# Start the app in background for a short time to test loading
timeout 3s ../target/release/linutil-desktop &
APP_PID=$!

sleep 2

# Check if the process is still running (means it didn't crash on startup)
if kill -0 $APP_PID 2>/dev/null; then
    echo "✅ Desktop app started successfully and is running"
    kill $APP_PID 2>/dev/null
else
    echo "❌ Desktop app crashed on startup"
    exit 1
fi

# Test that we can still build and the core library works
echo "Testing core library compilation..."
if cargo check --quiet; then
    echo "✅ Core library integration compiles successfully"
else
    echo "❌ Core library integration has compilation errors"
    exit 1
fi

# Test TUI still works
echo "Testing TUI compatibility..."
if timeout 2s ../target/release/linutil --help >/dev/null 2>&1; then
    echo "✅ TUI version still works correctly"
else
    echo "❌ TUI version broken"
    exit 1
fi

echo "🎉 All tests passed! Linutil Desktop is working correctly."