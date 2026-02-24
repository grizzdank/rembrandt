//! `pi_agent_rust` runtime adapter (skeleton).

use super::{AgentHandle, AgentRuntime, RuntimeAgentStatus, RuntimeSessionId};
use crate::isolation::IsolationContext;
use crate::{RembrandtError, Result};
use async_trait::async_trait;
use std::collections::HashMap;

/// Placeholder `pi` runtime adapter.
///
/// This is wired into the v2 architecture first; actual `pi` integration lands in a
/// follow-up phase once interfaces and state plumbing are in place.
pub struct PiRuntime;

impl PiRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PiRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRuntime for PiRuntime {
    fn name(&self) -> &'static str {
        "pi"
    }

    async fn spawn(
        &self,
        agent_id: &str,
        _workspace: &IsolationContext,
        _prompt: Option<&str>,
        model: Option<&str>,
    ) -> Result<AgentHandle> {
        let mut metadata = HashMap::new();
        metadata.insert("status".to_string(), "stub".to_string());

        Ok(AgentHandle {
            runtime_session_id: RuntimeSessionId(format!("stub-{}", agent_id)),
            agent_id: agent_id.to_string(),
            model: model.map(str::to_string),
            metadata,
        })
    }

    async fn send_message(
        &self,
        _runtime_session_id: &RuntimeSessionId,
        _message: &str,
    ) -> Result<()> {
        Err(RembrandtError::Runtime(
            "PiRuntime.send_message not implemented".to_string(),
        ))
    }

    async fn status(&self, _runtime_session_id: &RuntimeSessionId) -> Result<RuntimeAgentStatus> {
        Ok(RuntimeAgentStatus::Starting)
    }

    async fn stop(&self, _runtime_session_id: &RuntimeSessionId) -> Result<()> {
        Err(RembrandtError::Runtime(
            "PiRuntime.stop not implemented".to_string(),
        ))
    }
}
