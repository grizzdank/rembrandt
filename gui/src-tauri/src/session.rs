//! PTY Session management for Tauri
//!
//! Each PtySession wraps a single agent process running in a pseudo-terminal.

use crate::{buffer::RingBuffer, AppError, Result};
use chrono::{DateTime, Utc};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SessionStatus {
    /// Process is running
    Running,
    /// Process exited with code
    Exited(i32),
    /// Process failed to start or crashed
    Failed(String),
}

/// A single PTY session wrapping an agent process
pub struct PtySession {
    /// Unique session ID
    pub id: SessionId,
    /// Agent identity this session belongs to
    pub agent_id: String,
    /// PTY master for I/O
    master: Box<dyn MasterPty + Send>,
    /// Writer for PTY input (cloned from master)
    writer: Box<dyn Write + Send>,
    /// Child process handle
    child: Box<dyn Child + Send + Sync>,
    /// Ring buffer for output history
    output_buffer: Arc<Mutex<RingBuffer>>,
    /// Current session status
    pub status: SessionStatus,
    /// When this session was created
    pub created_at: DateTime<Utc>,
    /// Command that was spawned
    pub command: String,
    /// Working directory
    pub workdir: String,
    /// PTY reader for on-demand output reading
    reader: Option<Box<dyn Read + Send>>,
    /// Git branch this session is working on (if using worktree isolation)
    pub branch: Option<String>,
    /// Whether this session is using an isolated worktree
    pub isolated: bool,
    /// Beads task ID assigned to this session (if any)
    pub task_id: Option<String>,
    /// Beads task title (cached for display)
    pub task_title: Option<String>,
}

impl PtySession {
    /// Spawn a new agent process in a PTY
    pub fn spawn(
        agent_id: String,
        command: &str,
        args: &[&str],
        workdir: &Path,
        buffer_capacity: usize,
        rows: Option<u16>,
        cols: Option<u16>,
        branch: Option<String>,
        isolated: bool,
        task_id: Option<String>,
        task_title: Option<String>,
    ) -> Result<Self> {
        let pty_system = native_pty_system();

        let size = PtySize {
            rows: rows.unwrap_or(24),
            cols: cols.unwrap_or(80),
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(size)
            .map_err(|e| AppError::Pty(e.to_string()))?;

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        cmd.cwd(workdir);

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| AppError::Pty(e.to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| AppError::Pty(e.to_string()))?;

        let output_buffer = Arc::new(Mutex::new(RingBuffer::new(buffer_capacity)));

        // Create reader with non-blocking mode on Unix
        #[cfg(unix)]
        let reader = {
            use std::os::unix::io::FromRawFd;
            if let Some(master_fd) = pair.master.as_raw_fd() {
                let fd = unsafe { libc::dup(master_fd) };
                if fd >= 0 {
                    unsafe {
                        let flags = libc::fcntl(fd, libc::F_GETFL);
                        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                    }
                    let file = unsafe { std::fs::File::from_raw_fd(fd) };
                    Some(Box::new(file) as Box<dyn Read + Send>)
                } else {
                    pair.master
                        .try_clone_reader()
                        .map_err(|e| AppError::Pty(e.to_string()))?
                        .into()
                }
            } else {
                pair.master
                    .try_clone_reader()
                    .map_err(|e| AppError::Pty(e.to_string()))?
                    .into()
            }
        };

        #[cfg(not(unix))]
        let reader = Some(
            pair.master
                .try_clone_reader()
                .map_err(|e| AppError::Pty(e.to_string()))?,
        );

        Ok(Self {
            id: generate_session_id(),
            agent_id,
            master: pair.master,
            writer,
            child,
            output_buffer,
            status: SessionStatus::Running,
            created_at: Utc::now(),
            command: command.to_string(),
            workdir: workdir.display().to_string(),
            reader,
            branch,
            isolated,
            task_id,
            task_title,
        })
    }

    /// Read available PTY output into the buffer (non-blocking)
    pub fn read_available(&mut self) -> usize {
        let reader = match self.reader.as_mut() {
            Some(r) => r,
            None => return 0,
        };

        let mut total = 0;
        let mut buf = [0u8; 4096];

        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(mut guard) = self.output_buffer.lock() {
                        guard.write(&buf[..n]);
                    }
                    total += n;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        total
    }

    /// Write data to the PTY (agent's stdin)
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer
            .write_all(data)
            .map_err(|e| AppError::Pty(e.to_string()))?;
        self.writer
            .flush()
            .map_err(|e| AppError::Pty(e.to_string()))?;
        Ok(())
    }

    /// Send a nudge to wake a stalled agent
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
            .map_err(|e| AppError::Pty(e.to_string()))?;
        Ok(())
    }

    /// Read raw buffered output
    pub fn read_output_raw(&self) -> Vec<u8> {
        if let Ok(guard) = self.output_buffer.lock() {
            guard.read_all()
        } else {
            Vec::new()
        }
    }

    /// Poll the child process status
    pub fn poll(&mut self) -> SessionStatus {
        if self.status != SessionStatus::Running {
            return self.status.clone();
        }

        match self.child.try_wait() {
            Ok(Some(status)) => {
                let code = status.exit_code() as i32;
                self.status = SessionStatus::Exited(code);
            }
            Ok(None) => {}
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
            .map_err(|e| AppError::Pty(e.to_string()))?;
        self.status = SessionStatus::Exited(-1);
        Ok(())
    }

    /// Check if the session is still running
    pub fn is_running(&self) -> bool {
        self.status == SessionStatus::Running
    }
}
