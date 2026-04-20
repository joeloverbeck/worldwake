# S110DECHISEVE-009: Authoritative repair application events

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — expose a concrete repair-application seam and emit `RepairApplied`
**Deps**: archive/tickets/S110DECHISEVE-004.md

## Problem

S110 defines `RepairApplied`, but the live runtime currently records successful alternate-path outcomes indirectly through repair memory rather than through a dedicated repair-application decision seam carrying `RepairKind`. This ticket adds that seam and emits the authoritative event.

## Assumption Reassessment (2026-04-20)

1. `crates/worldwake-ai/src/failure_handling.rs` classifies failures and records blockers/discrepancies, but it does not currently expose a concrete "repair applied" event point with `RepairKind`.
2. Successful alternate-path outcomes are currently reflected later in `crates/worldwake-ai/src/agent_tick/mod.rs::record_repair_memory_from_completed_plan`.
3. Shared abstraction boundary under audit: the runtime path from successful alternate recovery to repair-memory recording.

## Architecture Check

1. Emitting from a dedicated repair-application seam is cleaner than inferring repairs from later repair-memory state alone.
2. The event should describe the actual repair chosen, not just that later memory exists.

## Verification Layers

1. Repair-kind classification -> focused runtime/unit test.
2. Event emission -> focused `agent_tick` runtime test.

## What to Change

### 1. Expose a repair-application result

Surface the concrete alternate-path choice (`AlternateTarget`, `AlternateRoute`, `AlternateMerchant`, `AlternateRecipe`) at the point the runtime actually accepts it.

### 2. Emit `RepairApplied`

Emit `RepairAppliedPayload` from the first authoritative seam that knows the agent, goal, step index, repair kind, and substitute target.

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` or sibling runtime seam (modify)

## Out of Scope

- Invalidation, suppression, or observer work

## Acceptance Criteria

### Tests That Must Pass

1. Focused test proves one successful alternate-path repair emits exactly one `RepairApplied` event with the correct `RepairKind`.
2. `cargo test -p worldwake-ai`

### Invariants

1. `RepairApplied` is emitted only when the runtime actually chooses and applies a repair.
2. The payload reflects the concrete repair path, not a later inferred memory summary.

## Test Plan

### New/Modified Tests

1. Focused failure-handling / agent_tick repair test.

### Commands

1. `cargo test -p worldwake-ai failure_handling`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
