# Rembrandt MVP Specification

**Last updated:** 2026-01-03
**Status:** Draft based on Q&A refinement

## Vision

Rembrandt is an orchestration layer for coding agents - enabling parallel execution without collision on the same codebase.

## Primary Use Cases (Priority Order)

1. **Solo dev, multiple agents** - Spawn 2-5 agents on different tasks in the same repo
2. **Competitive evaluation** - Same task to multiple agents, compare outputs/approaches
3. **Background workers** - Spawn agents and walk away, check results later

## Core Architecture

| Layer | Responsibility | Protocol |
|-------|----------------|----------|
| **Rembrandt Daemon** | Lifecycle, monitoring, coordination | PTY for process control |
| **Agents** | Model harness (Claude Code, OpenCode, etc.) | ACP for conversation |
| **TUI** | Visibility, control | Connects to daemon |

### Three Interaction Modes

1. **Overview** - Dashboard showing all agents with status, task, preview
2. **Zoom** - Fullscreen single agent (apprentice mode), direct PTY interaction
3. **Broadcast** - Send instructions to all/subset of agents

## MVP Features

### Must Have (Daily Use Bar)

- [ ] **Spawn agents** with task assignment
- [ ] **Daemon** keeps agents alive independent of TUI
- [ ] **Monitoring dashboard** - status, current task, output preview
- [ ] **Error/blocker alerts** - agent exits, crashes, or explicitly asks for help
- [ ] **Zoom/attach** - fullscreen direct interaction with one agent
- [ ] **Persisted logs** - output written to file, survives cleanup
- [ ] **Beads integration** - agents work within scope (epic or natural language)

### Should Have (Near-term)

- [ ] **Conflict handling** - queue then redirect (wait for file, timeout → new task)
- [ ] **PR + stop mode** - agent creates PR, session ends, async handoff
- [ ] **Broadcast** - send instructions to multiple agents
- [ ] **Diff at end** - compare agent outputs for competitive eval

### Could Have (Future)

- [ ] **Commit + next** - automatic task chaining from Beads
- [ ] **Metrics summary** - time, tokens, files changed, test results
- [ ] **Auto-spawn on PR comment** - agent responds to review feedback
- [ ] **Smart routing** - prevent conflicts upfront via task analysis

## Monitoring Specification

### What Triggers Attention

| Level | Trigger | Action |
|-------|---------|--------|
| **Critical** | Agent exit (non-zero), crash | Highlight in dashboard, preserve session |
| **Block** | Agent says "I need help", permission denial | Highlight, surface in alerts |
| **Info** | Commits, task transitions, idle | Update status, no alert |

### Output Handling

- **Buffer**: Recent output kept in memory for preview/attach
- **Persist**: Full session log written to file (`~/.rembrandt/logs/{session-id}.log`)
- **Parse** (future): Extract structured events (tool calls, errors, commits)

## TUI Design

### Overview Mode (Default)

```
┌─ Rembrandt ──────────────────────────────────────────────┐
│                                                          │
│  Agent         Task              Status      Preview     │
│  ────────────────────────────────────────────────────────│
│▶ claude-a3f    rembrandt-jds     ● active    editing... │
│  opencode-b7x  rembrandt-z7r     ○ idle      waiting... │
│  claude-c2k    rembrandt-8nv     ✗ blocked   need help  │
│                                                          │
│  ────────────────────────────────────────────────────────│
│  ! claude-c2k needs attention: "Can't find config file" │
│  ────────────────────────────────────────────────────────│
│                                                          │
│  [Enter] Zoom  [B]roadcast  [K]ill  [S]pawn  [Q]uit     │
└──────────────────────────────────────────────────────────┘
```

### Zoom Mode (Apprentice)

```
┌─ claude-a3f ─────────────────────────────────────────────┐
│                                                          │
│  $ claude                                                │
│  > Working on auth module...                             │
│  > Reading src/auth/middleware.rs                        │
│  > [full PTY passthrough - you type directly here]       │
│                                                          │
│                                                          │
│                                                          │
│  ────────────────────────────────────────────────────────│
│  [Esc] Back to Overview  [Ctrl+K] Kill  [Ctrl+B] Broadcast│
└──────────────────────────────────────────────────────────┘
```

## Task Binding (Beads Integration)

### Hybrid Model

1. **Human defines scope**: Epic ID or natural language description
2. **Agent picks task**: Within scope, agent uses `/beads:ready` or similar
3. **Agent updates status**: Marks task in-progress, adds comments
4. **On completion**: Commits work, optionally creates PR

### Scope Examples

- Epic: `--scope epic:rembrandt-mvp`
- Natural language: `--scope "handle all API endpoint work"`
- Labels: `--scope labels:backend,auth`

## Conflict Handling

### Queue Then Redirect Strategy

1. Agent A claims `src/auth/mod.rs`
2. Agent B wants same file → **queued**
3. If A releases within timeout → B proceeds
4. If timeout expires → B gets redirected to different task
5. Timeout configurable (default: 5 minutes?)

## Completion Modes

### Mode 1: Commit + Wait (MVP Start)

- Agent commits to branch
- Session stays alive, idle
- Human reviews, gives next instruction or kills

### Mode 2: PR + Stop (Target)

- Agent commits to branch
- Agent creates PR with summary
- Session ends (cleanup)
- Human reviews PR async

**Open question**: What happens after PR review if changes needed?
- Option A: Spawn fresh agent to address
- Option B: Some mechanism to "revive" session context

## Runtime Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    rembrandt daemon                      │
│  ┌─────────────────────────────────────────────────────┐│
│  │  SessionManager                                     ││
│  │  ├── PtySession (claude-a3f)                       ││
│  │  ├── PtySession (opencode-b7x)                     ││
│  │  └── PtySession (claude-c2k)                       ││
│  └─────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────┐│
│  │  LogManager (writes to ~/.rembrandt/logs/)          ││
│  └─────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────┐│
│  │  ConflictTracker (file reservations)                ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
          ▲                    ▲
          │ unix socket        │ unix socket
          │                    │
    ┌─────┴─────┐        ┌─────┴─────┐
    │  TUI      │        │  CLI      │
    │  (ratatui)│        │  (clap)   │
    └───────────┘        └───────────┘
```

## Future Vision (Post-MVP)

- **Multi-repo orchestration** - agents working across related repositories
- **Remote agents** - agents running on different machines/cloud
- **Team mode** - multiple humans coordinating shared agent pool
- **Learning from outcomes** - track which approaches/agents work best

## Open Questions

1. **PR review workflow**: How to handle "request changes" efficiently?
2. **Competitive eval metrics**: Which metrics are meaningful? (time, tokens, test pass rate?)
3. **Conflict timeout**: 5 minutes default? Configurable per-agent?
4. **Natural language scope**: How to parse/match against Beads tasks?
5. **Multi-repo**: How to handle cross-repo dependencies and coordination?

## Implementation Priority

1. **Daemon with SessionManager** - spawn, monitor, persist logs
2. **TUI overview mode** - status, task, preview, error alerts
3. **Zoom/attach** - fullscreen PTY passthrough
4. **Beads integration** - scope-based task assignment
5. **Conflict detection** - file reservation tracking
6. **Broadcast** - send to multiple agents
7. **PR + stop mode** - GitHub integration
