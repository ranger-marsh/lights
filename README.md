# govee-lights

A touch-friendly Govee lights controller for Raspberry Pi, written in Rust.
Controls lights locally over UDP — no internet connection, no cloud API, no rate limits.

## Features

- **Local LAN control** — pure UDP, works on an air-gapped network
- **Touch GUI** — egui/eframe (OpenGL), optimised for 800×480 touchscreens
- **Device discovery** — automatically finds all Govee devices on the network
- **Per-device control** — power, brightness, RGB color, color temperature
- **Device naming** — assign friendly names, persisted to disk
- **All Lights tab** — control every discovered light at once
- **Groups** — create up to 10 named groups, each with its own tab
- **Scenes** — 5 categories (Nature, Holiday, Relax, Cosmic, Party), 10+ scenes each
- **Auto-reconnect** — retries offline devices with stepped backoff (5 s → 10 s → 30 s → 1 min)
- **Config portability** — names and groups saved as plain JSON, easy to copy to a new Pi

## Hardware

Tested on **Raspberry Pi 5** running **Raspberry Pi OS Bookworm (64-bit, desktop)**.
A display server (X11 or Wayland) is required — the app opens a native GUI window.

## Quick Start

### 1 — Enable LAN Control on each light

In the **Govee Home** app:
`Device → Settings → LAN Control → Enable`

### 2 — Install dependencies (Pi)

```bash
sudo apt update && sudo apt install -y \
  build-essential pkg-config git \
  libgl1-mesa-dev libgles2-mesa-dev libegl1-mesa-dev \
  libx11-dev libxi-dev libxrandr-dev libxcursor-dev \
  libxkbcommon-dev libwayland-dev
```

### 3 — Install Rust (Pi)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 4 — Clone and build

```bash
git clone https://github.com/ranger-marsh/lights.git
cd lights
cargo build --release
```

First build takes 20–30 minutes on a Pi 5. Subsequent builds are much faster.

### 5 — Run

```bash
./target/release/govee
```

## Desktop Shortcut (Pi)

Create a desktop icon and auto-start entry in one step:

```bash
mkdir -p ~/Desktop ~/.config/autostart

cat > ~/Desktop/govee-lights.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Govee Lights
Exec=/home/rupert/lights/target/release/govee
Icon=utilities-terminal
Terminal=false
Categories=Utility;
EOF

chmod +x ~/Desktop/govee-lights.desktop
cp ~/Desktop/govee-lights.desktop ~/.config/autostart/
```

## Config Files

Names and groups are stored as plain JSON. Copy them to a new Pi to transfer your setup:

| File | Contents |
|------|----------|
| `~/.config/govee-lights/names.json` | MAC → friendly name map |
| `~/.config/govee-lights/groups.json` | Group definitions |

```bash
# Example: copy from Mac to Pi
scp ~/Library/Application\ Support/govee-lights/names.json rupert@govee-pi.local:~/.config/govee-lights/
scp ~/Library/Application\ Support/govee-lights/groups.json rupert@govee-pi.local:~/.config/govee-lights/
```

## Architecture

```
lights/
├── crates/
│   ├── govee-core/          # Library crate (no UI dependency)
│   │   └── src/
│   │       ├── lib.rs       # Public re-exports
│   │       ├── models.rs    # Device, Color, DeviceState, Command
│   │       ├── error.rs     # GoveeError, Result
│   │       ├── lan.rs       # LAN UDP client (primary)
│   │       └── http.rs      # Cloud HTTP client (feature-gated)
│   └── govee-app/           # Binary crate (GUI)
│       └── src/
│           ├── main.rs      # Window setup, style, launch
│           ├── app.rs       # GoveeApp state + event processing
│           ├── ui.rs        # egui rendering (panels, tabs, controls)
│           ├── worker.rs    # Background async task (UDP I/O, reconnect)
│           ├── config.rs    # Load/save names.json and groups.json
│           └── scenes.rs    # Built-in scene definitions (5 categories)
```

### Worker / GUI communication

```
GoveeApp (egui main thread)
    │  Command (mpsc::Sender)
    ▼
Worker (tokio background task)
    │  WorkerEvent (mpsc::Sender) + ctx.request_repaint()
    ▼
GoveeApp::process_events()
```

## Testing

```bash
# Run all unit tests (no hardware required)
cargo test

# Run only core library tests
cargo test -p govee-core

# With debug logging
RUST_LOG=debug cargo test
```

## Crates Used

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime (UDP I/O, background worker) |
| `serde` / `serde_json` | JSON serialization for LAN protocol and config |
| `eframe` / `egui` | GUI framework (OpenGL via glow) |
| `dirs` | Platform config directory (`~/.config` on Linux) |
| `thiserror` | Ergonomic error types |
| `tracing` | Structured logging |
| `reqwest` | Cloud HTTP API (optional, `http-api` feature only) |

## LAN API Protocol

| Port | Role |
|------|------|
| 4001 | Devices listen (multicast `239.255.255.250`) |
| 4002 | App listens for scan responses |
| 4003 | App sends control commands (per-device unicast) |

All messages are JSON:

```json
{"msg": {"cmd": "<cmd>", "data": {...}}}
```

| Command | Notes |
|---------|-------|
| `scan` | Multicast discovery |
| `devStatus` | Query device state |
| `turn` | `value: 0` or `1` |
| `brightness` | `value: 1–100` |
| `colorwc` | RGB + `colorTemInKelvin`; set kelvin > 0 for white mode |

> **Note:** When setting color temperature, send `color: {r:255, g:255, b:255}`.
> Some devices (e.g. H60C1 Pendant) ignore the command if color is black.
