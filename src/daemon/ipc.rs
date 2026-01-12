//! IPC Protocol for daemon communication
//!
//! The Rembrandt daemon listens on a Unix socket. Clients (TUI, CLI)
//! send commands and receive responses using this protocol.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::manager::SessionInfo;
use super::session::SessionId;

/// Commands that can be sent to the daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonCommand {
    /// Spawn a new agent session
    Spawn {
        agent_id: String,
        command: String,
        args: Vec<String>,
        workdir: PathBuf,
    },

    /// Send a nudge to wake a stalled agent
    Nudge { session_id: SessionId },

    /// Write data to a session's PTY
    Write { session_id: SessionId, data: Vec<u8> },

    /// Kill a session
    Kill { session_id: SessionId },

    /// List all sessions
    List,

    /// List sessions for a specific agent
    ListByAgent { agent_id: String },

    /// Get session info
    GetSession { session_id: SessionId },

    /// Attach to a session (start streaming output)
    Attach { session_id: SessionId },

    /// Detach from a session (stop streaming)
    Detach { session_id: SessionId },

    /// Get buffered output history
    GetHistory { session_id: SessionId },

    /// Resize a session's PTY
    Resize {
        session_id: SessionId,
        rows: u16,
        cols: u16,
    },

    /// Ping the daemon (health check)
    Ping,

    /// Request daemon shutdown
    Shutdown,
}

/// Responses from the daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    /// Success with optional message
    Ok { message: Option<String> },

    /// Session was spawned
    Spawned { session_id: SessionId },

    /// List of sessions
    Sessions { sessions: Vec<SessionInfo> },

    /// Single session info
    Session { info: SessionInfo },

    /// Output data (for attach/history)
    Output { data: Vec<u8> },

    /// Pong response to ping
    Pong,

    /// Error occurred
    Error { message: String },
}

/// Events streamed from daemon to attached clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonEvent {
    /// New output from a session
    Output { session_id: SessionId, data: Vec<u8> },

    /// Session status changed
    StatusChanged {
        session_id: SessionId,
        status: String,
    },

    /// Session exited
    Exited { session_id: SessionId, code: i32 },
}

/// Get the default socket path for the daemon
pub fn default_socket_path() -> PathBuf {
    // Use XDG_RUNTIME_DIR if available, otherwise /tmp
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("rembrandt.sock")
    } else {
        PathBuf::from("/tmp").join(format!("rembrandt-{}.sock", whoami()))
    }
}

/// Get current username for socket path
fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

// Need to implement Serialize/Deserialize for SessionInfo
// Since it's in manager.rs with chrono DateTime, we need to handle that

impl Serialize for SessionInfo {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SessionInfo", 6)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("agent_id", &self.agent_id)?;
        state.serialize_field("command", &self.command)?;
        state.serialize_field("workdir", &self.workdir)?;
        state.serialize_field("status", &format!("{:?}", self.status))?;
        state.serialize_field("created_at", &self.created_at.to_rfc3339())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SessionInfo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // For now, we primarily serialize (daemon -> client)
        // Deserialization can be added if needed
        todo!("Implement SessionInfo deserialization if needed")
    }
}
