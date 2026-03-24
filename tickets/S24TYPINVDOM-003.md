# S24TYPINVDOM-003: Decompose observation_snapshot_changed into typed domains

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `observation_snapshot_changed()` return type, `ReadPhaseResult` field removal, planning function signature changes
**Deps**: S24TYPINVDOM-002 (runtime.dirty is DirtySet)

## Problem

`observation_snapshot_changed()` compares 6 dimensions (position, needs, wounds, commodity, unique_items, facilities) but returns a single `bool`. When snapshot change triggers a replan, traces show only `SnapshotChanged` without indicating which dimension changed. Additionally, `refresh_runtime_for_read_phase()` maintains a `Vec<DirtyReason>` in parallel with `runtime.dirty`, creating divergence risk. This ticket decomposes the snapshot function into typed domain bits and eliminates the dual-tracking pattern.

## Assumption Reassessment (2026-03-24)

1. `observation_snapshot_changed()` at `observation.rs:346-375` returns `bool`, comparing 6 dimensions in a single `||` chain. Confirmed.
2. `refresh_runtime_for_read_phase()` at `observation.rs:64-157` builds `Vec<DirtyReason>` separately from `runtime.dirty`. The sync line at `observation.rs:120` bridges them. Confirmed.
3. `ReadPhaseResult` at `observation.rs:44-61` has `dirty_reasons: Vec<DirtyReason>` field. Confirmed.
4. `dirty_reasons` flows from `ReadPhaseResult` into two consumers:
   - `plan_and_validate_next_step_traced()` at `planning.rs:380` as `dirty_reasons: &[DirtyReason]` parameter
   - `plan_and_validate_next_step()` at `planning.rs:251` as `dirty_reasons: &[DirtyReason]` parameter
   - These functions pass it to `is_snapshot_changed_only()` at `planning.rs:258` and `planning.rs:429`
   - Also flows into `PlanningPipelineTrace` construction at `mod.rs:581` — but that field is migrated in S24TYPINVDOM-004
5. `is_snapshot_changed_only()` at `planning.rs:351-356` checks if all reasons are `SnapshotChanged`. Replaced by `runtime.dirty.is_snapshot_only()`.
6. After S24TYPINVDOM-002, `runtime.dirty` is already `DirtySet`. The observation bridge inserted by -002 is removed here — `observation_snapshot_changed()` returns `DirtySet` directly.
7. The `dirty_reasons` parameter on both planning functions can be removed because `is_snapshot_changed_only(dirty_reasons)` is replaced by `runtime.dirty.is_snapshot_only()` which reads from the runtime directly.
8. `ReadPhaseResult.dirty_reasons` is consumed at `mod.rs:518` (passed to planning) and `mod.rs:581` (into trace). After removing the planning parameter, only the trace construction at `mod.rs:581` remains — handled in S24TYPINVDOM-004. This ticket keeps the field but populates it from a conversion of `runtime.dirty` to avoid blocking -004.
11. No mismatch.

## Architecture Check

1. Returning `DirtySet` from `observation_snapshot_changed()` eliminates the information collapse from 6 booleans to 1. Each comparison sets its named bit, making the invalidation contract self-documenting.
2. Removing the `Vec<DirtyReason>` local variable and the dual-tracking sync line eliminates the divergence risk identified in the spec's Motivation §2.
3. No backwards-compatibility shims.

## Verification Layers

1. Each of 6 snapshot dimensions maps to its named bit → unit test for `observation_snapshot_changed()` with per-dimension changes
2. `runtime.dirty` accumulates snapshot bits + structural bits in single pass → existing golden tests pass (behavioral equivalence)
3. `is_snapshot_only()` replaces `is_snapshot_changed_only()` → existing plan-continuation tests pass
4. `dirty_reasons` parameter removed from planning functions → compile-time verification (any caller still passing it fails)
5. `ReadPhaseResult.dirty_reasons` field retained temporarily with conversion bridge → trace construction at `mod.rs:581` still compiles

## What to Change

### 1. Change `observation_snapshot_changed()` return type

At `observation.rs:346`: change return from `bool` to `DirtySet`. Each of the 6 comparisons inserts its domain bit:
- `last_effective_place != view.effective_place(agent)` → `DirtySet::POSITION`
- `last_needs != view.homeostatic_needs(agent)` → `DirtySet::NEEDS`
- `last_wounds != view.wounds(agent)` → `DirtySet::WOUNDS`
- filtered commodity signature mismatch → `DirtySet::COMMODITY`
- `last_unique_item_signature != unique_item_signature(...)` → `DirtySet::UNIQUE_ITEMS`
- `last_facility_access_signature != facility_access_signature(...)` → `DirtySet::FACILITIES`

### 2. Eliminate dual-tracking in `refresh_runtime_for_read_phase()`

At `observation.rs:64-120`:
- Remove `let mut dirty_reasons = Vec::new();` and all `dirty_reasons.push(DirtyReason::...)` calls
- Replace with direct `runtime.dirty.insert(DirtySet::...)` for each structural reason:
  - No plan → `DirtySet::NO_PLAN`
  - Plan finished → `DirtySet::PLAN_FINISHED`
  - Replan signal → `DirtySet::REPLAN_SIGNAL`
  - Queue transition → `DirtySet::QUEUE_TRANSITION`
  - Blocker cleanup → `DirtySet::BLOCKER_CLEANUP`
  - Queue patience → `DirtySet::QUEUE_PATIENCE`
- Replace `let snapshot_changed = observation_snapshot_changed(...); if snapshot_changed { ... }` with `let snapshot_domains = observation_snapshot_changed(...); runtime.dirty.insert(snapshot_domains);`
- Remove the dual-tracking sync line at `observation.rs:120`
- Remove the observation-bridge helper introduced in S24TYPINVDOM-002

### 3. Temporarily retain `ReadPhaseResult.dirty_reasons` with conversion

Populate `dirty_reasons` field by converting `runtime.dirty` to a `Vec<DirtyReason>` (reverse mapping) so that `mod.rs:581` (trace construction) still compiles. This bridge is removed in S24TYPINVDOM-004 when `PlanningPipelineTrace.dirty_reasons` becomes `DirtySet`.

### 4. Remove `dirty_reasons` parameter from planning functions

- `plan_and_validate_next_step()` at `planning.rs:251`: remove `dirty_reasons: &[DirtyReason]` parameter
- `plan_and_validate_next_step_traced()` at `planning.rs:380`: remove `dirty_reasons: &[DirtyReason]` parameter
- Update call site at `mod.rs:518` to drop `&read_result.dirty_reasons` argument
- Update internal delegation at `planning.rs:406` to drop the argument

### 5. Replace `is_snapshot_changed_only()` with `runtime.dirty.is_snapshot_only()`

- `planning.rs:258`: `is_snapshot_changed_only(dirty_reasons)` → `runtime.dirty.is_snapshot_only()`
- `planning.rs:429`: same replacement
- Remove `is_snapshot_changed_only()` function at `planning.rs:351-356`

### 6. Update test call sites

Tests in `agent_tick/tests.rs` that pass `&[DirtyReason::NoPlan]` to planning functions (lines 1436, 2545, 2644) must drop that argument. The test at line 3702 that asserts on `read_result.dirty_reasons` needs updating to check the conversion bridge output or assert on `runtime.dirty` directly.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — return type change, dual-tracking removal, bridge)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — parameter removal, function removal, read-site replacement)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — call site argument removal)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — drop `dirty_reasons` arguments, update assertion)

## Out of Scope

- Removing `DirtyReason` enum (S24TYPINVDOM-004)
- Changing `PlanningPipelineTrace.dirty_reasons` to `DirtySet` (S24TYPINVDOM-004)
- Modifying trace `format_outcome()` or `summary()` (S24TYPINVDOM-004)
- Removing `DirtyReason` from `lib.rs` exports (S24TYPINVDOM-004)
- Touching `decision_runtime.rs`, `frame.rs`, `active_action.rs`, `failure_handling.rs` (already handled in S24TYPINVDOM-002)
- Touching any crate other than `worldwake-ai`

## Acceptance Criteria

### Tests That Must Pass

1. All existing golden tests pass unchanged — snapshot decomposition is purely diagnostic
2. Plan-continuation behavior preserved: when only snapshot bits change and current plan revalidates, agent continues plan (existing revalidation tests)
3. `is_snapshot_changed_only` function no longer exists (compile-time — any stale reference fails)
4. Planning functions no longer accept `dirty_reasons` parameter (compile-time)
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `observation_snapshot_changed()` returns `DirtySet` with exactly the bits for dimensions that changed — no information collapse
2. `runtime.dirty` accumulates all domain bits in a single pass — no dual-tracking between separate bool and vec
3. `is_snapshot_only()` on `DirtySet` is semantically equivalent to the removed `is_snapshot_changed_only()` — snapshot-only continuation behavior unchanged
4. The sync line at former `observation.rs:120` is removed — no dual-tracking divergence risk

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — update planning function call sites (remove `dirty_reasons` argument), update `read_result.dirty_reasons` assertion
2. Optional: add a focused test for `observation_snapshot_changed()` returning per-dimension bits (can be inline in `observation.rs` or in `tests.rs`)

### Commands

1. `cargo test -p worldwake-ai` — full crate regression
2. `cargo clippy -p worldwake-ai` — no new warnings
3. `cargo build --workspace` — cross-crate compilation check
