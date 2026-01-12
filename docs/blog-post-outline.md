# Blog Post Outline: Rembrandt – Orchestrating AI Coding Agents

**Target:** LFG Consulting blog
**Audience:** Developers using AI coding assistants, AI/ML engineers
**Goal:** Establish thought leadership on AI-augmented development workflows

---

## Title Options

1. "Rembrandt's Workshop: Orchestrating Multiple AI Coding Agents Without Collision"
2. "Why I Built an Orchestration Layer for Claude Code, OpenCode, and Friends"
3. "The Master and His Apprentices: Running 5 AI Agents on One Codebase"

---

## Hook (Lead)

Open with the frustration: You're using Claude Code and it's amazing, but now you want to parallelize. Run three agents on different features. But they'll stomp on each other. They'll both edit `auth.rs`. They'll create merge conflicts. They have no idea the other exists.

What if there was a master orchestrating the apprentices?

---

## Section 1: The Problem Space

### Why Parallel Agents?

- Solo dev productivity: one human, multiple AI workers
- Different agents have different strengths (Claude's reasoning, Codex's speed)
- Competitive evaluation: same task, multiple approaches, pick the best
- Background workers: spawn and forget, check results later

### Why It's Hard

- **File conflicts**: Two agents editing the same file = corruption
- **Git conflicts**: Commits happening simultaneously
- **Context blindness**: Agents don't know about each other
- **Monitoring overhead**: Can't watch 5 terminals at once

---

## Section 2: The Rembrandt Metaphor

Rembrandt's workshop wasn't Rembrandt alone—it was apprentices working different areas of the canvas. The master directed, unified, corrected. The final piece was coherent because someone was orchestrating.

This is the model:
- **Agents = Apprentices** – each capable, each with their own brush
- **Rembrandt (the tool) = The Master** – assigns areas, prevents overlap, unifies
- **The Codebase = The Canvas** – one shared artifact, many concurrent workers

---

## Section 3: Architecture Decisions (Technical Depth)

### Why Git Worktrees?

Not Docker containers. Not separate clones. Worktrees because:

```
project/
├── .git/                    # Shared git database
├── main/                    # Human workspace  
├── .rembrandt/
│   ├── agents/
│   │   ├── agent-1/         # Isolated checkout
│   │   └── agent-2/         # Another isolated checkout
```

- **Lightweight**: Same git database, no full clone overhead
- **Branch per agent**: Clean merge history
- **Familiar**: Standard git workflow, PRs work naturally

### The Terminal-in-Terminal Problem

Original plan: TUI with ratatui. Run `rembrandt dashboard`, see all agents in a nice terminal interface.

Then I tried to attach to Claude Code running inside my TUI.

**Problem**: Claude Code is a full TUI app. Running a TUI inside a TUI is... cursed. PTY escapes fight. ANSI codes clash. Resize events conflict.

Options considered:
1. **Restricted view**: Don't show agent TUI, just status → loses the point
2. **Custom terminal emulator in terminal**: Madness
3. **GUI with proper terminal widgets**: xterm.js does this right

### Why Tauri Over Pure TUI

Decision: Tauri + Svelte + xterm.js

- **xterm.js**: Battle-tested terminal emulator (VS Code uses it)
- **Tauri**: Rust backend with lightweight webview, not Electron's bloat
- **Svelte**: Minimal framework, compiles away

Trade-off: Heavier than a TUI, but actually works. Each agent gets a real terminal widget. No escape code fighting.

The TUI code lives on a backup branch for when libghostty becomes embeddable.

### PTY vs ACP vs Agent SDK

Research: How do you actually talk to these agents?

| Approach | Pros | Cons |
|----------|------|------|
| **PTY (raw terminal)** | Works with ANY agent | Just bytes, no semantics |
| **ACP** | Structured conversation | Can't interrupt mid-turn |
| **Agent SDK** | Rich hooks | Claude-only, no steering |

**Decision: Hybrid with depth via attach**

- Orchestration happens at PTY level (spawn, kill, monitor)
- ACP for structured comms where supported
- When you need full control: **zoom in**, attach directly to the agent's native interface

Rembrandt doesn't try to replicate agent capabilities. It spawns, monitors, and steps aside.

---

## Section 4: The Developer Experience

### Spawn an Agent

```bash
rembrandt spawn claude --prompt "implement the login form"
```

Creates a worktree, launches Claude Code in it, sends the initial task.

### Monitor from 10,000 Feet

Dashboard shows all agents: status, recent output, errors. Spot the blocked one instantly.

### Zoom to Street Level

Click an agent (or press Enter): full terminal takeover. Now you're pair programming directly with that agent. Esc to zoom back out.

### Beads Integration

Integrate with task tracking (Beads) so agents work within scope:

```bash
rembrandt spawn claude --task rembrandt-0y1
```

Agent sees the task context. Updates status. Stays in its lane.

---

## Section 5: What's Next

- **Conflict handling**: Queue then redirect (agent waits for file, times out, gets new task)
- **PR + Stop mode**: Agent creates PR, session ends, async handoff
- **Agent Mail**: Agents can leave messages for each other
- **Competitive evaluation**: Same task to multiple agents, compare results

---

## Section 6: Try It / Contribute

Open source: [github.com/grizzdank/rembrandt](https://github.com/grizzdank/rembrandt)

Status: In development, CLI works, GUI coming together.

Interested in multi-agent orchestration? PRs welcome. Or hire LFG Consulting to help you think through your AI-augmented development workflow.

---

## Tone Notes

- Technical but accessible
- Show the decisions, not just the outcomes
- Honest about trade-offs ("this is cursed", "we tried X, it didn't work")
- The metaphor should enhance, not dominate
- End with credibility hook for LFG Consulting

---

## Visual Assets to Create

1. Architecture diagram (Tauri backend, PTY sessions, Svelte frontend)
2. Worktree isolation diagram
3. Screenshot of dashboard with multiple agents
4. Before/after: chaos vs orchestrated

---

## Estimated Length

2,500-3,500 words. Could split into a series:
1. "Why Multi-Agent Orchestration" (problem + metaphor)
2. "Building Rembrandt: Architecture Deep Dive" (technical decisions)
3. "Lessons from Terminal-in-Terminal Hell" (TUI pivot story)
