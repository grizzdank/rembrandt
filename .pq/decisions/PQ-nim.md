<!-- Generated from nodes.jsonl - edit via: pq edit PQ-nim -->

# PQ-nim: Merge Strategy: Continuous merge with layered validation

**Status:** accepted
**Created:** 2025-12-28
**Tags:** #architecture #merge

## Context

Multiple agents complete work in parallel. Need a strategy for integrating their changes back to main without breaking the codebase. Key concerns: merge timing, conflict handling, and validation depth.

## Decision

**Continuous merge** with **layered validation**:

1. Merge as each agent completes (respecting Beads dependency order)
2. Use `git merge --no-commit` to stage changes
3. Run validation pipeline before finalizing:
   - Pre-merge: Beads deps satisfied, `pq check` passes
   - Type check: `cargo check` / `tsc` / etc
   - Tests: Full test suite on merged state
4. Only commit if all validation passes
5. Human review for: textual conflicts, type failures, test failures

## Alternatives Considered

1. **Batch merge**: All agents finish, then merge together
   - Con: Maximizes conflict surface, delays feedback

2. **Sequential**: One agent at a time
   - Con: Underutilizes parallelism

3. **Integration branch**: Merge to staging, test, promote to main
   - Con: Adds latency, extra branch complexity

## Consequences

- Main stays pristine (failed merges never touch history)
- Fail-fast catches easy problems (type errors)
- Worktrees preserved on failure for debugging/fixing
- Human reviews only the genuinely ambiguous cases
- Respects Beads dependency order automatically
