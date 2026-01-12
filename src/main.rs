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

        Commands::Spawn { agent, task, branch, r#continue: continue_id, prompt, no_prompt, shell } => {
            let wt_manager = WorktreeManager::new(&repo_path)?;

            // Determine worktree: continue existing or create new
            let (agent_id, worktree_path) = if let Some(existing_id) = continue_id {
                // Find existing worktree
                let worktrees = wt_manager.list_worktrees()?;
                let existing = worktrees.iter().find(|wt| wt.agent_id == existing_id);

                match existing {
                    Some(wt) => {
                        println!("Continuing in existing worktree '{}'...", existing_id);
                        println!("  Worktree: {}", wt.path.display());
                        println!("  Branch:   {}", wt.branch);
                        (existing_id, wt.path.clone())
                    }
                    None => {
                        eprintln!("Error: No worktree found for '{}'", existing_id);
                        eprintln!("Available worktrees:");
                        for wt in worktrees {
                            eprintln!("  {}", wt.agent_id);
                        }
                        std::process::exit(1);
                    }
                }
            } else {
                // Generate a short agent ID: agent-type + short random suffix
                let suffix: String = (0..4)
                    .map(|_| format!("{:x}", rand::random::<u8>() % 16))
                    .collect();
                let agent_name = if shell {
                    "shell"
                } else {
                    agent.as_ref().expect("agent required when --shell not provided")
                };
                let agent_id = format!("{}-{}", agent_name, suffix);

                if shell {
                    println!("Spawning shell as '{}'...", agent_id);
                } else {
                    println!("Spawning {} agent as '{}'...", agent_name, agent_id);
                }

                // Create worktree
                let worktree = wt_manager.create_worktree(&agent_id, &branch)?;
                println!("  Worktree: {}", worktree.path.display());
                println!("  Branch:   {}", worktree.branch);

                (agent_id, worktree.path)
            };

            if let Some(task_id) = &task {
                println!("  Task:     {}", task_id);
            }

            // Get initial prompt
            let initial_prompt: Option<String> = if let Some(p) = prompt {
                Some(p)
            } else if no_prompt {
                None
            } else {
                // Interactive prompt
                print!("Starting task (empty to skip): ");
                std::io::Write::flush(&mut std::io::stdout())?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            };

            // Resolve command and args: shell or agent
            let (command, args) = if shell {
                // Spawn shell from $SHELL or fallback to /bin/sh
                let shell_path = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
                (shell_path, Vec::<String>::new())
            } else {
                // Resolve agent type to command
                let agent_name = agent.as_ref().expect("agent required when --shell not provided");
                let agent_type = AgentType::from_str(agent_name);
                let command = agent_type.command().to_string();
                let args = agent_type.default_args().iter().map(|s| s.to_string()).collect();
                (command, args)
            };

            println!("  Command:  {}", command);
            println!();

            // Spawn the agent in a PTY with current terminal size
            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));

            // Convert Vec<String> to Vec<&str> for PtySession::spawn
            let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

            let mut session = PtySession::spawn(
                agent_id.clone(),
                &command,
                &args_refs,
                &worktree_path,
                10 * 1024, // 10KB output buffer
                Some(rows),
                Some(cols),
            )?;

            println!("Agent spawned with session ID: {}", session.id);
            println!("Press Ctrl+D to detach (agent keeps running in worktree)");
            println!("{}", "─".repeat(60));

            // Send initial prompt if provided (after short delay for agent to start)
            if let Some(ref prompt_text) = initial_prompt {
                std::thread::sleep(std::time::Duration::from_millis(500));
                session.write(prompt_text.as_bytes())?;
                session.write(b"\n")?;
            }

            // Interactive mode: forward stdin to PTY, PTY output to stdout
            use crossterm::{
                event::{self, Event, KeyCode, KeyModifiers},
                terminal::{disable_raw_mode, enable_raw_mode},
            };
            use std::io::Write;

            let mut reader = session.try_clone_reader()?;
            let mut buf = [0u8; 1024];

            // Enable raw mode for keyboard input
            enable_raw_mode()?;

            let result: Result<()> = (|| {
                loop {
                    // Poll for keyboard events (non-blocking)
                    if event::poll(std::time::Duration::from_millis(10))? {
                        if let Event::Key(key) = event::read()? {
                            // Ctrl+D to detach
                            if key.code == KeyCode::Char('d')
                                && key.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                break;
                            }

                            // Forward key to PTY
                            let bytes: Vec<u8> = match key.code {
                                KeyCode::Char(c) => {
                                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                                        // Convert to control character
                                        vec![(c as u8) & 0x1f]
                                    } else {
                                        c.to_string().into_bytes()
                                    }
                                }
                                KeyCode::Enter => vec![b'\r'],
                                KeyCode::Backspace => vec![127],
                                KeyCode::Tab => vec![b'\t'],
                                KeyCode::Esc => vec![27],
                                KeyCode::Up => vec![27, b'[', b'A'],
                                KeyCode::Down => vec![27, b'[', b'B'],
                                KeyCode::Right => vec![27, b'[', b'C'],
                                KeyCode::Left => vec![27, b'[', b'D'],
                                _ => vec![],
                            };

                            if !bytes.is_empty() {
                                session.write(&bytes)?;
                            }
                        }
                    }

                    // Read PTY output (non-blocking via WouldBlock)
                    match reader.read(&mut buf) {
                        Ok(0) => {
                            // EOF - process exited
                            session.poll();
                            break;
                        }
                        Ok(n) => {
                            // Write to stdout
                            std::io::stdout().write_all(&buf[..n])?;
                            std::io::stdout().flush()?;
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // No data available, continue
                        }
                        Err(e) => {
                            return Err(e.into());
                        }
                    }

                    // Check if process exited
                    if !session.is_running() {
                        break;
                    }
                }
                Ok(())
            })();

            // Always restore terminal
            disable_raw_mode()?;

            // Handle result
            result?;

            println!("\n{}", "─".repeat(60));
            if session.is_running() {
                if shell {
                    println!("Detached. Shell still running in {}", worktree_path.display());
                    println!("Resume with: rembrandt spawn --shell -C {}", agent_id);
                } else {
                    println!("Detached. Agent still running in {}", worktree_path.display());
                    println!("Resume with: rembrandt spawn {} -C {}", agent.as_ref().unwrap(), agent_id);
                }
            } else {
                if shell {
                    println!("Shell exited: {:?}", session.status);
                } else {
                    println!("Agent exited: {:?}", session.status);
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

        Commands::Gc { dry_run } => {
            let manager = WorktreeManager::new(&repo_path)?;
            let worktrees = manager.list_worktrees()?;

            if worktrees.is_empty() {
                println!("No worktrees found");
                return Ok(());
            }

            println!("Found {} worktree(s):", worktrees.len());
            let mut to_clean = Vec::new();

            for wt in &worktrees {
                // All worktrees in .rembrandt/agents/ are candidates
                // In TUI mode, sessions are tracked in memory
                // Without daemon, we can't know if they're truly orphaned
                // So we list them all and let user decide
                println!("  {} → {} ({})", wt.agent_id, wt.branch, wt.path.display());
                to_clean.push(wt);
            }

            if dry_run {
                println!("\nDry run - {} worktree(s) would be removed", to_clean.len());
            } else {
                println!("\nCleaning {} worktree(s)...", to_clean.len());
                for wt in to_clean {
                    print!("  Removing {}... ", wt.agent_id);
                    match manager.remove_worktree(&wt.agent_id) {
                        Ok(_) => println!("done"),
                        Err(e) => println!("failed: {}", e),
                    }
                }
            }
        }

        Commands::Dashboard => {
            rembrandt::tui::run(repo_path)?;
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
