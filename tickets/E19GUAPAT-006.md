# E19GUAPAT-006: Implement belief-driven patrol route adaptation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — route mutation logic in candidate generation or patrol system hook
**Deps**: E19GUAPAT-001 (PatrolRoute, PatrolProfile), E19GUAPAT-004 (patrol candidate generation exists)

## Problem

Guards must adapt their patrol routes based on their belief state: adding waypoints where crimes have been reported and deprioritizing stale waypoints. Route adaptation must read only from the guard's `ViolationMemory` and institutional beliefs (Principle 14), never from world state.

## Assumption Reassessment (2026-03-30)

1. `PatrolRoute.assigned_places: Vec<EntityId>` is the mutable route. Route adaptation modifies this via `WorldTxn`.
2. `PatrolProfile.route_adaptation_sensitivity: Permille` controls how aggressively waypoints are added. Low sensitivity = resist route changes.
3. `ViolationMemory` (in `crates/worldwake-core/src/violation.rs`) stores `RecordedViolation` entries with place information. Method `unresolved_records()` returns currently unresolved violations.
4. Spec says: crime report at new location adds that place to `assigned_places` if not already present, subject to `route_adaptation_sensitivity` threshold relative to report recency.
5. Spec says: waypoints without recent crime reports may be moved to end of route or skipped. No permanent removal — only reordering.
6. The spec places route adaptation in candidate generation or a per-tick patrol system hook. The cleaner approach is during candidate generation (when the guard's belief state is already being read) to avoid adding a new per-tick system.
7. Route adaptation requires `WorldTxn` for mutation. If placed in candidate generation, this is a departure from the read-only candidate generation pattern. Alternative: a lightweight patrol system function run per-tick before candidate generation.
8. `RecordedViolation` likely has a `place` or `location` field and a timestamp. Must verify exact fields during implementation.
9. No adjacent contradictions found.

## Architecture Check

1. **Option A**: Route adaptation in a per-tick `patrol_route_adaptation_system()` run before candidate generation. Cleaner separation of concerns (systems mutate state, candidates read state).
2. **Option B**: Route adaptation in candidate generation. Violates the read-only candidate generation convention.
3. **Recommendation**: Option A — a lightweight system function in `worldwake-systems` that runs per-tick, checks each guard's `ViolationMemory` for new crime reports at locations not on their route, and modifies `PatrolRoute.assigned_places` accordingly. This follows the pattern of other per-tick systems like `needs_system()` and `trade_system_tick()`.
4. No backwards-compatibility shims.

## Verification Layers

1. Waypoint addition after crime report → focused unit test: guard receives crime report at new location, route grows
2. Sensitivity threshold enforcement → focused unit test: low-sensitivity guard ignores crime report
3. Waypoint deprioritization → focused unit test: stale waypoints reordered to end
4. Belief-only guarantee → focused unit test: guard's route unchanged by crimes they haven't heard about
5. Route mutation via WorldTxn → action trace or authoritative world state check

## What to Change

### 1. New function `patrol_route_adaptation()` in `crates/worldwake-systems/src/patrol_actions.rs`

Or a new file `crates/worldwake-systems/src/patrol.rs` for system-level patrol logic:

```rust
pub fn patrol_route_adaptation_system(world: &mut World, _tick: Tick) {
    // For each agent with PatrolRoute + PatrolProfile:
    //   1. Read agent's ViolationMemory
    //   2. For each unresolved violation at a place not in assigned_places:
    //      a. Check route_adaptation_sensitivity vs recency threshold
    //      b. If exceeds threshold: add place to assigned_places
    //   3. For waypoints in assigned_places with no recent violations:
    //      a. Move to end of route (deprioritize)
    //   4. Commit changes via WorldTxn
}
```

### 2. Register system in tick execution

Add `patrol_route_adaptation_system` to the system manifest / dispatch so it runs each tick. It should run after Perception (so guards have fresh violation memories) but the exact ordering needs to be determined during implementation.

### 3. Define recency/staleness thresholds

Define constants or derive from `PatrolProfile.route_adaptation_sensitivity`:
- Recency threshold for adding waypoints (recent crimes trigger additions)
- Staleness threshold for deprioritizing waypoints (old quiet areas get deprioritized)

## Files to Touch

- `crates/worldwake-systems/src/patrol_actions.rs` (modify — add route adaptation logic) OR `crates/worldwake-systems/src/patrol.rs` (new — if system logic is separated from action handler)
- `crates/worldwake-sim/src/system_manifest.rs` (modify — register patrol adaptation system)
- `crates/worldwake-sim/src/system_dispatch.rs` (modify — route dispatch to patrol system)

## Out of Scope

- Patrol action handler (E19GUAPAT-003 — already delivered)
- Patrol candidate generation (E19GUAPAT-004 — already delivered)
- Guard presence factor (E19GUAPAT-005)
- Captain-mediated route reassignment (deferred per spec to future epic)
- Permanent waypoint removal (spec explicitly defers this)
- Golden E2E tests (E19GUAPAT-007)

## Acceptance Criteria

### Tests That Must Pass

1. Guard who receives crime report at a new location has that location added to `assigned_places`
2. Guard with low `route_adaptation_sensitivity` does NOT add new location after crime report
3. Guard with high `route_adaptation_sensitivity` adds new location after crime report
4. Guard who has not received crime reports at a waypoint for N ticks sees that waypoint moved to end of route
5. Guard at remote location with no crime reports has unchanged route (Principle 7/14 — no world-state leakage)
6. Route adaptation never permanently removes waypoints (spec constraint)
7. Route changes go through WorldTxn
8. Existing suite: `cargo test -p worldwake-systems`
9. `cargo clippy --workspace`

### Invariants

1. Route adaptation reads ONLY from guard's belief state (ViolationMemory, institutional beliefs) — never from World directly (Principle 14)
2. No waypoints permanently removed — only reordered (spec constraint)
3. `assigned_places` uses `Vec<EntityId>` — order matters for patrol sequence
4. No `HashMap` or `f32`/`f64` introduced
5. System runs deterministically (BTreeMap iteration, seeded RNG if any randomness)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/patrol_actions.rs` or `crates/worldwake-systems/tests/` — focused tests for route adaptation logic
2. Tests for sensitivity threshold behavior
3. Tests for staleness-based deprioritization

### Commands

1. `cargo test -p worldwake-systems -- patrol`
2. `cargo test -p worldwake-systems -- route_adaptation`
3. `cargo clippy --workspace && cargo test --workspace`
