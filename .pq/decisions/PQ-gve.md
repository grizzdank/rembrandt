<!-- Generated from nodes.jsonl - edit via: pq edit PQ-gve -->

# PQ-gve: Hub Coordination Model: Rembrandt as ATC, not peer-to-peer

**Status:** accepted
**Created:** 2025-12-28
**Tags:** #architecture #coordination

## Context

Need to coordinate multiple AI coding agents working on the same codebase without file conflicts, semantic conflicts, or merge collisions. Two main approaches exist: hub (centralized coordinator) or peer-to-peer (agents communicate directly).

## Decision

Use hub model where Rembrandt acts as Air Traffic Controller (ATC). All agent coordination flows through Rembrandt:
- File claims and releases
- Task routing and assignment
- Merge orchestration
- Conflict detection and resolution

Agents communicate with Rembrandt, not with each other.

## Alternatives Considered

1. **Peer-to-peer (Agent Mail)**: Agents coordinate directly via MCP Agent Mail
   - Pro: Scales better, no SPOF
   - Con: Complex consensus, discovery problems, less visibility

2. **Hybrid**: Hub for control, P2P for "gossip"
   - Con: Risk of context pollution for agents, marginal value for <10 agents

## Consequences

- Simpler implementation (agents only need one connection)
- Full visibility for human director
- Single point of failure (if Rembrandt dies, agents are blind)
- Scales to ~10 agents; P2P deferred until revenue justifies complexity
- SQLite `state.db` holds shared state for file claims and agent status
