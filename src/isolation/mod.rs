//! Workspace isolation strategies for v2 orchestration.

use crate::worktree::WorktreeManager;
use crate::{RembrandtError, Result};
use async_trait::async_trait;
use git2::{BranchType, Repository};
use std::path::{Path, PathBuf};

/// Supported workspace isolation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationMode {
    Branch,
    Worktree,
}

impl std::fmt::Display for IsolationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsolationMode::Branch => write!(f, "branch"),
            IsolationMode::Worktree => write!(f, "worktree"),
        }
    }
}

/// Provisioned workspace details returned by an isolation strategy.
#[derive(Debug, Clone)]
pub struct IsolationContext {
    pub agent_id: String,
    pub mode: IsolationMode,
    pub repo_path: PathBuf,
    pub checkout_path: PathBuf,
    pub branch_name: String,
}

#[async_trait]
pub trait IsolationStrategy: Send + Sync {
    fn mode(&self) -> IsolationMode;

    async fn prepare(
        &self,
        repo_path: &Path,
        agent_id: &str,
        base_branch: &str,
    ) -> Result<IsolationContext>;

    async fn cleanup(&self, _ctx: &IsolationContext) -> Result<()> {
        Ok(())
    }
}

/// Worktree-backed isolation using the existing `WorktreeManager`.
pub struct WorktreeIsolation;

#[async_trait]
impl IsolationStrategy for WorktreeIsolation {
    fn mode(&self) -> IsolationMode {
        IsolationMode::Worktree
    }

    async fn prepare(
        &self,
        repo_path: &Path,
        agent_id: &str,
        base_branch: &str,
    ) -> Result<IsolationContext> {
        let manager = WorktreeManager::new(repo_path)?;
        let info = manager.create_worktree(agent_id, base_branch)?;
        Ok(IsolationContext {
            agent_id: agent_id.to_string(),
            mode: IsolationMode::Worktree,
            repo_path: repo_path.to_path_buf(),
            checkout_path: info.path,
            branch_name: info.branch,
        })
    }

    async fn cleanup(&self, ctx: &IsolationContext) -> Result<()> {
        let manager = WorktreeManager::new(&ctx.repo_path)?;
        manager.remove_worktree(&ctx.agent_id)
    }
}

/// Branch-only isolation: create a branch and use the shared checkout.
pub struct BranchIsolation;

#[async_trait]
impl IsolationStrategy for BranchIsolation {
    fn mode(&self) -> IsolationMode {
        IsolationMode::Branch
    }

    async fn prepare(
        &self,
        repo_path: &Path,
        agent_id: &str,
        base_branch: &str,
    ) -> Result<IsolationContext> {
        let repo = Repository::open(repo_path)?;
        let branch_name = format!("rembrandt/{}", agent_id);

        let base = repo
            .find_branch(base_branch, BranchType::Local)
            .map_err(RembrandtError::Git)?;
        let base_commit = base.get().peel_to_commit()?;

        if repo.find_branch(&branch_name, BranchType::Local).is_err() {
            repo.branch(&branch_name, &base_commit, false)?;
        }

        Ok(IsolationContext {
            agent_id: agent_id.to_string(),
            mode: IsolationMode::Branch,
            repo_path: repo_path.to_path_buf(),
            checkout_path: repo_path.to_path_buf(),
            branch_name,
        })
    }
}
