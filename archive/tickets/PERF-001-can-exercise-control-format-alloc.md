# PERF-001: Eliminate `format!` allocation on error path of `can_exercise_control`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core` ownership module
**Deps**: None

## Problem

`can_exercise_control` is called on every item lot in the world per agent per tick (via `controlled_item_lots_for` → `controlled_commodity_quantity`), both in perception's `build_observed_entity_snapshot` and in the AI's `update_runtime_observation_snapshot`. The function's error path (`BlockedByPossessor`, `NoRights`) calls `format!` to produce a `WorldError::PreconditionFailed(String)`. The caller immediately discards the error via `.is_ok()`.

Profiling (2880-tick flamegraph, 74K samples) shows `can_exercise_control` + `collect_control_rights` consuming **~12.5%** of total runtime, with **~7.6%** spent in `format_inner` / `core::fmt::write` on the error path alone. This makes string formatting the single largest leaf-level hotspot in the simulation.

## Assumption Reassessment (2026-04-07)

1. `controlled_item_lots_for` at `crates/worldwake-core/src/world/ownership.rs:14-20` calls `self.can_exercise_control(holder, *entity).is_ok()` for every item lot — confirmed via profiling and source read.
2. `can_exercise_control` at `ownership.rs:165-184` uses `format!` at lines 176 and 180 to build error strings that callers in `controlled_item_lots_for` and `controlled_commodity_quantity` discard.
3. `collect_control_rights` at `ownership.rs:209-263` allocates `Vec<EffectiveRight>` on every call, even on the hot "filter lots" path where only the `is_ok()` boolean matters.
4. Two primary call sites consume most of the runtime: perception's `build_observed_entity_snapshot` (8.3% total) and AI's `update_runtime_observation_snapshot` → `controlled_commodity_quantity` (7.5% total).
5. No golden test or invariant depends on the error message text — callers use `.is_ok()`, `.is_err()`, or match on the `WorldError` variant.

## Architecture Check

1. Replace `format!` with a non-allocating error variant. `WorldError::PreconditionFailed` currently wraps `String`. The clean approach is to add a structured variant (e.g., `ControlDenied { entity, reason: ControlDeniedReason }`) that carries the entity IDs without allocation. This is cleaner than lazy formatting because it preserves debuggability (FND-29) while eliminating allocation pressure. The `Display` impl can format the string on demand when actually printed.
2. No backwards-compatibility shims — the existing `PreconditionFailed(String)` variant remains for other callers; the control path gets its own variant.

## Verification Layers

1. No allocation on control-denied path → profiling comparison (before/after flamegraph)
2. Error messages still readable when printed → existing test coverage of `can_exercise_control` in `crates/worldwake-core/src/world/ownership.rs` (tests at line 3512+)
3. Single-layer ticket scoped to `worldwake-core`; no cross-system verification required.

## What to Change

### 1. Add `ControlDenied` variant to `WorldError`

Add a non-allocating error variant:
```rust
ControlDenied {
    actor: EntityId,
    entity: EntityId,
    reason: ControlDeniedReason,
}
```
Where `ControlDeniedReason` is `BlockedByPossessor(EntityId)` or `NoRights`. Implement `Display` to produce the same human-readable message.

### 2. Update `can_exercise_control` to use the new variant

Replace the two `format!` calls at `ownership.rs:176` and `ownership.rs:180` with the structured variant.

### 3. Consider a `has_control` boolean fast path

Add `pub fn has_control(&self, actor: EntityId, entity: EntityId) -> bool` that returns a boolean without constructing error or rights details, for use in filter contexts like `controlled_item_lots_for`. This avoids even the `Result` and `Vec<EffectiveRight>` overhead on the hot filter path.

## Files to Touch

- `crates/worldwake-core/src/world/ownership.rs` (modify)
- `crates/worldwake-core/src/world.rs` or `crates/worldwake-core/src/error.rs` (modify — wherever `WorldError` is defined)

## Out of Scope

- Refactoring `collect_control_rights` to avoid Vec allocation (separate ticket PERF-002)
- Broader ownership query optimization

## Acceptance Criteria

### Tests That Must Pass

1. All existing `can_exercise_control` tests in `crates/worldwake-core/src/world/ownership.rs`
2. Existing suite: `cargo test -p worldwake-core`
3. Existing suite: `cargo test --workspace`

### Invariants

1. `can_exercise_control` returns `Err` for the same inputs as before (behavioral equivalence)
2. Error messages remain human-readable when formatted via `Display`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world/ownership.rs` — update existing control tests if error matching changes from string comparison to variant matching

### Commands

1. `cargo test -p worldwake-core -- can_exercise_control`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completed on 2026-04-07.

- Added `ControlDeniedReason` enum (`BlockedByPossessor(EntityId)`, `NoRights`) and `WorldError::ControlDenied { actor, entity, reason }` variant to `crates/worldwake-core/src/error.rs`. `Display` impl produces identical human-readable messages on demand (no allocation on error construction).
- Replaced two `format!` calls in `can_exercise_control` (`ownership.rs:176`, `180`) with the structured variant.
- Added `ControlDenied` arm to `map_reservation_error` in `start_gate.rs`.
- Updated 6 test assertions in `world.rs` from `PreconditionFailed(_)` to `ControlDenied { .. }`.
- Exported `ControlDeniedReason` from `worldwake-core` crate root.
- Item 3 from "What to Change" (`has_control` boolean fast path) deferred — the `Vec<EffectiveRight>` allocation in `collect_control_rights` is scoped to PERF-004.

## Verification Result

- Passed `cargo test -p worldwake-core -- can_exercise_control` (10 tests)
- Passed `cargo test -p worldwake-core` (full crate suite)
- Passed `cargo clippy -p worldwake-core --all-targets -- -D warnings`
- Passed `cargo clippy -p worldwake-sim --all-targets -- -D warnings`
- Passed `cargo test --workspace`
- Note: `cargo clippy --workspace --all-targets -- -D warnings` has pre-existing failures in untracked `perf_diag.rs` binary, unrelated to this ticket.
