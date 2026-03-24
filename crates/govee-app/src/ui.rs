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
use crate::scenes;
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
        .exact_width(220.0)
        .show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("Devices");
            ui.add_space(3.0);

            // Connected count: named devices that are discovered and not offline.
            let total_known = app.names.len();
            if total_known > 0 {
                let connected = app.devices.iter()
                    .filter(|d| app.names.contains_key(&d.mac)
                        && !app.offline_macs.contains(&d.mac))
                    .count();
                let color = if connected == total_known {
                    Color32::from_rgb(80, 200, 80)
                } else {
                    Color32::from_rgb(220, 170, 50)
                };
                ui.label(
                    RichText::new(format!("{connected} / {total_known} connected"))
                        .small()
                        .color(color),
                );
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(6.0);

            // Lay out the buttons first using a bottom_up region so they
            // "claim" space from the bottom before the scroll area runs.
            // Everything inside the bottom_up closure is positioned from
            // the bottom of the panel upward.
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(6.0);
                if ui
                    .add_sized(
                        [ui.available_width(), 40.0],
                        egui::Button::new("\u{1f50d}  Scan again"),
                    )
                    .clicked()
                {
                    app.send(Command::Rediscover);
                }
                ui.add_space(4.0);
                if ui
                    .add_sized(
                        [ui.available_width(), 40.0],
                        egui::Button::new("\u{21ba}  Refresh All"),
                    )
                    .clicked()
                {
                    app.send(Command::RefreshAll);
                }
                ui.add_space(4.0);
                ui.separator();

                // Switch back to top_down for the device list, which now
                // fills exactly the space between the header and the buttons.
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    if app.devices.is_empty() {
                        ui.label(
                            RichText::new(
                                "No devices found.\n\nEnable LAN Control\nin the Govee app\nunder Settings \u{2192} LAN Control.",
                            )
                            .small()
                            .color(Color32::GRAY),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                let count = app.devices.len();
                                for i in 0..count {
                                    let device = &app.devices[i];
                                    let state = app.states.get(&device.mac);
                                    let is_on = state.map(|s| s.on);
                                    let is_selected = i == app.selected;

                                    let is_offline = app.offline_macs.contains(&device.mac);
                                    let dot = if is_offline {
                                        RichText::new("\u{2715}").color(Color32::from_rgb(220, 60, 60))
                                    } else {
                                        match is_on {
                                            Some(true) => RichText::new("\u{25cf}").color(Color32::from_rgb(80, 220, 80)),
                                            Some(false) => RichText::new("\u{25cb}").color(Color32::DARK_GRAY),
                                            None => RichText::new("\u{25cc}").color(Color32::GOLD),
                                        }
                                    };

                                    ui.horizontal(|ui| {
                                        ui.set_min_height(40.0);
                                        ui.label(dot);
                                        let name = device.display_name().to_string();
                                        if ui
                                            .add_sized(
                                                egui::vec2(ui.available_width(), 40.0),
                                                egui::SelectableLabel::new(is_selected, name),
                                            )
                                            .clicked()
                                        {
                                            app.selected = i;
                                        }
                                    });
                                }
                            });
                    }
                });
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

        egui::ScrollArea::vertical()
            .id_salt("central_scroll")
            .show(ui, |ui| {
                ui.add_space(6.0);
                match app.tab {
                    Tab::All => draw_all_lights(ui, app),
                    Tab::Individual => draw_individual(ui, app),
                    Tab::Group(i) => draw_group(ui, app, i),
                }
                ui.add_space(8.0);
            });
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
                ("\u{25cf}  ON  \u{2014} tap to turn off", Color32::from_rgb(80, 220, 80))
            } else {
                ("\u{25cb}  OFF  \u{2014} tap to turn on", Color32::GRAY)
            };
            if ui
                .add_sized(
                    egui::vec2(ui.available_width(), 44.0),
                    egui::Button::new(RichText::new(label).color(color).size(15.0)),
                )
                .clicked()
            {
                app.send(Command::SetPower(device.clone(), !is_on));
            }
        });

        ui.add_space(10.0);

        // ── Brightness ────────────────────────────────────────────────────
        section(ui, "Brightness", |ui| {
            ui.add_sized(
                [ui.available_width(), 40.0],
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
                ui.add_sized(
                    [ui.available_width(), 40.0],
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
        ui.columns(2, |cols| {
            if cols[0]
                .add_sized(
                    egui::vec2(cols[0].available_width(), 44.0),
                    egui::Button::new(
                        RichText::new("\u{25cf}  Turn All ON")
                            .color(Color32::from_rgb(80, 220, 80))
                            .size(15.0),
                    ),
                )
                .clicked()
            {
                app.broadcast(BroadcastAction::Power(true));
            }
            if cols[1]
                .add_sized(
                    egui::vec2(cols[1].available_width(), 44.0),
                    egui::Button::new(
                        RichText::new("\u{25cb}  Turn All OFF")
                            .color(Color32::GRAY)
                            .size(15.0),
                    ),
                )
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
            ui.add_sized(
                [ui.available_width(), 40.0],
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

    ui.add_space(10.0);

    // ── Scenes ────────────────────────────────────────────────────────────────
    let all_devices = app.devices.clone();
    draw_scene_picker(ui, app, all_devices);

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

    // ── Member checkboxes (collapsible) ───────────────────────────────────────
    let total = app.devices.len();
    let in_group = app.groups[idx].macs.len();
    let header = if total == 0 {
        "Members".to_string()
    } else {
        format!("Members  ({in_group} / {total})")
    };

    egui::CollapsingHeader::new(header)
        .id_salt(format!("members_{idx}"))
        .default_open(false)
        .show(ui, |ui| {
            ui.add_space(4.0);
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
                        .add_sized(
                            egui::vec2(ui.available_width(), 36.0),
                            egui::Checkbox::new(&mut checked, device.display_name()),
                        )
                        .changed()
                    {
                        to_toggle = Some(device.mac.clone());
                    }
                }
                if let Some(mac) = to_toggle {
                    app.toggle_device_in_group(idx, &mac);
                }
            }
            ui.add_space(4.0);
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
            ui.columns(2, |cols| {
                if cols[0]
                    .add_sized(
                        egui::vec2(cols[0].available_width(), 44.0),
                        egui::Button::new(
                            RichText::new("\u{25cf}  Turn ON")
                                .color(Color32::from_rgb(80, 220, 80))
                                .size(15.0),
                        ),
                    )
                    .clicked()
                {
                    app.broadcast_group(idx, BroadcastAction::Power(true));
                }
                if cols[1]
                    .add_sized(
                        egui::vec2(cols[1].available_width(), 44.0),
                        egui::Button::new(
                            RichText::new("\u{25cb}  Turn OFF")
                                .color(Color32::GRAY)
                                .size(15.0),
                        ),
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
                ui.add_sized(
                    [ui.available_width(), 40.0],
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

        ui.add_space(10.0);

        // Scenes
        let group_devices = app.group_devices(idx);
        draw_scene_picker(ui, app, group_devices);
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

// ── Scene picker ──────────────────────────────────────────────────────────────

/// Render the scene picker below the colour controls.
///
/// `salt` must be unique per call site to give egui distinct scroll-area IDs.
fn draw_scene_picker(
    ui: &mut egui::Ui,
    app: &mut GoveeApp,
    devices: Vec<govee_core::models::Device>,
) {
    section(ui, "Scenes", |ui| {
        // ── Category selector row ────────────────────────────────────────────
        ui.horizontal(|ui| {
            for (i, cat) in scenes::CATEGORIES.iter().enumerate() {
                ui.selectable_value(&mut app.selected_scene_cat, i, cat.name);
            }
        });

        ui.add_space(6.0);

        let cat = &scenes::CATEGORIES[app.selected_scene_cat];

        // ── Scene button grid ─────────────────────────────────────────────────
        // Compute a uniform button width once — before any column layout —
        // so every button in the row gets exactly the same size.
        const COLS: usize = 3;
        const GAP: f32 = 8.0;
        const BTN_H: f32 = 48.0;
        let btn_w = (ui.available_width() - (COLS - 1) as f32 * GAP) / COLS as f32;

        let mut scene_to_apply: Option<usize> = None;

        for (chunk_idx, chunk) in cat.scenes.chunks(COLS).enumerate() {
            if chunk_idx > 0 {
                ui.add_space(GAP);
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                for (col, scene) in chunk.iter().enumerate() {
                    let scene_idx = chunk_idx * COLS + col;
                    let resp = ui.add_sized(
                        [btn_w, BTN_H],
                        egui::Button::new(scene.name),
                    );

                    // Colour-palette swatches painted along the bottom of the button
                    let rect = resp.rect;
                    let n = scene.palette.len().min(6);
                    let sw = 10.0_f32;
                    let gap = 3.0_f32;
                    let total = n as f32 * sw + (n.saturating_sub(1)) as f32 * gap;
                    let mut sx = rect.center().x - total / 2.0;
                    let sy = rect.bottom() - sw - 4.0;
                    for j in 0..n {
                        let (r, g, b) = scene.palette[j];
                        let swatch = egui::Rect::from_min_size(
                            egui::pos2(sx, sy),
                            egui::vec2(sw, sw),
                        );
                        ui.painter().rect_filled(swatch, 2.0, Color32::from_rgb(r, g, b));
                        sx += sw + gap;
                    }

                    if resp.clicked() {
                        scene_to_apply = Some(scene_idx);
                    }
                }
            });
        }

        if let Some(i) = scene_to_apply {
            app.apply_scene_to_devices(devices.clone(), &cat.scenes[i]);
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
