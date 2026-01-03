//! Daemon module - PTY session management
//!
//! The Rembrandt daemon keeps agent sessions alive independently of the TUI.
//! It manages PTY sessions, handles attach/detach, and enables nudging.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     IPC      ┌─────────────────┐
//! │  TUI / CLI      │◄────────────►│     Daemon      │
//! │  (client)       │   (socket)   │                 │
//! └─────────────────┘              │  ┌───────────┐  │
//!                                  │  │ Session   │  │
//!                                  │  │ Manager   │  │
//!                                  │  └─────┬─────┘  │
//!                                  │        │        │
//!                                  │  ┌─────┴─────┐  │
//!                                  │  │ PtySession│  │
//!                                  │  │ PtySession│  │
//!                                  │  │ PtySession│  │
//!                                  │  └───────────┘  │
//!                                  └─────────────────┘
//! ```
//!
//! # "No Idle Hands" Principle
//!
//! When an agent session starts, if there's work on its easel (assignment),
//! it should begin immediately. The daemon supports nudging stalled agents.

pub mod buffer;
pub mod ipc;
pub mod manager;
pub mod session;

pub use buffer::RingBuffer;
pub use ipc::{DaemonCommand, DaemonEvent, DaemonResponse};
pub use manager::{SessionInfo, SessionManager};
pub use session::{PtySession, SessionId, SessionStatus};

use crate::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;

/// The Rembrandt daemon server
pub struct Daemon {
    /// Session manager (shared across client handlers)
    manager: Arc<Mutex<SessionManager>>,
    /// Path to the Unix socket
    socket_path: PathBuf,
}

impl Daemon {
    /// Create a new daemon instance
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            manager: Arc::new(Mutex::new(SessionManager::new())),
            socket_path,
        }
    }

    /// Run the daemon, listening for client connections
    pub async fn run(&self) -> Result<()> {
        // Remove stale socket if it exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .map_err(|e| crate::RembrandtError::Daemon(e.to_string()))?;
        }

        let listener = UnixListener::bind(&self.socket_path)
            .map_err(|e| crate::RembrandtError::Daemon(e.to_string()))?;

        tracing::info!("Daemon listening on {:?}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let manager = self.manager.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, manager).await {
                            tracing::error!("Client handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Accept error: {}", e);
                }
            }
        }
    }

    /// Get a reference to the session manager
    pub fn manager(&self) -> Arc<Mutex<SessionManager>> {
        self.manager.clone()
    }
}

/// Handle a single client connection
///
/// # TODO: Implement client handling logic
///
/// This is the core IPC handler. When a client connects:
/// 1. Read commands from the stream
/// 2. Execute them against the SessionManager
/// 3. Send responses back
///
/// For `Attach` commands, you'll need to:
/// - Send buffered history first
/// - Then stream new output as it arrives
///
/// Consider:
/// - How to frame messages (length-prefix? newline-delimited JSON?)
/// - How to handle multiple attached clients to same session
/// - Error handling and recovery
async fn handle_client(
    stream: UnixStream,
    manager: Arc<Mutex<SessionManager>>,
) -> Result<()> {
    // YOUR IMPLEMENTATION HERE
    //
    // Suggested approach:
    //
    // 1. Choose a framing protocol. Options:
    //    a) Length-prefixed: [4-byte len][JSON payload]
    //    b) Newline-delimited JSON (simpler, slightly less efficient)
    //
    // 2. Read loop:
    //    - Read a command from the stream
    //    - Deserialize to DaemonCommand
    //    - Match on command type and execute
    //    - Serialize response to DaemonResponse
    //    - Write response to stream
    //
    // 3. For Attach:
    //    - Get session's output buffer
    //    - Send history as DaemonResponse::Output
    //    - Switch to streaming mode: spawn a task that reads from
    //      the PTY and sends DaemonEvent::Output
    //    - Keep reading commands (Detach, Write, etc.)
    //
    // Example skeleton:
    //
    // let (reader, writer) = stream.into_split();
    // let mut reader = BufReader::new(reader);
    // let mut writer = BufWriter::new(writer);
    //
    // loop {
    //     let mut line = String::new();
    //     reader.read_line(&mut line).await?;
    //     if line.is_empty() { break; }
    //
    //     let cmd: DaemonCommand = serde_json::from_str(&line)?;
    //     let response = match cmd {
    //         DaemonCommand::Ping => DaemonResponse::Pong,
    //         DaemonCommand::List => {
    //             let mgr = manager.lock().await;
    //             DaemonResponse::Sessions { sessions: mgr.list() }
    //         }
    //         // ... handle other commands
    //     };
    //
    //     let json = serde_json::to_string(&response)?;
    //     writer.write_all(json.as_bytes()).await?;
    //     writer.write_all(b"\n").await?;
    //     writer.flush().await?;
    // }

    todo!("Implement client handling")
}

/// Daemon client for TUI/CLI to communicate with daemon
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    /// Create a new client
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Connect to the daemon
    pub async fn connect(&self) -> Result<UnixStream> {
        UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| crate::RembrandtError::Daemon(e.to_string()))
    }

    // TODO: Add convenience methods for each command
    // pub async fn spawn(...) -> Result<SessionId>
    // pub async fn list() -> Result<Vec<SessionInfo>>
    // pub async fn nudge(id: &str) -> Result<()>
    // etc.
}
