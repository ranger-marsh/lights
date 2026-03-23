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

fn main() -> eframe::Result {
    // Structured logging (respects RUST_LOG env var).
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Govee Lights")
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
