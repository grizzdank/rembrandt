# Issue Triage – Rembrandt

**Date:** 2026-01-10
**Total Issues:** 29
**Open:** 18 | **In Progress:** 4 | **Closed:** 10 | **Blocked:** 1

---

## Summary by Status

### 🔴 Blocked (1)

| ID | Title | Reason |
|----|-------|--------|
| `rembrandt-ka9` | Add plain terminal/shell option | Agent exit code 1 during testing |

### 🟡 In Progress (4)

| ID | Title | Notes |
|----|-------|-------|
| `rembrandt-0y1` | MVP: Tauri GUI with xterm.js terminals | **EPIC** – Core GUI work, active |
| `rembrandt-3j6` | MVP: Daemon + TUI + Beads | **EPIC** – Pivoted to Tauri, TUI on backup branch |
| `rembrandt-h9d` | Worktree cleanup UI controls | Worktree creation working |
| `rembrandt-p0c` | Beads integration | Basic integration working, needs polish |

### 🟢 Open (18)

Sorted by priority (0 = highest):

| Priority | ID | Title | Type | Quick Win? |
|----------|-----|-------|------|------------|
| 2 | `rembrandt-6tv` | Agent flags (permissions, dangerously_skip_permissions) | task | Maybe |
| 2 | `rembrandt-7xp` | Redesign status line with global stats | feature | Yes ⚡ |
| 2 | `rembrandt-821` | Agent Mail MCP integration | feature | No |
| 2 | `rembrandt-a3j` | Priority message queue for steering | feature | No |
| 2 | `rembrandt-bek` | Grid view for multiple agents | feature | No |
| 2 | `rembrandt-c7k` | Onboarding flow for first-time users | feature | No |
| 2 | `rembrandt-efp` | Merge command with pq check | feature | No |
| 2 | `rembrandt-lm4` | Filter bar in Symphony view | feature | Yes ⚡ |
| 2 | `rembrandt-xz6` | SQLite session persistence | feature | No |
| 2 | `rembrandt-z7r` | Wire broadcast/send commands | feature | Yes ⚡ |
| 3 | `rembrandt-0xx` | Pluggable LLM for model evaluator | feature | No |
| 3 | `rembrandt-2g6` | Activity log view | feature | Maybe |
| 3 | `rembrandt-8nv` | Wire CompetitionManager to spawning | feature | No |
| 3 | `rembrandt-a71` | Debug render mode for TUI | feature | No |
| 3 | `rembrandt-ejv` | Coach TUI for human evaluator | feature | No |

### ✅ Recently Closed (10)

| ID | Title | Close Reason |
|----|-------|--------------|
| `rembrandt-acd` | Kanban board in Symphony view | Implemented 4-column kanban |
| `rembrandt-jds` | Initial prompt support to spawn | CLI --prompt flag works |
| `rembrandt-cxo` | Persisted session logs | SessionLogger writes to ~/.rembrandt/logs/ |
| `rembrandt-cml` | Attach/detach for agent terminals | Full Solo view attach working |
| `rembrandt-omt` | PTY refactor for direct attach | On-demand reading implemented |
| `rembrandt-jf3` | Agent type picker dialog | 's' opens picker with options |
| `rembrandt-uu0` | Help overlay | '?' shows contextual keybindings |
| `rembrandt-vzw` | Scroll position indicator | Shows ↕ 1/5 in title |
| `rembrandt-cjp` | Session age display | Human-readable duration |
| `rembrandt-365` | TUI dashboard with ratatui | Completed (now pivoted to Tauri) |
| `rembrandt-5mq` | Agent spawning via PTY | Core spawn working |
| `rembrandt-qrx` | Worktree lifecycle management | Full worktree support |

---

## Quick Wins ⚡

Issues that could be done in a focused session (< 2 hours):

1. **`rembrandt-7xp`** – Status line with global stats
   - Add counters: "3 running │ 1 waiting │ 2 done │ 1 failed"
   - Low complexity, high visibility improvement

2. **`rembrandt-z7r`** – Wire broadcast/send commands
   - CLI framework exists, just wire to SessionManager.write()
   - Useful for testing multi-agent scenarios

3. **`rembrandt-lm4`** – Filter bar in Symphony view
   - 'f' to toggle, filter by status/type
   - Improves UX as session count grows

4. **`rembrandt-6tv`** (partial) – Initial prompt already done
   - Just agent flags remaining (permissions, /ralph loops)

---

## Priority Ordering Recommendation

### P0 – Complete MVP GUI

1. Finish `rembrandt-0y1` (Tauri GUI) – this is the active focus
2. Complete `rembrandt-p0c` (Beads integration polish)
3. Resolve `rembrandt-h9d` (worktree cleanup UI)

### P1 – Quick Wins for Polish

4. `rembrandt-7xp` – status line redesign
5. `rembrandt-z7r` – broadcast/send commands
6. `rembrandt-lm4` – filter bar

### P2 – Core Orchestration Features

7. `rembrandt-a3j` – priority message queue (steering vs guidance)
8. `rembrandt-efp` – merge command with pq check
9. `rembrandt-821` – Agent Mail integration

### P3 – Advanced Features (Post-MVP)

10. `rembrandt-xz6` – SQLite persistence
11. `rembrandt-bek` – grid view
12. `rembrandt-0xx` – pluggable LLM evaluator
13. `rembrandt-ejv` – coach TUI

### Defer

- `rembrandt-a71` (debug render) – only needed for CI/testing
- `rembrandt-c7k` (onboarding) – nice to have, not blocking
- `rembrandt-ka9` (plain terminal) – blocked, needs investigation

---

## Epics Tracking

### rembrandt-0y1: Tauri GUI MVP

**Status:** In progress

**Completed:**
- Tauri scaffold with Svelte frontend
- PTY session code in src-tauri
- Tauri commands: spawn, kill, write, resize, get_history
- Terminal.svelte with xterm.js
- AgentCard.svelte for session list
- Dashboard App.svelte with sidebar + terminal
- PTY output polling
- Session status detection
- Warm Rembrandt color theme
- Kanban board view

**Remaining:**
- Event-based streaming (replace polling)
- UI for viewing past session logs
- Fix spacing/scroll issues

### rembrandt-3j6: Original TUI MVP

**Status:** In progress (paused)

Pivoted to Tauri GUI. TUI preserved on `tui-ratatui-backup` branch.

---

## Dependencies Graph

```
rembrandt-8nv (CompetitionManager) 
    └── blocks: rembrandt-5mq (PTY spawning) ✅

rembrandt-ejv (Coach TUI)
    └── blocks: rembrandt-365 (TUI dashboard) ✅
```

Most issues are now independent since core infra is complete.

---

## Notes

- The pivot from TUI to Tauri was the right call – terminal-in-terminal is genuinely cursed
- Good momentum on closed issues – 10 closed in the past week
- `rembrandt-p0c` (Beads) should get a focused session to finish properly
- Competition mode features (`rembrandt-8nv`, `rembrandt-ejv`, `rembrandt-0xx`) are coherent group – could be a "v2" epic
