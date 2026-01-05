//! Direct PTY attachment mode
//!
//! When attached, the PTY has direct control of the terminal.
//! This allows full TUI applications like Claude Code to render correctly.

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};

use crate::daemon::SessionManager;

/// Result of an attach session
pub enum AttachResult {
    /// User detached (Ctrl+])
    Detached,
    /// Session exited
    SessionEnded,
    /// Error occurred
    Error(String),
}

/// Attach directly to a PTY session
///
/// This exits the alternate screen and gives the PTY direct terminal control.
/// Detach methods:
/// - Ctrl+] or Ctrl+\ (if not intercepted by the agent)
/// - Double-Escape (press Escape twice quickly)
pub fn attach_to_session(
    sessions: &mut SessionManager,
    session_id: &str,
) -> crate::Result<AttachResult> {
    // Get the session and take exclusive reader access
    let session = sessions
        .get_mut(session_id)
        .ok_or_else(|| crate::RembrandtError::SessionNotFound(session_id.to_string()))?;

    // Get current terminal size
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));

    // Take the reader - we get exclusive access
    let pty_reader = session.take_reader().ok_or_else(|| {
        crate::RembrandtError::Pty("PTY reader not available (already attached?)".to_string())
    })?;

    // Leave alternate screen so PTY output shows directly
    execute!(io::stdout(), LeaveAlternateScreen).ok();
    io::stdout().flush().ok();

    // Small delay then send SIGWINCH to force app redraw
    std::thread::sleep(std::time::Duration::from_millis(50));
    session.resize(rows, cols).ok();
    session.send_sigwinch();

    // Run the attach loop
    let result = run_attach_loop(sessions, session_id, pty_reader);

    // Disable mouse capture that the agent may have enabled
    execute!(io::stdout(), crossterm::event::DisableMouseCapture).ok();

    // Re-enter alternate screen and clear it for TUI
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0)
    )
    .ok();
    io::stdout().flush().ok();

    // Handle result - return reader and extract attach result
    match result {
        Ok((reader, attach_result)) => {
            // Return the reader to the session (if it still exists)
            if let Some(session) = sessions.get_mut(session_id) {
                session.return_reader(reader);
            }
            Ok(attach_result)
        }
        Err(e) => Ok(AttachResult::Error(e)),
    }
}

/// The main attach loop
fn run_attach_loop(
    sessions: &mut SessionManager,
    session_id: &str,
    mut pty_reader: Box<dyn Read + Send>,
) -> Result<(Box<dyn Read + Send>, AttachResult), String> {
    let mut stdout = io::stdout();

    // Set up stdin for raw reading
    let stdin_fd = io::stdin().as_raw_fd();
    let mut stdin_reader = unsafe { File::from_raw_fd(libc::dup(stdin_fd)) };

    // Save original stdin flags and set non-blocking
    let original_flags = unsafe { libc::fcntl(stdin_fd, libc::F_GETFL) };
    unsafe {
        libc::fcntl(stdin_fd, libc::F_SETFL, original_flags | libc::O_NONBLOCK);
    }

    // Helper to restore stdin flags
    let restore_stdin = || unsafe {
        libc::fcntl(stdin_fd, libc::F_SETFL, original_flags);
    };

    // Helper to drain buffered input
    fn drain_stdin(reader: &mut File) {
        let mut drain_buf = [0u8; 1024];
        loop {
            match reader.read(&mut drain_buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
    }

    let mut read_buf = [0u8; 4096];
    let mut stdin_buf = [0u8; 256];

    // Track last escape time for double-escape detection
    let mut last_escape: Option<std::time::Instant> = None;
    const DOUBLE_ESCAPE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300);

    loop {
        // Try to read from PTY (non-blocking since we set it up that way)
        match pty_reader.read(&mut read_buf) {
            Ok(0) => {
                // EOF - PTY closed
                drain_stdin(&mut stdin_reader);
                restore_stdin();
                return Ok((pty_reader, AttachResult::SessionEnded));
            }
            Ok(n) => {
                // Forward to stdout
                stdout.write_all(&read_buf[..n]).ok();
                stdout.flush().ok();
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // No data available - that's fine
            }
            Err(_) => {
                drain_stdin(&mut stdin_reader);
                restore_stdin();
                return Ok((pty_reader, AttachResult::SessionEnded));
            }
        }

        // Try to read from stdin (non-blocking)
        match stdin_reader.read(&mut stdin_buf) {
            Ok(0) => {
                drain_stdin(&mut stdin_reader);
                restore_stdin();
                return Ok((pty_reader, AttachResult::Detached));
            }
            Ok(n) => {
                // Check for detach sequences: Ctrl+] (0x1d) or Ctrl+\ (0x1c)
                if stdin_buf[..n].contains(&0x1d) || stdin_buf[..n].contains(&0x1c) {
                    drain_stdin(&mut stdin_reader);
                    restore_stdin();
                    return Ok((pty_reader, AttachResult::Detached));
                }

                // Check for double-escape (Escape = 0x1b)
                // Only count STANDALONE escapes, not escape sequences like arrow keys (\x1b[A)
                let has_standalone_escape = if n == 1 && stdin_buf[0] == 0x1b {
                    true // Single escape byte = standalone
                } else {
                    // Check for escape not followed by '[' (which would be an escape sequence)
                    let mut found = false;
                    for i in 0..n {
                        if stdin_buf[i] == 0x1b {
                            // Check if NOT followed by '['
                            if i + 1 >= n || stdin_buf[i + 1] != b'[' {
                                found = true;
                                break;
                            }
                        }
                    }
                    found
                };

                if has_standalone_escape {
                    if let Some(last) = last_escape {
                        if last.elapsed() < DOUBLE_ESCAPE_TIMEOUT {
                            // Double escape detected - detach!
                            drain_stdin(&mut stdin_reader);
                            restore_stdin();
                            return Ok((pty_reader, AttachResult::Detached));
                        }
                    }
                    last_escape = Some(std::time::Instant::now());
                }

                // Forward to PTY
                if let Some(session) = sessions.get_mut(session_id) {
                    session.write(&stdin_buf[..n]).ok();
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // No input available - that's fine
            }
            Err(_) => {
                drain_stdin(&mut stdin_reader);
                restore_stdin();
                return Ok((pty_reader, AttachResult::Error("stdin error".to_string())));
            }
        }

        // Check if session is still running
        if let Some(session) = sessions.get_mut(session_id) {
            if !session.is_running() {
                drain_stdin(&mut stdin_reader);
                restore_stdin();
                return Ok((pty_reader, AttachResult::SessionEnded));
            }
        } else {
            drain_stdin(&mut stdin_reader);
            restore_stdin();
            return Ok((pty_reader, AttachResult::SessionEnded));
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
