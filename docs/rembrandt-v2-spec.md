# Rembrandt v2 — Architecture Spec

**Date:** 2026-02-23
**Status:** Shaping (pre-build)
**Author:** Dave + Pulpito

---

## Executive Brief

Rembrandt pivots from a monolithic Rust agent harness + orchestrator to a **pure orchestration layer** that uses **pi_agent_rust** as the agent runtime. The terminal GUI targets macOS via **cmux/libghostty** with tmux as a headless fallback. This keeps the entire stack in Rust, enables single-binary distribution, and avoids reimplementing agent harness plumbing that pi_agent_rust already handles well.

The goal: run N agents in parallel on a codebase with isolation, coordination, and governance — without building another coding agent from scratch.

---

## Stack

```
┌─────────────────────────────────────────────────────┐
│  Shoal              — Governance (policy, approvals, │
│                       audit trail)                   │
├─────────────────────────────────────────────────────┤
│  Rembrandt          — Orchestration (spawn, route,   │
│                       isolate, merge, lifecycle)     │
├─────────────────────────────────────────────────────┤
│  pi_agent_rust      — Agent Harness (LLM, tools,    │
│                       extensions, security)          │
├─────────────────────────────────────────────────────┤
│  cmux / libghostty  — Terminal GUI (macOS native)    │
│  tmux               — Headless fallback (Linux)      │
├─────────────────────────────────────────────────────┤
│  Profundo           — Memory (semantic search,       │
│                       learnings, session recall)     │
└─────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Owns | Doesn't Own |
|-----------|------|-------------|
| **Rembrandt** | Agent lifecycle, worktree isolation, hub coordination, merge pipeline, competition mode, task routing | LLM calls, tool execution, model selection, terminal rendering |
| **pi_agent_rust** | LLM provider abstraction (17+), tool system (read/write/edit/bash), session management (JSONL tree, branching, compaction), extension API, security (capability gates, command mediation) | Multi-agent coordination, git isolation, governance |
| **Shoal** | Policy evaluation, approval workflows, audit logging, governance rules | Agent execution, orchestration |
| **cmux/libghostty** | Terminal rendering, vertical tabs, agent notifications (OSC sequences), in-app browser, macOS native UX | Agent logic, coordination |
| **Profundo** | Semantic search over past sessions, learning extraction, memory persistence | Real-time agent state |

---

## Integration: pi_agent_rust as Library

pi_agent_rust exposes a `[lib]` crate (`pi`). Rembrandt links it as a Rust dependency rather than spawning subprocesses.

```toml
# rembrandt/Cargo.toml
[dependencies]
pi = { git = "https://github.com/grizzdank/pi_agent_rust" }
```

### What Rembrandt Uses From pi

- **Agent runtime** — spawn agent sessions with model/provider config
- **Tool system** — built-in tools (read/write/edit/bash) + custom Rembrandt tools
- **Provider abstraction** — model-agnostic, 17+ providers, OAuth + API key auth
- **Session management** — JSONL persistence, branching, compaction
- **Security model** — capability-gated hostcalls, command mediation, policy enforcement
- **Extension API** — Rembrandt-specific behaviors as pi extensions

### What Rembrandt Replaces

| Current Rembrandt Code | Replaced By |
|----------------------|-------------|
| `src/agent/mod.rs` + `registry.rs` (hardcoded 5 agent types) | pi's provider/model system |
| `src/daemon/session.rs` (PTY management, ring buffer) | pi's session management |
| `src/daemon/manager.rs` (session lifecycle) | pi's agent runtime |
| `src/daemon/buffer.rs` (10KB ring buffer) | pi's JSONL session persistence |
| `src/tui/` (ratatui TUI) | cmux (macOS) / tmux (headless) |

### What Rembrandt Keeps

| Current Code | Why |
|-------------|-----|
| `src/worktree/mod.rs` | Git worktree isolation is Rembrandt's core value |
| `src/competition/` | Competition mode (same task → N agents → compare) |
| `src/integration/beads.rs` | Task tracking integration |
| `src/integration/porque.rs` | Decision context integration |
| `src/integration/agent_mail.rs` | Inter-agent communication (future) |

---

## Architecture

### Agent Lifecycle

```
User: "Fix the auth bug and add rate limiting"
                    │
                    ▼
┌─────────────────────────────────────────┐
│  REMBRANDT: Task Decomposition           │
│  1. Fix auth bug        → agent-a       │
│  2. Add rate limiting   → agent-b       │
└─────────────────────────────────────────┘
                    │
         ┌──────────┴──────────┐
         ▼                     ▼
┌─────────────────┐  ┌─────────────────┐
│  Worktree A     │  │  Worktree B     │
│  branch: fix/   │  │  branch: feat/  │
│  auth-bug       │  │  rate-limit     │
│                 │  │                 │
│  pi_agent_rust  │  │  pi_agent_rust  │
│  session        │  │  session        │
└─────────────────┘  └─────────────────┘
         │                     │
         ▼                     ▼
┌─────────────────────────────────────────┐
│  REMBRANDT: Merge Pipeline               │
│  1. Pre-merge checks (Beads deps)       │
│  2. git merge --no-commit               │
│  3. Type check (cargo check / tsc)      │
│  4. Test suite                          │
│  5. Commit + cleanup                    │
│                                         │
│  Human gates: conflict, type fail, test │
│  fail. Everything else auto-flows.      │
└─────────────────────────────────────────┘
```

### Hub Coordination (ATC Model)

Rembrandt is Air Traffic Control. Agents don't talk to each other — they talk to Rembrandt.

```
         ┌──────────┐
         │ Rembrandt │
         │   (ATC)   │
         └─────┬─────┘
        ┌──────┼──────┐
        ▼      ▼      ▼
    Agent A  Agent B  Agent C
```

- **File claims** — SQLite `state.db` tracks which agent owns which files
- **Conflict prevention** — Rembrandt rejects overlapping file claims
- **Status monitoring** — heartbeat polling, stuck detection
- **Broadcast** — send instructions to all/subset of agents

### Shoal Integration Points

```
User Request
     │
     ▼
┌──────────┐    ┌───────┐
│ Rembrandt │───▶│ Shoal │  Pre-execution policy check
└──────────┘    └───────┘
     │               │
     │          Allow/Deny
     │               │
     ▼               ▼
┌──────────────────────┐
│  pi_agent_rust       │
│  (tool execution)    │
│                      │
│  Extension hook ─────┼───▶ Shoal audit log
└──────────────────────┘
```

- **Pre-execution:** Rembrandt checks Shoal policy before spawning agent or allowing file claim
- **During execution:** pi_agent_rust extension calls Shoal on sensitive tool invocations
- **Post-execution:** Audit trail of all tool calls, model usage, costs

---

## Terminal Rendering

### Primary: cmux (macOS)

cmux wraps libghostty in a native Swift/AppKit app. Rembrandt integrates via:

1. **Socket API / CLI** — `cmux` exposes a socket for workspace management
2. **OSC sequences** — Rembrandt emits custom terminal sequences that cmux picks up for sidebar updates (agent status, cost tracking, current action)
3. **Notification hooks** — `cmux notify` for agent-needs-attention alerts with context

**UX:**
- Vertical sidebar with agent fleet status
- Each agent gets a tab with: git branch, CWD, status icon, cost
- Blue ring on pane when agent is waiting for input
- In-app scriptable browser for visual verification

### Fallback: tmux (Linux/headless)

For Poza and CI/server use:
- `tmux new-session -d -s agent-a` to spawn
- `tmux capture-pane -t agent-a` for status thumbnails
- `tmux attach -t agent-a` for direct interaction
- Rembrandt TUI (ratatui) as dashboard over tmux sessions

### Decision: Embed vs Integrate

**TBD.** Two paths for cmux:

| | Embed libghostty | Integrate with cmux |
|---|---|---|
| **Approach** | Build custom macOS app using libghostty C API directly | Use cmux as-is, integrate via socket API |
| **Control** | Full — own the entire UX | Partial — ride cmux's UX decisions |
| **Effort** | High — Swift/AppKit, C interop | Low — CLI/socket integration |
| **Updates** | Maintain libghostty binding | Ride cmux releases |
| **Risk** | libghostty API stability | cmux dev's roadmap alignment |

**Recommendation:** Start with cmux integration. If it chafes, the libghostty embedding path remains open. Use cmux for a week before deciding.

---

## Key Dependencies

| Dependency | Source | License | Risk |
|-----------|--------|---------|------|
| pi_agent_rust | github.com/Dicklesworthstone/pi_agent_rust (forked to grizzdank) | MIT + Rider | Single dev, but forked. ~497K lines. |
| libghostty | Part of Ghostty project | MIT | Stable enough for cmux to ship against. C API. |
| cmux | Third-party macOS app | TBD | Single dev. macOS only. |
| asupersync | Dicklesworthstone/asupersync | MIT | Not Tokio — potential async runtime conflicts |
| Shoal | grizzdank/shoal (private) | Proprietary | Ours |
| Profundo | Local Rust binary | Ours | Ours |

### Runtime Compatibility Note

pi_agent_rust uses **asupersync** (structured concurrency runtime), not Tokio. Current Rembrandt uses Tokio. Options:
1. Migrate Rembrandt to asupersync (clean but effort)
2. Run both runtimes (messy, potential conflicts)
3. Use pi_agent_rust via subprocess/RPC instead of lib linking (avoids runtime conflict but loses single-binary goal)

**This needs investigation before coding begins.**

---

## Competition Mode

Rembrandt's unique feature: run the same task against multiple agents/models and compare.

```
rembrandt compete "implement login form" --agents claude,codex,opencode
```

1. Spawn 3 pi_agent_rust sessions, each in own worktree
2. Same task prompt, same codebase snapshot
3. Each works independently
4. Rembrandt collects: time, tokens, cost, diff size, test pass rate
5. Human picks winner, Rembrandt merges that branch

With pi_agent_rust, competition mode gets model-agnostic for free — compete Claude vs Gemini vs DeepSeek on the same task.

---

## What Gets Deleted

From current Rembrandt codebase (~5.2K lines):

- `src/agent/` (245 lines) — replaced by pi provider system
- `src/daemon/` (817 lines) — replaced by pi session management
- `src/tui/` (1,075 lines) — replaced by cmux/tmux
- `src/cli/mod.rs` (146 lines) — rewritten to orchestrate pi sessions

**Kept:** `src/worktree/` (117 lines), `src/competition/` (1,202 lines), `src/integration/` (309 lines)

**Net:** ~2,283 lines deleted, ~1,628 kept, new orchestration code TBD.

---

## CLI (Revised)

```bash
# Initialize
rembrandt init

# Spawn agent in isolated worktree
rembrandt spawn --model claude-opus "fix the auth bug"
rembrandt spawn --model deepseek-r1 "add rate limiting"

# Fleet management
rembrandt list                    # Show all agents + status
rembrandt status agent-a          # Detailed status
rembrandt steer agent-a "focus on the middleware first"
rembrandt kill agent-a

# Competition
rembrandt compete "implement login" --models claude,gemini,deepseek

# Merge
rembrandt merge agent-a           # Run merge pipeline
rembrandt merge --auto            # Auto-merge all completed agents

# Broadcast
rembrandt broadcast "wrap up, we're merging in 10 min"

# Governance (Shoal)
rembrandt policy check agent-a    # Check policy compliance
rembrandt audit                   # Show audit trail
```

---

## Open Questions

1. **asupersync vs Tokio** — Can they coexist? Or does pi_agent_rust need to be subprocess/RPC?
2. **cmux socket API stability** — Is it documented? Versioned?
3. **pi_agent_rust lib API surface** — What's actually exported? Is the Agent type usable as a library?
4. **Shoal integration depth** — Pre-execution gates only, or inline tool-call interception?
5. **Profundo integration** — Should agents have access to memory? How does that work with worktree isolation?
6. **OpenClaw relationship** — Rembrandt could eventually replace OpenClaw's agent runtime for multi-agent use cases. Complementary or competitive?

---

## Build Phases

### Phase 1: Foundation
- [ ] Audit pi_agent_rust lib exports
- [ ] Resolve asupersync vs Tokio
- [ ] Strip Rembrandt to orchestration core (delete agent/daemon/tui)
- [ ] Integrate pi_agent_rust as dependency
- [ ] Spawn single pi session in a worktree via Rembrandt

### Phase 2: Multi-Agent
- [ ] Spawn N agents with independent worktrees
- [ ] Hub coordination (file claims, status tracking)
- [ ] Merge pipeline (pre-check → merge → typecheck → test)
- [ ] Competition mode with pi's model system

### Phase 3: GUI
- [ ] cmux integration via socket API
- [ ] OSC sequence hooks for fleet status
- [ ] tmux fallback for headless

### Phase 4: Governance
- [ ] Shoal policy hooks (pre-spawn, pre-merge)
- [ ] Audit trail integration
- [ ] Cost tracking per agent per session

---

*"Eight arms managing a fleet of painters, each with their own canvas, unified into a masterpiece."* 🐙
