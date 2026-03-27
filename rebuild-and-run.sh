#!/bin/bash
# Rebuild govee and relaunch it.
# Run this on the Pi after rsyncing updated source files.

set -e

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_BIN="$PROJECT_DIR/target/release/govee"


echo "==> Building release binary..."
cd "$PROJECT_DIR"
cargo build --release

echo "==> Stopping govee if running..."
pkill -x govee 2>/dev/null && echo "    Killed existing process." || echo "    Not running, continuing."
sleep 1

echo "==> Launching govee..."
export XDG_RUNTIME_DIR=/run/user/$(id -u)
WAYLAND_SOCK=$(ls "$XDG_RUNTIME_DIR"/wayland-* 2>/dev/null | grep -v '\.lock' | head -1)
if [ -n "$WAYLAND_SOCK" ]; then
    export WAYLAND_DISPLAY=$(basename "$WAYLAND_SOCK")
    echo "    Using Wayland display: $WAYLAND_DISPLAY"
else
    echo "    Warning: no Wayland socket found in $XDG_RUNTIME_DIR"
fi
export DISPLAY=:0

nohup "$APP_BIN" > "$HOME/govee-lights.log" 2>&1 &
echo ""
echo "Done! govee is running (PID $!)."
echo "Logs: $HOME/govee-lights.log"
