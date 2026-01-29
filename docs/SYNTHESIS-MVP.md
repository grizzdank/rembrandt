# Rembrandt MVP Synthesis
*Generated: 2025-01-28*

> **Dave's mandate:** "Just build the working MVP. Wasting a lot of time paralysis by analysis."

---

## What Rembrandt Actually Is

**Not** another chat UI. **Not** competing with Claude Code.

Rembrandt is a **cockpit for existing coding agents** — you spawn cc/pi/codex in isolated worktrees, see what they're doing, and orchestrate them.

```
┌─────────────────────────────────────┐
│  Editor (zed-quality, not zed)     │
├─────────────────────────────────────┤
│  Harness (terminal for cc/pi/codex)│  ← This is the core value
├─────────────────────────────────────┤
│  Sandbox (srt / microsandbox)       │
├─────────────────────────────────────┤
│  Orchestration (spawn/monitor/merge)│
└─────────────────────────────────────┘
```

---

## Current State of Code

### What Works (CLI)
```bash
rembrandt init              # ✅ Creates .rembrandt/
rembrandt spawn claude-code # ✅ PTY + worktree + attach
rembrandt list              # ✅ Shows active worktrees
rembrandt cleanup --all     # ✅ Removes worktrees
rembrandt status            # ✅ Shows integration status
rembrandt dashboard         # ⚠️ TUI exists but buggy
```

**~5400 lines of Rust** across:
- `src/daemon/session.rs` — PTY handling (works)
- `src/worktree/mod.rs` — Git worktree management (works)
- `src/agent/registry.rs` — Agent type definitions (works)
- `src/tui/` — ratatui dashboard (buggy, needs terminal-in-terminal fix)
- `src/competition/` — Competition mode (stubbed, not wired)

### GUI State
- **Backend:** Tauri session/manager scaffolding exists (~500 lines)
- **Frontend:** Empty boilerplate (Vite+Svelte template, no actual UI)

### The Bug That Caused Pivot
TUI had terminal-in-terminal issues with attach mode. Pivoted to Tauri GUI with xterm.js to solve this properly.

---

## MVP Feature Stack (Prioritized)

### Tier 1: Daily Driver (Build This First)
| Feature | Status | Effort |
|---------|--------|--------|
| Spawn agent in worktree | ✅ Done | — |
| PTY attach/detach | ✅ Done | — |
| List active agents | ✅ Done | — |
| **xterm.js terminal widget** | 🔴 Not started | Medium |
| **Multi-agent dashboard view** | 🔴 Not started | Medium |
| **Diff view** | 🔴 Not started | Medium |

### Tier 2: Workflow Polish
| Feature | Status | Effort |
|---------|--------|--------|
| Broadcast to agents | Stubbed | Low |
| Merge agent work | Stubbed | Medium |
| srt sandbox wrapper | Not started | Low |
| Beads integration | Wired | Low |

### Tier 3: Full Vision
| Feature | Status | Effort |
|---------|--------|--------|
| Kanban view | Not started | Medium |
| microsandbox integration | Not started | Medium |
| Competition mode | Stubbed | High |
| Multiagent orchestration | Not started | High |

---

## The Actual MVP (What To Build Now)

### Goal: Working GUI with 3 features

1. **Dashboard** — See all spawned agents, their status, output preview
2. **Terminal** — Click agent → full xterm.js terminal, interact directly
3. **Spawn** — Button/command to spawn new agent in worktree

That's it. No kanban. No competition mode. No fancy orchestration.

### Tech Stack (Already Chosen)
- **Backend:** Tauri + existing Rust PTY code
- **Frontend:** Svelte 5 + xterm.js
- **Terminal:** xterm.js (cross-platform, no libghostty complexity)

### Architecture

```
┌────────────────────────────────────────────────┐
│              SVELTE FRONTEND                    │
│  ┌──────────────────────────────────────────┐  │
│  │  Dashboard        │  Terminal (xterm.js) │  │
│  │  - Agent list     │  - Full PTY access   │  │
│  │  - Status badges  │  - Input/output      │  │
│  │  - Output preview │                      │  │
│  └──────────────────────────────────────────┘  │
├────────────────────────────────────────────────┤
│              TAURI COMMANDS                     │
│  spawn_agent, list_agents, write_to_agent,     │
│  read_from_agent, resize_terminal, kill_agent  │
├────────────────────────────────────────────────┤
│              RUST BACKEND                       │
│  SessionManager (existing) + WorktreeManager   │
└────────────────────────────────────────────────┘
```

---

## Next Steps (Ordered)

1. **Wire xterm.js to Tauri backend**
   - Frontend: xterm.js terminal component
   - Backend: Tauri commands to read/write PTY
   - This unblocks everything else

2. **Build agent list sidebar**
   - Show spawned agents
   - Status indicators (running/idle/error)
   - Click to focus terminal

3. **Add spawn dialog**
   - Pick agent type (claude-code, opencode, etc.)
   - Optional initial prompt
   - Creates worktree + starts agent

4. **Ship it. Use it daily.**

---

## What To Ignore For Now

- Competition mode (cool but not MVP)
- Kanban view (nice to have)
- microsandbox (srt is simpler first step)
- ACP protocol (just spawn CLI agents directly)
- Beads deep integration (basic scope is fine)
- Editor embedding (use external editor)

---

## Files To Reference

| Doc | Purpose |
|-----|---------|
| `rembrandt-plan.md` | Full architecture vision |
| `MVP.md` | Original MVP spec (pre-pivot) |
| `competition-mode.md` | Competitive eval design |
| `TAURI_MIGRATION_PLAN.md` | GUI pivot details |
| `sandbox-research-riff-2025-01-28.md` | srt/microsandbox research |

---

## The One Thing

> **Build the xterm.js terminal integration. Everything else follows.**

The terminal is the core primitive. Once you can see agent output and type into it from the GUI, you have a working product. Dashboard, spawn UI, and diff view are all additive from there.
