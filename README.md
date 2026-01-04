# Rembrandt

> Like Rembrandt's workshop - multiple apprentices working on different parts of the canvas, unified by the master into a cohesive masterpiece.

Orchestration layer for coding agents (Claude Code, OpenCode, AmpCode, Codex, etc.) that enables **parallel execution without collision**.

## Vision

Run multiple AI coding agents simultaneously on the same codebase without them stepping on each other. Zoom out to see the symphony, zoom in to conduct any agent.

## Quick Start

```bash
# Initialize in your project
rembrandt init

# Spawn an agent (interactive - prompts for starting task)
rembrandt spawn claude

# Spawn with an initial prompt
rembrandt spawn claude --prompt "implement the login form"

# Spawn from a specific branch
rembrandt spawn opencode --branch feature/auth

# Resume work in an existing worktree
rembrandt spawn claude --continue claude-a1b2

# Launch the TUI dashboard (Symphony/Solo views)
rembrandt dashboard

# Clean up orphaned worktrees
rembrandt gc
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     REMBRANDT                               │
│  ┌───────────────┐  ┌─────────────┐  ┌──────────────────┐  │
│  │ Symphony View │  │ Focus View  │  │ Context Panel    │  │
│  │ (Zoom Out)    │◄─►│ (Zoom In)  │◄─►│ (Beads+Porque)  │  │
│  └───────────────┘  └─────────────┘  └──────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│  Agent Registry │ Worktree Manager │ Task Router           │
├─────────────────────────────────────────────────────────────┤
│  Agent Mail │ Beads Integration │ Porque Integration       │
├─────────────────────────────────────────────────────────────┤
│  Claude Code │ OpenCode │ AmpCode │ Codex │ Aider          │
└─────────────────────────────────────────────────────────────┘
```

## Isolation Strategy

Each agent gets an isolated git worktree:

```
project/
├── .git/                    # Shared git database
├── main/                    # Human workspace
├── .rembrandt/
│   ├── agents/
│   │   ├── agent-1/         # Agent 1's worktree
│   │   └── agent-2/         # Agent 2's worktree
│   └── state.db             # Session state
```

## Integrations

- **[Beads](https://github.com/steveyegge/beads)** - Task tracking (`bd ready`, `bd sync`)
- **[Porque](https://github.com/davegraham/porque)** - ADR context (`pq context`, `pq check`)
- **[Agent Mail](https://github.com/Dicklesworthstone/mcp_agent_mail)** - Inter-agent communication

## Commands

| Command | Description |
|---------|-------------|
| `rembrandt init` | Initialize in current repository |
| `rembrandt spawn <agent>` | Spawn agent in new worktree |
| `rembrandt dashboard` | Launch TUI (Symphony/Solo views) |
| `rembrandt list` | List active agent sessions |
| `rembrandt attach <id>` | Zoom into agent terminal |
| `rembrandt broadcast <msg>` | Message all agents |
| `rembrandt merge <id>` | Merge agent's work to main |
| `rembrandt stop <id>` | Stop an agent session |
| `rembrandt cleanup` | Remove completed worktrees |
| `rembrandt gc` | Garbage collect orphaned worktrees |
| `rembrandt status` | Show integration status |

### Spawn Options

| Flag | Description |
|------|-------------|
| `-p, --prompt <TEXT>` | Initial task to send to the agent |
| `-C, --continue <ID>` | Resume in existing worktree |
| `-t, --task <ID>` | Beads task ID to assign |
| `-b, --branch <NAME>` | Base branch to fork from (default: main) |
| `--no-prompt` | Skip interactive prompt |

### TUI Keybindings

| Key | Symphony View | Solo View |
|-----|---------------|-----------|
| `j/k` | Navigate sessions | - |
| `Enter` | Zoom into session | - |
| `Esc` | - | Return to Symphony |
| `s` | Spawn new agent | - |
| `n` | Nudge selected | Nudge agent |
| `K` | Kill (with confirm) | Kill (with confirm) |
| `c` | Cleanup completed | - |
| `q` | Quit | - |

## Development

```bash
# Build
cargo build

# Run
cargo run -- status

# Test
cargo test
```

## See Also

- [MVP Specification](docs/MVP.md)
- [Architecture Decision (PTY + ACP Hybrid)](.pq/decisions/PQ-kwg.md)
- [Agent Client Protocol (ACP)](https://agentclientprotocol.com/)
