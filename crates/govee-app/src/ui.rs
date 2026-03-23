//! TUI rendering with ratatui.

use govee_core::models::DeviceState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color as TColor, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::app::{App, InputMode};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // title
            Constraint::Min(0),     // device list
            Constraint::Length(3),  // status / prompt bar
        ])
        .split(f.area());

    render_title(f, chunks[0]);
    render_devices(f, app, chunks[1]);
    render_status(f, app, chunks[2]);
}

fn render_title(f: &mut Frame, area: ratatui::layout::Rect) {
    let title = Paragraph::new("Govee Light Controller")
        .style(Style::default().fg(TColor::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn render_devices(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .title(" Devices  [\u{2191}/\u{2193}] select  [Space] toggle  [b] brightness  [c] color  [t] temp  [r] refresh  [q] quit ")
        .borders(Borders::ALL);

    if app.devices.is_empty() {
        let msg = Paragraph::new("No devices found.\n\nEnable LAN Control in the Govee Home app\nfor each device under Settings \u{2192} LAN Control.")
            .alignment(Alignment::Center)
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .devices
        .iter()
        .enumerate()
        .map(|(i, device)| {
            let state = app.states.get(&device.mac);
            let is_selected = i == app.selected;

            let power_indicator = match state.map(|s| s.on) {
                Some(true) => Span::styled("\u{25cf} ON ", Style::default().fg(TColor::Green)),
                Some(false) => Span::styled("\u{25cb} OFF", Style::default().fg(TColor::DarkGray)),
                None => Span::styled("? ---", Style::default().fg(TColor::Yellow)),
            };

            let name = Span::styled(
                format!("  {:<24}", device.display_name()),
                if is_selected {
                    Style::default().fg(TColor::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TColor::White)
                },
            );

            let details = state.map(|s| render_state_inline(s)).unwrap_or_default();

            let line = Line::from(vec![
                power_indicator,
                name,
                Span::raw(details),
            ]);

            let style = if is_selected {
                Style::default().bg(TColor::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(TColor::DarkGray));

    f.render_stateful_widget(list, area, &mut list_state);
}

fn render_state_inline(state: &DeviceState) -> String {
    if state.color_temp_kelvin > 0 {
        format!("  {}%  {}K", state.brightness, state.color_temp_kelvin)
    } else {
        format!(
            "  {}%  rgb({},{},{})",
            state.brightness, state.color.r, state.color.g, state.color.b
        )
    }
}

fn render_status(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let (text, style) = match &app.input_mode {
        InputMode::Normal => (
            app.status.clone(),
            Style::default().fg(TColor::Gray),
        ),
        InputMode::Prompt(_) => (
            format!("{} {}_", app.status, app.input_buf),
            Style::default().fg(TColor::Yellow),
        ),
    };

    let status = Paragraph::new(text)
        .style(style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, area);
}
