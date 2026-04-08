# PERF-004: Eliminate per-call `Vec` allocation in `collect_control_rights`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core` ownership module
**Deps**: None (can be done independently of PERF-001, but combines well)

## Problem

`collect_control_rights` at `crates/worldwake-core/src/world/ownership.rs:209-263` allocates a `Vec<EffectiveRight>` on every call, even when the caller only needs a boolean (the `controlled_item_lots_for` filter path). The function is called for every item lot per agent per tick via `controlled_commodity_quantity` and `build_observed_entity_snapshot`.

Profiling shows `collect_control_rights` at **5.1%** self-time (2880 ticks, 74K samples). The function is recursive (follows container chains at line 211), allocating fresh `Vec`s at each recursion level.

## Assumption Reassessment (2026-04-07)

1. `collect_control_rights` at `ownership.rs:209-263` — confirmed via source read. Creates `let mut rights = Vec::new()` at line 220 on every call, pushes `EffectiveRight` structs, returns `ControlOutcome::Allowed(rights)`.
2. `controlled_item_lots_for` at `ownership.rs:14-20` calls `can_exercise_control(holder, *entity).is_ok()` — only needs a boolean result. The `Vec<EffectiveRight>` is constructed inside `collect_control_rights` and then discarded by `can_exercise_control` which only checks the variant.
3. `can_exercise_control` at `ownership.rs:165-184` matches `ControlOutcome::Allowed(_)` → `Ok(())` — the rights Vec is never read by this path.
4. `effective_rights` at `ownership.rs:187-200` does use the rights Vec — this is a separate, less-hot call path.
5. Container chain recursion at line 210-218 is typically 0-1 levels deep (items in containers), but allocates at each level.

## Architecture Check

1. Add a `has_control` boolean fast-path that mirrors `collect_control_rights` logic but returns `bool` without allocating. This is architecturally clean: the boolean path answers "can this actor control this entity?" without constructing the rights chain. `can_exercise_control` uses this fast path. `effective_rights` continues to use the full `collect_control_rights`. This follows FND-12 (performance may compress computation, never causality) — same causal outcome, less allocation.
2. Alternatively, use `SmallVec<[EffectiveRight; 2]>` in `collect_control_rights` since most calls produce 0-2 rights. This avoids heap allocation for the common case while preserving the existing API. Less invasive but smaller win.
3. No backwards-compatibility shims.

## Verification Layers

1. `has_control` returns same boolean as `can_exercise_control(...).is_ok()` → unit tests
2. `effective_rights` unchanged → existing ownership tests
3. Single-layer ticket scoped to `worldwake-core` ownership; no cross-system verification required.

## What to Change

### 1. Add `fn has_control(&self, actor: EntityId, entity: EntityId) -> bool`

Boolean-only version that walks the same logic as `collect_control_rights` but returns `true` at first found right, `false` otherwise. No Vec allocation.

### 2. Update `controlled_item_lots_for` to use `has_control`

Replace `self.can_exercise_control(holder, *entity).is_ok()` with `self.has_control(holder, *entity)`.

### 3. Optionally use `SmallVec` in `collect_control_rights`

If `effective_rights` callers remain performance-relevant, switch `Vec<EffectiveRight>` to `SmallVec<[EffectiveRight; 2]>` to avoid heap allocation for the common 0-2 rights case.

## Files to Touch

- `crates/worldwake-core/src/world/ownership.rs` (modify)

## Out of Scope

- Changing `can_exercise_control` error types (covered by PERF-001)
- Caching control results across queries within a tick

## Acceptance Criteria

### Tests That Must Pass

1. All existing ownership tests in `crates/worldwake-core/src/world/ownership.rs`
2. Existing suite: `cargo test -p worldwake-core`
3. Existing suite: `cargo test --workspace`

### Invariants

1. `has_control(actor, entity)` ≡ `can_exercise_control(actor, entity).is_ok()` for all inputs
2. `controlled_commodity_quantity` returns identical results

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world/ownership.rs` — add `has_control` test mirroring each `can_exercise_control` test to assert boolean equivalence

### Commands

1. `cargo test -p worldwake-core -- has_control`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completed on 2026-04-07.

- Added `pub fn has_control(&self, actor, entity) -> bool` and private `fn has_control_inner` to `ownership.rs`. Mirrors the logic of `collect_control_rights` but returns `true` at first found right without allocating `Vec<EffectiveRight>`.
- Updated `controlled_item_lots_for` and `controlled_unique_items_for` to use `has_control` instead of `can_exercise_control(..).is_ok()`.
- Added `has_control_agrees_with_can_exercise_control` equivalence test covering: unowned, direct ownership, possession override, faction authority, and office authority.
- Item 3 (SmallVec in `collect_control_rights`) deferred — the remaining callers (`effective_rights`, `can_exercise_control`) are not on the hot filter path.

## Verification Result

- Passed `cargo test -p worldwake-core` (full crate suite including new equivalence test)
- Passed `cargo clippy -p worldwake-core --all-targets -- -D warnings`
- Passed `cargo test --workspace`
