//! egui rendering — called once per frame from [`GoveeApp::update`].
//!
//! Layout:
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │  [left panel]  Device list + Scan button               │
//! │  [central]     Controls + rename for selected device   │
//! │  [bottom bar]  Status message + config file path       │
//! └────────────────────────────────────────────────────────┘
//! ```

use egui::{Color32, RichText};
use govee_core::models::Color;

use crate::app::{GoveeApp, Tab};
use crate::config;
use crate::worker::{BroadcastAction, Command};

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
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            // Status message
            let text = RichText::new(&app.status).small();
            let colored = if app.status_is_error {
                text.color(Color32::from_rgb(220, 80, 80))
            } else {
                text.color(Color32::GRAY)
            };
            ui.label(colored);

            // Config file path hint (right-aligned)
            if let Some(path) = config::config_path() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(path.display().to_string())
                            .small()
                            .color(Color32::from_gray(70)),
                    );
                    ui.label(RichText::new("names file:").small().color(Color32::from_gray(55)));
                });
            }
        });
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

// ── Central panel: tab bar + routed content ───────────────────────────────────

fn draw_controls(ctx: &egui::Context, app: &mut GoveeApp) {
    egui::CentralPanel::default().show(ctx, |ui| {
        // Tab bar — horizontal scroll in case many groups are present
        egui::ScrollArea::horizontal()
            .id_salt("tab_bar")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut app.tab, Tab::All, "All Lights");
                    ui.selectable_value(&mut app.tab, Tab::Individual, "Individual");

                    let group_count = app.groups.len();
                    for i in 0..group_count {
                        let name = app.groups[i].name.clone();
                        ui.selectable_value(&mut app.tab, Tab::Group(i), name);
                    }

                    if group_count < crate::app::MAX_GROUPS {
                        if ui
                            .small_button("+")
                            .on_hover_text("New group")
                            .clicked()
                        {
                            app.add_group();
                        }
                    }
                });
            });
        ui.separator();
        ui.add_space(6.0);

        match app.tab {
            Tab::All => draw_all_lights(ui, app),
            Tab::Individual => draw_individual(ui, app),
            Tab::Group(i) => draw_group(ui, app, i),
        }
    });
}

// ── Individual device controls ────────────────────────────────────────────────

fn draw_individual(ui: &mut egui::Ui, app: &mut GoveeApp) {
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

        // ── Device heading + rename ───────────────────────────────────────
        draw_rename_section(ui, app, &device.mac);

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
}

// ── All Lights tab ────────────────────────────────────────────────────────────

fn draw_all_lights(ui: &mut egui::Ui, app: &mut GoveeApp) {
    if app.devices.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("No devices found. Try scanning again.")
                    .color(Color32::GRAY)
                    .size(16.0),
            );
        });
        return;
    }

    // ── Power ─────────────────────────────────────────────────────────────────
    section(ui, "Power — All Lights", |ui| {
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("\u{25cf}  Turn All ON").color(Color32::from_rgb(80, 220, 80)).size(15.0))
                .clicked()
            {
                app.broadcast(BroadcastAction::Power(true));
            }
            ui.add_space(8.0);
            if ui
                .button(RichText::new("\u{25cb}  Turn All OFF").color(Color32::GRAY).size(15.0))
                .clicked()
            {
                app.broadcast(BroadcastAction::Power(false));
            }
        });
    });

    ui.add_space(10.0);

    // ── Brightness ────────────────────────────────────────────────────────────
    section(ui, "Brightness — All Lights", |ui| {
        ui.add(
            egui::Slider::new(&mut app.pending_brightness, 1_u8..=100).suffix("%"),
        );
        if ui.button("Apply to All").clicked() {
            let pct = app.pending_brightness;
            app.broadcast(BroadcastAction::Brightness(pct));
        }
    });

    ui.add_space(10.0);

    // ── Color ─────────────────────────────────────────────────────────────────
    section(ui, "Color — All Lights", |ui| {
        ui.horizontal(|ui| {
            ui.radio_value(&mut app.use_color_temp, false, "RGB");
            ui.radio_value(&mut app.use_color_temp, true, "White / Color Temp");
        });
        ui.add_space(6.0);

        if app.use_color_temp {
            ui.add(
                egui::Slider::new(&mut app.pending_color_temp, 2_000_u16..=9_000).suffix(" K"),
            );
            ui.horizontal(|ui| {
                ui.label(RichText::new("Warm").color(Color32::from_rgb(255, 180, 80)).small());
                ui.label(RichText::new("\u{2194}").small().color(Color32::GRAY));
                ui.label(RichText::new("Cool").color(Color32::from_rgb(160, 200, 255)).small());
            });
            if ui.button("Apply to All").clicked() {
                let k = app.pending_color_temp;
                app.broadcast(BroadcastAction::ColorTemp(k));
            }
        } else {
            egui::color_picker::color_edit_button_rgb(ui, &mut app.pending_color);
            if ui.button("Apply to All").clicked() {
                let [r, g, b] = app.pending_color;
                let color = Color::new(
                    (r * 255.0).round() as u8,
                    (g * 255.0).round() as u8,
                    (b * 255.0).round() as u8,
                );
                app.broadcast(BroadcastAction::Color(color));
            }
        }
    });

    ui.add_space(12.0);

    // ── Device summary grid ───────────────────────────────────────────────────
    ui.separator();
    ui.add_space(6.0);
    ui.label(RichText::new("Device summary").small().color(Color32::GRAY));
    ui.add_space(4.0);

    let devices: Vec<_> = app.devices.clone();
    egui::Grid::new("all_lights_grid")
        .num_columns(4)
        .striped(true)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            // Header row
            ui.label(RichText::new("Device").small().strong());
            ui.label(RichText::new("Power").small().strong());
            ui.label(RichText::new("Brightness").small().strong());
            ui.label(RichText::new("Color").small().strong());
            ui.end_row();

            for device in &devices {
                let state = app.states.get(&device.mac);

                ui.label(RichText::new(device.display_name()).small());

                match state.map(|s| s.on) {
                    Some(true) => ui.label(RichText::new("ON").small().color(Color32::from_rgb(80, 220, 80))),
                    Some(false) => ui.label(RichText::new("OFF").small().color(Color32::DARK_GRAY)),
                    None => ui.label(RichText::new("?").small().color(Color32::GOLD)),
                };

                match state.map(|s| s.brightness) {
                    Some(b) => ui.label(RichText::new(format!("{b}%")).small()),
                    None => ui.label(RichText::new("?").small().color(Color32::GOLD)),
                };

                if let Some(s) = state {
                    if s.color_temp_kelvin > 0 {
                        ui.label(RichText::new(format!("{}K", s.color_temp_kelvin)).small());
                    } else {
                        let swatch = Color32::from_rgb(s.color.r, s.color.g, s.color.b);
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, swatch);
                    }
                } else {
                    ui.label(RichText::new("?").small().color(Color32::GOLD));
                }

                ui.end_row();
            }
        });
}

// ── Group tab ─────────────────────────────────────────────────────────────────

fn draw_group(ui: &mut egui::Ui, app: &mut GoveeApp, idx: usize) {
    if app.groups.get(idx).is_none() {
        // Guard: tab index stale after a delete races with a frame.
        app.tab = crate::app::Tab::All;
        return;
    }

    ui.add_space(8.0);

    // ── Group heading + rename ────────────────────────────────────────────────
    if app.renaming_group == Some(idx) {
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut app.group_rename_buf)
                    .hint_text("Group name\u{2026}")
                    .desired_width(180.0),
            );
            response.request_focus();

            let save = ui.button("Save").clicked()
                || (response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if save {
                app.commit_rename_group();
            } else if ui.button("Cancel").clicked()
                || ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
                app.cancel_rename_group();
            }
        });
    } else {
        let name = app.groups[idx].name.clone();
        ui.horizontal(|ui| {
            ui.heading(&name);
            if ui
                .small_button("\u{270f}")
                .on_hover_text("Rename this group")
                .clicked()
            {
                app.start_rename_group(idx);
            }
        });
    }

    ui.add_space(10.0);

    // ── Member checkboxes ─────────────────────────────────────────────────────
    section(ui, "Members", |ui| {
        let devices: Vec<_> = app.devices.clone();
        if devices.is_empty() {
            ui.label(
                RichText::new("No devices discovered yet.")
                    .small()
                    .color(Color32::GRAY),
            );
        } else {
            let macs = app.groups[idx].macs.clone();
            let mut to_toggle: Option<String> = None;
            for device in &devices {
                let mut checked = macs.contains(&device.mac);
                if ui
                    .checkbox(&mut checked, device.display_name())
                    .changed()
                {
                    to_toggle = Some(device.mac.clone());
                }
            }
            if let Some(mac) = to_toggle {
                app.toggle_device_in_group(idx, &mac);
            }
        }
    });

    ui.add_space(10.0);

    // ── Controls (only shown when there are members) ──────────────────────────
    let member_count = app.group_devices(idx).len();
    if member_count == 0 {
        ui.label(
            RichText::new("Add devices above to control this group.")
                .color(Color32::GRAY),
        );
    } else {
        // Power
        section(ui, "Power", |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(
                        RichText::new("\u{25cf}  Turn ON")
                            .color(Color32::from_rgb(80, 220, 80))
                            .size(15.0),
                    )
                    .clicked()
                {
                    app.broadcast_group(idx, BroadcastAction::Power(true));
                }
                ui.add_space(8.0);
                if ui
                    .button(
                        RichText::new("\u{25cb}  Turn OFF")
                            .color(Color32::GRAY)
                            .size(15.0),
                    )
                    .clicked()
                {
                    app.broadcast_group(idx, BroadcastAction::Power(false));
                }
            });
        });

        ui.add_space(10.0);

        // Brightness
        section(ui, "Brightness", |ui| {
            ui.add(
                egui::Slider::new(&mut app.pending_brightness, 1_u8..=100).suffix("%"),
            );
            if ui.button("Apply to Group").clicked() {
                let pct = app.pending_brightness;
                app.broadcast_group(idx, BroadcastAction::Brightness(pct));
            }
        });

        ui.add_space(10.0);

        // Color
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
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Warm")
                            .color(Color32::from_rgb(255, 180, 80))
                            .small(),
                    );
                    ui.label(RichText::new("\u{2194}").small().color(Color32::GRAY));
                    ui.label(
                        RichText::new("Cool")
                            .color(Color32::from_rgb(160, 200, 255))
                            .small(),
                    );
                });
                if ui.button("Apply to Group").clicked() {
                    let k = app.pending_color_temp;
                    app.broadcast_group(idx, BroadcastAction::ColorTemp(k));
                }
            } else {
                egui::color_picker::color_edit_button_rgb(ui, &mut app.pending_color);
                if ui.button("Apply to Group").clicked() {
                    let [r, g, b] = app.pending_color;
                    let color = Color::new(
                        (r * 255.0).round() as u8,
                        (g * 255.0).round() as u8,
                        (b * 255.0).round() as u8,
                    );
                    app.broadcast_group(idx, BroadcastAction::Color(color));
                }
            }
        });
    }

    // ── Delete group ─────────────────────────────────────────────────────────
    ui.add_space(16.0);
    ui.separator();
    ui.add_space(6.0);
    if ui
        .button(RichText::new("Delete Group").color(Color32::from_rgb(200, 80, 80)))
        .clicked()
    {
        app.delete_group(idx);
    }
}

// ── Rename section ────────────────────────────────────────────────────────────

fn draw_rename_section(ui: &mut egui::Ui, app: &mut GoveeApp, mac: &str) {
    let is_renaming = app.renaming.as_deref() == Some(mac);

    if is_renaming {
        // Text field + Save / Cancel
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut app.rename_buf)
                    .hint_text("Device name\u{2026}")
                    .desired_width(180.0),
            );
            // Auto-focus the field when it first appears.
            response.request_focus();

            let save = ui.button("Save").clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));

            if save {
                app.commit_rename();
            } else if ui.button("Cancel").clicked()
                || ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
                app.cancel_rename();
            }
        });
    } else {
        // Heading + pencil button
        let name = app
            .devices
            .iter()
            .find(|d| d.mac == mac)
            .map(|d| d.display_name().to_string())
            .unwrap_or_default();

        ui.horizontal(|ui| {
            ui.heading(&name);
            if ui
                .small_button("\u{270f}")
                .on_hover_text("Rename this device")
                .clicked()
            {
                app.start_rename(mac);
            }
        });
    }
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
