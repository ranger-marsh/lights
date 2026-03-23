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

// ── Timing constants ──────────────────────────────────────────────────────────

/// How long to wait for a devStatus response from a device.
const STATE_TIMEOUT: Duration = Duration::from_secs(2);

/// How long to wait after sending a control command before querying state.
/// Gives the device time to apply the change before we read it back.
const POST_COMMAND_DELAY: Duration = Duration::from_millis(400);

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
    /// Send the same action to every device in the list, then refresh each.
    Broadcast(Vec<Device>, BroadcastAction),
}

/// The control action applied to all devices in a [`Command::Broadcast`].
#[derive(Debug, Clone)]
pub enum BroadcastAction {
    Power(bool),
    Brightness(u8),
    Color(Color),
    ColorTemp(u16),
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

    // Discover devices at startup, then immediately refresh all their states.
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

/// Discover all devices, then refresh the state of every found device.
async fn discover(lan: &LanClient, send: &impl Fn(WorkerEvent)) {
    send(WorkerEvent::Status("Discovering devices\u{2026}".into()));

    match lan.discover(Duration::from_secs(3)).await {
        Ok(devices) => {
            if devices.is_empty() {
                send(WorkerEvent::Discovered(vec![]));
                send(WorkerEvent::Status(
                    "No devices found. Enable LAN Control in the Govee app.".into(),
                ));
                return;
            }

            let count = devices.len();
            send(WorkerEvent::Discovered(devices.clone()));
            send(WorkerEvent::Status(format!(
                "Found {count} device(s). Refreshing state\u{2026}"
            )));

            // Refresh each device sequentially — the socket is shared, so
            // doing them one at a time avoids mixing up responses.
            for device in &devices {
                refresh_device(lan, send, device).await;
            }

            send(WorkerEvent::Status(format!("Ready — {count} device(s).")));
        }
        Err(e) => {
            warn!("Discovery error: {e}");
            send(WorkerEvent::Discovered(vec![]));
            send(WorkerEvent::Error(format!("Discovery failed: {e}")));
        }
    }
}

/// Query a single device's state and send a [`WorkerEvent::StateUpdated`].
async fn refresh_device(lan: &LanClient, send: &impl Fn(WorkerEvent), device: &Device) {
    match lan.get_state(device, STATE_TIMEOUT).await {
        Ok(state) => send(WorkerEvent::StateUpdated {
            mac: device.mac.clone(),
            state,
        }),
        Err(e) => {
            warn!("State refresh failed for {}: {e}", device.display_name());
            send(WorkerEvent::Error(format!(
                "Refresh error ({}): {e}",
                device.display_name()
            )));
        }
    }
}

async fn handle_command(lan: &LanClient, send: &impl Fn(WorkerEvent), cmd: Command) {
    match cmd {
        Command::SetPower(device, on) => {
            match lan.set_power(&device, on).await {
                Ok(_) => send(WorkerEvent::Status(format!(
                    "{} turned {}.",
                    device.display_name(),
                    if on { "ON" } else { "OFF" }
                ))),
                Err(e) => {
                    send(WorkerEvent::Error(format!("Power error: {e}")));
                    return;
                }
            }
            post_command_refresh(lan, send, &device).await;
        }

        Command::SetBrightness(device, pct) => {
            match lan.set_brightness(&device, pct).await {
                Ok(_) => send(WorkerEvent::Status(format!(
                    "{} brightness \u{2192} {}%.",
                    device.display_name(),
                    pct
                ))),
                Err(e) => {
                    send(WorkerEvent::Error(format!("Brightness error: {e}")));
                    return;
                }
            }
            post_command_refresh(lan, send, &device).await;
        }

        Command::SetColor(device, color) => {
            match lan.set_color(&device, color).await {
                Ok(_) => send(WorkerEvent::Status(format!(
                    "{} color \u{2192} {}.",
                    device.display_name(),
                    color
                ))),
                Err(e) => {
                    send(WorkerEvent::Error(format!("Color error: {e}")));
                    return;
                }
            }
            post_command_refresh(lan, send, &device).await;
        }

        Command::SetColorTemp(device, k) => {
            match lan.set_color_temp(&device, k).await {
                Ok(_) => send(WorkerEvent::Status(format!(
                    "{} color temp \u{2192} {}K.",
                    device.display_name(),
                    k
                ))),
                Err(e) => {
                    send(WorkerEvent::Error(format!("Color temp error: {e}")));
                    return;
                }
            }
            post_command_refresh(lan, send, &device).await;
        }

        Command::RefreshState(device) => {
            refresh_device(lan, send, &device).await;
            send(WorkerEvent::Status(format!(
                "{} state refreshed.",
                device.display_name()
            )));
        }

        Command::Rediscover => discover(lan, send).await,

        Command::Broadcast(devices, action) => {
            let label = match &action {
                BroadcastAction::Power(on) => {
                    format!("Turning all lights {}…", if *on { "ON" } else { "OFF" })
                }
                BroadcastAction::Brightness(pct) => format!("Setting all brightness → {pct}%…"),
                BroadcastAction::Color(c) => format!("Setting all color → {c}…"),
                BroadcastAction::ColorTemp(k) => format!("Setting all color temp → {k}K…"),
            };
            send(WorkerEvent::Status(label));

            // Send the command to every device, ignoring per-device errors so
            // one unresponsive light doesn't block the rest.
            for device in &devices {
                let result = match &action {
                    BroadcastAction::Power(on) => lan.set_power(device, *on).await,
                    BroadcastAction::Brightness(pct) => lan.set_brightness(device, *pct).await,
                    BroadcastAction::Color(c) => lan.set_color(device, *c).await,
                    BroadcastAction::ColorTemp(k) => lan.set_color_temp(device, *k).await,
                };
                if let Err(e) = result {
                    warn!("Broadcast error for {}: {e}", device.display_name());
                    send(WorkerEvent::Error(format!("{}: {e}", device.display_name())));
                }
            }

            // Wait for all devices to apply the change, then refresh each.
            tokio::time::sleep(POST_COMMAND_DELAY).await;
            for device in &devices {
                refresh_device(lan, send, device).await;
            }
            send(WorkerEvent::Status("All lights updated.".into()));
        }
    }
}

/// Wait briefly for the device to apply a command, then read its state back.
async fn post_command_refresh(lan: &LanClient, send: &impl Fn(WorkerEvent), device: &Device) {
    tokio::time::sleep(POST_COMMAND_DELAY).await;
    refresh_device(lan, send, device).await;
}
