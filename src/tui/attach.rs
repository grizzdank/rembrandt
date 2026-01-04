//! Direct PTY attachment mode
//!
//! When attached, the PTY has direct control of the terminal.
//! This allows full TUI applications like Claude Code to render correctly.

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::fs::File;

use crate::daemon::SessionManager;

/// Result of an attach session
pub enum AttachResult {
    /// User detached
    Detached,
    /// Session exited
    SessionEnded,
    /// Error occurred
    Error(String),
}

/// Attach directly to a PTY session
///
/// This exits the alternate screen and gives the PTY direct terminal control.
/// Press Ctrl+] to detach and return to the TUI.
pub fn attach_to_session(
    sessions: &mut SessionManager,
    session_id: &str,
) -> crate::Result<AttachResult> {
    // Get the session
    let session = sessions
        .get(session_id)
        .ok_or_else(|| crate::RembrandtError::SessionNotFound(session_id.to_string()))?;

    // Get a reader for PTY output
    let mut pty_reader = session.try_clone_reader()?;

    // Leave alternate screen so PTY output shows directly
    execute!(io::stdout(), LeaveAlternateScreen).ok();

    // Flag to signal threads to stop
    let running = Arc::new(AtomicBool::new(true));

    // Spawn thread to copy PTY output to stdout
    let running_clone = running.clone();
    let output_handle = thread::spawn(move || {
        let mut stdout = io::stdout();
        let mut buf = [0u8; 4096];

        while running_clone.load(Ordering::Relaxed) {
            match pty_reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    stdout.write_all(&buf[..n]).ok();
                    stdout.flush().ok();
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    });

    // Read from stdin using raw file descriptor
    // This bypasses Rust's stdin buffering which can interfere
    let stdin_fd = io::stdin().as_raw_fd();

    // Create a raw reader from the fd (don't close it - stdin owns it)
    let mut stdin_reader = unsafe { File::from_raw_fd(libc::dup(stdin_fd)) };

    let result = loop {
        let mut buf = [0u8; 256];
        match stdin_reader.read(&mut buf) {
            Ok(0) => break AttachResult::Detached,
            Ok(n) => {
                // Ctrl+] (0x1d) detaches
                if buf[..n].contains(&0x1d) {
                    break AttachResult::Detached;
                }
                // Forward to PTY
                if let Some(session) = sessions.get_mut(session_id) {
                    session.write(&buf[..n]).ok();
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(_) => break AttachResult::Error("stdin error".to_string()),
        }

        // Check session
        if let Some(session) = sessions.get_mut(session_id) {
            if !session.is_running() {
                break AttachResult::SessionEnded;
            }
        } else {
            break AttachResult::SessionEnded;
        }
    };

    // Stop output thread
    running.store(false, Ordering::Relaxed);
    output_handle.join().ok();

    // Re-enter alternate screen
    execute!(io::stdout(), EnterAlternateScreen).ok();

    Ok(result)
}
