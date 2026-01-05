//! Rembrandt GUI - Tauri backend
//!
//! Agent orchestration desktop app powered by Tauri + Svelte + xterm.js

pub mod buffer;
pub mod session;
pub mod manager;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("PTY error: {0}")]
    Pty(String),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;
