//! Session log persistence
//!
//! Writes full session output to persistent log files for post-mortem analysis.
//! Logs survive session cleanup and daemon restarts.
//!
//! Location: `~/.rembrandt/logs/{session-id}.log`

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::Result;

/// Directory name for logs within ~/.rembrandt/
const LOGS_DIR: &str = "logs";

/// Get the path to the rembrandt logs directory
///
/// Returns `~/.rembrandt/logs/`, creating it if necessary.
pub fn logs_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        crate::RembrandtError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        ))
    })?;

    let logs_path = home.join(".rembrandt").join(LOGS_DIR);
    fs::create_dir_all(&logs_path)?;
    Ok(logs_path)
}

/// Get the log file path for a session
pub fn log_path_for_session(session_id: &str) -> Result<PathBuf> {
    Ok(logs_dir()?.join(format!("{}.log", session_id)))
}

/// Logger for a single session
///
/// Handles appending output to a persistent log file.
/// Designed for efficient incremental writes during session lifetime.
pub struct SessionLogger {
    file: File,
    session_id: String,
    bytes_written: usize,
}

impl SessionLogger {
    /// Create a new session logger
    ///
    /// Opens (or creates) a log file for the given session ID.
    /// The file is opened in append mode to handle daemon restarts.
    pub fn new(session_id: &str) -> Result<Self> {
        let path = log_path_for_session(session_id)?;

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        tracing::debug!("Session logger created for {} at {:?}", session_id, path);

        Ok(Self {
            file,
            session_id: session_id.to_string(),
            bytes_written: 0,
        })
    }

    /// Write data to the log file
    ///
    /// Writes raw bytes to the log. ANSI escape codes are preserved
    /// for faithful replay of terminal output.
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.file.write_all(data)?;
        self.bytes_written += data.len();
        Ok(())
    }

    /// Flush buffered writes to disk
    ///
    /// Call periodically to ensure data is persisted.
    pub fn flush(&mut self) -> Result<()> {
        self.file.flush()?;
        Ok(())
    }

    /// Get the session ID this logger is for
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get total bytes written to the log
    pub fn bytes_written(&self) -> usize {
        self.bytes_written
    }
}

impl Drop for SessionLogger {
    fn drop(&mut self) {
        // Best-effort flush on drop
        let _ = self.file.flush();
        tracing::debug!(
            "Session logger for {} closed ({} bytes written)",
            self.session_id,
            self.bytes_written
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_logs_dir_creation() {
        // This test verifies the logs directory can be created
        let dir = logs_dir();
        assert!(dir.is_ok());
        assert!(dir.unwrap().exists());
    }

    #[test]
    fn test_session_logger_write() {
        let session_id = format!("test-{}", std::process::id());
        let logger = SessionLogger::new(&session_id);
        assert!(logger.is_ok());

        let mut logger = logger.unwrap();
        let test_data = b"Hello, session log!";
        assert!(logger.write(test_data).is_ok());
        assert!(logger.flush().is_ok());
        assert_eq!(logger.bytes_written(), test_data.len());

        // Verify file contents
        let path = log_path_for_session(&session_id).unwrap();
        let mut contents = Vec::new();
        File::open(&path)
            .unwrap()
            .read_to_end(&mut contents)
            .unwrap();
        assert_eq!(contents, test_data);

        // Cleanup
        let _ = fs::remove_file(path);
    }
}
