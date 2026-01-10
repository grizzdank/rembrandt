//! Agent registry - tracks available and active agents

use super::{AgentSession, AgentStatus, AgentType};
use crate::{RembrandtError, Result};
use std::collections::HashMap;
use std::path::Path;

/// Registry of available agent configurations and active sessions
pub struct AgentRegistry {
    /// Available agent configurations
    available: HashMap<AgentType, AgentConfig>,
    /// Active agent sessions
    sessions: HashMap<String, AgentSession>,
}

/// Configuration for an agent type
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    /// Command to spawn the agent
    pub command: String,
    /// Default arguments
    pub args: Vec<String>,
    /// Whether this agent supports ACP
    pub supports_acp: bool,
}

impl AgentRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            available: HashMap::new(),
            sessions: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        // Claude Code
        self.available.insert(
            AgentType::ClaudeCode,
            AgentConfig {
                agent_type: AgentType::ClaudeCode,
                command: "claude".to_string(),
                args: vec![],
                supports_acp: false, // Not yet, needs adapter
            },
        );

        // OpenCode
        self.available.insert(
            AgentType::OpenCode,
            AgentConfig {
                agent_type: AgentType::OpenCode,
                command: "opencode".to_string(),
                args: vec![],
                supports_acp: false,
            },
        );

        // Codex
        self.available.insert(
            AgentType::Codex,
            AgentConfig {
                agent_type: AgentType::Codex,
                command: "codex".to_string(),
                args: vec![],
                supports_acp: false,
            },
        );

        // Aider
        self.available.insert(
            AgentType::Aider,
            AgentConfig {
                agent_type: AgentType::Aider,
                command: "aider".to_string(),
                args: vec![],
                supports_acp: false,
            },
        );
    }

    /// Get configuration for an agent type
    pub fn get_config(&self, agent_type: &AgentType) -> Option<&AgentConfig> {
        self.available.get(agent_type)
    }

    /// Register a new agent session
    pub fn register_session(&mut self, session: AgentSession) {
        self.sessions.insert(session.id.clone(), session);
    }

    /// Get all active sessions
    pub fn active_sessions(&self) -> Vec<&AgentSession> {
        self.sessions
            .values()
            .filter(|s| matches!(s.status, AgentStatus::Active | AgentStatus::Idle))
            .collect()
    }

    /// Get a session by ID
    pub fn get_session(&self, id: &str) -> Option<&AgentSession> {
        self.sessions.get(id)
    }

    /// Get a mutable session by ID
    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut AgentSession> {
        self.sessions.get_mut(id)
    }

    /// Update session status
    pub fn update_status(&mut self, id: &str, status: AgentStatus) -> Result<()> {
        self.sessions
            .get_mut(id)
            .map(|s| s.status = status)
            .ok_or_else(|| RembrandtError::Agent(format!("Session not found: {}", id)))
    }

    /// Remove a session
    pub fn remove_session(&mut self, id: &str) -> Option<AgentSession> {
        self.sessions.remove(id)
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
