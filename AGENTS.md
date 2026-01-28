# Agent Instructions for Rembrandt

> *Like Rembrandt's workshop - multiple apprentices working on different parts of the canvas, unified by the master into a cohesive masterpiece.*

## Project Overview

Rembrandt is an orchestration layer for coding agents. It manages parallel execution of multiple AI coding agents (Claude Code, OpenCode, Codex, etc.) on the same codebase without conflicts.

**Differentiation from prior art** (Claude Squad, Uzi, Cursor 2.0):

- Integration layer with Beads (tasks), Porque (decisions), Agent Mail (messaging)
- Zoom in/out UI paradigm (symphony view ↔ single agent focus)
- ACP protocol for heterogeneous agent support

## Core Problem Solved

Running multiple agents simultaneously causes:

1. **File conflicts** - Two agents editing the same file
2. **Semantic conflicts** - Agent A's change breaks Agent B's assumptions
3. **Git conflicts** - Merging concurrent branches

**Solution**: Git worktrees for physical isolation + hub coordination (Rembrandt as ATC).

## Key Concepts

- **Git Worktrees**: Each agent runs in an isolated worktree with its own branch
- **Agent Registry**: Tracks active agent sessions and their status
- **Integrations**: Connects with Beads-rust (tasks), Porque (decisions), Agent Mail (messaging)
- **Zoom In/Out**: UI paradigm - symphony view (all agents) vs focus view (single agent)
- **ACP**: Agent Client Protocol - like LSP but for spawning/managing agents

## Project Structure

```
src/                    # Rust CLI/library
├── lib.rs              # Library entry, error types
├── main.rs             # CLI entry point
├── agent/              # Agent registry and session management
│   ├── mod.rs          # Types: AgentType, AgentStatus, AgentSession
│   └── registry.rs     # AgentRegistry implementation
├── daemon/             # PTY session management
│   ├── session.rs      # PtySession - PTY wrapper
│   ├── manager.rs      # SessionManager - lifecycle
│   └── buffer.rs       # Ring buffer for late-attach
├── competition/        # Competition mode (parallel eval, pick best)
│   ├── manager.rs      # CompetitionManager - lifecycle orchestration
│   ├── evaluator.rs    # Evaluator trait + Metrics/Model/Human implementations
│   └── validator.rs    # Solution validation (type check, tests)
├── worktree/           # Git worktree management
│   └── mod.rs          # WorktreeManager
├── tui/                # Terminal UI (preserved on tui-ratatui-backup branch)
├── integration/        # External tool integrations
│   ├── mod.rs          # Integration trait
│   ├── beads.rs        # Beads task tracking
│   ├── porque.rs       # Porque ADR context
│   └── agent_mail.rs   # MCP Agent Mail
└── cli/                # CLI command definitions
    └── mod.rs          # Clap commands

gui/                    # Tauri desktop app (NEW - replacing TUI)
├── src/                # Svelte frontend
│   ├── lib/            # Components (Terminal, Dashboard, AgentList)
│   └── App.svelte      # Main app
├── src-tauri/          # Tauri Rust backend
│   └── src/            # PTY session, manager, commands
└── package.json        # Frontend dependencies (xterm.js, Tauri API)
```

## Current State

**Architecture pivot (2026-01-04):** Migrating from ratatui TUI to **Tauri + Svelte + xterm.js** GUI.

The TUI "attach" mode had fundamental terminal-in-terminal issues (blank screens on attach, display corruption when switching agents). Rather than implementing a full terminal emulator (libghostty), we're pivoting to a GUI approach where each agent gets its own xterm.js terminal widget.

**Completed:**

- Git worktree management for agent isolation
- PTY session management (portable-pty)
- CLI commands: spawn, list, attach, merge, cleanup
- Tauri scaffold with Svelte frontend
- PTY session code migrated to Tauri backend

**In Progress:**

- Svelte frontend with xterm.js terminals
- PTY output streaming via Tauri events

**Preserved:**

- TUI implementation saved on `tui-ratatui-backup` branch
- Can revisit if libghostty becomes stable (expected 1.0 in ~6 months)

## Development Workflow

1. Check `rembrandt status` to see integration availability
2. Use Beads-rust for task tracking: `br ready`, `br update`, `br sync`
3. Keep changes focused - one feature at a time
4. Test with `cargo test` and `cargo run -- <command>`

## Landing the Plane

Before ending a session:

1. Ensure all changes compile: `cargo build`
2. Update any Beads issues: `br update <id> --status <status>`
3. Commit changes with clear message
4. Sync: `br sync`
5. Push to remote

## Key Dependencies

- `ratatui` - TUI framework
- `git2` - Git operations
- `clap` - CLI parsing
- `tokio` - Async runtime
- `portable-pty` - PTY for terminal embedding

## Protocol Stack

| Layer | Protocol | Purpose |
|-------|----------|---------|
| Agent Control | ACP (Zed) | Spawn agents, send prompts, receive outputs |
| Agent-to-Agent | MCP Agent Mail | File reservations, inter-agent messaging |
| Task Tracking | Beads-rust CLI | `br ready`, `br sync` - Rust binary |
| Decision Context | Porque CLI | `pq context`, `pq check` - Rust binary |

**Note**: Beads-rust and Porque are standalone CLIs, not MCP servers. Agent Mail is the MCP layer.

## Resolved Decisions

1. **Agent Adapter Interface**: Use ACP (Agent Client Protocol)
   - Zed's standard for editor-to-agent communication
   - For non-ACP agents: thin wrapper (PTY + output parsing)

2. **MVP approach**: TUI first (ratatui), then Tauri GUI

3. **Isolation strategy**: Git worktrees for physical isolation

4. **Coordination model**: Hub (Rembrandt as ATC), not peer-to-peer
   - Rembrandt manages all coordination: claims, routing, merge decisions
   - Agents communicate through Rembrandt, not directly to each other
   - SQLite `state.db` for shared state (file claims, agent status)
   - Scales to ~10 agents; P2P deferred until revenue justifies API costs

5. **No gossip layer**: Skip Agent Mail P2P for MVP
   - Risk of context pollution for agents
   - Marginal value vs complexity for <10 agents
   - Hub relay is sufficient; revisit if bottleneck emerges

6. **Beads-rust/Rembrandt boundary**: Clear separation
   - Beads-rust = task graph (WHAT needs to be done)
   - Rembrandt = execution engine (WHO does it, WHERE, HOW to merge)

7. **Human review**: Conflicts only
   - Clean merges auto-proceed
   - Human intervention on file conflicts, type check failures, or test failures

8. **Merge timing**: Continuous
   - Merge each agent's work as they complete
   - Respect Beads-rust dependency order

9. **Merge mechanics**: `git merge --no-commit`
   - Only finalize commit if all validation passes
   - Keeps main pristine; failed merges never touch history
   - Worktree preserved on failure for debugging

## Open Questions

1. **ACP Adoption**: Which agents support ACP? May need wrappers initially

## Phase Roadmap

### Phase 1: MVP (Current)

Focus: **Parallel execution without collision**

- Agent Registry + Worktree Manager
- Agent Mail integration (file reservations)
- Simple TUI (list agents, spawn, attach, merge)

### Phase 2: Intelligence Layer

- Auto task decomposition (supervisor agent)
- Capability-based routing
- Conflict prediction
- Porque integration

### Phase 3: Full GUI (IN PROGRESS)

- Tauri + Svelte + xterm.js desktop app
- Symphony view with multiple agent terminals
- Real-time agent activity visualization
- (libghostty option preserved on `tui-ratatui-backup` branch for future native TUI)

## MVP Commands (Target)

```bash
rembrandt init                    # Initialize in project
rembrandt spawn claude-code       # Spawn agent in new worktree
rembrandt list                    # Show active agents + status
rembrandt attach agent-1          # Zoom into agent terminal
rembrandt broadcast "focus on X"  # Message all agents
rembrandt merge agent-1           # Merge agent's work to main
rembrandt cleanup                 # Remove completed worktrees
rembrandt compete "<prompt>" --agents <list>  # Competition mode
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    REMBRANDT (TUI → Tauri)                   │
│  Symphony View ◄─► Terminal Embed ◄─► Context Panel         │
├─────────────────────────────────────────────────────────────┤
│                    ORCHESTRATION CORE (Rust)                 │
│  Agent Registry │ Worktree Manager │ Task Router │ Merge    │
├─────────────────────────────────────────────────────────────┤
│                    INTEGRATION LAYER                         │
│  Agent Mail (MCP) │ Beads │ Porque │ Git Operations         │
├─────────────────────────────────────────────────────────────┤
│                    AGENT ADAPTERS                            │
│  Claude Code │ OpenCode │ AmpCode │ Codex                   │
└─────────────────────────────────────────────────────────────┘
```

## Worktree Layout

```
project/
├── .git/                    # Shared git database
├── main/                    # Main worktree (human workspace)
├── .rembrandt/
│   ├── agents/
│   │   ├── agent-1/         # Worktree for agent 1
│   │   ├── agent-2/         # Worktree for agent 2
│   └── state.db             # SQLite: agent status, assignments
```

## Merge Pipeline

When an agent completes a task, Rembrandt runs this validation pipeline:

```
Agent completes task
         │
         ▼
┌─────────────────────────────────┐
│ 1. PRE-MERGE CHECKS             │
│    - Dependencies satisfied?    │◄── Beads graph
│    - pq check passes?           │◄── Porque constraints
└─────────────────────────────────┘
         │ pass
         ▼
┌─────────────────────────────────┐
│ 2. MERGE ATTEMPT                │
│    - git merge --no-commit      │
│    - Textual conflict? → HUMAN  │
└─────────────────────────────────┘
         │ clean merge
         ▼
┌─────────────────────────────────┐
│ 3. TYPE CHECK                   │
│    - cargo check / tsc / etc    │
│    - Fail? → HUMAN              │
└─────────────────────────────────┘
         │ pass
         ▼
┌─────────────────────────────────┐
│ 4. TEST SUITE                   │
│    - Run tests on merged state  │
│    - Fail? → HUMAN              │
└─────────────────────────────────┘
         │ pass
         ▼
┌─────────────────────────────────┐
│ 5. COMMIT & CLEANUP             │
│    - Finalize merge commit      │
│    - Update Beads status        │
│    - Remove worktree            │
└─────────────────────────────────┘
```

**Human gates:** Textual conflict, type check fail, test fail. Everything else flows automatically.
