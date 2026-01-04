# Competition Mode

Inspired by [Nakazawa's insight](https://cpojer.net/posts/you-are-absolutely-right) that generating multiple solutions reveals approaches you'd miss with a single agent.

## Concept

Instead of agents working on different tasks in parallel (standard Rembrandt mode), **competition mode** has multiple agents work on the **same task independently**. An evaluation pipeline selects the best solution:

```
Agents work in parallel (isolated worktrees)
         ↓
All complete (or timeout)
         ↓
Validate each (type check, tests)
         ↓
Evaluate passing solutions
         ↓
Merge winner only
```

This exploits LLM variance as a feature—different agents take different paths, revealing approaches you'd miss with a single attempt.

## CLI Usage

```bash
# Basic: 3 agents compete, metrics picks winner
rembrandt compete "add dark mode toggle" \
  --agents claude-code,opencode,aider

# With model evaluation for nuanced comparison
rembrandt compete "refactor auth to use JWT" \
  --agents claude-code,codex \
  --evaluator model

# Human review of solutions
rembrandt compete "implement payment flow" \
  --agents claude-code,opencode,ampcode \
  --evaluator human \
  --timeout 45

# Check status
rembrandt compete-status [id]

# Cancel
rembrandt compete-cancel <id>
```

## Evaluator Strategies

| Strategy | Flag | Behavior |
|----------|------|----------|
| Metrics | `--evaluator metrics` | Score by tests passed, code simplicity, completion speed |
| Model | `--evaluator model` | LLM compares diffs, explains trade-offs |
| Human | `--evaluator human` | TUI presents solutions for selection |

### Evaluation Pipeline

The strategies can be composed:

```
Metrics (filter & rank)
    ↓
Model (optional nuanced comparison)
    ↓
Human (optional final review)
    ↓
Merge winner
```

Default: `--evaluator metrics` auto-merges the highest-scoring solution.

### Metrics Scoring

Weighted combination (configurable):
- **Tests**: 50% — proportion of tests passed
- **Simplicity**: 30% — inverse of lines changed (fewer = better)
- **Speed**: 20% — faster completion scores higher

### Model Evaluation

Uses an LLM to compare solution diffs and provide reasoning:
- Considers correctness, maintainability, idiomatic style
- Explains trade-offs between approaches
- Produces ranked list with justifications

### Human Evaluation

Presents a TUI for the human "coach" to:
- See all solutions at a glance with scores
- View diffs for any solution
- Accept recommendation or override with manual selection

## State Machine

```
┌──────────┐    ┌─────────┐    ┌───────────┐    ┌──────────┐    ┌───────────┐
│ SPAWNING │───►│ RUNNING │───►│ EVALUATING│───►│  MERGING │───►│ COMPLETED │
└──────────┘    └─────────┘    └───────────┘    └──────────┘    └───────────┘
     │               │               │               │
     └───────────────┴───────────────┴───────────────┘
                              ↓
                    ┌─────────────────┐
                    │ FAILED/CANCELLED│
                    └─────────────────┘
```

- **Spawning**: Creating worktrees and starting agents
- **Running**: Agents working, tracking completion count
- **Evaluating**: Validating and scoring completed solutions
- **Merging**: Winner goes through standard merge pipeline
- **Completed**: Winner merged, losers cleaned up
- **Failed**: No valid solutions, or merge failed
- **Cancelled**: User cancelled via `compete-cancel`

## Implementation

### Key Types

```rust
// Competition group tracking all competitors
pub struct CompetitionGroup {
    pub id: CompetitionId,
    pub prompt: String,
    pub status: CompetitionStatus,
    pub evaluator_strategy: EvaluatorStrategy,
    pub competitors: Vec<CompetitorSolution>,
    pub winner: Option<String>,
    pub timeout_at: DateTime<Utc>,
}

// Individual competitor's solution
pub struct CompetitorSolution {
    pub agent_id: String,
    pub agent_type: AgentType,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub validation: Option<ValidationResult>,
    pub diff_stats: Option<DiffStats>,
}

// Validation results per solution
pub struct ValidationResult {
    pub type_check_passed: bool,
    pub tests_passed: bool,
    pub test_count: Option<usize>,
    pub test_failures: Option<usize>,
}
```

### Evaluator Trait

```rust
#[async_trait]
pub trait Evaluator: Send + Sync {
    async fn evaluate(
        &self,
        prompt: &str,
        solutions: &[&CompetitorSolution],
        repo_path: &Path,
    ) -> Result<EvaluationResult>;
}
```

Implementations: `MetricsEvaluator`, `ModelEvaluator`, `HumanEvaluator`

## Future Enhancements

- **Consensus detection**: If N agents converge on similar approach, weight that signal
- **Adaptive timeout**: Extend if agents are making progress
- **Learning**: Track which agent types win for which task types
- **Hybrid evaluation**: Combine multiple strategies with configurable weights
