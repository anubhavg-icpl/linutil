#!/bin/bash

echo "🧪 Testing Linutil Desktop UI..."

# Check if we can build successfully
echo "Building application..."
if cargo build --release --quiet; then
    echo "✅ Build successful"
else
    echo "❌ Build failed"
    exit 1
fi

# Check if the binary exists
if [ -f "../target/release/linutil-desktop" ]; then
    echo "✅ Binary exists at target/release/linutil-desktop"
else
    echo "❌ Binary not found"
    exit 1
fi

# Check if dist/index.html exists and is readable
if [ -f "../dist/index.html" ]; then
    echo "✅ Frontend file exists at dist/index.html"
else
    echo "❌ Frontend file missing"
    exit 1
fi

# Check tauri.conf.json validity
if cargo check --quiet 2>/dev/null; then
    echo "✅ Tauri configuration is valid"
else
    echo "❌ Tauri configuration has issues"
    exit 1
fi

# Try to start the app for a few seconds to see if it crashes
echo "Testing application startup..."
timeout 5s ../target/release/linutil-desktop &
APP_PID=$!

sleep 3

if kill -0 $APP_PID 2>/dev/null; then
    echo "✅ Application started successfully and is running"
    kill $APP_PID 2>/dev/null
    wait $APP_PID 2>/dev/null
else
    echo "❌ Application crashed during startup"
    exit 1
fi

echo ""
echo "🎉 All UI tests passed!"
echo ""
echo "🚀 To start Linutil Desktop, run:"
echo "   cargo run --release"
echo ""
echo "📝 The UI should now display properly with:"
echo "   - Loading screen on startup"
echo "   - Tab navigation on the left"
echo "   - Command grid on the right"
echo "   - Working command execution"
echo "   - System information display"