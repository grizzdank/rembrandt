use anyhow::Result;
use clap::Parser;
use rembrandt::cli::{Cli, Commands};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rembrandt=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            println!("Initializing Rembrandt...");
            // TODO: Create .rembrandt directory, state.db
            println!("Created .rembrandt/ directory");
        }

        Commands::Spawn { agent, task, branch } => {
            println!("Spawning {} agent from branch '{}'...", agent, branch);
            if let Some(task_id) = task {
                println!("Assigned to task: {}", task_id);
            }
            // TODO: Create worktree, spawn agent process
        }

        Commands::List { verbose } => {
            println!("Active agent sessions:");
            println!("  (none)");
            // TODO: List from registry
            if verbose {
                println!("\nIntegrations:");
                println!("  beads: checking...");
                println!("  porque: checking...");
                println!("  agent-mail: not configured");
            }
        }

        Commands::Attach { agent } => {
            println!("Attaching to agent {}...", agent);
            // TODO: Attach to agent PTY
        }

        Commands::Broadcast { message, to } => {
            if let Some(target) = to {
                println!("Sending to {}: {}", target, message);
            } else {
                println!("Broadcasting: {}", message);
            }
            // TODO: Send via Agent Mail
        }

        Commands::Merge { agent, no_check } => {
            println!("Merging work from agent {}...", agent);
            if !no_check {
                println!("Running pq check...");
            }
            // TODO: Merge worktree branch
        }

        Commands::Stop { agent } => {
            println!("Stopping agent {}...", agent);
            // TODO: Stop agent process
        }

        Commands::Cleanup { all } => {
            if all {
                println!("Cleaning up all worktrees...");
            } else {
                println!("Cleaning up completed worktrees...");
            }
            // TODO: Remove worktrees
        }

        Commands::Dashboard => {
            println!("Launching TUI dashboard...");
            // TODO: Launch ratatui TUI
        }

        Commands::Status => {
            println!("Rembrandt Status");
            println!("================");
            println!();
            println!("Integrations:");

            // Check beads
            let beads = rembrandt::integration::beads::BeadsIntegration::new();
            println!(
                "  beads:      {}",
                if beads.is_available() { "available" } else { "not found" }
            );

            // Check porque
            let porque = rembrandt::integration::porque::PorqueIntegration::new();
            println!(
                "  porque:     {}",
                if porque.is_available() { "available" } else { "not found" }
            );

            // Check agent-mail
            let agent_mail = rembrandt::integration::agent_mail::AgentMailIntegration::new();
            println!(
                "  agent-mail: {}",
                if agent_mail.is_available() { "connected" } else { "not configured" }
            );
        }
    }

    Ok(())
}

use rembrandt::integration::Integration;
