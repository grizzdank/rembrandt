use anyhow::Result;
use clap::Parser;
use rembrandt::agent::AgentType;
use rembrandt::cli::{Cli, Commands};
use rembrandt::daemon::session::PtySession;
use rembrandt::worktree::WorktreeManager;
use std::io::Read;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rembrandt=info".parse()?),
        )
        .init();

    let cli = Cli::parse();
    let repo_path = cli.repo.unwrap_or_else(|| PathBuf::from("."));

    match cli.command {
        Commands::Init => {
            println!("Initializing Rembrandt...");
            let manager = WorktreeManager::new(&repo_path)?;
            println!("Created {}", manager.rembrandt_dir().display());
        }

        Commands::Spawn { agent, task, branch } => {
            let wt_manager = WorktreeManager::new(&repo_path)?;

            // Generate a short agent ID: agent-type + short random suffix
            let suffix: String = (0..4)
                .map(|_| format!("{:x}", rand::random::<u8>() % 16))
                .collect();
            let agent_id = format!("{}-{}", agent, suffix);

            println!("Spawning {} agent as '{}'...", agent, agent_id);

            // Create worktree
            let worktree = wt_manager.create_worktree(&agent_id, &branch)?;
            println!("  Worktree: {}", worktree.path.display());
            println!("  Branch:   {}", worktree.branch);

            if let Some(task_id) = &task {
                println!("  Task:     {}", task_id);
            }

            // Resolve agent type to command
            let agent_type = AgentType::from_str(&agent);
            let command = agent_type.command();
            let args = agent_type.default_args();

            println!("  Command:  {}", command);
            println!();

            // Spawn the agent in a PTY
            let mut session = PtySession::spawn(
                agent_id.clone(),
                command,
                &args,
                &worktree.path,
                10 * 1024, // 10KB output buffer
            )?;

            println!("Agent spawned with session ID: {}", session.id);
            println!("Press Ctrl+C to detach (agent keeps running in worktree)");
            println!("{}", "─".repeat(60));

            // Simple foreground mode: read output and display it
            // TODO: Full attach/detach with daemon (rembrandt-cml)
            let mut reader = session.try_clone_reader()?;
            let mut buf = [0u8; 1024];

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF - process exited
                        session.poll();
                        println!("\n{}", "─".repeat(60));
                        println!("Agent exited: {:?}", session.status);
                        break;
                    }
                    Ok(n) => {
                        // Print output
                        print!("{}", String::from_utf8_lossy(&buf[..n]));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data available, continue
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => {
                        eprintln!("Read error: {}", e);
                        break;
                    }
                }

                // Check if process exited
                if !session.is_running() {
                    break;
                }
            }
        }

        Commands::Compete {
            prompt,
            agents,
            evaluator,
            model,
            timeout,
            branch,
        } => {
            use rembrandt::agent::AgentType;
            use rembrandt::competition::{EvaluatorStrategy, MetricWeights};

            println!("Starting competition mode...");
            println!("  Prompt: {}", prompt);
            println!("  Agents: {}", agents.join(", "));
            println!("  Evaluator: {}", evaluator);
            println!("  Timeout: {} minutes", timeout);
            println!("  Base branch: {}", branch);
            println!();

            // Parse agent types
            let agent_types: Vec<AgentType> = agents
                .iter()
                .map(|s| match s.as_str() {
                    "claude-code" => AgentType::ClaudeCode,
                    "opencode" => AgentType::OpenCode,
                    "ampcode" => AgentType::AmpCode,
                    "codex" => AgentType::Codex,
                    "aider" => AgentType::Aider,
                    other => AgentType::Custom(other.to_string()),
                })
                .collect();

            // Parse evaluator strategy
            let evaluator_strategy = match evaluator.as_str() {
                "model" => EvaluatorStrategy::Model { model_name: model },
                "human" => EvaluatorStrategy::Human,
                _ => EvaluatorStrategy::Metrics(MetricWeights::default()),
            };

            println!("Competition would start with:");
            println!("  {} agents", agent_types.len());
            println!("  Strategy: {:?}", evaluator_strategy);
            println!();
            println!("(Competition manager not yet wired to agent spawning)");
            // TODO: Actually start competition via CompetitionManager
        }

        Commands::CompeteStatus { id } => {
            println!("Competition status: {}", id);
            println!("  (no active competitions)");
            // TODO: Look up competition and display status
        }

        Commands::CompeteCancel { id } => {
            println!("Cancelling competition: {}", id);
            // TODO: Cancel via CompetitionManager
        }

        Commands::List { verbose } => {
            let manager = WorktreeManager::new(&repo_path)?;
            let worktrees = manager.list_worktrees()?;

            if worktrees.is_empty() {
                println!("No active agent sessions");
            } else {
                println!("Active agent sessions:");
                for wt in &worktrees {
                    println!("  {} → {} ({})", wt.agent_id, wt.branch, wt.path.display());
                }
            }

            if verbose {
                println!("\nIntegrations:");
                let beads = rembrandt::integration::beads::BeadsIntegration::new();
                let porque = rembrandt::integration::porque::PorqueIntegration::new();
                let agent_mail = rembrandt::integration::agent_mail::AgentMailIntegration::new();
                println!(
                    "  beads:      {}",
                    if beads.is_available() { "available" } else { "not found" }
                );
                println!(
                    "  porque:     {}",
                    if porque.is_available() { "available" } else { "not found" }
                );
                println!(
                    "  agent-mail: {}",
                    if agent_mail.is_available() { "connected" } else { "not configured" }
                );
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
            let manager = WorktreeManager::new(&repo_path)?;
            let worktrees = manager.list_worktrees()?;

            if worktrees.is_empty() {
                println!("No worktrees to clean up");
                return Ok(());
            }

            if all {
                println!("Cleaning up all {} worktrees...", worktrees.len());
                for wt in &worktrees {
                    print!("  Removing {}... ", wt.agent_id);
                    match manager.remove_worktree(&wt.agent_id) {
                        Ok(_) => println!("done"),
                        Err(e) => println!("failed: {}", e),
                    }
                }
            } else {
                // TODO: Only remove worktrees with Completed/Stopped status
                // For now, list what would be cleaned (requires agent registry)
                println!("Worktrees that would be cleaned (once registry tracks status):");
                for wt in &worktrees {
                    println!("  {} (status unknown - use --all to force)", wt.agent_id);
                }
            }
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
