<!-- Generated from nodes.jsonl - edit via: pq edit PQ-bfj -->

# PQ-bfj: Beads/Rembrandt Boundary: Task graph vs execution engine

**Status:** accepted
**Created:** 2025-12-28
**Tags:** #architecture #integration

## Context

Both Beads and Rembrandt involve "task management" - there's apparent overlap. Need to clarify which tool owns what responsibility to avoid duplication and ensure clean integration.

## Decision

Clear separation of concerns:

| Tool | Owns | Analogy |
|------|------|---------|
| **Beads** | WHAT needs to be done | Jira |
| **Rembrandt** | WHO does it, WHERE, HOW to merge | Kubernetes |

**Beads responsibilities:**
- Task decomposition and creation
- Dependency tracking between tasks
- Status management (open, in-progress, done)
- `bd ready` to find unblocked work

**Rembrandt responsibilities:**
- Agent spawning in isolated worktrees
- File claim tracking
- Task-to-agent assignment
- Merge orchestration
- Agent lifecycle (attach, kill, cleanup)

## Alternatives Considered

1. **Rembrandt owns task decomposition**: Break down work into sub-tasks
   - Con: Duplicates Beads, tighter coupling

2. **Beads tracks agents**: Add agent assignment to issues
   - Con: Beads is tool-agnostic, shouldn't know about orchestration

## Consequences

- Beads works without Rembrandt (single agent, manual workflow)
- Rembrandt works without Beads (manual task assignment)
- Together: Rembrandt reads from Beads (`bd ready`), updates Beads on completion
- Clean integration point: Beads issues → Rembrandt assignment → Beads status update
