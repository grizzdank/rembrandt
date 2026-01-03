//! PTY Session management
//!
//! Each PtySession wraps a single agent process running in a pseudo-terminal.
//! Sessions survive TUI disconnects - the daemon keeps them alive.

use crate::{RembrandtError, Result};
use chrono::{DateTime, Utc};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::buffer::RingBuffer;

/// Unique session identifier
pub type SessionId = String;

/// Generate a unique session ID
pub fn generate_session_id() -> SessionId {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("ses-{:x}", timestamp)
}

/// Status of a PTY session
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    /// Process is running
    Running,
    /// Process exited with code
    Exited(i32),
    /// Process failed to start or crashed
    Failed(String),
}

/// A single PTY session wrapping an agent process
///
/// The session owns:
/// - The PTY master (for reading output, writing input)
/// - The child process handle
/// - An output buffer for late-attach scenarios
pub struct PtySession {
    /// Unique session ID
    pub id: SessionId,
    /// Agent identity this session belongs to (Bead ID)
    pub agent_id: String,
    /// PTY master for I/O
    master: Box<dyn MasterPty + Send>,
    /// Writer for PTY input (cloned from master)
    writer: Box<dyn Write + Send>,
    /// Child process handle
    child: Box<dyn Child + Send + Sync>,
    /// Ring buffer for output history (allows late-attach)
    output_buffer: Arc<Mutex<RingBuffer>>,
    /// Current session status
    pub status: SessionStatus,
    /// When this session was created
    pub created_at: DateTime<Utc>,
    /// Command that was spawned
    pub command: String,
    /// Working directory
    pub workdir: String,
}

impl PtySession {
    /// Spawn a new agent process in a PTY
    ///
    /// # Arguments
    /// * `agent_id` - The Bead ID of the agent identity
    /// * `command` - The command to run (e.g., "claude")
    /// * `args` - Command arguments
    /// * `workdir` - Working directory for the process
    /// * `buffer_capacity` - How many bytes of output to buffer for late-attach
    pub fn spawn(
        agent_id: String,
        command: &str,
        args: &[&str],
        workdir: &Path,
        buffer_capacity: usize,
    ) -> Result<Self> {
        let pty_system = native_pty_system();

        // Default terminal size - can be resized later
        let size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(size)
            .map_err(|e| RembrandtError::Pty(e.to_string()))?;

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        cmd.cwd(workdir);

        // Spawn the process in the PTY
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| RembrandtError::Pty(e.to_string()))?;

        // Get a writer for sending input to the PTY
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| RembrandtError::Pty(e.to_string()))?;

        Ok(Self {
            id: generate_session_id(),
            agent_id,
            master: pair.master,
            writer,
            child,
            output_buffer: Arc::new(Mutex::new(RingBuffer::new(buffer_capacity))),
            status: SessionStatus::Running,
            created_at: Utc::now(),
            command: command.to_string(),
            workdir: workdir.display().to_string(),
        })
    }

    /// Write data to the PTY (agent's stdin)
    ///
    /// Use this for sending input or nudging stalled agents.
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer
            .write_all(data)
            .map_err(|e| RembrandtError::Pty(e.to_string()))?;
        self.writer
            .flush()
            .map_err(|e| RembrandtError::Pty(e.to_string()))?;
        Ok(())
    }

    /// Send a nudge to wake a stalled agent
    ///
    /// This sends a newline, which often prompts Claude Code
    /// to continue if it's waiting for input.
    pub fn nudge(&mut self) -> Result<()> {
        self.write(b"\n")
    }

    /// Resize the PTY
    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| RembrandtError::Pty(e.to_string()))?;
        Ok(())
    }

    /// Get a reader for the PTY output
    ///
    /// Returns a clone of the master that can be used to read output.
    /// Output is also buffered in the ring buffer for late-attach.
    pub fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>> {
        self.master
            .try_clone_reader()
            .map_err(|e| RembrandtError::Pty(e.to_string()))
    }

    /// Get the output buffer for reading historical output
    pub fn output_buffer(&self) -> Arc<Mutex<RingBuffer>> {
        self.output_buffer.clone()
    }

    /// Poll the child process status
    ///
    /// Updates internal status and returns current state.
    pub fn poll(&mut self) -> SessionStatus {
        if self.status != SessionStatus::Running {
            return self.status.clone();
        }

        match self.child.try_wait() {
            Ok(Some(status)) => {
                let code = status.exit_code() as i32;
                self.status = SessionStatus::Exited(code);
            }
            Ok(None) => {
                // Still running
            }
            Err(e) => {
                self.status = SessionStatus::Failed(e.to_string());
            }
        }

        self.status.clone()
    }

    /// Kill the child process
    pub fn kill(&mut self) -> Result<()> {
        self.child
            .kill()
            .map_err(|e| RembrandtError::Pty(e.to_string()))?;
        self.status = SessionStatus::Exited(-1);
        Ok(())
    }

    /// Check if the session is still running
    pub fn is_running(&self) -> bool {
        self.status == SessionStatus::Running
    }
}

impl std::fmt::Debug for PtySession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtySession")
            .field("id", &self.id)
            .field("agent_id", &self.agent_id)
            .field("status", &self.status)
            .field("command", &self.command)
            .field("workdir", &self.workdir)
            .field("created_at", &self.created_at)
            .finish()
    }
}
