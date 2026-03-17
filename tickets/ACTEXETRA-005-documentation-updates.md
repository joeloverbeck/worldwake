# ACTEXETRA-005: Update CLAUDE.md and AGENTS.md documentation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — documentation only
**Deps**: ACTEXETRA-001, ACTEXETRA-002, ACTEXETRA-003, ACTEXETRA-004

## Problem

The action execution trace system exists and is tested but is not documented in the project's two key reference files (`CLAUDE.md` and `AGENTS.md`). Without documentation, future developers (and Claude) won't know the trace system exists or how to use it for debugging.

## Assumption Reassessment (2026-03-17)

1. `CLAUDE.md` has a "Debugging AI Decisions with Decision Traces" section — confirmed. The new section goes immediately after it.
2. `AGENTS.md` exists at project root — confirmed. It also has a decision traces section to place the new section after.
3. The `worldwake-sim` modules table in `CLAUDE.md` (`### worldwake-sim modules`) does NOT yet list `action_trace`. Must be updated.

## Architecture Check

1. Documentation follows the existing pattern — `CLAUDE.md` gets the detailed section with code examples and guidance table; `AGENTS.md` gets a shorter summary referencing `CLAUDE.md`.
2. No code changes — markdown only.

## What to Change

### 1. Add `action_trace` to the `worldwake-sim modules` table in `CLAUDE.md`

Add row: `| action_trace | ActionTraceSink, ActionTraceEvent, ActionTraceKind — opt-in action lifecycle recording for debugging |`

### 2. Add "Debugging Action Execution with Action Traces" section to `CLAUDE.md`

Place after the "Debugging AI Decisions with Decision Traces" section. Include:
- How to enable in golden tests (`h.enable_action_tracing()`)
- Query API examples (`events_for`, `events_at`, `events_for_at`, `last_committed`, `dump_agent`, `summary()`)
- Decision-trace vs action-trace guidance table (when to use which)
- Golden test observation strategy (1-tick vs multi-tick actions)
- Note that tracing is opt-in and zero-cost when disabled

### 3. Add brief section to `AGENTS.md`

Place after the decision traces section. Brief summary with key types and a pointer to `CLAUDE.md` for details.

## Files to Touch

- `CLAUDE.md` (modify — modules table row + new documentation section)
- `AGENTS.md` (modify — new brief documentation section)

## Out of Scope

- Any Rust source code changes
- Any test changes
- Spec file updates
- Changes to any file other than `CLAUDE.md` and `AGENTS.md`
- Creating new markdown files

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — no regressions (docs changes are markdown-only)
2. `cargo clippy --workspace` — clean

### Invariants

1. `CLAUDE.md` modules table includes `action_trace` row
2. `CLAUDE.md` has a "Debugging Action Execution with Action Traces" section with working code examples
3. `AGENTS.md` has a corresponding brief section
4. Documentation accurately reflects the API as implemented in ACTEXETRA-001 through ACTEXETRA-004
5. No code examples reference types or methods that don't exist

## Test Plan

### New/Modified Tests

1. None — documentation only.

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace`
