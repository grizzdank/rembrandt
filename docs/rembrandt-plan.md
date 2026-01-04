# Rembrandt: Agent Orchestration Layer

> *Like Rembrandt's workshop - multiple apprentices working on different parts of the canvas, unified by the master into a cohesive masterpiece.*

## Vision

A lightweight orchestration layer for coding agents (Claude Code, OpenCode, AmpCode, Codex, etc.) that enables **parallel execution without collision**. Simple, fast, intuitive.

## Core Problem

Running multiple agents simultaneously causes:
1. **File conflicts** - Two agents editing the same file
2. **Semantic conflicts** - Agent A's change breaks Agent B's assumptions
3. **Git conflicts** - Merging concurrent branches

## Existing Solutions (Prior Art)

| Tool | Approach | Gap |
|------|----------|-----|
| [Tmux Orchestrator](https://github.com/absmartly/Tmux-Orchestrator) | Hierarchical tmux sessions | No task/decision integration |
| [Claude Squad](https://dev.to/skeptrune/llm-codegen-go-brrr-parallelization-with-git-worktrees-and-tmux-2gop) | tmux + git worktrees | CLI-only, no zoom in/out |
| [Uzi](https://www.vibesparking.com/en/blog/ai/claude-code/uzi/2025-08-23-uzi-parallel-ai-coders-git-worktrees-tmux/) | Parallel agents + checkpoint merging | No Beads/Porque context |
| Cursor 2.0 | 8 concurrent agents, worktrees | Cursor-locked, not multi-agent |

**Rembrandt's differentiation**: Integration layer (Beads tasks, Porque decisions) + zoom in/out UI paradigm + ACP for heterogeneous agents + hub coordination model

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    REMBRANDT (TUI → Tauri)                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │  Symphony   │  │  Terminal   │  │    Context Panel        │ │
│  │  View       │◄─►│  Embed     │◄─►│  (Beads + Porque)      │ │
│  │  (Zoom Out) │  │  (Zoom In)  │  │                         │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                    ORCHESTRATION CORE (Rust)                    │
│  ┌──────────────┬───────────────┬──────────────┬─────────────┐ │
│  │ Agent        │ Worktree      │ Task         │ Merge       │ │
│  │ Registry     │ Manager       │ Router       │ Engine      │ │
│  └──────────────┴───────────────┴──────────────┴─────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                    INTEGRATION LAYER                            │
│  ┌──────────────┬───────────────┬──────────────┬─────────────┐ │
│  │ Hub          │ Beads         │ Porque       │ Git         │ │
│  │ Coordinator  │ Integration   │ Integration  │ Operations  │ │
│  └──────────────┴───────────────┴──────────────┴─────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                    AGENT ADAPTERS                               │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌─────────────┐  │
│  │ Claude     │ │ OpenCode   │ │ AmpCode    │ │ Codex       │  │
│  │ Code       │ │            │ │            │ │             │  │
│  └────────────┘ └────────────┘ └────────────┘ └─────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Isolation Strategy: Git Worktrees + Hub Coordination

### Physical Isolation (Worktrees)
Each agent gets its own worktree branched from main:
```
project/
├── .git/                    # Shared git database
├── main/                    # Main worktree (human workspace)
├── .rembrandt/
│   ├── agents/
│   │   ├── agent-1/         # Worktree for agent 1
│   │   ├── agent-2/         # Worktree for agent 2
│   │   └── agent-3/         # Worktree for agent 3
│   └── state.db             # SQLite: agent status, file claims
```

### Coordination Layer (Hub Model)
Rembrandt acts as **Air Traffic Controller (ATC)**:
- All agent coordination flows through Rembrandt
- Agents communicate with Rembrandt, not with each other
- SQLite `state.db` holds shared state for file claims and agent status
- **Beads**: Task assignment via `bd ready`, dependency tracking
- **Porque**: Decision context, constraint checking via `pq check`

**Why hub over peer-to-peer?**
- Simpler implementation (agents only need one connection)
- Full visibility for human director
- Sufficient for <10 agents
- P2P deferred until revenue justifies complexity

### Merge Pipeline

When agent completes work:

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

## Key Features

### 1. Symphony View (Zoom Out)
Dashboard showing all active agents:
- Agent name, status, current task
- File claims (visual overlap detection)
- Task dependency visualization (Beads graph)

### 2. Terminal Embed (Zoom In)
Full terminal access to any agent:
- Option A: libghostty embedding (macOS native)
- Option B: xterm.js + PTY bridge (cross-platform)
- Seamless switch between agents
- Session history preserved

### 3. Task Router
Intelligent task distribution:
```
User: "Implement auth system with tests"
      ↓
Rembrandt decomposes (via Beads):
  Agent 1: Auth middleware (backend specialist)
  Agent 2: Auth UI components (frontend specialist)
  Agent 3: Auth tests (test specialist)
      ↓
Creates Beads issues with dependencies
Assigns to agents based on capability tags
```

### 4. Context Bridge
Unified context available to all agents:
- **Beads**: Current task, blockers, related issues
- **Porque**: Relevant architectural decisions

## Protocol Stack

| Layer | Protocol | Purpose |
|-------|----------|---------|
| **Agent Control** | [ACP (Zed)](https://agentclientprotocol.com/) | Spawn agents, send prompts, receive outputs |
| **Task Tracking** | Beads CLI | Go binary - `bd ready`, `bd sync` |
| **Decision Context** | Porque CLI | Rust binary - `pq context`, `pq check` |

**ACP + Claude Code Status:**
- Claude Code doesn't natively support ACP yet ([feature request](https://github.com/anthropics/claude-code/issues/6686))
- [Zed built an adapter](https://zed.dev/blog/claude-code-via-acp) wrapping Claude Agent SDK → ACP (Apache licensed)
- Strategy: Use ACP adapter for Claude Code, or Claude Agent SDK directly + ACP for others

Note: Beads and Porque are standalone CLIs that agents invoke directly.

## Tech Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Core | Rust | Performance, safety, learning goal |
| MVP UI | ratatui (TUI) | Fast iteration, validate core mechanics |
| Full GUI | Tauri v2 | Native feel, small binary, Rust backend |
| Terminal | libghostty (macOS/Linux), xterm.js (Windows fallback) | Rich terminal emulation |
| State | SQLite | Simple, embedded, works with Beads pattern |
| Agent Control | ACP | Standard protocol for agent spawning/management |
| Git | git2-rs | Native Rust bindings, worktree support |

## MVP Scope (Phase 1)

Focus: **Parallel execution without collision**

1. **Agent Registry**
   - Register available agents (Claude Code, OpenCode, etc.)
   - Track capabilities, status, current assignment

2. **Worktree Manager**
   - Create/destroy worktrees per agent session
   - Handle branch naming, cleanup

3. **Hub Coordinator**
   - File claim tracking via SQLite
   - Agent-to-Rembrandt communication

4. **Simple TUI** (before full Tauri GUI)
   - List active agents
   - Spawn new agent in worktree
   - View agent output streams
   - Manual task assignment

### MVP Commands
```bash
rembrandt init                    # Initialize in project
rembrandt spawn claude-code       # Spawn agent in new worktree
rembrandt spawn opencode          # Spawn another agent
rembrandt list                    # Show active agents + status
rembrandt attach agent-1          # Zoom into agent terminal
rembrandt broadcast "focus on X"  # Message all agents
rembrandt merge agent-1           # Merge agent's work to main
rembrandt cleanup                 # Remove completed worktrees
```

## Phase 2: Intelligence Layer

- Auto task decomposition (supervisor agent)
- Capability-based routing
- Conflict prediction (ML model on file patterns)
- Porque integration for constraint checking

## Phase 3: Full GUI

- Tauri app with Symphony View
- libghostty terminal embedding
- Drag-and-drop task assignment
- Real-time agent activity visualization
- Decision audit trail

## Resolved Decisions

1. **Agent Adapter Interface**: Use ACP (Agent Client Protocol)
   - Zed's standard for editor-to-agent communication
   - For non-ACP agents: thin ACP wrapper (PTY + output parsing)

2. **MVP approach**: TUI first (ratatui), then Tauri GUI

3. **Coordination model**: Hub (Rembrandt as ATC), not peer-to-peer
   - Simpler implementation, sufficient for <10 agents
   - P2P (Agent Mail) deferred until revenue justifies scale
   - See: `pq show PQ-gve`

4. **Merge strategy**: Continuous with layered validation
   - `git merge --no-commit` keeps main pristine
   - Type check → tests → human (fail-fast)
   - See: `pq show PQ-nim`

5. **Beads/Rembrandt boundary**: Clear separation
   - Beads = task graph (WHAT)
   - Rembrandt = execution engine (WHO/WHERE/HOW)
   - See: `pq show PQ-bfj`

6. **Human review**: Conflicts only
   - Clean merges auto-proceed
   - Worktrees preserved on failure for debugging

7. **Worktree lifecycle**: Cleanup on success, preserve on failure

## Open Questions

1. **ACP Adoption**: Which agents currently support ACP? May need wrappers for Claude Code, OpenCode initially

## References

- [Agent Client Protocol (ACP)](https://agentclientprotocol.com/) - Zed's standard for agent control (like LSP for agents)
- [Beads](https://github.com/steveyegge/beads) - Git-backed issue tracking
- [Porque](~/Projects/porque) - ADR context system
- [libghostty](https://mitchellh.com/writing/libghostty-is-coming) - Terminal embedding (cross-platform, C API coming)
- [Tauri v2](https://tauri.app/) - Cross-platform GUI framework
- [ratatui](https://ratatui.rs/) - Rust TUI framework
- [Linux Foundation AAIF](https://www.linuxfoundation.org/press/linux-foundation-announces-the-formation-of-the-agentic-ai-foundation) - MCP, goose, AGENTS.md foundation

## Next Steps

1. ~~Create project structure at `~/Projects/rembrandt`~~ Done
2. Implement core Agent Registry + Worktree Manager in Rust
3. Build minimal TUI with ratatui
4. Add hub coordinator with SQLite state
5. Test with 2 Claude Code instances on a real task
