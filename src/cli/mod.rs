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

    /// Launch the TUI dashboard
    Dashboard,

    /// Show status of all integrations
    Status,
}
