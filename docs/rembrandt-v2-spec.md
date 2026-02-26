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
│  libghostty (MIT)   — Terminal rendering (embedded)  │
│  agent-browser      — Browser automation (Apache-2.0)│
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
| **libghostty** | GPU-accelerated terminal rendering, Ghostty config compat | Agent logic, coordination, UI chrome |
| **agent-browser** | Headless browser automation, accessibility tree snapshots, element interaction | Visual browser display |
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

### License Analysis (Feb 2026)

cmux (AGPL-3.0) was originally considered as the terminal layer. After analysis:

- **cmux is AGPL-3.0** — embedding or distributing with Rembrandt would require open-sourcing the entire Rembrandt stack. Incompatible with Shoal's commercial model.
- **libghostty is MIT** — the underlying rendering engine from Ghostty is freely embeddable. cmux just wraps it in a Swift app with UI chrome.
- **agent-browser is Apache-2.0** — Vercel's headless browser automation CLI. cmux ported this to WKWebView, but the original standalone CLI is superior (full Chromium vs WKWebView) and permissively licensed.

**Decision: Build a thin custom shell around libghostty (MIT), not cmux (AGPL).**

cmux validated the approach — libghostty works as an embeddable renderer, OSC notification parsing is straightforward (~100 lines), and the workspace model is sound. We take the design patterns, not the code.

### Primary: Custom macOS GUI (libghostty + AppKit)

A thin Swift/AppKit app embedding libghostty directly. Rembrandt owns the entire UX:

1. **libghostty C API** — GPU-accelerated terminal rendering, reads existing `~/.config/ghostty/config` for themes/fonts/colors
2. **OSC 9/99/777 parser** — Parse terminal notification sequences to detect agent-needs-attention state. Ghostty's own OSC parser handles most of this; add kitty OSC 99 support (~100 lines, same patch cmux's Ghostty fork adds)
3. **Notification system** — Blue ring on pane + sidebar badge when agent emits notification. `Cmd+Shift+U` to jump to latest unread
4. **Unix socket API** — For external scripting: create workspaces, split panes, send keystrokes, query agent status
5. **agent-browser integration** — `agent-browser` (Apache-2.0) as CLI backend for headless browser automation. Optionally embed WKWebView for visual split pane (our own code, no AGPL)

**UX:**
- Vertical sidebar with agent fleet status
- Each agent gets a tab with: git branch, CWD, status icon, cost
- Notification rings on panes when agent is waiting for input
- Optional browser split pane for visual verification (WKWebView, our code)

**Design patterns borrowed from cmux (inspiration, not code):**
- Workspace → panes → surfaces abstraction
- Per-surface type (terminal | browser) with unified tab management
- Socket API verb design: `system.identify`, workspace/pane CRUD, send-keys
- Session persistence schema: layout + working dirs + scrollback on relaunch

### Fallback: tmux (Linux/headless)

For Poza and CI/server use:
- `tmux new-session -d -s agent-a` to spawn
- `tmux capture-pane -t agent-a` for status thumbnails
- `tmux attach -t agent-a` for direct interaction
- Rembrandt TUI (ratatui) as dashboard over tmux sessions
- `agent-browser` CLI works identically in headless mode (no WKWebView needed)

---

## Key Dependencies

| Dependency | Source | License | Risk |
|-----------|--------|---------|------|
| pi_agent_rust | github.com/Dicklesworthstone/pi_agent_rust (forked to grizzdank) | MIT + Rider | Single dev, but forked. ~497K lines. |
| libghostty | Part of Ghostty project | **MIT** | Stable — cmux ships against it. C API. |
| agent-browser | github.com/vercel-labs/agent-browser | **Apache-2.0** | Vercel-backed, 16K stars, Rust+Node. Headless Chromium via Playwright. |
| asupersync | Dicklesworthstone/asupersync | MIT | Not Tokio — potential async runtime conflicts |
| Shoal | grizzdank/shoal (private) | Proprietary | Ours |
| Profundo | Local Rust binary | Ours | Ours |

**Explicitly excluded:** cmux (AGPL-3.0) — validated the libghostty embedding approach but license is incompatible with commercial distribution. Design patterns referenced, no code used.

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
- `src/tui/` (1,075 lines) — replaced by custom libghostty GUI (macOS) / tmux (Linux)
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
2. **libghostty C API surface** — Document the embedding API. Reference: Ghostty source + cmux's `GhosttyTerminalView.swift` as integration example (pattern only, no AGPL code).
3. **pi_agent_rust lib API surface** — What's actually exported? Is the Agent type usable as a library?
4. **Shoal integration depth** — Pre-execution gates only, or inline tool-call interception?
5. **Profundo integration** — Should agents have access to memory? How does that work with worktree isolation?
6. **OpenClaw relationship** — Rembrandt could eventually replace OpenClaw's agent runtime for multi-agent use cases. Complementary or competitive?
7. **agent-browser headed mode** — For the visual browser split, evaluate: (a) WKWebView embed (our code), (b) agent-browser `--headed` with Chromium window, (c) headless-only with screenshot feedback. Trade-off: development effort vs visual polish.

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
- [ ] Build thin Swift/AppKit shell around libghostty C API
- [ ] Implement OSC 9/99/777 notification parser (~100 lines)
- [ ] Sidebar with agent fleet status (git branch, CWD, cost, status)
- [ ] Notification rings + jump-to-unread (Cmd+Shift+U)
- [ ] Unix socket API for external scripting
- [ ] agent-browser integration (headless CLI + optional WKWebView split)
- [ ] tmux fallback for headless/Linux

### Phase 4: Governance
- [ ] Shoal policy hooks (pre-spawn, pre-merge)
- [ ] Audit trail integration
- [ ] Cost tracking per agent per session

---

*"Eight arms managing a fleet of painters, each with their own canvas, unified into a masterpiece."* 🐙
