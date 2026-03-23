//! Govee lights controller — TUI frontend.
//!
//! Usage:
//!   GOVEE_API_KEY=<key> govee            # LAN discovery + optional HTTP fallback
//!   govee                                 # LAN only
//!
//! Controls:
//!   ↑/↓   Select device
//!   Space  Toggle power
//!   b      Set brightness (prompts)
//!   c      Set color (hex prompt)
//!   t      Set color temperature (prompt)
//!   r      Refresh device state
//!   q/Esc  Quit

mod app;
mod ui;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let mut terminal = setup_terminal()?;
    let result = app::run(&mut terminal).await;
    restore_terminal(&mut terminal)?;

    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
