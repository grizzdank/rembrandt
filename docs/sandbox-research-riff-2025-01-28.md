# Rembrandt Sandbox Research Riff
*2025-01-28 — Signal conversation with Pulpito*

## Feature List (Refined)

1. **Diffs** — Visual diff view for code changes
2. **Harness Mode** — Terminal shell to run/interact with cc/pi/codex (not custom chat)
3. **Editor** — Zed-like quality (fast, minimal, keyboard-driven) but not literal Zed
4. **Sandbox** — srt (lightweight) & microsandbox (full VM isolation)
5. **Kanban** — Plan view for task management
6. **Multiagent Orchestration** — Spawn, monitor, coordinate multiple agents

## Key Insight

Rembrandt is not competing with Claude Code — it's **wrapping it**. The value is:
- The harness (visibility into agent work)
- The sandbox (safe execution)
- The orchestration (multiple agents)

Not another chat UI.

## Architecture Stack

```
┌─────────────────────────────────────┐
│  Editor (zed-like)    │  Kanban     │
├───────────────────────┴─────────────┤
│  Harness (terminal for cc/pi/codex) │
├─────────────────────────────────────┤
│  Sandbox (srt / microsandbox)       │
├─────────────────────────────────────┤
│  Multiagent orchestration           │
│  (spawn, monitor, coordinate)       │
└─────────────────────────────────────┘
```

## Sandbox Options Researched

| Tool | Isolation | Speed | Use Case |
|------|-----------|-------|----------|
| **Docker** | Namespace (shared kernel) | ~50ms | Trusted apps, CI/CD |
| **srt** | FS/network filters | instant | Quick command sandboxing |
| **Devbox** | Package versions only | seconds | Reproducible dev envs |
| **microsandbox** | Hardware VM | ~150ms | Untrusted code, full isolation |

### srt (Anthropic Sandbox Runtime)
- `npm install -g @anthropic-ai/sandbox-runtime`
- Uses bubblewrap (Linux) / sandbox-exec (macOS)
- Wraps any command with fs/network restrictions
- Built for Claude Code, open-sourced Apache-2.0

### microsandbox
- `curl -sSL https://get.microsandbox.dev | sh`
- MicroVMs via libkrun — true hardware isolation
- ~150ms startup, needs KVM
- SDK for Python/JS/Rust

## Complexity Assessment

| Feature | Complexity | Notes |
|---------|------------|-------|
| Chat/Harness | Medium | Terminal + PTY management |
| Diffs | Medium | Editor integration |
| Editor (zed-like) | High | Monaco/CodeMirror/custom |
| Sandbox (srt) | Low | npm + OS deps |
| Sandbox (microsandbox) | Medium | KVM daemon |
| Kanban | Medium | UI framework |
| Multiagent orchestration | High | Protocol, state mgmt |

## MVP Slice Suggestion

1. Harness mode (terminal for cc/pi/codex)
2. Diffs
3. srt sandbox

Then layer: Kanban → microsandbox → multiagent

## Dave's Note
> "Need to crunch them together and just build the working mvp. Wasting a lot of time paralysis by analysis"
