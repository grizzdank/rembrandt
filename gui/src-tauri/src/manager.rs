//! Session Manager for Tauri
//!
//! Manages the lifecycle of all PTY sessions.

use crate::session::{PtySession, SessionId, SessionStatus};
use crate::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Default output buffer size (256KB per session)
/// Claude Code can output significant content, especially during startup
const DEFAULT_BUFFER_CAPACITY: usize = 256 * 1024;

/// Summary of a session for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub agent_id: String,
    pub command: String,
    pub workdir: String,
    pub status: SessionStatus,
    pub created_at: String,
}

impl From<&PtySession> for SessionInfo {
    fn from(session: &PtySession) -> Self {
        Self {
            id: session.id.clone(),
            agent_id: session.agent_id.clone(),
            command: session.command.clone(),
            workdir: session.workdir.clone(),
            status: session.status.clone(),
            created_at: session.created_at.to_rfc3339(),
        }
    }
}

/// Manages all active PTY sessions
pub struct SessionManager {
    sessions: HashMap<SessionId, PtySession>,
    buffer_capacity: usize,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
        }
    }

    /// Spawn a new agent session with specific terminal size
    pub fn spawn(
        &mut self,
        agent_id: String,
        command: &str,
        args: &[&str],
        workdir: &Path,
        rows: Option<u16>,
        cols: Option<u16>,
    ) -> Result<SessionId> {
        let session = PtySession::spawn(
            agent_id,
            command,
            args,
            workdir,
            self.buffer_capacity,
            rows,
            cols,
        )?;
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        Ok(id)
    }

    /// Get a mutable session by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut PtySession> {
        self.sessions.get_mut(id)
    }

    /// Send a nudge to a session
    pub fn nudge(&mut self, id: &str) -> Result<()> {
        self.sessions
            .get_mut(id)
            .ok_or_else(|| AppError::SessionNotFound(id.to_string()))?
            .nudge()
    }

    /// Write data to a session's PTY
    pub fn write(&mut self, id: &str, data: &[u8]) -> Result<()> {
        self.sessions
            .get_mut(id)
            .ok_or_else(|| AppError::SessionNotFound(id.to_string()))?
            .write(data)
    }

    /// Resize a session's PTY
    pub fn resize(&self, id: &str, rows: u16, cols: u16) -> Result<()> {
        self.sessions
            .get(id)
            .ok_or_else(|| AppError::SessionNotFound(id.to_string()))?
            .resize(rows, cols)
    }

    /// Get output history for a session
    ///
    /// This also reads any new output from the PTY into the buffer first.
    pub fn get_history(&mut self, id: &str) -> Result<Vec<u8>> {
        // First, read any available output from the PTY into the buffer
        if let Some(session) = self.sessions.get_mut(id) {
            session.read_available();
        }

        self.sessions
            .get(id)
            .ok_or_else(|| AppError::SessionNotFound(id.to_string()))
            .map(|s| s.read_output_raw())
    }

    /// Kill a session
    pub fn kill(&mut self, id: &str) -> Result<()> {
        self.sessions
            .get_mut(id)
            .ok_or_else(|| AppError::SessionNotFound(id.to_string()))?
            .kill()
    }

    /// List all sessions
    pub fn list(&self) -> Vec<SessionInfo> {
        self.sessions.values().map(SessionInfo::from).collect()
    }

    /// Poll all sessions and update their status
    pub fn poll_all(&mut self) {
        for session in self.sessions.values_mut() {
            session.poll();
        }
    }

    /// Read available PTY output from all sessions
    pub fn read_all_available(&mut self) {
        for session in self.sessions.values_mut() {
            session.read_available();
        }
    }

    /// Remove exited sessions
    pub fn cleanup(&mut self) -> Vec<SessionId> {
        let exited: Vec<SessionId> = self
            .sessions
            .iter()
            .filter(|(_, s)| !s.is_running())
            .map(|(id, _)| id.clone())
            .collect();

        for id in &exited {
            self.sessions.remove(id);
        }

        exited
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
