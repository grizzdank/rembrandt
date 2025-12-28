//! Rembrandt: Orchestration layer for coding agents
//!
//! Like Rembrandt's workshop - multiple apprentices working on different parts
//! of the canvas, unified by the master into a cohesive masterpiece.

pub mod agent;
pub mod cli;
pub mod integration;
pub mod tui;
pub mod worktree;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RembrandtError {
    #[error("Git operation failed: {0}")]
    Git(#[from] git2::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Worktree error: {0}")]
    Worktree(String),
}

pub type Result<T> = std::result::Result<T, RembrandtError>;
