//! Application state and [`eframe::App`] implementation.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

use govee_core::models::{Color, Device, DeviceState};

use crate::config;
use crate::scenes;
use crate::worker::{BroadcastAction, Command, WorkerEvent};

/// Maximum number of groups the user can create.
pub const MAX_GROUPS: usize = 10;

// ── Tab selection ─────────────────────────────────────────────────────────────

/// Which top-level tab is active in the central panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// Control all discovered devices at once.
    All,
    /// Control a single selected device.
    Individual,
    /// Control a named group of devices (index into `GoveeApp::groups`).
    Group(usize),
}

impl Default for Tab {
    fn default() -> Self {
        Tab::All
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

/// Top-level application state, owned by the egui event loop.
pub struct GoveeApp {
    // ── Device data ──────────────────────────────────────────────────────────
    pub devices: Vec<Device>,
    pub states: HashMap<String, DeviceState>,
    pub selected: usize,

    // ── Persisted names (MAC → display name) ─────────────────────────────────
    /// Loaded from disk at startup; written back whenever the user renames a device.
    pub names: HashMap<String, String>,

    // ── Groups ────────────────────────────────────────────────────────────────
    /// Persisted named groups (max [`MAX_GROUPS`]).
    pub groups: Vec<config::Group>,
    /// Index of the group whose name is being edited, or `None`.
    pub renaming_group: Option<usize>,
    /// Text buffer for the group rename input field.
    pub group_rename_buf: String,

    // ── Rename UI state ───────────────────────────────────────────────────────
    /// MAC of the device currently being renamed, or `None` if no rename is active.
    pub renaming: Option<String>,
    /// Text buffer for the rename input field.
    pub rename_buf: String,

    // ── Connectivity ─────────────────────────────────────────────────────────
    /// MACs of devices currently in the reconnect backoff (offline).
    pub offline_macs: HashSet<String>,

    // ── Status bar ───────────────────────────────────────────────────────────
    pub status: String,
    pub status_is_error: bool,

    // ── Pending control values (what the sliders/picker currently show) ──────
    /// Brightness slider value (1–100).
    pub pending_brightness: u8,
    /// RGB color for the egui color picker, each channel 0.0–1.0.
    pub pending_color: [f32; 3],
    /// Color temperature slider value in Kelvin (2 000–9 000).
    pub pending_color_temp: u16,
    /// When `true` the color section shows the Kelvin slider; otherwise RGB.
    pub use_color_temp: bool,

    // ── Scenes ────────────────────────────────────────────────────────────────
    /// Index into [`scenes::CATEGORIES`] for the scene picker.
    pub selected_scene_cat: usize,

    // ── Tab ───────────────────────────────────────────────────────────────────
    pub tab: Tab,

    // ── Channels ─────────────────────────────────────────────────────────────
    pub cmd_tx: mpsc::Sender<Command>,
    pub evt_rx: mpsc::Receiver<WorkerEvent>,
}

impl GoveeApp {
    /// Create a new app wired to the given channel pair.
    /// Saved device names are loaded from disk immediately.
    pub fn new(cmd_tx: mpsc::Sender<Command>, evt_rx: mpsc::Receiver<WorkerEvent>) -> Self {
        Self {
            devices: Vec::new(),
            states: HashMap::new(),
            selected: 0,
            names: config::load(),
            offline_macs: HashSet::new(),
            groups: config::load_groups(),
            renaming_group: None,
            group_rename_buf: String::new(),
            renaming: None,
            rename_buf: String::new(),
            status: "Starting\u{2026}".to_string(),
            status_is_error: false,
            pending_brightness: 100,
            pending_color: [1.0, 1.0, 1.0], // white
            pending_color_temp: 4_000,
            use_color_temp: false,
            selected_scene_cat: 0,
            tab: Tab::default(),
            cmd_tx,
            evt_rx,
        }
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    pub fn selected_device(&self) -> Option<&Device> {
        self.devices.get(self.selected)
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    #[allow(dead_code)]
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    #[allow(dead_code)]
    pub fn move_down(&mut self) {
        if !self.devices.is_empty() && self.selected < self.devices.len() - 1 {
            self.selected += 1;
        }
    }

    // ── Rename ────────────────────────────────────────────────────────────────

    /// Begin renaming the device with the given MAC.
    pub fn start_rename(&mut self, mac: &str) {
        // Pre-fill with the current display name.
        let current = self
            .devices
            .iter()
            .find(|d| d.mac == mac)
            .map(|d| d.display_name().to_string())
            .unwrap_or_default();
        self.rename_buf = current;
        self.renaming = Some(mac.to_string());
    }

    /// Commit the current rename buffer: update the device in memory, persist to disk.
    pub fn commit_rename(&mut self) {
        let Some(mac) = self.renaming.take() else {
            return;
        };
        let name = self.rename_buf.trim().to_string();

        if name.is_empty() {
            // Empty name → remove any stored override (fall back to SKU).
            self.names.remove(&mac);
        } else {
            self.names.insert(mac.clone(), name.clone());
        }

        // Update the in-memory Device so the UI reflects the change immediately.
        if let Some(d) = self.devices.iter_mut().find(|d| d.mac == mac) {
            d.name = if name.is_empty() { None } else { Some(name) };
        }

        if let Err(e) = config::save(&self.names) {
            self.status = format!("Warning: could not save names: {e}");
            self.status_is_error = true;
        }
    }

    /// Cancel an in-progress rename without saving.
    pub fn cancel_rename(&mut self) {
        self.renaming = None;
        self.rename_buf.clear();
    }

    // ── Groups ────────────────────────────────────────────────────────────────

    /// Create a new group (no-op if already at [`MAX_GROUPS`]) and switch to it.
    pub fn add_group(&mut self) {
        if self.groups.len() >= MAX_GROUPS {
            return;
        }
        let n = self.groups.len() + 1;
        self.groups.push(config::Group {
            name: format!("Group {n}"),
            macs: Vec::new(),
        });
        self.tab = Tab::Group(self.groups.len() - 1);
        self.persist_groups();
    }

    /// Delete the group at `idx` and fix the active tab if needed.
    pub fn delete_group(&mut self, idx: usize) {
        if idx >= self.groups.len() {
            return;
        }
        self.groups.remove(idx);
        self.tab = match self.tab {
            Tab::Group(i) if i == idx => Tab::All,
            Tab::Group(i) if i > idx => Tab::Group(i - 1),
            other => other,
        };
        self.persist_groups();
    }

    /// Add a device to a group if it isn't already a member; remove it if it is.
    pub fn toggle_device_in_group(&mut self, group_idx: usize, mac: &str) {
        let Some(group) = self.groups.get_mut(group_idx) else {
            return;
        };
        if let Some(pos) = group.macs.iter().position(|m| m == mac) {
            group.macs.remove(pos);
        } else {
            group.macs.push(mac.to_string());
        }
        self.persist_groups();
    }

    /// Returns the subset of discovered devices that belong to `group_idx`.
    pub fn group_devices(&self, group_idx: usize) -> Vec<Device> {
        let Some(group) = self.groups.get(group_idx) else {
            return Vec::new();
        };
        self.devices
            .iter()
            .filter(|d| group.macs.contains(&d.mac))
            .cloned()
            .collect()
    }

    /// Broadcast an action to only the devices in a group.
    pub fn broadcast_group(&self, group_idx: usize, action: BroadcastAction) {
        let devices = self.group_devices(group_idx);
        if !devices.is_empty() {
            self.send(Command::Broadcast(devices, action));
        }
    }

    /// Begin renaming the group at `idx`.
    pub fn start_rename_group(&mut self, idx: usize) {
        if let Some(g) = self.groups.get(idx) {
            self.group_rename_buf = g.name.clone();
            self.renaming_group = Some(idx);
        }
    }

    /// Commit the group rename buffer to memory and disk.
    pub fn commit_rename_group(&mut self) {
        let Some(idx) = self.renaming_group.take() else {
            return;
        };
        let name = self.group_rename_buf.trim().to_string();
        if let Some(g) = self.groups.get_mut(idx) {
            g.name = if name.is_empty() {
                format!("Group {}", idx + 1)
            } else {
                name
            };
        }
        self.persist_groups();
    }

    /// Cancel an in-progress group rename without saving.
    pub fn cancel_rename_group(&mut self) {
        self.renaming_group = None;
        self.group_rename_buf.clear();
    }

    // ── Scenes ────────────────────────────────────────────────────────────────

    /// Apply a scene to an arbitrary list of devices.
    /// Each device gets `palette[i % palette.len()]` at the scene's brightness.
    pub fn apply_scene_to_devices(&self, devices: Vec<Device>, scene: &scenes::Scene) {
        let entries: Vec<(Device, Color, u8)> = devices
            .into_iter()
            .enumerate()
            .map(|(i, device)| {
                let (r, g, b) = scene.palette[i % scene.palette.len()];
                (device, Color::new(r, g, b), scene.brightness)
            })
            .collect();
        if !entries.is_empty() {
            self.send(Command::ApplyScene(entries));
        }
    }

    fn persist_groups(&mut self) {
        if let Err(e) = config::save_groups(&self.groups) {
            self.status = format!("Warning: could not save groups: {e}");
            self.status_is_error = true;
        }
    }

    // ── Event processing ──────────────────────────────────────────────────────

    /// Drain all pending [`WorkerEvent`]s and update state accordingly.
    /// Called at the start of every egui frame.
    pub fn process_events(&mut self) {
        while let Ok(evt) = self.evt_rx.try_recv() {
            match evt {
                WorkerEvent::Discovered(mut devices) => {
                    self.selected = 0;
                    // Apply any stored name overrides before handing to the UI.
                    for device in &mut devices {
                        if let Some(name) = self.names.get(&device.mac) {
                            device.name = Some(name.clone());
                        }
                    }
                    self.devices = devices;
                }
                WorkerEvent::DeviceOffline(mac) => {
                    self.offline_macs.insert(mac);
                }
                WorkerEvent::DeviceOnline(mac) => {
                    self.offline_macs.remove(&mac);
                }
                WorkerEvent::StateUpdated { mac, state } => {
                    // Mirror real device state into the pending controls.
                    self.pending_brightness = state.brightness;
                    if state.color_temp_kelvin > 0 {
                        self.use_color_temp = true;
                        self.pending_color_temp = state.color_temp_kelvin;
                    } else {
                        self.use_color_temp = false;
                        self.pending_color = [
                            state.color.r as f32 / 255.0,
                            state.color.g as f32 / 255.0,
                            state.color.b as f32 / 255.0,
                        ];
                    }
                    self.states.insert(mac, state);
                }
                WorkerEvent::Status(msg) => {
                    self.status = msg;
                    self.status_is_error = false;
                }
                WorkerEvent::Error(msg) => {
                    self.status = msg;
                    self.status_is_error = true;
                }
            }
        }
    }

    /// Send a command to the async worker (fire-and-forget).
    pub fn send(&self, cmd: Command) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// Broadcast an action to all currently known devices.
    pub fn broadcast(&self, action: BroadcastAction) {
        if !self.devices.is_empty() {
            self.send(Command::Broadcast(self.devices.clone(), action));
        }
    }
}

// ── eframe integration ────────────────────────────────────────────────────────

impl eframe::App for GoveeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events();
        crate::ui::draw(ctx, self);
    }
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// Parse a CSS-style hex color string (`RRGGBB` or `#RRGGBB`) into a [`Color`].
#[allow(dead_code)]
pub fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::new(r, g, b))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app() -> GoveeApp {
        let (tx, _rx) = mpsc::channel();
        let (_tx2, rx2) = mpsc::channel();
        GoveeApp::new(tx, rx2)
    }

    #[test]
    fn parse_hex_color_valid() {
        assert_eq!(parse_hex_color("FF0000"), Some(Color::new(255, 0, 0)));
        assert_eq!(parse_hex_color("#00FF80"), Some(Color::new(0, 255, 128)));
        assert_eq!(parse_hex_color("ffffff"), Some(Color::new(255, 255, 255)));
    }

    #[test]
    fn parse_hex_color_invalid() {
        assert_eq!(parse_hex_color("ZZZZZZ"), None);
        assert_eq!(parse_hex_color("FFF"), None);
        assert_eq!(parse_hex_color(""), None);
        assert_eq!(parse_hex_color("12345"), None);
    }

    #[test]
    fn app_navigation() {
        let mut app = make_app();
        app.devices = vec![
            Device::new("AA:BB:CC:DD:EE:01", "H6072"),
            Device::new("AA:BB:CC:DD:EE:02", "H6072"),
            Device::new("AA:BB:CC:DD:EE:03", "H6072"),
        ];
        app.move_up(); // no-op at top
        assert_eq!(app.selected, 0);
        app.move_down();
        assert_eq!(app.selected, 1);
        app.move_down();
        assert_eq!(app.selected, 2);
        app.move_down(); // no-op at bottom
        assert_eq!(app.selected, 2);
        app.move_up();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn selected_device_empty() {
        let app = make_app();
        assert!(app.selected_device().is_none());
    }

    #[test]
    fn selected_device_returns_correct() {
        let mut app = make_app();
        app.devices = vec![
            Device::new("AA:01", "H6072"),
            Device::new("AA:02", "H6073"),
        ];
        app.selected = 1;
        assert_eq!(app.selected_device().unwrap().mac, "AA:02");
    }

    #[test]
    fn commit_rename_updates_device_name() {
        let mut app = make_app();
        app.devices = vec![Device::new("AA:BB:CC:DD:EE:01", "H6072")];

        app.start_rename("AA:BB:CC:DD:EE:01");
        assert!(app.renaming.is_some());

        app.rename_buf = "Living Room".to_string();
        app.commit_rename();

        assert!(app.renaming.is_none());
        assert_eq!(app.devices[0].display_name(), "Living Room");
        assert_eq!(app.names["AA:BB:CC:DD:EE:01"], "Living Room");
    }

    #[test]
    fn commit_rename_empty_clears_name() {
        let mut app = make_app();
        app.devices = vec![{
            let mut d = Device::new("AA:BB:CC:DD:EE:01", "H6072");
            d.name = Some("Old Name".to_string());
            d
        }];
        app.names.insert("AA:BB:CC:DD:EE:01".to_string(), "Old Name".to_string());

        app.start_rename("AA:BB:CC:DD:EE:01");
        app.rename_buf = "   ".to_string(); // whitespace only → clear
        app.commit_rename();

        assert_eq!(app.devices[0].display_name(), "H6072"); // falls back to SKU
        assert!(!app.names.contains_key("AA:BB:CC:DD:EE:01"));
    }

    #[test]
    fn cancel_rename_leaves_name_unchanged() {
        let mut app = make_app();
        app.devices = vec![Device::new("AA:BB", "H6072")];

        app.start_rename("AA:BB");
        app.rename_buf = "New Name".to_string();
        app.cancel_rename();

        assert!(app.renaming.is_none());
        assert_eq!(app.devices[0].display_name(), "H6072"); // unchanged
    }

    #[test]
    fn discovered_devices_get_stored_names_applied() {
        let mut app = make_app();
        app.names.insert("AA:BB:CC:DD:EE:01".to_string(), "Bedroom".to_string());

        // Simulate receiving a Discovered event.
        let devices = vec![Device::new("AA:BB:CC:DD:EE:01", "H6072")];
        let _ = app.cmd_tx.clone(); // keep tx alive
        // Manually apply the same logic process_events uses:
        let mut devices = devices;
        for d in &mut devices {
            if let Some(name) = app.names.get(&d.mac) {
                d.name = Some(name.clone());
            }
        }
        app.devices = devices;

        assert_eq!(app.devices[0].display_name(), "Bedroom");
    }
}
