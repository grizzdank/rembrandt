# Rembrandt Architecture Evolution: Gastown Analysis & Strategic Direction

> **Note**: Move to `/Users/davegraham/Projects/rembrandt/docs/gastown-analysis.md` after approval

## Executive Summary

Analysis of Steve Yegge's Gastown orchestrator reveals architectural patterns that should inform Rembrandt's evolution. This document captures key insights and charts a path forward that adopts proven patterns while maintaining Rembrandt's unique differentiators.

---

## Gastown Key Concepts (Reference)

| Gastown Term | What It Is |
|--------------|------------|
| **GUPP** | "If there is work on your hook, YOU MUST RUN IT" - autonomous execution |
| **Hook** | Persistent work attached to an agent identity, survives session crashes |
| **Molecule** | Workflow encoded as chain of Beads (issues) |
| **Wisp** | Ephemeral molecule - not persisted to Git, burned after completion |
| **Convoy** | Work order wrapper - all work rolls up into trackable convoys |
| **Polecat** | Ephemeral worker (spawn, execute, disappear) |
| **Crew** | Persistent worker (human-managed, long-lived) |

---

## Strategic Positioning

### What Rembrandt Is NOT

- **Not a Gastown clone** - Different language (Rust), different goals
- **Not a Stage 5-7 on-ramp** - Intermediate stages are transitional artifacts of immature tooling; they won't exist in a year
- **Not Claude Code-specific** - Heterogeneous agent support is a core differentiator

### What Rembrandt IS

1. **Competition-first orchestrator** - Multiple agents solve same task, evaluator picks best. Unique capability.
2. **Heterogeneous agent layer** - ACP abstraction for Claude/OpenCode/Codex/Aider
3. **TUI-first, GUI-ready** - ratatui now, Tauri later
4. **Beads-integrated** - Shared task graph foundation with Gastown

---

## Patterns to Adopt (With Rembrandt Metaphors)

### 1. Separate Agent Identity from Session

**Gastown**: Agent = Bead (persistent), Session = cattle (ephemeral)

**Rembrandt Approach**:
- `AgentIdentity` - Persistent record stored as a Bead (name, capabilities, work history)
- `AgentSession` - Ephemeral running instance attached to an identity
- **Storage**: Use Beads directly (like Gastown) - agent identities are beads with `type=agent`

**Metaphor**: Like Rembrandt's workshop - the *apprentice* is persistent (has a name, skills, reputation), but any given *work session* is ephemeral.

### 2. The Hook (Work Assignment)

**Gastown**: Hook = pinned bead where you "hang" molecules

**Rembrandt Approach**:
- `Assignment` - Work attached to an agent identity
- Survives session crashes - next session picks up where predecessor left off
- Stored in agent's identity record

**Metaphor**: The **Easel** - each apprentice has an easel where their current canvas (assignment) sits. If they step away, the canvas remains on the easel.

### 3. Autonomous Execution (GUPP equivalent)

**Gastown**: "If your hook has work, YOU MUST RUN IT"

**Rembrandt Approach**:
- On session start, agent checks its easel
- If assignment present, begin work immediately without confirmation
- Rembrandt as orchestrator can "nudge" stalled agents

**Metaphor**: **No Idle Hands** - the workshop principle: if there's a canvas on your easel, you paint. No waiting for permission, no idleness when work is assigned.

### 4. Ephemeral Orchestration (Wisps)

**Gastown**: Wisps = beads that don't pollute Git history

**Rembrandt Approach**:
- `Sketch` - Ephemeral work tracking for orchestration internals
- Stored in SQLite `state.db`, not Git
- Can be "rendered" to a summary if audit needed

**Metaphor**: **Sketches** vs **Paintings** - sketches are working notes, paintings are the final work that gets preserved.

### 5. Work Order Wrapper (Convoys)

**Gastown**: Convoy = ticketing wrapper for all slung work. Convoys batch multiple assignments together for tracking and completion visibility.

**Rembrandt Approach**:
- All work wrapped in trackable units with an ID
- Even single-agent tasks get a wrapper
- Provides dashboard visibility and completion tracking
- Contains: title, list of assignments, status, timing

**Metaphor**: **Brief** - a patron's brief describing the desired work. Contains scope, assignments, and deliverables.

---

## Rembrandt Unique Features (Keep/Enhance)

### Competition Mode

**Current State**: `CompetitionGroup`, `CompetitorSolution`, `EvaluatorStrategy`

**Enhancement Direction**:
- Integrate with Brief tracking
- Support for competition-as-canvas (multi-step competitions)
- Model-based evaluation using different LLMs as judges

### Heterogeneous Agent Support

**Current State**: `AgentType` enum (ClaudeCode, OpenCode, AmpCode, Codex, Aider, Custom)

**Enhancement Direction**:
- ACP adapter layer for each agent type
- Capability registry (what each agent type is good at)
- Smart routing based on task requirements

### Merge Pipeline

**Current State**: Documented in AGENTS.md but not fully implemented

**Enhancement Direction**:
- Integrate with Beads dependency graph
- Porque constraint checking pre-merge
- Brief-aware merge ordering

---

## Proposed Type Evolution

All persistent types stored as Beads (via `bd create --type=<type>`).

```rust
// Persistent identity - stored as Bead with type=agent
// Maps to: bd create --type=agent --title="claude-1" ...
pub struct AgentIdentity {
    pub id: String,              // Bead ID (e.g., "rm-abc123")
    pub name: String,            // Human-readable name
    pub agent_type: AgentType,   // ClaudeCode, OpenCode, etc.
    pub capabilities: Vec<Capability>,
    pub easel: Option<String>,   // Bead ID of current assignment
    pub created_at: DateTime<Utc>,
}

// Ephemeral running instance - stored in SQLite state.db
pub struct AgentSession {
    pub session_id: String,
    pub identity_id: String,     // Links to AgentIdentity bead
    pub pid: Option<u32>,
    pub worktree_path: PathBuf,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
}

// Work assignment - stored as Bead with type=assignment
pub struct Assignment {
    pub id: String,              // Bead ID
    pub brief_id: String,        // Parent Brief
    pub task_id: Option<String>, // Linked Beads issue if any
    pub instructions: String,
    pub assigned_at: DateTime<Utc>,
}

// Patron's brief - stored as Bead with type=brief
pub struct Brief {
    pub id: String,              // Bead ID
    pub title: String,
    pub assignments: Vec<String>, // Assignment bead IDs
    pub status: BriefStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// Ephemeral orchestration - stored in SQLite only (not Git)
pub struct Sketch {
    pub id: String,
    pub brief_id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}
```

---

## Metaphor Summary (Final)

| Gastown | Rembrandt | Meaning |
|---------|-----------|---------|
| Hook | **Easel** | Where current work sits |
| Molecule | **Canvas** | Multi-step workflow |
| Wisp | **Sketch** | Ephemeral work notes |
| Convoy | **Brief** | Patron's work order |
| Polecat | **Apprentice** | Ephemeral worker (spawn, execute, disappear) |
| Crew | **Journeyman** | Persistent worker (human-managed) |
| GUPP | **No Idle Hands** | Autonomous execution principle |
| (Human) | **Master** | Human operator/overseer |

### Worker Tier System (Potential)

Could expand to three tiers if needed:
- **Apprentice** - Ephemeral, task-specific (Gastown Polecat)
- **Journeyman** - Persistent, human-managed (Gastown Crew)
- **Craftsman** - Senior/trusted persistent worker (future: more autonomy, complex work)

---

## Implementation Phases

### Phase 1: Identity/Session Split
- [ ] Create `AgentIdentity` type backed by Beads (`type=agent`)
- [ ] Refactor `AgentSession` to reference identity bead ID
- [ ] Update registry to manage identities via `bd` commands
- [ ] **Files**: `src/agent/mod.rs`, `src/agent/registry.rs`

### Phase 2: Easel & Assignment
- [ ] Add `Assignment` type backed by Beads (`type=assignment`)
- [ ] Implement easel mechanics (link assignment to identity bead)
- [ ] Add session startup hook to check easel via `bd show`
- [ ] **Files**: `src/agent/mod.rs`, new `src/assignment/mod.rs`

### Phase 3: Brief Tracking
- [ ] Create `Brief` type backed by Beads (`type=brief`)
- [ ] Wrap all work in briefs - even single assignments
- [ ] Add brief dashboard to TUI
- [ ] **Files**: new `src/brief/mod.rs`, `src/tui/app.rs`

### Phase 4: Sketch (Ephemeral Orchestration)
- [ ] Add `Sketch` table to SQLite schema
- [ ] Use sketches for internal orchestration state
- [ ] Optional render-to-summary via `bd create` for audit
- [ ] **Files**: `src/lib.rs` (schema), new `src/sketch/mod.rs`

### Phase 5: Competition Integration
- [ ] Connect competitions to briefs
- [ ] Support competition-as-canvas (multi-step evaluation)
- [ ] Add model-based evaluation strategies
- [ ] **Files**: `src/competition/*.rs`

---

## Resolved Decisions

1. **Scope**: Single project orchestration (not multi-rig like Gastown)
2. **Identity storage**: Use Beads directly - agent identities are beads with `type=agent`
3. **Metaphors**: Easel, Canvas, Sketch, Brief, Apprentice, Journeyman, No Idle Hands
4. **Convoy → Brief**: Patron's brief describing desired work
5. **GUPP → No Idle Hands**: Workshop principle for autonomous execution
6. **Worker hierarchy**: Human = Master, persistent worker = Journeyman, ephemeral = Apprentice

## Open Questions (Research Needed)

### Agent Communication Layer
The hub model requires Rembrandt to communicate with agents. Options to explore:

1. **Nudge mechanism** - How to wake stalled agents?
   - PTY injection (send keystrokes like Gastown's `gt nudge`)
   - Signals (SIGUSR1?)
   - File-based triggers (agent polls a file)
   - stdin injection via portable-pty

2. **Messaging between agents** - How do agents coordinate through Rembrandt?
   - Rembrandt polls agent output and routes messages?
   - Shared state in SQLite that agents can read?
   - MCP-based messaging (Agent Mail)?
   - File-based message queues?

### Agent Control Protocol

3. **ACP vs Agent SDK vs Hybrid** - May need to revisit approach:
   - **ACP (Zed)**: Cross-platform standard, but may limit agent capabilities
   - **Agent SDK**: Full power per-agent, but vendor-specific
   - **Hybrid**: ACP for control, SDK for advanced features
   - **PTY-only**: Simple but parsing-heavy

   *Need to evaluate what capabilities ACP exposes vs what we need for:*
   - Session management (start, stop, resume)
   - Context injection (assignment, brief context)
   - Output streaming
   - Tool invocation control

---

## References

- [Gastown Repository](https://github.com/steveyegge/gastown)
- [Welcome to Gas Town (Medium)](https://steve-yegge.medium.com/welcome-to-gas-town-4f25ee16dd04)
- Rembrandt AGENTS.md (current architecture)
