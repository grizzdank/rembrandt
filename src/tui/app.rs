//! Main TUI application state and event handling

use super::ViewMode;
use crate::daemon::{SessionInfo, SessionManager, SessionStatus};
use crate::worktree::WorktreeManager;
use std::path::PathBuf;

/// Pending confirmation action
#[derive(Debug, Clone)]
pub enum PendingConfirm {
    /// Confirm kill of session (agent_id, session_id)
    Kill { agent_id: String, session_id: String },
}

/// Main application state
pub struct App {
    /// Current view mode (Symphony = overview, Solo = zoom)
    pub view_mode: ViewMode,
    /// Session manager (owns the PTY sessions)
    pub sessions: SessionManager,
    /// Worktree manager
    pub worktrees: WorktreeManager,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Currently selected session index (in symphony view)
    pub selected_index: usize,
    /// Status message to display
    pub status_message: Option<String>,
    /// Repository path
    pub repo_path: PathBuf,
    /// Pending confirmation (if any)
    pub pending_confirm: Option<PendingConfirm>,
}

impl App {
    pub fn new(repo_path: PathBuf) -> crate::Result<Self> {
        let worktrees = WorktreeManager::new(&repo_path).map_err(|e| {
            crate::RembrandtError::Worktree(format!(
                "Failed to open repo at {:?}: {}",
                repo_path, e
            ))
        })?;

        Ok(Self {
            view_mode: ViewMode::Symphony,
            sessions: SessionManager::new(),
            worktrees,
            should_quit: false,
            selected_index: 0,
            status_message: Some("Press 's' to spawn, 'q' to quit".to_string()),
            repo_path,
            pending_confirm: None,
        })
    }

    /// Get list of all sessions for display
    pub fn session_list(&self) -> Vec<SessionInfo> {
        self.sessions.list()
    }

    /// Get the currently selected session
    pub fn selected_session(&self) -> Option<SessionInfo> {
        let sessions = self.session_list();
        sessions.get(self.selected_index).cloned()
    }

    /// Select next session
    pub fn next_session(&mut self) {
        let count = self.sessions.total_count();
        if count > 0 {
            self.selected_index = (self.selected_index + 1) % count;
        }
    }

    /// Select previous session
    pub fn prev_session(&mut self) {
        let count = self.sessions.total_count();
        if count > 0 {
            self.selected_index = self.selected_index.checked_sub(1).unwrap_or(count - 1);
        }
    }

    /// Zoom into the selected session (Symphony -> Solo)
    pub fn zoom_in(&mut self) {
        if self.sessions.total_count() > 0 {
            self.view_mode = ViewMode::Solo(self.selected_index);
            self.status_message = None; // Clear so help text shows
        }
    }

    /// Zoom out to symphony view (Solo -> Symphony)
    pub fn zoom_out(&mut self) {
        self.view_mode = ViewMode::Symphony;
        self.status_message = Some("Press 's' to spawn, Enter to zoom".to_string());
    }

    /// Poll all sessions to update their status
    pub fn poll_sessions(&mut self) {
        self.sessions.poll_all();
    }

    /// Spawn a new agent session
    pub fn spawn_agent(&mut self, agent_type: &str, task: Option<&str>) -> crate::Result<String> {
        use crate::agent::AgentType;

        // Generate agent ID
        let suffix: String = (0..4)
            .map(|_| format!("{:x}", rand::random::<u8>() % 16))
            .collect();
        let agent_id = format!("{}-{}", agent_type, suffix);

        // Create worktree from current branch (HEAD)
        // The worktree manager will create a new branch rembrandt/{agent_id}
        let base_branch = self.get_current_branch().unwrap_or_else(|| "main".to_string());
        let worktree = self.worktrees.create_worktree(&agent_id, &base_branch)?;

        // Resolve command
        let agent = AgentType::from_str(agent_type);
        let command = agent.command();
        let args = agent.default_args();

        // Spawn PTY session
        let session_id = self.sessions.spawn(
            agent_id.clone(),
            command,
            &args,
            &worktree.path,
        )?;

        self.status_message = Some(format!("Spawned {} ({})", agent_id, session_id));
        Ok(session_id)
    }

    /// Request kill confirmation for the selected session
    pub fn request_kill(&mut self) {
        if let Some(session) = self.selected_session() {
            self.pending_confirm = Some(PendingConfirm::Kill {
                agent_id: session.agent_id.clone(),
                session_id: session.id.clone(),
            });
            self.status_message = Some(format!(
                "Kill {} and DELETE worktree? (y/n)",
                session.agent_id
            ));
        }
    }

    /// Cancel pending confirmation
    pub fn cancel_confirm(&mut self) {
        self.pending_confirm = None;
        self.status_message = Some("Cancelled".to_string());
    }

    /// Confirm and execute pending action
    pub fn confirm_action(&mut self) -> crate::Result<()> {
        if let Some(confirm) = self.pending_confirm.take() {
            match confirm {
                PendingConfirm::Kill { agent_id, session_id } => {
                    // Kill the PTY session
                    self.sessions.kill(&session_id)?;

                    // Remove from session manager
                    self.sessions.remove(&session_id);

                    // Cleanup the worktree
                    if let Err(e) = self.worktrees.remove_worktree(&agent_id) {
                        self.status_message = Some(format!(
                            "Killed {} (worktree cleanup failed: {})",
                            agent_id, e
                        ));
                    } else {
                        self.status_message = Some(format!("Killed {} + cleaned worktree", agent_id));
                    }

                    // Adjust selected index if needed
                    let count = self.sessions.total_count();
                    if self.selected_index >= count && count > 0 {
                        self.selected_index = count - 1;
                    }

                    // Return to symphony view if we were in solo
                    if matches!(self.view_mode, ViewMode::Solo(_)) {
                        self.view_mode = ViewMode::Symphony;
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if there's a pending confirmation
    pub fn has_pending_confirm(&self) -> bool {
        self.pending_confirm.is_some()
    }

    /// Nudge the selected session
    pub fn nudge_selected(&mut self) -> crate::Result<()> {
        if let Some(session) = self.selected_session() {
            self.sessions.nudge(&session.id)?;
            self.status_message = Some(format!("Nudged {}", session.agent_id));
        }
        Ok(())
    }

    /// Get count of sessions needing attention (failed/exited non-zero)
    pub fn attention_count(&self) -> usize {
        self.sessions.failed_sessions().len()
    }

    /// Get status display for a session
    pub fn status_display(status: &SessionStatus) -> (&'static str, &'static str) {
        match status {
            SessionStatus::Running => ("●", "active"),
            SessionStatus::Exited(0) => ("✓", "done"),
            SessionStatus::Exited(_) => ("✗", "failed"),
            SessionStatus::Failed(_) => ("!", "error"),
        }
    }

    /// Get the current git branch name
    fn get_current_branch(&self) -> Option<String> {
        use git2::Repository;
        let repo = Repository::open(&self.repo_path).ok()?;
        let head = repo.head().ok()?;
        head.shorthand().map(|s| s.to_string())
    }
}
