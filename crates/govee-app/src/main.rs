//! Govee lights controller — egui/eframe GUI frontend.
//!
//! Architecture:
//! - The **main thread** runs the eframe event loop (egui renders here).
//! - A **background thread** owns a Tokio runtime and handles all async LAN I/O.
//! - The two threads communicate via a pair of `std::sync::mpsc` channels:
//!   - `cmd_tx` / `cmd_rx`  — GUI sends [`worker::Command`]s to the worker.
//!   - `evt_tx` / `evt_rx`  — Worker sends [`worker::WorkerEvent`]s to the GUI.

mod app;
mod config;
mod scenes;
mod ui;
mod worker;

use std::sync::mpsc;

use app::GoveeApp;

// ── App icon ──────────────────────────────────────────────────────────────────

/// Generate a 64×64 light-bulb icon as raw RGBA bytes.
///
/// Shape:
///  • Golden circular bulb (top half of the icon), brighter at the centre.
///  • Narrow amber neck connecting bulb to base.
///  • Silver screw-base with three horizontal ridges.
///  • Transparent background.
fn light_bulb_icon() -> egui::IconData {
    const S: u32 = 64;
    let mut rgba = vec![0u8; (S * S * 4) as usize];

    // Bulb parameters
    let cx = 32.0_f32;
    let cy = 26.0_f32;
    let r_bulb = 19.0_f32; // glass sphere radius
    let r_glow = 11.0_f32; // bright core radius

    for row in 0..S {
        for col in 0..S {
            let x = col as f32 + 0.5;
            let y = row as f32 + 0.5;
            let i = ((row * S + col) * 4) as usize;

            let dx = x - cx;
            let dy = y - cy;
            let d = (dx * dx + dy * dy).sqrt();

            if d <= r_bulb {
                // Interior: warm yellow fading to golden orange at the edge.
                let t = (1.0 - (d / r_bulb)).powf(0.6);
                let core = (d <= r_glow) as u8;
                let r = 255u8;
                let g = (180.0 + 75.0 * t + 20.0 * core as f32).min(255.0) as u8;
                let b = (20.0 * t) as u8;
                rgba[i]     = r;
                rgba[i + 1] = g;
                rgba[i + 2] = b;
                rgba[i + 3] = 255;
            } else if d <= r_bulb + 1.5 {
                // Thin amber outline around the glass.
                let alpha = (1.0 - (d - r_bulb) / 1.5) * 220.0;
                rgba[i]     = 200;
                rgba[i + 1] = 130;
                rgba[i + 2] = 0;
                rgba[i + 3] = alpha as u8;
            } else if x >= 27.0 && x <= 37.0 && y >= 44.0 && y <= 48.0 {
                // Neck
                rgba[i]     = 190;
                rgba[i + 1] = 150;
                rgba[i + 2] = 40;
                rgba[i + 3] = 255;
            } else if x >= 24.0 && x <= 40.0 && y >= 48.0 && y <= 61.0 {
                // Screw base — three ridges that get progressively darker.
                let ridge = ((y as u32 - 48) / 4) as u8;
                let v = 160u8.saturating_sub(ridge * 20);
                rgba[i]     = v;
                rgba[i + 1] = v;
                rgba[i + 2] = v;
                rgba[i + 3] = 255;
            }
            // else: transparent background (already 0)
        }
    }

    egui::IconData { rgba, width: S, height: S }
}

fn main() -> eframe::Result {
    // Structured logging (respects RUST_LOG env var).
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Govee Lights")
            .with_icon(light_bulb_icon())
            .with_inner_size([800.0, 480.0])
            .with_min_inner_size([800.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Govee Lights",
        options,
        Box::new(|cc| {
            // ── Touch-friendly style for 800×480 ─────────────────────────────
            let mut style = (*cc.egui_ctx.style()).clone();

            // Minimum interactive height — comfortable finger tap target.
            style.spacing.interact_size = egui::vec2(44.0, 40.0);
            // More padding inside buttons so they feel substantial.
            style.spacing.button_padding = egui::vec2(14.0, 8.0);
            // A bit more breathing room between widgets.
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            // Fatter slider rail — easier to grab with a finger.
            style.spacing.slider_rail_height = 8.0;
            // Wide slider track — fills the panel instead of the 100px default.
            style.spacing.slider_width = 500.0;
            // Slightly larger text throughout.
            {
                use egui::{FontFamily::Proportional, FontId, TextStyle::*};
                style.text_styles = [
                    (Small,    FontId::new(12.0, Proportional)),
                    (Body,     FontId::new(15.0, Proportional)),
                    (Monospace, FontId::new(14.0, egui::FontFamily::Monospace)),
                    (Button,   FontId::new(15.0, Proportional)),
                    (Heading,  FontId::new(20.0, Proportional)),
                ]
                .into();
            }
            cc.egui_ctx.set_style(style);

            let (cmd_tx, cmd_rx) = mpsc::channel();
            let (evt_tx, evt_rx) = mpsc::channel();

            // Spawn the async LAN worker on a background thread.
            // We pass the egui Context so it can call request_repaint() when
            // new events arrive, waking the GUI even while the user is idle.
            worker::spawn(cmd_rx, evt_tx, cc.egui_ctx.clone());

            Ok(Box::new(GoveeApp::new(cmd_tx, evt_rx)))
        }),
    )
}
