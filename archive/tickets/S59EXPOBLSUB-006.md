# S59EXPOBLSUB-006: SystemId::ExpectationCheck + overdue detection SystemFn

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — SystemId enum, system manifest canonical order, dispatch table, new SystemFn
**Deps**: S59EXPOBLSUB-002

## Problem

Expectations transition from Active to Overdue based on tick deadlines, but no system currently checks this. A new SystemFn is needed to scan each agent's `ExpectationStore` and transition stale expectations. This must run after Perception (so beliefs are current) and before goal generation (so overdue state feeds into search goals).

## Assumption Reassessment (2026-04-06)

1. `SystemId` is defined via `define_system_ids!` in `crates/worldwake-sim/src/system_manifest.rs`, and `SystemManifest::canonical()` is already allowed to differ from `SystemId::ALL` specifically to preserve existing ordinals when execution order changes. This ticket should append the new variant in declaration order, then place it after `Perception` only in `canonical()`.
2. `SystemDispatchTable` in `crates/worldwake-sim/src/system_dispatch.rs` is a positional array `[SystemFn; SYSTEM_COUNT]`, so `crates/worldwake-systems/src/lib.rs:dispatch_table()` must stay aligned with `SystemId::ALL`, not with canonical execution order.
3. The expectation substrate currently exposes only `ExpectationStore` / `ExpectationState` component data. There is no existing expectation-specific event payload type, so this ticket should rely on the normal `WorldTxn` state-delta event path rather than inventing a bespoke transition event.
4. The spec text has a live tension: its design-goal prose says overdue detection is local to the owner, while Deliverable 7 explicitly reserves a `SystemFn` that scans expectation stores each tick. For this ticket, the owned behavior is only authoritative clock-driven `Active -> Overdue` mutation of stored expectation state. Expected-place observation and violation creation remain deferred.
5. `SystemFn` still uses the standard `fn(SystemExecutionContext<'_>) -> Result<(), SystemError>` signature, and existing systems batch deterministic component edits through `WorldTxn`.

## Architecture Check

1. Adding a `SystemId` plus `SystemFn` follows the established system-registration pattern, but the new id should be appended in ordinal order while canonical execution inserts it after `Perception`.
2. No backward-compatibility shims. The dispatch table is rebuilt at startup, and the system should emit only ordinary state-delta system events.

## Verification Layers

1. ExpectationCheck runs in correct position → system ordering test in system_manifest.rs
2. Active→Overdue transition fires at correct tick → focused unit test with mock world
3. Expectations within grace period remain Active → focused unit test
4. Single-system ticket — no cross-system layer mapping needed.

## What to Change

### 1. Add SystemId::ExpectationCheck

In `crates/worldwake-sim/src/system_manifest.rs`, add `(ExpectationCheck, "expectation_check")` to `define_system_ids!`, appended so existing ordinals remain stable.

### 2. Update canonical order

In `SystemManifest::canonical()`, insert `SystemId::ExpectationCheck` between `SystemId::Perception` and `SystemId::EvidenceDecay`. Update ordering comments.

### 3. Implement check_overdue_expectations

Create `crates/worldwake-systems/src/expectation_check.rs`:

```rust
pub fn check_overdue_expectations(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    // For each agent with ExpectationStore:
    //   For each record where state == Active:
    //     If ctx.tick > record.deadline_tick + record.grace_ticks:
    //       Transition state to Overdue via normal component mutation
}
```

### 4. Register in dispatch table

In `crates/worldwake-systems/src/lib.rs`, add `check_overdue_expectations` to the `dispatch_table()` array at the correct position (matching SystemId::ALL order).

### 5. Export from lib.rs

Add `mod expectation_check;` and `use expectation_check::check_overdue_expectations;` to `crates/worldwake-systems/src/lib.rs`.

## Files to Touch

- `crates/worldwake-sim/src/system_manifest.rs` (modify — add SystemId variant + canonical order)
- `crates/worldwake-systems/src/expectation_check.rs` (new — SystemFn implementation)
- `crates/worldwake-systems/src/lib.rs` (modify — add module + dispatch table entry)

## Out of Scope

- Candidate generation based on overdue state — ticket 011
- Actions that create or resolve expectations — tickets 007-010
- Perception integration (updating LastSeenMemory from observation) — separate concern

## Acceptance Criteria

### Tests That Must Pass

1. Active expectation with `deadline_tick + grace_ticks < current_tick` transitions to Overdue
2. Active expectation within grace period remains Active
3. Already-Overdue expectations are not re-transitioned
4. Resolved and Expired expectations are not affected
5. SystemId::ExpectationCheck appears in canonical order after Perception, before EvidenceDecay
6. Existing suite: `cargo test -p worldwake-sim && cargo test -p worldwake-systems`

### Invariants

1. Overdue detection is a pure clock check over stored expectation records only; this ticket does not add expected-place observation, subject lookup, or violation creation
2. SystemId::ALL ordering matches dispatch_table positional array
3. Canonical order preserves existing system ordering rationale while leaving pre-existing ordinals intact

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/expectation_check.rs` — unit tests for `Active -> Overdue`, grace-window preservation, no-op cases, and dispatch routing
2. `crates/worldwake-sim/src/system_manifest.rs` — canonical-order test updated for `ExpectationCheck`

### Commands

1. `cargo test -p worldwake-sim system_manifest`
2. `cargo test -p worldwake-systems expectation_check`
3. `cargo test -p worldwake-sim`
4. `cargo test -p worldwake-systems`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`

## Outcome

Completed on 2026-04-06.

Implemented `SystemId::ExpectationCheck` in `crates/worldwake-sim/src/system_manifest.rs`, appended in declaration order to preserve existing ordinals while inserting it after `Perception` in `SystemManifest::canonical()`. Added the new `check_overdue_expectations` system in `crates/worldwake-systems/src/expectation_check.rs` and registered it from `crates/worldwake-systems/src/lib.rs` via the positional dispatch table.

Reassessment correction: the active spec/ticket text mixed two different boundaries. This ticket now owns only authoritative clock-driven `ExpectationState::Active -> Overdue` mutation through the normal `WorldTxn` state-delta event path. It does not add expected-place observation, subject lookup, violation creation, or a bespoke expectation event payload.

## Verification Result

Passed:

1. `cargo test -p worldwake-sim system_manifest`
2. `cargo test -p worldwake-systems expectation_check`
3. `cargo test -p worldwake-sim`
4. `cargo test -p worldwake-systems`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`
