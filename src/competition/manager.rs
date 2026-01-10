//! CompetitionManager - orchestrates competition lifecycle

use crate::agent::{AgentRegistry, AgentSession, AgentStatus, AgentType};
use crate::competition::{
    create_evaluator, CompetitionGroup, CompetitionId, CompetitionStatus, CompetitorSolution,
    EvaluatorStrategy, SolutionValidator,
};
use crate::worktree::WorktreeManager;
use crate::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;

/// Manages competition lifecycle and state
pub struct CompetitionManager {
    /// Path to the repository
    repo_path: PathBuf,
    /// Worktree manager for creating agent worktrees
    worktree_manager: WorktreeManager,
    /// In-memory storage of active competitions
    competitions: HashMap<CompetitionId, CompetitionGroup>,
    /// Base branch for worktrees
    base_branch: String,
}

impl CompetitionManager {
    /// Create a new competition manager
    pub fn new(repo_path: PathBuf, base_branch: String) -> Result<Self> {
        let worktree_manager = WorktreeManager::new(&repo_path)?;
        Ok(Self {
            repo_path,
            worktree_manager,
            competitions: HashMap::new(),
            base_branch,
        })
    }

    /// Start a new competition
    pub async fn start_competition(
        &mut self,
        prompt: String,
        agent_types: Vec<AgentType>,
        evaluator_strategy: EvaluatorStrategy,
        timeout_minutes: u64,
        registry: &mut AgentRegistry,
    ) -> Result<CompetitionId> {
        // Create competition group
        let mut competition = CompetitionGroup::new(prompt.clone(), evaluator_strategy, timeout_minutes);
        let competition_id = competition.id.clone();

        // Spawn each agent
        for agent_type in agent_types {
            let agent_id = format!("{}-{}", competition_id, agent_type);

            // Create worktree for this agent
            let worktree_info = self
                .worktree_manager
                .create_worktree(&agent_id, &self.base_branch)?;

            // Create agent session
            let session = AgentSession {
                id: agent_id.clone(),
                agent_type: agent_type.clone(),
                status: AgentStatus::Active,
                worktree_path: worktree_info.path.clone(),
                branch: worktree_info.branch.clone(),
                task_id: None,
                pid: None,
                reserved_files: Vec::new(),
                started_at: Utc::now(),
                competition_id: Some(competition_id.clone()),
            };
            registry.register_session(session);

            // Add competitor to competition
            competition.competitors.push(CompetitorSolution {
                agent_id,
                agent_type,
                branch: worktree_info.branch,
                worktree_path: worktree_info.path,
                completed_at: None,
                validation: None,
                diff_stats: None,
            });

            // TODO: Actually spawn the agent process with the prompt
            // self.spawn_agent_with_prompt(&agent_id, &prompt)?;
        }

        // Update status to running
        competition.status = CompetitionStatus::Running {
            completed: 0,
            total: competition.competitors.len(),
        };

        // Store competition
        self.competitions.insert(competition_id.clone(), competition);

        Ok(competition_id)
    }

    /// Update competition status based on agent states
    pub async fn update_competition(
        &mut self,
        competition_id: &str,
        registry: &AgentRegistry,
    ) -> Result<CompetitionStatus> {
        // First, check what state we're in
        let current_status = {
            let competition = self
                .competitions
                .get(competition_id)
                .ok_or_else(|| {
                    crate::RembrandtError::Competition(format!(
                        "Competition not found: {}",
                        competition_id
                    ))
                })?;
            competition.status.clone()
        };

        match current_status {
            CompetitionStatus::Running { .. } => {
                self.update_running_competition(competition_id, registry).await
            }
            CompetitionStatus::Evaluating => {
                self.run_evaluation(competition_id).await
            }
            CompetitionStatus::Merging => {
                // Merge is handled separately
                Ok(current_status)
            }
            _ => Ok(current_status),
        }
    }

    /// Update a running competition - check for completions and timeout
    async fn update_running_competition(
        &mut self,
        competition_id: &str,
        registry: &AgentRegistry,
    ) -> Result<CompetitionStatus> {
        let competition = self
            .competitions
            .get_mut(competition_id)
            .ok_or_else(|| {
                crate::RembrandtError::Competition(format!(
                    "Competition not found: {}",
                    competition_id
                ))
            })?;

        let mut completed = 0;
        let total = competition.competitors.len();

        // Check each competitor's status
        for competitor in &mut competition.competitors {
            if let Some(session) = registry.get_session(&competitor.agent_id) {
                match &session.status {
                    AgentStatus::Completed => {
                        if competitor.completed_at.is_none() {
                            competitor.completed_at = Some(Utc::now());
                        }
                        completed += 1;
                    }
                    AgentStatus::Failed(_) | AgentStatus::Stopped => {
                        // Mark as completed but with no valid solution
                        if competitor.completed_at.is_none() {
                            competitor.completed_at = Some(Utc::now());
                        }
                        completed += 1;
                    }
                    _ => {}
                }
            }
        }

        // Check for timeout or all complete
        let timed_out = competition.is_timed_out();
        let all_complete = completed == total;

        if all_complete || timed_out {
            if completed == 0 {
                competition.status = CompetitionStatus::Failed(
                    "No agents completed before timeout".to_string(),
                );
            } else {
                competition.status = CompetitionStatus::Evaluating;
            }
        } else {
            competition.status = CompetitionStatus::Running { completed, total };
        }

        Ok(competition.status.clone())
    }

    /// Run evaluation on completed solutions
    async fn run_evaluation(
        &mut self,
        competition_id: &str,
    ) -> Result<CompetitionStatus> {
        // Validate each completed solution
        let validator = SolutionValidator::new(self.base_branch.clone());

        let competition = self
            .competitions
            .get_mut(competition_id)
            .ok_or_else(|| {
                crate::RembrandtError::Competition(format!(
                    "Competition not found: {}",
                    competition_id
                ))
            })?;

        for competitor in &mut competition.competitors {
            if competitor.completed_at.is_some() && competitor.validation.is_none() {
                // Run validation
                match validator.validate(competitor).await {
                    Ok(result) => {
                        competitor.validation = Some(result);
                    }
                    Err(e) => {
                        competitor.validation = Some(crate::competition::ValidationResult {
                            agent_id: competitor.agent_id.clone(),
                            type_check_passed: false,
                            type_check_output: None,
                            tests_passed: false,
                            tests_output: None,
                            test_count: None,
                            test_failures: None,
                            validation_time_ms: 0,
                            error_message: Some(e.to_string()),
                        });
                    }
                }

                // Calculate diff stats
                if let Ok(stats) = validator.calculate_diff_stats(competitor) {
                    competitor.diff_stats = Some(stats);
                }
            }
        }

        // Get valid solutions
        let valid_solutions = competition.valid_solutions();

        if valid_solutions.is_empty() {
            competition.status = CompetitionStatus::Failed(
                "No solutions passed validation".to_string(),
            );
            return Ok(competition.status.clone());
        }

        // Run evaluator
        let evaluator = create_evaluator(&competition.evaluator_strategy);
        let prompt = competition.prompt.clone();
        let repo_path = self.repo_path.clone();

        match evaluator
            .evaluate(&prompt, &valid_solutions, &repo_path)
            .await
        {
            Ok(result) => {
                competition.winner = Some(result.winner_id.clone());
                competition.evaluation_result = Some(result);
                competition.status = CompetitionStatus::Merging;
            }
            Err(e) => {
                competition.status = CompetitionStatus::Failed(format!(
                    "Evaluation failed: {}",
                    e
                ));
            }
        }

        Ok(competition.status.clone())
    }

    /// Get a competition by ID
    pub fn get_competition(&self, id: &str) -> Option<&CompetitionGroup> {
        self.competitions.get(id)
    }

    /// Get mutable reference to a competition
    pub fn get_competition_mut(&mut self, id: &str) -> Option<&mut CompetitionGroup> {
        self.competitions.get_mut(id)
    }

    /// List all competitions
    pub fn list_competitions(&self) -> Vec<&CompetitionGroup> {
        self.competitions.values().collect()
    }

    /// Get active (non-terminal) competitions
    pub fn active_competitions(&self) -> Vec<&CompetitionGroup> {
        self.competitions
            .values()
            .filter(|c| !c.status.is_terminal())
            .collect()
    }

    /// Cancel a competition
    pub fn cancel_competition(
        &mut self,
        competition_id: &str,
        registry: &mut AgentRegistry,
    ) -> Result<()> {
        let competition = self
            .competitions
            .get_mut(competition_id)
            .ok_or_else(|| {
                crate::RembrandtError::Competition(format!(
                    "Competition not found: {}",
                    competition_id
                ))
            })?;

        if competition.status.is_terminal() {
            return Err(crate::RembrandtError::Competition(
                "Cannot cancel a completed competition".to_string(),
            ));
        }

        // Stop all agents
        for competitor in &competition.competitors {
            if let Some(_session) = registry.get_session(&competitor.agent_id) {
                let _ = registry.update_status(&competitor.agent_id, AgentStatus::Stopped);
                // TODO: Actually kill the agent process
            }
        }

        competition.status = CompetitionStatus::Cancelled;
        competition.completed_at = Some(Utc::now());

        Ok(())
    }

    /// Cleanup after a competition (remove losing worktrees)
    pub fn cleanup_competition(&mut self, competition_id: &str) -> Result<()> {
        let competition = self.competitions.get(competition_id).ok_or_else(|| {
            crate::RembrandtError::Competition(format!(
                "Competition not found: {}",
                competition_id
            ))
        })?;

        let winner_id = competition.winner.as_ref();

        for competitor in &competition.competitors {
            // Skip the winner
            if winner_id == Some(&competitor.agent_id) {
                continue;
            }

            // Remove the worktree
            if let Err(e) = self.worktree_manager.remove_worktree(&competitor.agent_id) {
                eprintln!(
                    "Warning: Failed to remove worktree for {}: {}",
                    competitor.agent_id, e
                );
            }
        }

        Ok(())
    }

    /// Mark competition as completed after successful merge
    pub fn complete_competition(&mut self, competition_id: &str) -> Result<()> {
        let competition = self
            .competitions
            .get_mut(competition_id)
            .ok_or_else(|| {
                crate::RembrandtError::Competition(format!(
                    "Competition not found: {}",
                    competition_id
                ))
            })?;

        if let Some(winner_id) = competition.winner.clone() {
            competition.status = CompetitionStatus::Completed { winner_id };
            competition.completed_at = Some(Utc::now());
        }

        Ok(())
    }
}
