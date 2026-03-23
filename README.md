# govee-lights

A lightweight Govee lights controller for Raspberry Pi, written in Rust.

## Features

- **LAN API** (primary): local UDP control, no internet, no rate limits
- **HTTP API** (optional): cloud control with your Govee API key
- **TUI frontend**: terminal UI via `ratatui`, works over SSH
- Zero GPU dependency — runs headless on any Raspberry Pi

## Quick Start

### Prerequisites

1. Enable **LAN Control** in the Govee Home app for each device:
   `Device Settings → LAN Control → Enable`

2. Ensure your Raspberry Pi is on the same network as the lights.

### Build & Run

```bash
# Install Rust (if not present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build in release mode
cargo build --release

# Run the TUI
./target/release/govee

# With HTTP API fallback (optional)
GOVEE_API_KEY=your-key-here ./target/release/govee
```

### Keyboard Controls

| Key | Action |
|-----|--------|
| `↑` / `k` | Select previous device |
| `↓` / `j` | Select next device |
| `Space` | Toggle power on/off |
| `b` | Set brightness (1–100) |
| `c` | Set color (hex, e.g. `FF8000`) |
| `t` | Set color temperature (2000–9000 K) |
| `r` | Refresh device state |
| `q` / `Esc` | Quit |

## Architecture

```
lights/
├── crates/
│   ├── govee-core/          # Library crate
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── models.rs    # Device, Color, DeviceState, Command
│   │       ├── error.rs     # GoveeError, Result
│   │       ├── lan.rs       # LAN UDP client (primary)
│   │       └── http.rs      # Cloud HTTP client (optional)
│   └── govee-app/           # Binary crate (TUI)
│       └── src/
│           ├── main.rs      # Terminal setup / teardown
│           ├── app.rs       # State machine + event loop
│           └── ui.rs        # ratatui rendering
```

## Testing

```bash
# Run all unit tests (no hardware required)
cargo test

# Run with logging
RUST_LOG=debug cargo test

# Run only core library tests
cargo test -p govee-core
```

## Crates Used

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime (UDP + HTTP) |
| `serde` / `serde_json` | JSON serialization |
| `reqwest` | HTTP cloud API |
| `thiserror` | Ergonomic error types |
| `ratatui` | Terminal UI |
| `crossterm` | Cross-platform terminal input/output |
| `tracing` | Structured logging |

## Raspberry Pi Notes

- Tested on Pi 4 / Pi 5 with Raspberry Pi OS (64-bit)
- Runs over SSH — no display server needed
- Release build size: ~3 MB (with `strip = true`, `opt-level = "z"`)
- Recommend `tmux` or `screen` for persistent sessions

## LAN API Protocol Reference

| Port | Role |
|------|------|
| 4001 | Devices listen (multicast `239.255.255.250`) |
| 4002 | Client listens for responses |
| 4003 | Client sends control commands |

All messages are JSON: `{"msg": {"cmd": "<cmd>", "data": {...}}}`
