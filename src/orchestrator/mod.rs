//! V2 orchestration service layer.

use crate::isolation::{BranchIsolation, IsolationContext, IsolationMode, IsolationStrategy, WorktreeIsolation};
use crate::runtime::{AgentRuntime, RuntimeAgentStatus};
use crate::state::{SessionRecord, SessionStatus, StateStore};
use crate::Result;
use chrono::Utc;
use std::path::{Path, PathBuf};

/// Parameters for spawning an agent session through the v2 orchestration path.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub agent_id: String,
    pub base_branch: String,
    pub isolation_mode: IsolationMode,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub task_id: Option<String>,
}

/// Summary returned after a successful spawn.
#[derive(Debug, Clone)]
pub struct SpawnResult {
    pub session: SessionRecord,
    pub workspace: IsolationContext,
}

/// Orchestration service coordinating runtime, isolation, and persistent state.
pub struct Orchestrator<R: AgentRuntime> {
    repo_path: PathBuf,
    runtime: R,
    state: StateStore,
}

impl<R: AgentRuntime> Orchestrator<R> {
    pub fn new(repo_path: impl AsRef<Path>, runtime: R) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let state = StateStore::open(&repo_path)?;
        Ok(Self {
            repo_path,
            runtime,
            state,
        })
    }

    pub fn state(&self) -> &StateStore {
        &self.state
    }

    pub async fn spawn_agent(&self, req: SpawnRequest) -> Result<SpawnResult> {
        let strategy = self.strategy_for(req.isolation_mode);
        let workspace = strategy
            .prepare(&self.repo_path, &req.agent_id, &req.base_branch)
            .await?;

        let handle = self
            .runtime
            .spawn(
                &req.agent_id,
                &workspace,
                req.prompt.as_deref(),
                req.model.as_deref(),
            )
            .await?;

        let now = Utc::now();
        let session = SessionRecord {
            agent_id: req.agent_id,
            runtime_kind: self.runtime.name().to_string(),
            runtime_session_id: Some(handle.runtime_session_id.0),
            isolation_mode: workspace.mode,
            branch_name: workspace.branch_name.clone(),
            checkout_path: workspace.checkout_path.clone(),
            task_id: req.task_id,
            status: SessionStatus::Starting,
            model: handle.model,
            created_at: now,
            updated_at: now,
        };

        self.state.upsert_session(&session)?;
        self.state.touch_heartbeat(&session.agent_id, Some("spawned"))?;

        Ok(SpawnResult { session, workspace })
    }

    pub fn list_agents(&self) -> Result<Vec<SessionRecord>> {
        self.state.list_sessions()
    }

    pub fn get_status(&self, agent_id: &str) -> Result<Option<SessionRecord>> {
        self.state.get_session(agent_id)
    }

    pub async fn refresh_runtime_status(&self, agent_id: &str) -> Result<Option<SessionStatus>> {
        let Some(record) = self.state.get_session(agent_id)? else {
            return Ok(None);
        };
        let Some(runtime_session_id) = &record.runtime_session_id else {
            return Ok(None);
        };

        let runtime_status = self
            .runtime
            .status(&crate::runtime::RuntimeSessionId(runtime_session_id.clone()))
            .await?;

        let mapped = map_runtime_status(runtime_status);
        self.state.update_status(agent_id, mapped)?;
        self.state.touch_heartbeat(agent_id, Some("status-refreshed"))?;
        Ok(Some(mapped))
    }

    pub async fn kill_agent(&self, agent_id: &str) -> Result<()> {
        if let Some(record) = self.state.get_session(agent_id)? {
            if let Some(runtime_session_id) = record.runtime_session_id {
                let _ = self
                    .runtime
                    .stop(&crate::runtime::RuntimeSessionId(runtime_session_id))
                    .await;
            }
            self.state.update_status(agent_id, SessionStatus::Stopped)?;
            self.state.touch_heartbeat(agent_id, Some("stopped"))?;
        }
        Ok(())
    }

    pub async fn steer_agent(&self, agent_id: &str, message: &str) -> Result<()> {
        if let Some(record) = self.state.get_session(agent_id)? {
            if let Some(runtime_session_id) = record.runtime_session_id {
                self.runtime
                    .send_message(
                        &crate::runtime::RuntimeSessionId(runtime_session_id),
                        message,
                    )
                    .await?;
                self.state.touch_heartbeat(agent_id, Some("message-sent"))?;
            }
        }
        Ok(())
    }

    fn strategy_for(&self, mode: IsolationMode) -> Box<dyn IsolationStrategy> {
        match mode {
            IsolationMode::Branch => Box::new(BranchIsolation),
            IsolationMode::Worktree => Box::new(WorktreeIsolation),
        }
    }
}

fn map_runtime_status(status: RuntimeAgentStatus) -> SessionStatus {
    match status {
        RuntimeAgentStatus::Starting => SessionStatus::Starting,
        RuntimeAgentStatus::Running => SessionStatus::Active,
        RuntimeAgentStatus::Idle => SessionStatus::Idle,
        RuntimeAgentStatus::Completed => SessionStatus::Completed,
        RuntimeAgentStatus::Failed(_) => SessionStatus::Failed,
        RuntimeAgentStatus::Stopped => SessionStatus::Stopped,
    }
}
