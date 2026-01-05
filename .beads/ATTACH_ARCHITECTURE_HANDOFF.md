# Attach Architecture Handoff

**Date:** 2026-01-04
**Issue:** Attach mode shows blank screen / corrupted display when switching between agents

## Current State

### What Works
- Dashboard TUI (spawn, monitor, kill, cleanup)
- PTY session creation with correct terminal size
- Worktree management per agent
- Detach sequences (Ctrl+], Ctrl+\, double-Esc)
- Mouse capture cleanup on exit

### What's Broken
- **Attach shows blank screen** until user types something
- **Switching agents shows corrupted display** - TUI outputs overlay each other
- SIGWINCH doesn't trigger app redraw reliably

## Root Cause

We're **naively forwarding PTY bytes** to stdout. When switching sessions:
1. We have no memory of what's on screen
2. Apps don't know we "attached" - they're not prompted to redraw
3. SIGWINCH to process group doesn't reliably trigger full redraw

**tmux solves this differently:** It's a full terminal emulator that:
- Maintains internal screen buffer (cell grid) for each pane
- Parses ALL escape sequences, updates internal state
- Redraws from memory when switching - doesn't ask apps to redraw

## Three Architecture Options

### Option 1: tmux as Backend
Use tmux to manage PTY/terminal emulation, we orchestrate on top.

**How it would work:**
```bash
# Spawn agent in tmux session
tmux new-session -d -s "agent-{id}" -c "{worktree}" "{command}"

# Get screen content for dashboard preview
tmux capture-pane -t "agent-{id}" -p

# Attach to interact
tmux attach -t "agent-{id}"

# Send input programmatically
tmux send-keys -t "agent-{id}" "input text" Enter

# Kill
tmux kill-session -t "agent-{id}"
```

**Pros:**
- Battle-tested terminal multiplexing
- Fast to implement
- All the hard PTY stuff is handled
- Users already know tmux

**Cons:**
- Dependency on tmux being installed
- Less control over UX
- Attach leaves our TUI entirely (or we embed tmux somehow)

### Option 2: libghostty (Embedded Terminal Emulator)
Use Ghostty's terminal emulation library to parse/buffer output.

**Status:** Alpha/Beta - https://github.com/ghostty-org/ghostty

**How it would work:**
- Use libghostty to create virtual terminals for each agent
- Library handles escape sequence parsing, screen buffer
- We render the buffer to our TUI or extract text for dashboard
- Switching is just rendering a different buffer

**Pros:**
- Modern, actively developed
- Potentially excellent performance (Zig/Rust)
- Full control over rendering
- Native integration possible

**Cons:**
- Alpha/beta stability concerns
- Learning curve for the API
- May require significant integration work
- Unclear Rust bindings status

**Research needed:**
- Current API stability
- Rust bindings availability
- Minimum viable integration example

### Option 3: Full GUI (Tauri + xterm.js)
Abandon TUI, build GUI with embedded terminal widgets.

**How it would work:**
- Tauri app with web frontend
- Each agent gets an xterm.js terminal widget
- Widgets connect to PTY via Tauri backend
- Native window/tab management

**Pros:**
- No terminal-in-terminal conflicts
- Rich UI possibilities (graphs, dashboards)
- xterm.js is mature and well-documented
- Cross-platform

**Cons:**
- Bigger architectural change
- Web stack complexity
- Heavier resource usage
- Different developer experience

## Files Changed This Session

| File | Changes |
|------|---------|
| `src/daemon/session.rs` | Removed background reader, added `read_available()`, `take_reader()`, `return_reader()`, `send_sigwinch()` |
| `src/daemon/manager.rs` | Added `spawn_with_size()`, `read_all_available()` |
| `src/tui/attach.rs` | Multiple attempts at fixing attach (all failed) |
| `src/tui/events.rs` | Attach handling, detach sequences |
| `src/tui/mod.rs` | Terminal clear on return, mouse capture disable |
| `src/tui/app.rs` | Terminal size detection, `needs_clear` flag |

## Recommendation

**Start with Option 1 (tmux backend)** for fastest path to working orchestration:
1. Requires tmux but most devs have it
2. Proven terminal multiplexing
3. Can prototype in a day
4. Dashboard shows `tmux capture-pane` output
5. "Attach" just does `tmux attach`

**Then evaluate Option 2 (libghostty)** for native solution:
1. Research current state of library
2. Check Rust bindings
3. Build minimal proof-of-concept
4. Compare complexity vs tmux approach

**Option 3 (GUI)** if we want richer UX beyond terminal management.

## Quick Start for Next Session

```bash
# Current state
cd /Users/davegraham/Projects/rembrandt
cargo run --release -- dashboard

# Test attach (broken - blank screen)
# Press 's' to spawn, 'Enter' to attach

# To explore tmux approach:
tmux new-session -d -s test-agent -c "$(pwd)" "claude"
tmux attach -t test-agent
# Ctrl+b d to detach
tmux capture-pane -t test-agent -p  # see what's on screen
```

## Related Beads Issues
- rembrandt-omt (closed) - Original PTY refactor
- Need new issue for architecture decision
