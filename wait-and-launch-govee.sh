#!/bin/bash
# Wait for Wayland and network, then launch govee.
# Used by the desktop autostart entry.

APP_BIN="$(cd "$(dirname "$0")" && pwd)/target/release/govee"

echo "Finding runtime"
export XDG_RUNTIME_DIR=/run/user/$(id -u)

echo "Wait to socket appears"
until WAYLAND_SOCK=$(ls "$XDG_RUNTIME_DIR"/wayland-* 2>/dev/null | grep -v '\.lock' | head -1) && [ -n "$WAYLAND_SOCK" ]; do
    sleep 1
done
export WAYLAND_DISPLAY=$(basename "$WAYLAND_SOCK")

echo "Wait for network"
until ip route | grep -q "^default"; do
    sleep 2
done

echo Launch Lights app
exec "$APP_BIN"

exit 0