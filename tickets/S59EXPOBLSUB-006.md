# S59EXPOBLSUB-006: SystemId::ExpectationCheck + overdue detection SystemFn

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — SystemId enum, system manifest canonical order, dispatch table, new SystemFn
**Deps**: S59EXPOBLSUB-002

## Problem

Expectations transition from Active to Overdue based on tick deadlines, but no system currently checks this. A new SystemFn is needed to scan each agent's `ExpectationStore` and transition stale expectations. This must run after Perception (so beliefs are current) and before goal generation (so overdue state feeds into search goals).

## Assumption Reassessment (2026-04-06)

1. `SystemId` defined via `define_system_ids!` macro at `crates/worldwake-sim/src/system_manifest.rs:54-66`. Currently 11 variants.
2. `SystemManifest::canonical()` at `system_manifest.rs:100-114` defines the authoritative tick order. ExpectationCheck inserts between Perception (position 8) and EvidenceDecay (position 9 in canonical).
3. `SystemDispatchTable` at `crates/worldwake-sim/src/system_dispatch.rs:47` is a positional array `[SystemFn; SYSTEM_COUNT]`. Adding a SystemId increases SYSTEM_COUNT.
4. `dispatch_table()` at `crates/worldwake-systems/src/lib.rs:76-89` lists handler functions positionally matching `SystemId::ALL`. New entry needed.
5. `SystemFn` signature: `fn(SystemExecutionContext<'_>) -> Result<(), SystemError>`. Context provides world, event_log, rng, tick.

## Architecture Check

1. Adding a SystemId + SystemFn follows the exact established pattern. The canonical ordering rationale (after Perception, before EvidenceDecay) is documented in the spec.
2. No backward compatibility shims. The dispatch table is rebuilt at startup.

## Verification Layers

1. ExpectationCheck runs in correct position → system ordering test in system_manifest.rs
2. Active→Overdue transition fires at correct tick → focused unit test with mock world
3. Expectations within grace period remain Active → focused unit test
4. Single-system ticket — no cross-system layer mapping needed.

## What to Change

### 1. Add SystemId::ExpectationCheck

In `crates/worldwake-sim/src/system_manifest.rs`, add `(ExpectationCheck, "expectation_check")` to the `define_system_ids!` macro.

### 2. Update canonical order

In `SystemManifest::canonical()`, insert `SystemId::ExpectationCheck` between `SystemId::Perception` and `SystemId::EvidenceDecay`. Update ordering comments.

### 3. Implement check_overdue_expectations

Create `crates/worldwake-systems/src/expectation_check.rs`:

```rust
pub fn check_overdue_expectations(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    // For each agent with ExpectationStore:
    //   For each record where state == Active:
    //     If ctx.tick > record.deadline_tick + record.grace_ticks:
    //       Transition state to Overdue
    //       Log state transition event
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

1. Overdue detection is a pure clock check — no global entity lookup (P7)
2. SystemId::ALL ordering matches dispatch_table positional array
3. Canonical order preserves existing system ordering rationale

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/expectation_check.rs` — unit tests for Active→Overdue transitions
2. `crates/worldwake-sim/src/system_manifest.rs` — ordering test updated for new SystemId

### Commands

1. `cargo test -p worldwake-systems expectation && cargo test -p worldwake-sim system_manifest`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
