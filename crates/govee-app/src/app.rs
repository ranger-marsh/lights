//! Application state and [`eframe::App`] implementation.

use std::collections::HashMap;
use std::sync::mpsc;

use govee_core::models::{Color, Device, DeviceState};

use crate::worker::{Command, WorkerEvent};

// ── App state ─────────────────────────────────────────────────────────────────

/// Top-level application state, owned by the egui event loop.
pub struct GoveeApp {
    // ── Device data ──────────────────────────────────────────────────────────
    pub devices: Vec<Device>,
    pub states: HashMap<String, DeviceState>,
    pub selected: usize,

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

    // ── Channels ─────────────────────────────────────────────────────────────
    pub cmd_tx: mpsc::Sender<Command>,
    pub evt_rx: mpsc::Receiver<WorkerEvent>,
}

impl GoveeApp {
    /// Create a new app wired to the given channel pair.
    pub fn new(cmd_tx: mpsc::Sender<Command>, evt_rx: mpsc::Receiver<WorkerEvent>) -> Self {
        Self {
            devices: Vec::new(),
            states: HashMap::new(),
            selected: 0,
            status: "Starting\u{2026}".to_string(),
            status_is_error: false,
            pending_brightness: 100,
            pending_color: [1.0, 1.0, 1.0], // white
            pending_color_temp: 4_000,
            use_color_temp: false,
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

    // ── Event processing ──────────────────────────────────────────────────────

    /// Drain all pending [`WorkerEvent`]s and update state accordingly.
    /// Called at the start of every egui frame.
    pub fn process_events(&mut self) {
        while let Ok(evt) = self.evt_rx.try_recv() {
            match evt {
                WorkerEvent::Discovered(devices) => {
                    self.selected = 0;
                    self.devices = devices;
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
}

// ── eframe integration ────────────────────────────────────────────────────────

impl eframe::App for GoveeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events();
        crate::ui::draw(ctx, self);
    }
}

// ── Utilities ─────────────────────────────────────────────────────────────────

#[allow(dead_code)]
/// Parse a CSS-style hex color string (`RRGGBB` or `#RRGGBB`) into a [`Color`].
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
}
