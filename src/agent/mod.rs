//! Agent registry and management
//!
//! Handles registration, tracking, and lifecycle of coding agents.

mod registry;

pub use registry::*;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported agent types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum AgentType {
    ClaudeCode,
    OpenCode,
    AmpCode,
    Codex,
    Aider,
    Custom(String),
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::ClaudeCode => write!(f, "claude-code"),
            AgentType::OpenCode => write!(f, "opencode"),
            AgentType::AmpCode => write!(f, "ampcode"),
            AgentType::Codex => write!(f, "codex"),
            AgentType::Aider => write!(f, "aider"),
            AgentType::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl AgentType {
    /// Parse agent type from CLI string
    pub fn from_str(s: &str) -> Self {
        match s {
            "claude-code" | "claude" => AgentType::ClaudeCode,
            "opencode" => AgentType::OpenCode,
            "ampcode" | "amp" => AgentType::AmpCode,
            "codex" => AgentType::Codex,
            "aider" => AgentType::Aider,
            other => AgentType::Custom(other.to_string()),
        }
    }

    /// Get the command to spawn this agent
    pub fn command(&self) -> &str {
        match self {
            AgentType::ClaudeCode => "claude",
            AgentType::OpenCode => "opencode",
            AgentType::AmpCode => "amp",
            AgentType::Codex => "codex",
            AgentType::Aider => "aider",
            AgentType::Custom(name) => name.as_str(),
        }
    }

    /// Get default arguments for this agent type
    pub fn default_args(&self) -> Vec<&'static str> {
        match self {
            // Most agents work fine with no args, they'll prompt interactively
            AgentType::ClaudeCode => vec![],
            AgentType::OpenCode => vec![],
            AgentType::AmpCode => vec![],
            AgentType::Codex => vec![],
            AgentType::Aider => vec![],
            AgentType::Custom(_) => vec![],
        }
    }
}

/// Status of an agent session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is running and working
    Active,
    /// Agent is idle, waiting for input
    Idle,
    /// Agent completed its task
    Completed,
    /// Agent encountered an error
    Failed(String),
    /// Agent was stopped by user
    Stopped,
}

/// An active agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    /// Unique session ID
    pub id: String,
    /// Type of agent
    pub agent_type: AgentType,
    /// Current status
    pub status: AgentStatus,
    /// Path to the worktree this agent is using
    pub worktree_path: PathBuf,
    /// Branch name for this agent's work
    pub branch: String,
    /// Current task/issue ID (Beads)
    pub task_id: Option<String>,
    /// Competition ID if this agent is part of a competition
    pub competition_id: Option<String>,
    /// Process ID if running
    pub pid: Option<u32>,
    /// Files currently reserved by this agent
    pub reserved_files: Vec<PathBuf>,
    /// When the session started
    pub started_at: chrono::DateTime<chrono::Utc>,
}
