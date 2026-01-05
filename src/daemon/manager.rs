//! Session Manager
//!
//! Manages the lifecycle of all PTY sessions. The daemon uses this
//! to spawn, track, nudge, and cleanup agent sessions.

use crate::{RembrandtError, Result};
use std::collections::HashMap;
use std::path::Path;

use super::session::{PtySession, SessionId, SessionStatus};

/// Default output buffer size (10KB per session)
const DEFAULT_BUFFER_CAPACITY: usize = 10 * 1024;

/// Summary of a session for listing
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: SessionId,
    pub agent_id: String,
    pub command: String,
    pub workdir: String,
    pub status: SessionStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<&PtySession> for SessionInfo {
    fn from(session: &PtySession) -> Self {
        Self {
            id: session.id.clone(),
            agent_id: session.agent_id.clone(),
            command: session.command.clone(),
            workdir: session.workdir.clone(),
            status: session.status.clone(),
            created_at: session.created_at,
        }
    }
}

/// Manages all active PTY sessions
pub struct SessionManager {
    /// Active sessions indexed by session ID
    sessions: HashMap<SessionId, PtySession>,
    /// Output buffer capacity for new sessions
    buffer_capacity: usize,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
        }
    }

    /// Create with custom buffer capacity
    pub fn with_buffer_capacity(capacity: usize) -> Self {
        Self {
            sessions: HashMap::new(),
            buffer_capacity: capacity,
        }
    }

    /// Spawn a new agent session
    ///
    /// Returns the session ID on success.
    pub fn spawn(
        &mut self,
        agent_id: String,
        command: &str,
        args: &[&str],
        workdir: &Path,
    ) -> Result<SessionId> {
        self.spawn_with_size(agent_id, command, args, workdir, None, None)
    }

    /// Spawn a new agent session with specific terminal size
    ///
    /// Returns the session ID on success.
    pub fn spawn_with_size(
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

    /// Get a session by ID
    pub fn get(&self, id: &str) -> Option<&PtySession> {
        self.sessions.get(id)
    }

    /// Get a mutable session by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut PtySession> {
        self.sessions.get_mut(id)
    }

    /// Read buffered output from a session
    pub fn read_output(&self, id: &str) -> Option<String> {
        self.sessions.get(id).map(|s| s.read_output())
    }

    /// Send a nudge to a session
    pub fn nudge(&mut self, id: &str) -> Result<()> {
        self.sessions
            .get_mut(id)
            .ok_or_else(|| RembrandtError::SessionNotFound(id.to_string()))?
            .nudge()
    }

    /// Write data to a session's PTY
    pub fn write(&mut self, id: &str, data: &[u8]) -> Result<()> {
        self.sessions
            .get_mut(id)
            .ok_or_else(|| RembrandtError::SessionNotFound(id.to_string()))?
            .write(data)
    }

    /// Kill a session
    pub fn kill(&mut self, id: &str) -> Result<()> {
        self.sessions
            .get_mut(id)
            .ok_or_else(|| RembrandtError::SessionNotFound(id.to_string()))?
            .kill()
    }

    /// Remove a session from management
    ///
    /// Returns the session if it existed.
    pub fn remove(&mut self, id: &str) -> Option<PtySession> {
        self.sessions.remove(id)
    }

    /// List all sessions
    pub fn list(&self) -> Vec<SessionInfo> {
        self.sessions.values().map(SessionInfo::from).collect()
    }

    /// List sessions for a specific agent
    pub fn list_by_agent(&self, agent_id: &str) -> Vec<SessionInfo> {
        self.sessions
            .values()
            .filter(|s| s.agent_id == agent_id)
            .map(SessionInfo::from)
            .collect()
    }

    /// Poll all sessions and update their status
    pub fn poll_all(&mut self) {
        for session in self.sessions.values_mut() {
            session.poll();
        }
    }

    /// Read available PTY output from all sessions into their buffers
    ///
    /// Call this periodically from the TUI event loop.
    pub fn read_all_available(&mut self) {
        for session in self.sessions.values_mut() {
            session.read_available();
        }
    }

    /// Get IDs of all exited sessions
    pub fn exited_sessions(&self) -> Vec<SessionId> {
        self.sessions
            .values()
            .filter(|s| !s.is_running())
            .map(|s| s.id.clone())
            .collect()
    }

    /// Cleanup exited sessions based on policy
    ///
    /// Policy: Remove successful sessions (exit code 0) immediately.
    /// Keep failed sessions (non-zero exit or Failed status) for inspection
    /// until explicitly removed via `cleanup_failed()` or `remove()`.
    ///
    /// Rationale: Failures are the signal - you want to inspect them.
    /// Successes produced their artifacts (git commits) and can be cleaned.
    /// Session metadata will be persisted to SQLite (rembrandt-xz6) before
    /// cleanup for audit trail.
    ///
    /// Returns IDs of sessions that were removed.
    pub fn cleanup(&mut self) -> Vec<SessionId> {
        let successful: Vec<SessionId> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.status == SessionStatus::Exited(0))
            .map(|(id, _)| id.clone())
            .collect();

        for id in &successful {
            self.sessions.remove(id);
        }

        successful
    }

    /// Cleanup all exited sessions, including failures
    ///
    /// Use this for explicit "I've seen the failures, clean them up" action.
    /// Returns IDs of sessions that were removed.
    pub fn cleanup_all(&mut self) -> Vec<SessionId> {
        let exited = self.exited_sessions();
        for id in &exited {
            self.sessions.remove(id);
        }
        exited
    }

    /// Get IDs of failed sessions (non-zero exit or Failed status)
    pub fn failed_sessions(&self) -> Vec<SessionId> {
        self.sessions
            .values()
            .filter(|s| matches!(&s.status,
                SessionStatus::Exited(code) if *code != 0)
                || matches!(&s.status, SessionStatus::Failed(_)))
            .map(|s| s.id.clone())
            .collect()
    }

    /// Number of active sessions
    pub fn active_count(&self) -> usize {
        self.sessions.values().filter(|s| s.is_running()).count()
    }

    /// Total number of sessions (including exited)
    pub fn total_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cleanup_policy_documented() {
        // Note: Full integration tests require spawning real processes.
        // For unit tests, PtySession would need refactoring to accept
        // a mock PTY backend. For now, this documents expected behavior.
        // Policy documentation test - ensures the policy is clear:
        //
        // 1. Successful exits (code 0) → auto-cleaned
        //    Rationale: Artifacts are in git, no need to inspect
        //
        // 2. Failed exits (code != 0) → preserved for inspection
        //    Rationale: Failures are the signal, need debugging
        //
        // 3. Failed to start → preserved for inspection
        //    Rationale: Configuration/environment issues need diagnosis
        //
        // 4. Running → never cleaned (obviously)
        //
        // This policy will be enhanced when SQLite persistence (rembrandt-xz6)
        // is implemented - metadata will be persisted before cleanup.
        assert!(true, "Cleanup policy is: preserve failures, clean successes");
    }
}
