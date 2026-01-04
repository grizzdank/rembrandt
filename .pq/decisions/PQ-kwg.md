# PQ-kwg: Agent communication architecture: ACP vs Agent SDK vs PTY harness

**Status:** decided
**Created:** 2026-01-03
**Decided:** 2026-01-03
**Tags:** #architecture #decided

## Context

Rembrandt needs to orchestrate multiple coding agents on the same codebase. The key capability needed is **steering/injecting messages** to running agents - both high-priority interrupts and queued guidance.

Research revealed that neither ACP nor Claude Agent SDK were designed for orchestration-style steering:
- **ACP**: Conversation-oriented (editor <-> agent chat), no mid-execution injection
- **Agent SDK**: Hook-oriented (control points), no direct messaging to running agents
- **Pi's approach**: Queue messages, inject between turns - more pragmatic for orchestration

## Decision

**Option D (Hybrid)** with a key insight: **depth is achieved via native attach, not protocol extension**.

### Architecture

| Layer | Responsibility | Protocol |
|-------|----------------|----------|
| **Rembrandt** | Orchestration (spawn, coordinate, monitor) | PTY for lifecycle |
| **Agents** | Model harness (prompting, tools, reasoning) | ACP for conversation |

### Three Interaction Modes

1. **Broadcast** - Send instructions to all/subset via ACP or PTY write
2. **Zoom (Apprentice)** - Attach directly to agent PTY for full native control
3. **Decompose** - Break down tasks, spawn agents with Beads task IDs

### What's PTY-native (must be)
- Process lifecycle (spawn, kill, cleanup)
- Emergency interrupts (Esc/Ctrl+C to abort mid-execution)
- Terminal resize
- Direct attach for "apprentice mode"

### What uses ACP
- Initial task assignment
- Turn-based guidance
- Structured responses
- Status queries

### Key Insight
When you need deep control, you **zoom in** and use the agent's native interface directly. Rembrandt doesn't need to replicate every agent capability - it just needs to spawn, monitor, interrupt, and attach. This keeps the orchestrator thin and agent-agnostic.

## Alternatives Considered

### Option A: ACP + Turn-Based Injection
- Standard protocol, multi-agent support
- Can't steer mid-execution, limited to ACP-supporting agents
- Agents: Claude Code, OpenCode, Goose, Gemini CLI, Codex CLI

### Option B: Claude Agent SDK Direct (Claude-focused)
- Rich hooks (PreToolUse, PostToolUse, etc.), subagent support
- Claude-only, still can't interrupt mid-execution
- Can inject systemMessage/additionalContext at hook points

### Option C: PTY-Level Custom Harness (Pi-style)
- Works with ANY agent, true stdin injection
- No semantic awareness (just bytes), agent-specific parsing needed
- Message queue with turn-boundary injection

### Option D: Hybrid Approach
- Claude Code -> Agent SDK hooks + PTY fallback
- OpenCode -> ACP + PTY fallback
- Others -> PTY-only with timeout-based idle detection
- Steering (high priority) -> PTY write immediately
- Guidance (normal priority) -> Queue, inject at turn boundary

## Key Trade-off

- **Breadth** (many agent types) -> PTY-based, Pi-style
- **Depth** (rich control of one agent) -> Agent SDK, Claude-focused

**Resolution**: Breadth via PTY/ACP, depth via native attach. No trade-off needed.

## Consequences

1. **Rembrandt stays thin** - orchestration logic only, not agent internals
2. **Agent-agnostic** - any PTY-based agent works (Claude, OpenCode, Aider, etc.)
3. **No capability duplication** - agent features accessed natively via attach
4. **Monitoring is core** - comprehensive dashboard parsing PTY output streams
5. **ACP optional** - works with or without ACP support (PTY fallback)

## Implementation Priority

1. **Comprehensive monitoring** (parse output, status, activity, errors)
2. **Attach/zoom** (full PTY passthrough for apprentice mode)
3. **Broadcast** (send to all/subset)
4. **ACP integration** (for agents that support it, cleaner than raw PTY)

## References

- Pi coding agent: https://mariozechner.at/posts/2025-11-30-pi-coding-agent/
- Mario's tweet on hooks: https://x.com/badlogicgames/status/1998562079138082917
- Claude Agent SDK: https://www.anthropic.com/engineering/building-agents-with-the-claude-agent-sdk
- ccswarm (Rust multi-agent): https://github.com/nwiizo/ccswarm
- ACP spec: https://agentclientprotocol.com/
- ACP agents list: https://agentclientprotocol.com/overview/agents
