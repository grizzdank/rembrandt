# Rembrandt

> Like Rembrandt's workshop - multiple apprentices working on different parts of the canvas, unified by the master into a cohesive masterpiece.

Orchestration layer for coding agents (Claude Code, OpenCode, AmpCode, Codex, etc.) that enables **parallel execution without collision**.

## Vision

Run multiple AI coding agents simultaneously on the same codebase without them stepping on each other. Zoom out to see the symphony, zoom in to conduct any agent.

## Quick Start

```bash
# Initialize in your project
rembrandt init

# Spawn agents on different tasks
rembrandt spawn claude-code --task BD-001
rembrandt spawn opencode --task BD-002

# See all agents working
rembrandt dashboard

# Zoom into a specific agent
rembrandt attach agent-1

# Merge completed work
rembrandt merge agent-1
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
| `rembrandt list` | List active agent sessions |
| `rembrandt attach <id>` | Zoom into agent terminal |
| `rembrandt broadcast <msg>` | Message all agents |
| `rembrandt merge <id>` | Merge agent's work to main |
| `rembrandt cleanup` | Remove completed worktrees |
| `rembrandt dashboard` | Launch TUI |
| `rembrandt status` | Show integration status |

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

- [Design Plan](../research-plans/rembrandt-plan.md)
- [Agent Client Protocol (ACP)](https://agentclientprotocol.com/)
