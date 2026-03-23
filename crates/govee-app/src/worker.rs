//! Background async worker — bridges egui (sync) and the Govee LAN API (async).
//!
//! The GUI sends [`Command`]s over an `mpsc` channel; the worker executes them
//! against the LAN client and sends [`WorkerEvent`]s back.  Because egui only
//! repaints on user interaction, the worker also calls [`egui::Context::request_repaint`]
//! every time it pushes an event so the UI updates immediately.

use std::sync::mpsc;
use std::time::Duration;

use govee_core::{
    lan::LanClient,
    models::{Color, Device, DeviceState},
};
use tracing::warn;

// ── Public channel types ──────────────────────────────────────────────────────

/// Commands sent from the GUI thread to the async worker.
#[derive(Debug)]
pub enum Command {
    SetPower(Device, bool),
    SetBrightness(Device, u8),
    SetColor(Device, Color),
    SetColorTemp(Device, u16),
    RefreshState(Device),
    Rediscover,
}

/// Events sent from the async worker back to the GUI thread.
#[derive(Debug)]
pub enum WorkerEvent {
    Discovered(Vec<Device>),
    StateUpdated { mac: String, state: DeviceState },
    Status(String),
    Error(String),
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Spawn the background worker on a dedicated OS thread with its own Tokio runtime.
///
/// `ctx` is used to wake egui whenever a new event arrives.
pub fn spawn(
    cmd_rx: mpsc::Receiver<Command>,
    evt_tx: mpsc::Sender<WorkerEvent>,
    ctx: egui::Context,
) {
    std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(run(cmd_rx, evt_tx, ctx));
    });
}

// ── Async implementation ──────────────────────────────────────────────────────

async fn run(
    cmd_rx: mpsc::Receiver<Command>,
    evt_tx: mpsc::Sender<WorkerEvent>,
    ctx: egui::Context,
) {
    // Helper: send an event and wake the GUI.
    let send = |evt: WorkerEvent| {
        let _ = evt_tx.send(evt);
        ctx.request_repaint();
    };

    let lan = match LanClient::new().await {
        Ok(c) => c,
        Err(e) => {
            send(WorkerEvent::Error(format!("LAN init failed: {e}")));
            return;
        }
    };

    // Discover devices at startup.
    discover(&lan, &send).await;

    // Poll for commands. Using try_recv + a short sleep keeps the async
    // executor free for LAN I/O without burning CPU.
    loop {
        match cmd_rx.try_recv() {
            Ok(cmd) => handle_command(&lan, &send, cmd).await,
            Err(mpsc::TryRecvError::Empty) => {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
    }
}

async fn discover(lan: &LanClient, send: &impl Fn(WorkerEvent)) {
    send(WorkerEvent::Status("Discovering devices\u{2026}".into()));

    match lan.discover(Duration::from_secs(3)).await {
        Ok(devices) => {
            let msg = if devices.is_empty() {
                "No devices found. Enable LAN Control in the Govee app.".to_string()
            } else {
                format!("Found {} device(s).", devices.len())
            };
            send(WorkerEvent::Discovered(devices));
            send(WorkerEvent::Status(msg));
        }
        Err(e) => {
            warn!("Discovery error: {e}");
            send(WorkerEvent::Discovered(vec![]));
            send(WorkerEvent::Error(format!("Discovery failed: {e}")));
        }
    }
}

async fn handle_command(lan: &LanClient, send: &impl Fn(WorkerEvent), cmd: Command) {
    match cmd {
        Command::SetPower(device, on) => match lan.set_power(&device, on).await {
            Ok(_) => send(WorkerEvent::Status(format!(
                "{} turned {}.",
                device.display_name(),
                if on { "ON" } else { "OFF" }
            ))),
            Err(e) => send(WorkerEvent::Error(format!("Power error: {e}"))),
        },

        Command::SetBrightness(device, pct) => match lan.set_brightness(&device, pct).await {
            Ok(_) => send(WorkerEvent::Status(format!(
                "{} brightness \u{2192} {}%.",
                device.display_name(),
                pct
            ))),
            Err(e) => send(WorkerEvent::Error(format!("Brightness error: {e}"))),
        },

        Command::SetColor(device, color) => match lan.set_color(&device, color).await {
            Ok(_) => send(WorkerEvent::Status(format!(
                "{} color \u{2192} {}.",
                device.display_name(),
                color
            ))),
            Err(e) => send(WorkerEvent::Error(format!("Color error: {e}"))),
        },

        Command::SetColorTemp(device, k) => match lan.set_color_temp(&device, k).await {
            Ok(_) => send(WorkerEvent::Status(format!(
                "{} color temp \u{2192} {}K.",
                device.display_name(),
                k
            ))),
            Err(e) => send(WorkerEvent::Error(format!("Color temp error: {e}"))),
        },

        Command::RefreshState(device) => {
            match lan.get_state(&device, Duration::from_secs(2)).await {
                Ok(state) => {
                    let mac = device.mac.clone();
                    send(WorkerEvent::Status(format!(
                        "{} state refreshed.",
                        device.display_name()
                    )));
                    send(WorkerEvent::StateUpdated { mac, state });
                }
                Err(e) => send(WorkerEvent::Error(format!("Refresh error: {e}"))),
            }
        }

        Command::Rediscover => discover(lan, send).await,
    }
}
