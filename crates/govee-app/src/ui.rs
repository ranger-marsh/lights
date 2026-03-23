//! egui rendering — called once per frame from [`GoveeApp::update`].
//!
//! Layout:
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │  [left panel]  Device list + Scan button               │
//! │  [central]     Controls for the selected device        │
//! │  [bottom bar]  Status message                          │
//! └────────────────────────────────────────────────────────┘
//! ```

use egui::{Color32, RichText};
use govee_core::models::Color;

use crate::app::GoveeApp;
use crate::worker::Command;

// ── Top-level draw ────────────────────────────────────────────────────────────

/// Render the full UI for one frame.
pub fn draw(ctx: &egui::Context, app: &mut GoveeApp) {
    draw_status_bar(ctx, app);
    draw_device_panel(ctx, app);
    draw_controls(ctx, app);
}

// ── Status bar (bottom) ───────────────────────────────────────────────────────

fn draw_status_bar(ctx: &egui::Context, app: &GoveeApp) {
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        ui.add_space(4.0);
        let text = RichText::new(&app.status).small();
        let colored = if app.status_is_error {
            text.color(Color32::from_rgb(220, 80, 80))
        } else {
            text.color(Color32::GRAY)
        };
        ui.label(colored);
        ui.add_space(2.0);
    });
}

// ── Device list (left panel) ──────────────────────────────────────────────────

fn draw_device_panel(ctx: &egui::Context, app: &mut GoveeApp) {
    egui::SidePanel::left("device_panel")
        .resizable(false)
        .min_width(190.0)
        .show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("Devices");
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            if app.devices.is_empty() {
                ui.label(
                    RichText::new(
                        "No devices found.\n\nEnable LAN Control\nin the Govee app\nunder Settings \u{2192} LAN Control.",
                    )
                    .small()
                    .color(Color32::GRAY),
                );
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let count = app.devices.len();
                    for i in 0..count {
                        let device = &app.devices[i];
                        let state = app.states.get(&device.mac);
                        let is_on = state.map(|s| s.on);
                        let is_selected = i == app.selected;

                        // Power indicator dot
                        let dot = match is_on {
                            Some(true) => RichText::new("\u{25cf}").color(Color32::from_rgb(80, 220, 80)),
                            Some(false) => RichText::new("\u{25cb}").color(Color32::DARK_GRAY),
                            None => RichText::new("\u{25cc}").color(Color32::GOLD),
                        };

                        ui.horizontal(|ui| {
                            ui.label(dot);
                            let name = device.display_name().to_string();
                            if ui.selectable_label(is_selected, &name).clicked() {
                                app.selected = i;
                            }
                        });
                    }
                });
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(6.0);
                if ui.button("\u{1f50d}  Scan again").clicked() {
                    app.send(Command::Rediscover);
                }
                ui.add_space(4.0);
                ui.separator();
            });
        });
}

// ── Controls (central panel) ──────────────────────────────────────────────────

fn draw_controls(ctx: &egui::Context, app: &mut GoveeApp) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let Some(device) = app.selected_device().cloned() else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("Select a device on the left.")
                        .color(Color32::GRAY)
                        .size(16.0),
                );
            });
            return;
        };

        ui.add_space(8.0);
        ui.heading(device.display_name());
        ui.add_space(2.0);
        ui.label(RichText::new(&device.mac).small().color(Color32::DARK_GRAY));
        ui.add_space(12.0);

        let state = app.states.get(&device.mac).cloned();

        // ── Power ─────────────────────────────────────────────────────────
        section(ui, "Power", |ui| {
            let is_on = state.as_ref().map(|s| s.on).unwrap_or(false);
            let (label, color) = if is_on {
                ("\u{25cf}  ON  (tap to turn off)", Color32::from_rgb(80, 220, 80))
            } else {
                ("\u{25cb}  OFF  (tap to turn on)", Color32::GRAY)
            };
            if ui
                .button(RichText::new(label).color(color).size(15.0))
                .clicked()
            {
                app.send(Command::SetPower(device.clone(), !is_on));
            }
        });

        ui.add_space(10.0);

        // ── Brightness ────────────────────────────────────────────────────
        section(ui, "Brightness", |ui| {
            ui.add(
                egui::Slider::new(&mut app.pending_brightness, 1_u8..=100)
                    .suffix("%"),
            );
            if ui.button("Apply").clicked() {
                let pct = app.pending_brightness;
                app.send(Command::SetBrightness(device.clone(), pct));
            }
        });

        ui.add_space(10.0);

        // ── Color ─────────────────────────────────────────────────────────
        section(ui, "Color", |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut app.use_color_temp, false, "RGB");
                ui.radio_value(&mut app.use_color_temp, true, "White / Color Temp");
            });
            ui.add_space(6.0);

            if app.use_color_temp {
                ui.add(
                    egui::Slider::new(&mut app.pending_color_temp, 2_000_u16..=9_000)
                        .suffix(" K"),
                );
                // Visual warm→cool gradient label
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Warm").color(Color32::from_rgb(255, 180, 80)).small());
                    ui.label(RichText::new("\u{2194}").small().color(Color32::GRAY));
                    ui.label(RichText::new("Cool").color(Color32::from_rgb(160, 200, 255)).small());
                });
                if ui.button("Apply").clicked() {
                    let k = app.pending_color_temp;
                    app.send(Command::SetColorTemp(device.clone(), k));
                }
            } else {
                egui::color_picker::color_edit_button_rgb(ui, &mut app.pending_color);
                if ui.button("Apply").clicked() {
                    let [r, g, b] = app.pending_color;
                    let color = Color::new(
                        (r * 255.0).round() as u8,
                        (g * 255.0).round() as u8,
                        (b * 255.0).round() as u8,
                    );
                    app.send(Command::SetColor(device.clone(), color));
                }
            }
        });

        ui.add_space(12.0);

        // ── Refresh ───────────────────────────────────────────────────────
        if ui.button("\u{21ba}  Refresh state").clicked() {
            app.send(Command::RefreshState(device.clone()));
        }

        // ── Current state readout ─────────────────────────────────────────
        if let Some(s) = &state {
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(RichText::new("Last known state").small().color(Color32::GRAY));
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let power_text = if s.on {
                    RichText::new("ON").color(Color32::from_rgb(80, 220, 80))
                } else {
                    RichText::new("OFF").color(Color32::DARK_GRAY)
                };
                ui.label(power_text);
                ui.label(
                    RichText::new(format!("  {}%", s.brightness)).color(Color32::LIGHT_GRAY),
                );
                if s.color_temp_kelvin > 0 {
                    ui.label(
                        RichText::new(format!("  {}K", s.color_temp_kelvin))
                            .color(Color32::LIGHT_GRAY),
                    );
                } else {
                    let swatch = Color32::from_rgb(s.color.r, s.color.g, s.color.b);
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(16.0, 16.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 3.0, swatch);
                    ui.label(
                        RichText::new(format!(
                            "  rgb({},{},{})",
                            s.color.r, s.color.g, s.color.b
                        ))
                        .small()
                        .color(Color32::GRAY),
                    );
                }
            });
        }
    });
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Render a labelled, framed section of controls.
fn section(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new(title).strong());
        ui.add_space(4.0);
        content(ui);
    });
}
