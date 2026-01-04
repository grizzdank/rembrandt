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

/// Handle keys in solo (zoom) mode - forwards input to PTY
fn handle_solo_key(app: &mut App, key: KeyEvent) -> crate::Result<()> {
    // Special keys that we handle ourselves (not forwarded to PTY)
    match key.code {
        // Zoom out (detach from PTY)
        KeyCode::Esc => {
            app.zoom_out();
            return Ok(());
        }

        // Quit
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return Ok(());
        }

        // Help overlay
        KeyCode::Char('?') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
            app.toggle_help();
            return Ok(());
        }

        _ => {}
    }

    // Forward all other keys to the PTY
    let bytes = key_to_bytes(key);
    if !bytes.is_empty() {
        if let Err(e) = app.write_to_session(&bytes) {
            app.status_message = Some(format!("Write failed: {}", e));
        }
    }

    Ok(())
}

/// Convert a key event to bytes for PTY input
fn key_to_bytes(key: KeyEvent) -> Vec<u8> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    match key.code {
        // Regular characters
        KeyCode::Char(c) => {
            if ctrl {
                // Ctrl+letter produces control codes (Ctrl+A = 0x01, etc.)
                if c.is_ascii_lowercase() {
                    vec![(c as u8) - b'a' + 1]
                } else if c.is_ascii_uppercase() {
                    vec![(c.to_ascii_lowercase() as u8) - b'a' + 1]
                } else {
                    vec![]
                }
            } else if alt {
                // Alt+key sends ESC followed by the key
                let mut bytes = vec![0x1b];
                bytes.extend(c.to_string().as_bytes());
                bytes
            } else {
                c.to_string().into_bytes()
            }
        }

        // Special keys
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Backspace => vec![0x7f], // DEL
        KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],

        // Arrow keys (ANSI escape sequences)
        KeyCode::Up => vec![0x1b, b'[', b'A'],
        KeyCode::Down => vec![0x1b, b'[', b'B'],
        KeyCode::Right => vec![0x1b, b'[', b'C'],
        KeyCode::Left => vec![0x1b, b'[', b'D'],

        // Home/End
        KeyCode::Home => vec![0x1b, b'[', b'H'],
        KeyCode::End => vec![0x1b, b'[', b'F'],

        // Page Up/Down
        KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],

        // Insert
        KeyCode::Insert => vec![0x1b, b'[', b'2', b'~'],

        // Function keys
        KeyCode::F(1) => vec![0x1b, b'O', b'P'],
        KeyCode::F(2) => vec![0x1b, b'O', b'Q'],
        KeyCode::F(3) => vec![0x1b, b'O', b'R'],
        KeyCode::F(4) => vec![0x1b, b'O', b'S'],
        KeyCode::F(5) => vec![0x1b, b'[', b'1', b'5', b'~'],
        KeyCode::F(6) => vec![0x1b, b'[', b'1', b'7', b'~'],
        KeyCode::F(7) => vec![0x1b, b'[', b'1', b'8', b'~'],
        KeyCode::F(8) => vec![0x1b, b'[', b'1', b'9', b'~'],
        KeyCode::F(9) => vec![0x1b, b'[', b'2', b'0', b'~'],
        KeyCode::F(10) => vec![0x1b, b'[', b'2', b'1', b'~'],
        KeyCode::F(11) => vec![0x1b, b'[', b'2', b'3', b'~'],
        KeyCode::F(12) => vec![0x1b, b'[', b'2', b'4', b'~'],
        KeyCode::F(_) => vec![],

        // Escape itself
        KeyCode::Esc => vec![0x1b],

        // Ignore others
        _ => vec![],
    }
}
