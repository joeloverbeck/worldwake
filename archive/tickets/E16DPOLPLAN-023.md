# E16DPOLPLAN-023: Support-aware planning snapshot and hypothetical support counting

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planning_snapshot.rs, planning_state.rs
**Deps**: E16DPOLPLAN-022

## Problem

The GOAP planner has no way to count support declarations during hypothetical plan evaluation. Without this, `is_satisfied` for `ClaimOffice` cannot determine whether the actor has built a winning coalition. The planner needs:

1. A snapshot of existing support declarations (base world state captured at planning time)
2. A hypothetical support count that accounts for both snapshot declarations and overrides added by Bribe/DeclareSupport planning steps

## Assumption Reassessment (2026-03-18)

1. `PlanningSnapshot` captures per-entity data in `SnapshotEntity` and per-place data — confirmed (planning_snapshot.rs).
2. `PlanningState` already has `support_declaration_overrides: BTreeMap<(EntityId, EntityId), EntityId>` for hypothetical declarations written by `with_support_declaration()` — confirmed (planning_state.rs).
3. `PlanningState::support_declaration(supporter, office)` reads from overrides then snapshot — confirmed (planning_state.rs:1232-1242).
4. `build_planning_snapshot` calls `build_snapshot_entity` for each entity in the planning set — confirmed. Office entities may or may not be in the planning set depending on evidence_entities.
5. `GoalKind::ClaimOffice { office }` carries the office EntityId — confirmed (goal_model.rs).

## Architecture Check

1. **Principle 3 (Concrete State Over Abstract Scores)**: Support counts are derived from actual support declaration relations, not abstract "political power" scores.
2. **Principle 25 (Derived Summaries Are Caches)**: The snapshot data is captured once at planning time and the hypothetical count is recomputed from declarations + overrides. No derived value becomes authoritative.
3. **Principle 12 (World State ≠ Belief State)**: Snapshot data comes from the belief view (E16DPOLPLAN-022), not world state directly.
4. Reuses existing `support_declaration_overrides` infrastructure — no new PlanningState fields for overrides. Only adds a base snapshot and a counting method.
5. No backwards-compatibility shims.

## What to Change

### 1. Add per-office support snapshot to `PlanningSnapshot` (planning_snapshot.rs)

Add a new field to `PlanningSnapshot`:

```rust
/// Base support declarations per office: (supporter, candidate) pairs.
/// Captured at snapshot build time from belief view.
pub(crate) office_support_declarations: BTreeMap<EntityId, Vec<(EntityId, EntityId)>>,
```

Populate during `PlanningSnapshot::build()`: for each office entity in the snapshot entity set, call `view.support_declarations_for_office(office)` and store the result.

Note: The snapshot must include office entities. The `ClaimOffice { office }` goal provides the office EntityId through `evidence_entities` in `GroundedGoal`. Verify that `build_planning_snapshot` receives this in the evidence set.

### 2. Add `base_support_declarations_for_office` accessor to `PlanningSnapshot`

```rust
pub(crate) fn base_support_declarations_for_office(
    &self,
    office: EntityId,
) -> &[(EntityId, EntityId)] {
    self.office_support_declarations
        .get(&office)
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}
```

### 3. Add `hypothetical_support_count` to `PlanningState` (planning_state.rs)

```rust
/// Count hypothetical support declarations for `candidate` at `office`,
/// combining base snapshot declarations with planning overrides.
pub(crate) fn hypothetical_support_count(
    &self,
    office: EntityId,
    candidate: EntityId,
) -> usize {
    let base_declarations = self.snapshot.base_support_declarations_for_office(office);

    // Start with base declarations, applying any overrides
    let mut count = 0usize;
    for &(supporter, base_candidate) in base_declarations {
        let effective_candidate = self
            .support_declaration_overrides
            .get(&(supporter, office))
            .copied()
            .unwrap_or(base_candidate);
        if effective_candidate == candidate {
            count += 1;
        }
    }

    // Add purely hypothetical declarations (supporters NOT in base)
    for (&(supporter, decl_office), &decl_candidate) in &self.support_declaration_overrides {
        if decl_office == office
            && decl_candidate == candidate
            && !base_declarations.iter().any(|(s, _)| *s == supporter)
        {
            count += 1;
        }
    }

    count
}
```

### 4. Add `has_support_majority` convenience method to `PlanningState`

```rust
/// Returns true if `candidate` has strictly more hypothetical support
/// declarations than every other candidate for `office`.
pub(crate) fn has_support_majority(&self, office: EntityId, candidate: EntityId) -> bool {
    let actor_count = self.hypothetical_support_count(office, candidate);
    if actor_count == 0 {
        return false;
    }

    // Collect all known candidates (from base + overrides)
    let base = self.snapshot.base_support_declarations_for_office(office);
    let mut all_candidates = BTreeSet::new();
    for &(_, c) in base {
        all_candidates.insert(c);
    }
    for (&(_, decl_office), &c) in &self.support_declaration_overrides {
        if decl_office == office {
            all_candidates.insert(c);
        }
    }

    // Actor must have strictly more than every other candidate
    all_candidates
        .into_iter()
        .filter(|&c| c != candidate)
        .all(|c| self.hypothetical_support_count(office, c) < actor_count)
}
```

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — new field, population, accessor)
- `crates/worldwake-ai/src/planning_state.rs` (modify — counting + majority methods)

## Out of Scope

- Changes to `is_satisfied` or `is_progress_barrier` (separate ticket E16DPOLPLAN-024)
- Changes to search algorithm (separate ticket E16DPOLPLAN-025)
- Belief-gated filtering of support declarations (post-E14)

## Acceptance Criteria

### Tests That Must Pass

1. `hypothetical_support_count` returns correct count from base snapshot alone (no overrides)
2. `hypothetical_support_count` accounts for overrides that change existing declarations
3. `hypothetical_support_count` accounts for purely hypothetical new declarations
4. `has_support_majority` returns `true` when actor has strictly more support than all competitors
5. `has_support_majority` returns `false` on ties
6. `has_support_majority` returns `false` when actor has 0 support
7. `has_support_majority` returns `true` when actor has 1 support and no competitors
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Support counting is purely derived — no new authoritative state stored
2. Base declarations are captured once at snapshot build time, never mutated
3. Overrides use the existing `support_declaration_overrides` infrastructure (no new override maps)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` — test base support declaration capture
2. `crates/worldwake-ai/src/planning_state.rs` — test `hypothetical_support_count` with base, overrides, and mixed scenarios
3. `crates/worldwake-ai/src/planning_state.rs` — test `has_support_majority` edge cases

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-18
- **What changed**:
  - `planning_snapshot.rs`: Added `office_support_declarations` field to `PlanningSnapshot`, populated during `build()` from belief view. Added `base_support_declarations_for_office()` accessor.
  - `planning_state.rs`: Added `hypothetical_support_count(office, candidate)` merging base snapshot with overrides, and `has_support_majority(office, candidate)` for strict majority check.
- **Deviations from original plan**: The ticket assumed `support_declaration_overrides` has value type `EntityId`, but the actual type is `Option<EntityId>` (where `Some(None)` = withdrawn). The counting logic was adapted to handle all three cases correctly.
- **Verification**: 8 new tests pass, full workspace suite (1924 tests) passes, `cargo clippy --workspace` clean.
