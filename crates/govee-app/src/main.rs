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
            .with_inner_size([680.0, 460.0])
            .with_min_inner_size([480.0, 340.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Govee Lights",
        options,
        Box::new(|cc| {
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
