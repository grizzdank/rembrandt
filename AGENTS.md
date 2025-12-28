# Agent Instructions for Rembrandt

## Project Overview

Rembrandt is an orchestration layer for coding agents. It manages parallel execution of multiple AI coding agents (Claude Code, OpenCode, Codex, etc.) on the same codebase without conflicts.

## Key Concepts

- **Git Worktrees**: Each agent runs in an isolated worktree with its own branch
- **Agent Registry**: Tracks active agent sessions and their status
- **Integrations**: Connects with Beads (tasks), Porque (decisions), Agent Mail (messaging)
- **Zoom In/Out**: UI paradigm - symphony view (all agents) vs focus view (single agent)

## Project Structure

```
src/
├── lib.rs              # Library entry, error types
├── main.rs             # CLI entry point
├── agent/              # Agent registry and session management
│   ├── mod.rs          # Types: AgentType, AgentStatus, AgentSession
│   └── registry.rs     # AgentRegistry implementation
├── worktree/           # Git worktree management
│   └── mod.rs          # WorktreeManager
├── tui/                # Terminal UI
│   ├── mod.rs          # Layout helpers
│   └── app.rs          # App state, view modes
├── integration/        # External tool integrations
│   ├── mod.rs          # Integration trait
│   ├── beads.rs        # Beads task tracking
│   ├── porque.rs       # Porque ADR context
│   └── agent_mail.rs   # MCP Agent Mail
└── cli/                # CLI command definitions
    └── mod.rs          # Clap commands
```

## Current State

This is an MVP scaffold. Key areas needing implementation:

1. **Agent spawning** - Actually spawn agent processes in worktrees
2. **PTY management** - Attach/detach from agent terminals
3. **TUI dashboard** - Full ratatui interface
4. **Agent Mail MCP** - Real MCP client integration

## Development Workflow

1. Check `rembrandt status` to see integration availability
2. Use Beads for task tracking: `bd ready`, `bd update`, `bd sync`
3. Keep changes focused - one feature at a time
4. Test with `cargo test` and `cargo run -- <command>`

## Landing the Plane

Before ending a session:
1. Ensure all changes compile: `cargo build`
2. Update any Beads issues: `bd update <id> --status <status>`
3. Commit changes with clear message
4. Sync: `bd sync`
5. Push to remote

## Key Dependencies

- `ratatui` - TUI framework
- `git2` - Git operations
- `clap` - CLI parsing
- `tokio` - Async runtime
- `portable-pty` - PTY for terminal embedding
