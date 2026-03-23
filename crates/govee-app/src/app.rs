//! Application state and event loop.

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use govee_core::{
    lan::LanClient,
    models::{Color, Device, DeviceState},
};
use ratatui::{Terminal, backend::Backend};
use tracing::warn;

use crate::ui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Prompt(PromptKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    Brightness,
    Color,
    ColorTemp,
}

pub struct App {
    pub devices: Vec<Device>,
    pub states: std::collections::HashMap<String, DeviceState>,
    pub selected: usize,
    pub input_mode: InputMode,
    pub input_buf: String,
    pub status: String,
    pub quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            states: std::collections::HashMap::new(),
            selected: 0,
            input_mode: InputMode::Normal,
            input_buf: String::new(),
            status: "Discovering devices\u{2026}".to_string(),
            quit: false,
        }
    }

    pub fn selected_device(&self) -> Option<&Device> {
        self.devices.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.devices.is_empty() && self.selected < self.devices.len() - 1 {
            self.selected += 1;
        }
    }
}

pub async fn run<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    let lan = LanClient::new().await?;
    let mut app = App::new();

    match lan.discover(Duration::from_secs(3)).await {
        Ok(devices) => {
            app.status = if devices.is_empty() {
                "No devices found. Enable LAN Control in the Govee app.".to_string()
            } else {
                format!("Found {} device(s). Press ? for help.", devices.len())
            };
            app.devices = devices;
        }
        Err(e) => {
            app.status = format!("Discovery failed: {e}");
            warn!("Discovery error: {e}");
        }
    }

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }

        let evt = event::read()?;
        let mode = app.input_mode.clone();

        match mode {
            InputMode::Normal => handle_normal(&mut app, &lan, evt).await,
            InputMode::Prompt(kind) => handle_prompt(&mut app, &lan, kind, evt).await,
        }

        if app.quit {
            break;
        }
    }

    Ok(())
}

async fn handle_normal(app: &mut App, lan: &LanClient, evt: Event) {
    let Event::Key(key) = evt else { return };
    if key.kind != KeyEventKind::Press { return; }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),

        KeyCode::Char(' ') => {
            if let Some(device) = app.selected_device().cloned() {
                let current_on = app.states.get(&device.mac).map(|s| s.on).unwrap_or(false);
                match lan.set_power(&device, !current_on).await {
                    Ok(_) => {
                        app.states.entry(device.mac.clone()).or_default().on = !current_on;
                        app.status = format!(
                            "{} turned {}",
                            device.display_name(),
                            if !current_on { "ON" } else { "OFF" }
                        );
                    }
                    Err(e) => app.status = format!("Error: {e}"),
                }
            }
        }

        KeyCode::Char('b') => {
            app.input_mode = InputMode::Prompt(PromptKind::Brightness);
            app.input_buf.clear();
            app.status = "Enter brightness (1\u{2013}100):".to_string();
        }
        KeyCode::Char('c') => {
            app.input_mode = InputMode::Prompt(PromptKind::Color);
            app.input_buf.clear();
            app.status = "Enter hex color (e.g. FF8000):".to_string();
        }
        KeyCode::Char('t') => {
            app.input_mode = InputMode::Prompt(PromptKind::ColorTemp);
            app.input_buf.clear();
            app.status = "Enter color temp in Kelvin (2000\u{2013}9000):".to_string();
        }

        KeyCode::Char('r') => {
            if let Some(device) = app.selected_device().cloned() {
                match lan.get_state(&device, Duration::from_secs(2)).await {
                    Ok(state) => {
                        app.status = format!(
                            "{}: {}  brightness {}%  color {}",
                            device.display_name(),
                            if state.on { "ON" } else { "OFF" },
                            state.brightness,
                            state.color,
                        );
                        app.states.insert(device.mac.clone(), state);
                    }
                    Err(e) => app.status = format!("State query failed: {e}"),
                }
            }
        }
        _ => {}
    }
}

async fn handle_prompt(app: &mut App, lan: &LanClient, kind: PromptKind, evt: Event) {
    let Event::Key(key) = evt else { return };
    if key.kind != KeyEventKind::Press { return; }

    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.status = "Cancelled.".to_string();
        }
        KeyCode::Backspace => { app.input_buf.pop(); }
        KeyCode::Char(c) => { app.input_buf.push(c); }
        KeyCode::Enter => {
            let input = app.input_buf.trim().to_string();
            app.input_mode = InputMode::Normal;

            if let Some(device) = app.selected_device().cloned() {
                let result: std::result::Result<(), String> = match kind {
                    PromptKind::Brightness => {
                        match input.parse::<u8>() {
                            Ok(v) => lan.set_brightness(&device, v).await.map_err(|e| e.to_string()),
                            Err(_) => Err("invalid number (1\u{2013}100)".to_string()),
                        }
                    }
                    PromptKind::Color => {
                        match parse_hex_color(&input) {
                            Some(c) => lan.set_color(&device, c).await.map_err(|e| e.to_string()),
                            None => Err("invalid hex color (e.g. FF8000)".to_string()),
                        }
                    }
                    PromptKind::ColorTemp => {
                        match input.parse::<u16>() {
                            Ok(k) => lan.set_color_temp(&device, k).await.map_err(|e| e.to_string()),
                            Err(_) => Err("invalid number (2000\u{2013}9000)".to_string()),
                        }
                    }
                };

                app.status = match result {
                    Ok(_) => format!("Command sent to {}", device.display_name()),
                    Err(e) => format!("Error: {e}"),
                };
            }
        }
        _ => {}
    }
}

pub fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 { return None; }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::new(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut app = App::new();
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
        let app = App::new();
        assert!(app.selected_device().is_none());
    }

    #[test]
    fn selected_device_returns_correct() {
        let mut app = App::new();
        app.devices = vec![
            Device::new("AA:01", "H6072"),
            Device::new("AA:02", "H6073"),
        ];
        app.selected = 1;
        assert_eq!(app.selected_device().unwrap().mac, "AA:02");
    }
}
