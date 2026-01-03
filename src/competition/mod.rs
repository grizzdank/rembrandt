//! Competition mode for parallel agent evaluation
//!
//! Multiple agents work on the same task independently, with an evaluation
//! pipeline selecting the best solution for merging.

mod evaluator;
mod manager;
mod validator;

pub use evaluator::*;
pub use manager::*;
pub use validator::*;

use crate::agent::AgentType;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique identifier for a competition
pub type CompetitionId = String;

/// Generate a unique competition ID
pub fn generate_competition_id() -> CompetitionId {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("comp-{:x}", timestamp)
}

/// Status of a competition (state machine)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompetitionStatus {
    /// Agents are being spawned
    Spawning,
    /// All agents running, waiting for completion
    Running {
        completed: usize,
        total: usize,
    },
    /// All agents done (or timeout), evaluation in progress
    Evaluating,
    /// Winner selected, merge in progress
    Merging,
    /// Competition finished successfully
    Completed {
        winner_id: String,
    },
    /// Competition failed (no valid solutions, timeout with none complete, etc.)
    Failed(String),
    /// Competition was cancelled by user
    Cancelled,
}

impl CompetitionStatus {
    /// Check if the competition is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            CompetitionStatus::Completed { .. }
                | CompetitionStatus::Failed(_)
                | CompetitionStatus::Cancelled
        )
    }
}

/// Git diff statistics for a solution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub files_added: Vec<PathBuf>,
    pub files_modified: Vec<PathBuf>,
    pub files_deleted: Vec<PathBuf>,
}

impl DiffStats {
    /// Total lines changed (insertions + deletions)
    pub fn total_lines(&self) -> usize {
        self.insertions + self.deletions
    }
}

/// Result of validating a single solution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub agent_id: String,
    pub type_check_passed: bool,
    pub type_check_output: Option<String>,
    pub tests_passed: bool,
    pub tests_output: Option<String>,
    pub test_count: Option<usize>,
    pub test_failures: Option<usize>,
    pub validation_time_ms: u64,
    pub error_message: Option<String>,
}

impl ValidationResult {
    /// Check if the solution passed all validation checks
    pub fn is_valid(&self) -> bool {
        self.type_check_passed && self.tests_passed
    }
}

/// A solution submitted by a competing agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorSolution {
    pub agent_id: String,
    pub agent_type: AgentType,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub completed_at: Option<DateTime<Utc>>,
    pub validation: Option<ValidationResult>,
    pub diff_stats: Option<DiffStats>,
}

impl CompetitorSolution {
    /// Check if the solution is complete and validated
    pub fn is_validated(&self) -> bool {
        self.validation.is_some()
    }

    /// Check if the solution passed validation
    pub fn is_valid(&self) -> bool {
        self.validation.as_ref().map_or(false, |v| v.is_valid())
    }
}

/// Strategy for selecting the winning solution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EvaluatorStrategy {
    /// Automated metrics comparison (test coverage, complexity, lines changed)
    Metrics(MetricWeights),
    /// Use an LLM to compare solutions and pick best
    Model {
        /// Model identifier (provider-specific)
        model_name: String,
    },
    /// Present solutions for human selection via TUI
    Human,
}

impl Default for EvaluatorStrategy {
    fn default() -> Self {
        EvaluatorStrategy::Metrics(MetricWeights::default())
    }
}

/// Weights for metrics-based evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricWeights {
    /// Weight for test pass rate (0.0-1.0)
    pub tests: f64,
    /// Weight for fewer lines changed (simpler solution)
    pub simplicity: f64,
    /// Weight for faster completion time
    pub speed: f64,
}

impl Default for MetricWeights {
    fn default() -> Self {
        Self {
            tests: 0.5,
            simplicity: 0.3,
            speed: 0.2,
        }
    }
}

/// Ranking of a single solution after evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionRanking {
    pub agent_id: String,
    pub rank: usize,
    pub score: f64,
    pub reasoning: String,
}

/// Result of evaluating all solutions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub winner_id: String,
    pub strategy_used: EvaluatorStrategy,
    pub reasoning: String,
    pub rankings: Vec<SolutionRanking>,
    pub evaluated_at: DateTime<Utc>,
}

/// A competition group tracking multiple agents on the same task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionGroup {
    pub id: CompetitionId,
    pub prompt: String,
    pub status: CompetitionStatus,
    pub evaluator_strategy: EvaluatorStrategy,
    pub competitors: Vec<CompetitorSolution>,
    pub winner: Option<String>,
    pub started_at: DateTime<Utc>,
    pub timeout_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub evaluation_result: Option<EvaluationResult>,
}

impl CompetitionGroup {
    /// Create a new competition group
    pub fn new(
        prompt: String,
        evaluator_strategy: EvaluatorStrategy,
        timeout_minutes: u64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: generate_competition_id(),
            prompt,
            status: CompetitionStatus::Spawning,
            evaluator_strategy,
            competitors: Vec::new(),
            winner: None,
            started_at: now,
            timeout_at: now + Duration::minutes(timeout_minutes as i64),
            completed_at: None,
            evaluation_result: None,
        }
    }

    /// Check if the competition has timed out
    pub fn is_timed_out(&self) -> bool {
        Utc::now() >= self.timeout_at
    }

    /// Get count of completed competitors
    pub fn completed_count(&self) -> usize {
        self.competitors
            .iter()
            .filter(|c| c.completed_at.is_some())
            .count()
    }

    /// Get valid (passed validation) solutions
    pub fn valid_solutions(&self) -> Vec<&CompetitorSolution> {
        self.competitors.iter().filter(|c| c.is_valid()).collect()
    }
}
