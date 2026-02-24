//! CLI command definitions

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rembrandt")]
#[command(about = "Orchestration layer for coding agents", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the repository (defaults to current directory)
    #[arg(short, long, global = true)]
    pub repo: Option<PathBuf>,

    /// Use v2 orchestration paths for commands that support it
    #[arg(long, global = true)]
    pub v2: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize Rembrandt in the current repository
    Init,

    /// Spawn a new agent in an isolated worktree
    Spawn {
        /// Agent type (claude-code, opencode, codex, aider)
        agent: String,

        /// Optional task ID from Beads to assign
        #[arg(short, long)]
        task: Option<String>,

        /// Base branch to create worktree from
        #[arg(short, long, default_value = "main")]
        branch: String,

        /// Continue in existing worktree (agent-id from previous session)
        #[arg(short = 'C', long)]
        r#continue: Option<String>,

        /// Initial prompt/task to send to the agent
        #[arg(short, long)]
        prompt: Option<String>,

        /// Skip the interactive prompt for starting task
        #[arg(long)]
        no_prompt: bool,
    },

    /// Run agents in competition mode on the same task
    Compete {
        /// The prompt/task for all agents to work on
        prompt: String,

        /// Comma-separated list of agent types (e.g., claude-code,opencode,codex)
        #[arg(short, long, value_delimiter = ',')]
        agents: Vec<String>,

        /// Evaluator strategy: metrics, model, human
        #[arg(short, long, default_value = "metrics")]
        evaluator: String,

        /// Model name for model evaluator
        #[arg(long, default_value = "claude-3-5-sonnet")]
        model: String,

        /// Timeout in minutes for agent completion
        #[arg(short, long, default_value = "30")]
        timeout: u64,

        /// Base branch to create worktrees from
        #[arg(short, long, default_value = "main")]
        branch: String,
    },

    /// Show status of a competition
    CompeteStatus {
        /// Competition ID (or "latest" for most recent)
        #[arg(default_value = "latest")]
        id: String,
    },

    /// Cancel a running competition
    CompeteCancel {
        /// Competition ID
        id: String,
    },

    /// List active agent sessions
    List {
        /// Show detailed output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Attach to an agent's terminal (zoom in)
    Attach {
        /// Agent session ID or index
        agent: String,
    },

    /// Send a message to agents
    Broadcast {
        /// Message to send
        message: String,

        /// Send only to specific agent
        #[arg(short, long)]
        to: Option<String>,
    },

    /// Merge an agent's work back to main
    Merge {
        /// Agent session ID
        agent: String,

        /// Skip decision check (pq check)
        #[arg(long)]
        no_check: bool,
    },

    /// Stop an agent session
    Stop {
        /// Agent session ID
        agent: String,
    },

    /// Clean up completed agent worktrees
    Cleanup {
        /// Remove all worktrees (including active)
        #[arg(long)]
        all: bool,
    },

    /// Garbage collect orphaned worktrees (no active session)
    Gc {
        /// Dry run - show what would be cleaned without deleting
        #[arg(long)]
        dry_run: bool,
    },

    /// Launch the TUI dashboard
    Dashboard,

    /// Show status of all integrations
    Status,
}
