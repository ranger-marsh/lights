//! Background async worker — bridges egui (sync) and the Govee LAN API (async).
//!
//! The GUI sends [`Command`]s over an `mpsc` channel; the worker executes them
//! against the LAN client and sends [`WorkerEvent`]s back.  Because egui only
//! repaints on user interaction, the worker also calls [`egui::Context::request_repaint`]
//! every time it pushes an event so the UI updates immediately.
//!
//! # Reconnect behaviour
//! When a device fails a state refresh it is added to an offline table.
//! The worker retries it in the background using a stepped backoff:
//! 5 s → 10 s → 30 s → 60 s (stays at 60 s until it responds).

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use govee_core::{
    lan::LanClient,
    models::{Color, Device, DeviceState},
};
use tracing::warn;

// ── Timing constants ──────────────────────────────────────────────────────────

/// How long to wait for a devStatus response from a device.
const STATE_TIMEOUT: Duration = Duration::from_secs(2);

/// How long to wait after sending a control command before querying state.
const POST_COMMAND_DELAY: Duration = Duration::from_millis(400);

/// Stepped backoff intervals (seconds): 5 → 10 → 30 → 60.
const BACKOFF_SECS: [u64; 4] = [5, 10, 30, 60];

fn backoff(retry_count: u32) -> Duration {
    Duration::from_secs(BACKOFF_SECS[retry_count.min(3) as usize])
}

// ── Offline tracking ──────────────────────────────────────────────────────────

struct OfflineEntry {
    device: Device,
    retry_count: u32,
    next_retry: Instant,
}

// ── Public channel types ──────────────────────────────────────────────────────

/// Commands sent from the GUI thread to the async worker.
#[derive(Debug)]
pub enum Command {
    SetPower(Device, bool),
    SetBrightness(Device, u8),
    SetColor(Device, Color),
    SetColorTemp(Device, u16),
    RefreshState(Device),
    /// Refresh the state of every known device at once.
    RefreshAll,
    Rediscover,
    /// Send the same action to every device in the list, then refresh each.
    Broadcast(Vec<Device>, BroadcastAction),
    /// Apply a scene: turn each device on, set its paired colour and brightness,
    /// then refresh.  Each entry is `(device, rgb_color, brightness_pct)`.
    ApplyScene(Vec<(Device, Color, u8)>),
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
    /// A device failed a state refresh and has entered the reconnect backoff.
    DeviceOffline(String),
    /// A device that was offline has successfully responded again.
    DeviceOnline(String),
    Status(String),
    Error(String),
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Spawn the background worker on a dedicated OS thread with its own Tokio runtime.
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

    // All devices ever discovered this session — used for RefreshAll and
    // as the source of truth for offline reconnect attempts.
    let mut known: Vec<Device> = Vec::new();
    // Devices that failed a state refresh, keyed by MAC.
    let mut offline: HashMap<String, OfflineEntry> = HashMap::new();

    discover(&lan, &send, &mut known, &mut offline).await;

    loop {
        // Retry any offline devices whose backoff timer has expired.
        if offline.values().any(|e| e.next_retry <= Instant::now()) {
            retry_offline(&lan, &send, &mut offline).await;
        }

        match cmd_rx.try_recv() {
            Ok(cmd) => handle_command(&lan, &send, cmd, &mut known, &mut offline).await,
            Err(mpsc::TryRecvError::Empty) => {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
    }
}

// ── Discovery ────────────────────────────────────────────────────────────────

/// Discover all devices, refresh their state, and populate the offline map
/// for any that don't respond.
async fn discover(
    lan: &LanClient,
    send: &impl Fn(WorkerEvent),
    known: &mut Vec<Device>,
    offline: &mut HashMap<String, OfflineEntry>,
) {
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

            // Fresh discovery clears stale offline entries for rediscovered devices.
            for d in &devices {
                offline.remove(&d.mac);
            }

            for device in &devices {
                try_refresh(lan, send, device, offline).await;
            }

            *known = devices;
            send(WorkerEvent::Status(format!("Ready \u{2014} {count} device(s).")));
        }
        Err(e) => {
            warn!("Discovery error: {e}");
            send(WorkerEvent::Discovered(vec![]));
            send(WorkerEvent::Error(format!("Discovery failed: {e}")));
        }
    }
}

// ── Offline retry ─────────────────────────────────────────────────────────────

/// Attempt to refresh every offline device whose backoff timer has expired.
async fn retry_offline(
    lan: &LanClient,
    send: &impl Fn(WorkerEvent),
    offline: &mut HashMap<String, OfflineEntry>,
) {
    let now = Instant::now();
    let due: Vec<String> = offline
        .iter()
        .filter(|(_, e)| e.next_retry <= now)
        .map(|(mac, _)| mac.clone())
        .collect();

    for mac in due {
        let (device, retry_count) = match offline.get(&mac) {
            Some(e) => (e.device.clone(), e.retry_count),
            None => continue,
        };

        match lan.get_state(&device, STATE_TIMEOUT).await {
            Ok(state) => {
                offline.remove(&mac);
                send(WorkerEvent::DeviceOnline(mac.clone()));
                send(WorkerEvent::StateUpdated { mac, state });
                send(WorkerEvent::Status(format!(
                    "{} reconnected.",
                    device.display_name()
                )));
            }
            Err(_) => {
                if let Some(e) = offline.get_mut(&mac) {
                    e.retry_count = retry_count + 1;
                    e.next_retry = Instant::now() + backoff(retry_count + 1);
                }
            }
        }
    }
}

// ── State refresh ─────────────────────────────────────────────────────────────

/// Query a device's state. On success sends [`WorkerEvent::StateUpdated`] and
/// removes the device from the offline map. On failure, adds it to the offline
/// map (or updates its backoff if already there) and sends [`WorkerEvent::DeviceOffline`].
async fn try_refresh(
    lan: &LanClient,
    send: &impl Fn(WorkerEvent),
    device: &Device,
    offline: &mut HashMap<String, OfflineEntry>,
) {
    match lan.get_state(device, STATE_TIMEOUT).await {
        Ok(state) => {
            // If it was previously offline, announce reconnection.
            if offline.remove(&device.mac).is_some() {
                send(WorkerEvent::DeviceOnline(device.mac.clone()));
            }
            send(WorkerEvent::StateUpdated {
                mac: device.mac.clone(),
                state,
            });
        }
        Err(e) => {
            warn!("State refresh failed for {}: {e}", device.display_name());
            let already_tracked = offline.contains_key(&device.mac);
            offline
                .entry(device.mac.clone())
                .or_insert_with(|| OfflineEntry {
                    device: device.clone(),
                    retry_count: 0,
                    next_retry: Instant::now() + backoff(0),
                });
            // Only fire the event the first time a device goes offline.
            if !already_tracked {
                send(WorkerEvent::DeviceOffline(device.mac.clone()));
                send(WorkerEvent::Error(format!(
                    "{} is offline \u{2014} retrying\u{2026}",
                    device.display_name()
                )));
            }
        }
    }
}

/// Wait briefly after a control command, then refresh state.
async fn post_command_refresh(
    lan: &LanClient,
    send: &impl Fn(WorkerEvent),
    device: &Device,
    offline: &mut HashMap<String, OfflineEntry>,
) {
    tokio::time::sleep(POST_COMMAND_DELAY).await;
    try_refresh(lan, send, device, offline).await;
}

// ── Command handling ──────────────────────────────────────────────────────────

async fn handle_command(
    lan: &LanClient,
    send: &impl Fn(WorkerEvent),
    cmd: Command,
    known: &mut Vec<Device>,
    offline: &mut HashMap<String, OfflineEntry>,
) {
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
            post_command_refresh(lan, send, &device, offline).await;
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
            post_command_refresh(lan, send, &device, offline).await;
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
            post_command_refresh(lan, send, &device, offline).await;
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
            post_command_refresh(lan, send, &device, offline).await;
        }

        Command::RefreshState(device) => {
            try_refresh(lan, send, &device, offline).await;
            if !offline.contains_key(&device.mac) {
                send(WorkerEvent::Status(format!(
                    "{} state refreshed.",
                    device.display_name()
                )));
            }
        }

        Command::RefreshAll => {
            if known.is_empty() {
                send(WorkerEvent::Status("No devices to refresh.".into()));
                return;
            }
            let count = known.len();
            send(WorkerEvent::Status(format!(
                "Refreshing {count} device(s)\u{2026}"
            )));
            let snapshot = known.clone();
            for device in &snapshot {
                try_refresh(lan, send, device, offline).await;
            }
            let online = count - offline.len();
            send(WorkerEvent::Status(format!(
                "Refreshed \u{2014} {online}/{count} online."
            )));
        }

        Command::Rediscover => {
            offline.clear();
            discover(lan, send, known, offline).await;
        }

        Command::ApplyScene(entries) => {
            let count = entries.len();
            send(WorkerEvent::Status(format!(
                "Applying scene to {count} device(s)\u{2026}"
            )));

            for (device, color, brightness) in &entries {
                // Turn the light on so the scene is immediately visible.
                let _ = lan.set_power(device, true).await;
                if let Err(e) = lan.set_brightness(device, *brightness).await {
                    warn!("Scene brightness error for {}: {e}", device.display_name());
                }
                if let Err(e) = lan.set_color(device, *color).await {
                    warn!("Scene color error for {}: {e}", device.display_name());
                }
            }

            tokio::time::sleep(POST_COMMAND_DELAY).await;
            for (device, _, _) in &entries {
                try_refresh(lan, send, device, offline).await;
            }
            send(WorkerEvent::Status("Scene applied.".into()));
        }

        Command::Broadcast(devices, action) => {
            let label = match &action {
                BroadcastAction::Power(on) => {
                    format!("Turning all lights {}\u{2026}", if *on { "ON" } else { "OFF" })
                }
                BroadcastAction::Brightness(pct) => {
                    format!("Setting all brightness \u{2192} {pct}%\u{2026}")
                }
                BroadcastAction::Color(c) => format!("Setting all color \u{2192} {c}\u{2026}"),
                BroadcastAction::ColorTemp(k) => {
                    format!("Setting all color temp \u{2192} {k}K\u{2026}")
                }
            };
            send(WorkerEvent::Status(label));

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

            tokio::time::sleep(POST_COMMAND_DELAY).await;
            for device in &devices {
                try_refresh(lan, send, device, offline).await;
            }
            send(WorkerEvent::Status("All lights updated.".into()));
        }
    }
}
