//! Evaluator trait and implementations for comparing solutions
//!
//! The evaluation pipeline: Metrics → Model → Human
//! Each layer is optional and builds on the previous.

use crate::competition::{
    CompetitorSolution, EvaluationResult, EvaluatorStrategy, MetricWeights, SolutionRanking,
};
use crate::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::path::Path;

/// Trait for evaluation strategies
#[async_trait]
pub trait Evaluator: Send + Sync {
    /// Evaluate solutions and return rankings with a winner
    async fn evaluate(
        &self,
        prompt: &str,
        solutions: &[&CompetitorSolution],
        repo_path: &Path,
    ) -> Result<EvaluationResult>;

    /// Get the name of this evaluator for logging
    fn name(&self) -> &'static str;
}

/// Metrics-based evaluator using automated scoring
pub struct MetricsEvaluator {
    weights: MetricWeights,
}

impl MetricsEvaluator {
    pub fn new(weights: MetricWeights) -> Self {
        Self { weights }
    }

    /// Calculate score for a single solution
    fn score_solution(&self, solution: &CompetitorSolution, max_time_ms: u64) -> f64 {
        let validation = match &solution.validation {
            Some(v) if v.is_valid() => v,
            _ => return 0.0,
        };

        // Test score: proportion of tests passed
        let test_score = if let (Some(total), Some(failures)) =
            (validation.test_count, validation.test_failures)
        {
            if total > 0 {
                (total - failures) as f64 / total as f64
            } else {
                1.0 // No tests = assume pass
            }
        } else {
            1.0
        };

        // Simplicity score: inverse of lines changed (normalized)
        let simplicity_score = if let Some(diff) = &solution.diff_stats {
            let lines = diff.total_lines().max(1) as f64;
            // Score decreases as lines increase, capped at reasonable range
            1.0 / (1.0 + (lines / 100.0).ln())
        } else {
            0.5
        };

        // Speed score: inverse of time taken (normalized)
        let speed_score = if max_time_ms > 0 {
            1.0 - (validation.validation_time_ms as f64 / max_time_ms as f64)
        } else {
            0.5
        };

        // Weighted combination
        (self.weights.tests * test_score)
            + (self.weights.simplicity * simplicity_score)
            + (self.weights.speed * speed_score)
    }
}

#[async_trait]
impl Evaluator for MetricsEvaluator {
    async fn evaluate(
        &self,
        _prompt: &str,
        solutions: &[&CompetitorSolution],
        _repo_path: &Path,
    ) -> Result<EvaluationResult> {
        if solutions.is_empty() {
            return Err(crate::RembrandtError::Competition(
                "No solutions to evaluate".to_string(),
            ));
        }

        // Find max validation time for normalization
        let max_time_ms = solutions
            .iter()
            .filter_map(|s| s.validation.as_ref())
            .map(|v| v.validation_time_ms)
            .max()
            .unwrap_or(1);

        // Score all solutions
        let mut rankings: Vec<SolutionRanking> = solutions
            .iter()
            .map(|s| {
                let score = self.score_solution(s, max_time_ms);
                SolutionRanking {
                    agent_id: s.agent_id.clone(),
                    rank: 0, // Will be set after sorting
                    score,
                    reasoning: format!(
                        "Score: {:.2} (tests: {:.0}%, simplicity: {:.0}%, speed: {:.0}%)",
                        score,
                        self.weights.tests * 100.0,
                        self.weights.simplicity * 100.0,
                        self.weights.speed * 100.0
                    ),
                }
            })
            .collect();

        // Sort by score descending
        rankings.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Assign ranks
        for (i, ranking) in rankings.iter_mut().enumerate() {
            ranking.rank = i + 1;
        }

        let winner = rankings
            .first()
            .ok_or_else(|| {
                crate::RembrandtError::Competition("No valid rankings produced".to_string())
            })?
            .clone();

        Ok(EvaluationResult {
            winner_id: winner.agent_id.clone(),
            strategy_used: EvaluatorStrategy::Metrics(self.weights.clone()),
            reasoning: format!(
                "Winner: {} with score {:.2}. {}",
                winner.agent_id, winner.score, winner.reasoning
            ),
            rankings,
            evaluated_at: Utc::now(),
        })
    }

    fn name(&self) -> &'static str {
        "metrics"
    }
}

/// Model-based evaluator using an LLM
pub struct ModelEvaluator {
    model_name: String,
}

impl ModelEvaluator {
    pub fn new(model_name: String) -> Self {
        Self { model_name }
    }

    /// Build a comparison prompt for the LLM
    fn build_prompt(&self, task: &str, solutions: &[&CompetitorSolution]) -> String {
        let mut prompt = format!(
            "You are evaluating {} solutions to this coding task:\n\n\
             Task: {}\n\n\
             Compare the solutions and rank them from best to worst.\n\
             Consider: correctness, maintainability, idiomatic style, and elegance.\n\n",
            solutions.len(),
            task
        );

        for (i, solution) in solutions.iter().enumerate() {
            prompt.push_str(&format!(
                "=== Solution {} (Agent: {}) ===\n",
                i + 1,
                solution.agent_id
            ));
            if let Some(diff) = &solution.diff_stats {
                prompt.push_str(&format!(
                    "Files changed: {}, +{} -{}\n\n",
                    diff.files_changed, diff.insertions, diff.deletions
                ));
            }
            // Note: In real implementation, we'd include the actual diff content here
            prompt.push_str("[Diff content would be included here]\n\n");
        }

        prompt.push_str(
            "Respond with JSON: {\"rankings\": [{\"agent_id\": \"...\", \"rank\": 1, \"reasoning\": \"...\"}]}",
        );
        prompt
    }
}

#[async_trait]
impl Evaluator for ModelEvaluator {
    async fn evaluate(
        &self,
        prompt: &str,
        solutions: &[&CompetitorSolution],
        _repo_path: &Path,
    ) -> Result<EvaluationResult> {
        // TODO: Implement actual LLM call via pluggable provider
        // For now, fall back to metrics-based scoring
        let _comparison_prompt = self.build_prompt(prompt, solutions);

        // Placeholder: delegate to metrics evaluator
        let metrics = MetricsEvaluator::new(MetricWeights::default());
        let mut result = metrics.evaluate(prompt, solutions, _repo_path).await?;
        result.strategy_used = EvaluatorStrategy::Model {
            model_name: self.model_name.clone(),
        };
        result.reasoning = format!(
            "[Model evaluation not yet implemented, used metrics fallback] {}",
            result.reasoning
        );
        Ok(result)
    }

    fn name(&self) -> &'static str {
        "model"
    }
}

/// Human evaluator - presents solutions for interactive selection
pub struct HumanEvaluator;

impl HumanEvaluator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HumanEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Evaluator for HumanEvaluator {
    async fn evaluate(
        &self,
        prompt: &str,
        solutions: &[&CompetitorSolution],
        _repo_path: &Path,
    ) -> Result<EvaluationResult> {
        // TODO: Implement TUI for human selection
        // For now, just pick the first valid solution
        let winner = solutions.first().ok_or_else(|| {
            crate::RembrandtError::Competition("No solutions for human review".to_string())
        })?;

        let rankings: Vec<SolutionRanking> = solutions
            .iter()
            .enumerate()
            .map(|(i, s)| SolutionRanking {
                agent_id: s.agent_id.clone(),
                rank: i + 1,
                score: if i == 0 { 1.0 } else { 0.0 },
                reasoning: if i == 0 {
                    "Selected by human".to_string()
                } else {
                    "Not selected".to_string()
                },
            })
            .collect();

        Ok(EvaluationResult {
            winner_id: winner.agent_id.clone(),
            strategy_used: EvaluatorStrategy::Human,
            reasoning: format!(
                "[Human selection TUI not yet implemented] Auto-selected first solution: {}",
                winner.agent_id
            ),
            rankings,
            evaluated_at: Utc::now(),
        })
    }

    fn name(&self) -> &'static str {
        "human"
    }
}

/// Create an evaluator based on strategy
pub fn create_evaluator(strategy: &EvaluatorStrategy) -> Box<dyn Evaluator> {
    match strategy {
        EvaluatorStrategy::Metrics(weights) => Box::new(MetricsEvaluator::new(weights.clone())),
        EvaluatorStrategy::Model { model_name } => {
            Box::new(ModelEvaluator::new(model_name.clone()))
        }
        EvaluatorStrategy::Human => Box::new(HumanEvaluator::new()),
    }
}
