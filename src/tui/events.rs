//! Event handling for the TUI

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use super::{App, ViewMode};

/// Handle keyboard events
/// Returns true if the app should continue running
pub fn handle_events(app: &mut App) -> crate::Result<bool> {
    // Poll for events with a timeout (allows periodic status updates)
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            // Priority order: help overlay > spawn picker > confirmation > normal
            if app.show_help {
                handle_help_key(app, key)?;
            } else if app.spawn_picker.is_some() {
                handle_spawn_picker_key(app, key)?;
            } else if app.has_pending_confirm() {
                handle_confirm_key(app, key)?;
            } else {
                match app.view_mode {
                    ViewMode::Symphony => handle_symphony_key(app, key)?,
                    ViewMode::Solo(_) => handle_solo_key(app, key)?,
                }
            }
        }
    }

    // Poll session status
    app.poll_sessions();

    Ok(!app.should_quit)
}

/// Handle keys when help overlay is showing
fn handle_help_key(app: &mut App, key: KeyEvent) -> crate::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
            app.show_help = false;
        }
        _ => {
            // Any other key closes help
            app.show_help = false;
        }
    }
    Ok(())
}

/// Handle keys when spawn picker is showing
fn handle_spawn_picker_key(app: &mut App, key: KeyEvent) -> crate::Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.close_spawn_picker();
        }
        KeyCode::Enter => {
            if let Err(e) = app.confirm_spawn() {
                app.status_message = Some(format!("Spawn failed: {}", e));
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(picker) = &mut app.spawn_picker {
                picker.next();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(picker) = &mut app.spawn_picker {
                picker.prev();
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handle confirmation prompts (y/n)
fn handle_confirm_key(app: &mut App, key: KeyEvent) -> crate::Result<()> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.confirm_action()?;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.cancel_confirm();
        }
        _ => {
            // Ignore other keys during confirmation
        }
    }
    Ok(())
}

/// Handle keys in symphony (overview) mode
fn handle_symphony_key(app: &mut App, key: KeyEvent) -> crate::Result<()> {
    match key.code {
        // Quit
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // Help
        KeyCode::Char('?') => {
            app.toggle_help();
        }

        // Navigation
        KeyCode::Down | KeyCode::Char('j') => {
            app.next_session();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.prev_session();
        }

        // Zoom in
        KeyCode::Enter => {
            app.zoom_in();
        }

        // Spawn new agent (opens picker)
        KeyCode::Char('s') => {
            app.open_spawn_picker();
        }

        // Kill selected (with confirmation)
        KeyCode::Char('K') | KeyCode::Delete => {
            app.request_kill();
        }

        // Nudge selected
        KeyCode::Char('n') => {
            if let Err(e) = app.nudge_selected() {
                app.status_message = Some(format!("Nudge failed: {}", e));
            }
        }

        // Cleanup exited sessions
        KeyCode::Char('c') => {
            let cleaned = app.sessions.cleanup();
            if cleaned.is_empty() {
                app.status_message = Some("No completed sessions to clean".to_string());
            } else {
                app.status_message = Some(format!("Cleaned {} session(s)", cleaned.len()));
            }
        }

        _ => {}
    }

    Ok(())
}

/// Handle keys in solo (zoom) mode
fn handle_solo_key(app: &mut App, key: KeyEvent) -> crate::Result<()> {
    match key.code {
        // Zoom out
        KeyCode::Esc => {
            app.zoom_out();
        }

        // Quit (with confirmation maybe?)
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // Help
        KeyCode::Char('?') => {
            app.toggle_help();
        }

        // Nudge
        KeyCode::Char('n') => {
            if let Err(e) = app.nudge_selected() {
                app.status_message = Some(format!("Nudge failed: {}", e));
            }
        }

        // Kill (with confirmation, then zoom out)
        KeyCode::Char('k') | KeyCode::Char('K') => {
            app.request_kill();
            // Will zoom out after confirmation in confirm_action
        }

        // TODO: In full PTY passthrough mode, forward all other keys to the PTY
        // For now, just ignore them
        _ => {}
    }

    Ok(())
}
