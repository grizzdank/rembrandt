//! Git worktree management for agent isolation
//!
//! Each agent runs in an isolated worktree with its own branch,
//! preventing file conflicts between concurrent agents.

use crate::{AppError, Result};
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Manages git worktrees for agent isolation
pub struct WorktreeManager {
    /// Path to the main repository
    repo_path: PathBuf,
    /// Path to the .rembrandt directory
    rembrandt_dir: PathBuf,
}

impl WorktreeManager {
    /// Initialize worktree manager for a repository
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let rembrandt_dir = repo_path.join(".rembrandt");

        // Ensure .rembrandt/agents directory exists
        std::fs::create_dir_all(rembrandt_dir.join("agents"))?;

        Ok(Self {
            repo_path,
            rembrandt_dir,
        })
    }

    /// Create a new worktree for an agent
    pub fn create_worktree(&self, agent_id: &str, base_branch: &str) -> Result<WorktreeInfo> {
        let repo = Repository::open(&self.repo_path)?;

        let worktree_path = self.rembrandt_dir.join("agents").join(agent_id);
        let branch_name = format!("rembrandt/{}", agent_id);

        // Check if worktree already exists and is valid
        if worktree_path.exists() {
            // Verify it's actually a git worktree
            if worktree_path.join(".git").exists() {
                return Ok(WorktreeInfo {
                    path: worktree_path,
                    branch: branch_name,
                    agent_id: agent_id.to_string(),
                });
            } else {
                // Directory exists but isn't a valid worktree - remove it
                std::fs::remove_dir_all(&worktree_path)?;
            }
        }

        // Check if git already knows about this worktree (might be stale)
        if let Ok(worktree) = repo.find_worktree(agent_id) {
            // Prune stale worktree reference
            let _ = worktree.prune(Some(
                git2::WorktreePruneOptions::new()
                    .working_tree(true)
                    .valid(true),
            ));
        }

        // Create branch from base
        let base_ref = repo.find_branch(base_branch, git2::BranchType::Local)?;
        let base_commit = base_ref.get().peel_to_commit()?;

        // Delete existing branch if it exists (it might be stale from a previous session)
        if let Ok(mut existing_branch) = repo.find_branch(&branch_name, git2::BranchType::Local) {
            // Only delete if it's not the current branch
            if !existing_branch.is_head() {
                let _ = existing_branch.delete();
            }
        }

        // Create the new branch
        let new_branch = repo.branch(&branch_name, &base_commit, false)?;
        let branch_ref = new_branch.into_reference();

        // Create the worktree with the branch
        repo.worktree(
            agent_id,
            &worktree_path,
            Some(git2::WorktreeAddOptions::new().reference(Some(&branch_ref))),
        )?;

        Ok(WorktreeInfo {
            path: worktree_path,
            branch: branch_name,
            agent_id: agent_id.to_string(),
        })
    }

    /// Remove a worktree and optionally its branch
    pub fn remove_worktree(&self, agent_id: &str, delete_branch: bool) -> Result<()> {
        let repo = Repository::open(&self.repo_path)?;

        // Prune the worktree
        if let Ok(worktree) = repo.find_worktree(agent_id) {
            worktree.prune(Some(
                git2::WorktreePruneOptions::new()
                    .working_tree(true)
                    .valid(true),
            ))?;
        }

        // Remove the directory
        let worktree_path = self.rembrandt_dir.join("agents").join(agent_id);
        if worktree_path.exists() {
            std::fs::remove_dir_all(worktree_path)?;
        }

        // Optionally delete the branch
        if delete_branch {
            let branch_name = format!("rembrandt/{}", agent_id);
            if let Ok(mut branch) = repo.find_branch(&branch_name, git2::BranchType::Local) {
                // Only delete if not merged (safe delete would fail anyway)
                let _ = branch.delete();
            }
        }

        Ok(())
    }

    /// List all active worktrees
    pub fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let repo = Repository::open(&self.repo_path)?;
        let mut worktrees = Vec::new();

        for name in repo.worktrees()?.iter() {
            if let Some(name) = name {
                if let Ok(worktree) = repo.find_worktree(name) {
                    if let Some(path) = worktree.path().to_str() {
                        worktrees.push(WorktreeInfo {
                            path: PathBuf::from(path),
                            branch: format!("rembrandt/{}", name),
                            agent_id: name.to_string(),
                        });
                    }
                }
            }
        }

        Ok(worktrees)
    }

    /// Get repo path
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }
}

/// Information about a worktree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub agent_id: String,
}

/// Find the git repository root from a starting path
pub fn find_repo_root(start: &Path) -> Result<PathBuf> {
    let repo = Repository::discover(start)?;
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| AppError::Git(git2::Error::from_str("Bare repository not supported")))
}
