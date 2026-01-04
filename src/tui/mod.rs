//! Terminal UI for Rembrandt
//!
//! Provides the dashboard interface for agent orchestration.
//! - Dashboard: see all agents, spawn, kill, nudge
//! - Attach: (WIP) direct PTY control of an agent

mod app;
mod attach;  // WIP - needs PTY refactor
mod events;
mod render;

pub use app::App;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::path::PathBuf;

/// View mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Symphony view - see all agents at once (zoom out)
    Symphony,
    /// Solo view - interact with a single agent (zoom in)
    Solo(usize),
}

/// Run the TUI application
pub fn run(repo_path: PathBuf) -> crate::Result<()> {
    // Check if we have a proper TTY
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        return Err(crate::RembrandtError::Io(std::io::Error::new(
            std::io::ErrorKind::NotConnected,
            "Dashboard requires an interactive terminal. Run from a TTY, not a pipe or script.",
        )));
    }

    // Create app state first (before messing with terminal)
    let mut app = App::new(repo_path)?;

    // Setup terminal
    enable_raw_mode().map_err(|e| {
        crate::RembrandtError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to enable raw mode: {}", e),
        ))
    })?;

    let mut stdout = stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        disable_raw_mode().ok();
        return Err(crate::RembrandtError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to enter alternate screen: {}", e),
        )));
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            disable_raw_mode().ok();
            return Err(e.into());
        }
    };

    // Main loop
    let result = run_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}

/// The main event loop
fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> crate::Result<()> {
    loop {
        // Render
        terminal.draw(|frame| render::render(frame, app))?;

        // Handle events
        if !events::handle_events(app)? {
            break;
        }
    }

    Ok(())
}
