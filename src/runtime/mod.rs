//! Agent runtime abstraction for v2 orchestration.

mod pi;

pub use pi::PiRuntime;

use crate::isolation::IsolationContext;
use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// Runtime-specific session identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSessionId(pub String);

/// Minimal runtime session handle tracked by the orchestrator.
#[derive(Debug, Clone)]
pub struct AgentHandle {
    pub runtime_session_id: RuntimeSessionId,
    pub agent_id: String,
    pub model: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Runtime lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeAgentStatus {
    Starting,
    Running,
    Idle,
    Completed,
    Failed(String),
    Stopped,
}

#[async_trait]
pub trait AgentRuntime: Send + Sync {
    fn name(&self) -> &'static str;

    async fn spawn(
        &self,
        agent_id: &str,
        workspace: &IsolationContext,
        prompt: Option<&str>,
        model: Option<&str>,
    ) -> Result<AgentHandle>;

    async fn send_message(&self, runtime_session_id: &RuntimeSessionId, message: &str) -> Result<()>;

    async fn status(&self, runtime_session_id: &RuntimeSessionId) -> Result<RuntimeAgentStatus>;

    async fn stop(&self, runtime_session_id: &RuntimeSessionId) -> Result<()>;
}
