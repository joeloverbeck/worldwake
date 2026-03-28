# S33OPPSCOGOAIDE-001: Define OpportunityAnchor and OpportunityKey types in worldwake-core

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new types in worldwake-core goal module
**Deps**: S31 ✅, S23 ✅, S22 ✅

## Problem

S33 requires opportunity-scoped identity for goals to separate desire-level identity (`GoalKey`) from opportunity-level identity. No `OpportunityAnchor` or `OpportunityKey` types exist yet. All downstream tickets depend on these foundational types.

## Assumption Reassessment (2026-03-28)

1. `GoalKey` is defined at `crates/worldwake-core/src/goal.rs` with fields `{ kind, commodity, entity, place }`. It currently derives `Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize` but not `Hash`. The original ticket overstated the live trait set and is corrected below.
2. No `OpportunityAnchor` or `OpportunityKey` types exist anywhere in the codebase. Verified via grep.
3. `EntityId` is imported from `crates/worldwake-core/src/ids.rs` and already derives all necessary traits (`Copy, Clone, Debug, Eq, Ord, Hash, Serialize, Deserialize`).
4. This ticket is additive at the architecture boundary, but not literally "new types only": it also updates `worldwake-core` exports and focused core tests.
5. Not a planner/golden-driven ticket; purely core identity-type work. The downstream planner, candidate-generation, exhaustion, and save/load behavior changes remain in later S33 tickets.
6. N/A — no AI regression, no ordering, no heuristic removal, no stale-request, no political office, no ControlSource, no golden scenario, no adjacent contradictions, no cumulative arithmetic.

## Architecture Check

1. Placing `OpportunityAnchor` and `OpportunityKey` in `worldwake-core::goal` alongside `GoalKey` keeps all goal-identity types co-located. The alternative — placing them in `worldwake-ai` — would force later save/load and runtime plumbing to depend on AI-only symbols. Core identity types belong in core.
2. The cleaner design is to mirror the current `GoalKey` value-bound contract (`Eq`/`Ord`/serde) instead of widening the entire existing goal identity stack to add dormant hash support. Worldwake's live deterministic collections are `BTreeMap`/`BTreeSet`; forcing `Hash` into this ticket would expand scope without improving the actual architecture under delivery.
3. No backward-compatibility shims. Add the new types directly and export them from `worldwake-core`.

## Verification Layers

1. Type derives correct trait set → focused compile test + `#[test]` exercising `Ord` and `Serialize`/`Deserialize` round-trip.
2. `OpportunityKey` composes `GoalKey` + `OpportunityAnchor` correctly → unit test constructing all anchor variants.
3. Single-layer ticket (type definitions only); additional layer mapping not applicable.

## What to Change

### 1. Add `OpportunityAnchor` enum to `crates/worldwake-core/src/goal.rs`

```rust
/// Concrete world-state anchor distinguishing one opportunity from another
/// for the same desire (GoalKey).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OpportunityAnchor {
    Place(EntityId),
    Entity(EntityId),
    None,
}
```

### 2. Add `OpportunityKey` struct to `crates/worldwake-core/src/goal.rs`

```rust
/// Identifies a specific opportunity: a desire + the concrete anchor being pursued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OpportunityKey {
    pub goal_key: GoalKey,
    pub anchor: OpportunityAnchor,
}
```

### 3. Export from `worldwake-core` lib

Add `OpportunityAnchor` and `OpportunityKey` to the public exports of `worldwake-core`.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — add two types)
- `crates/worldwake-core/src/lib.rs` (modify — export new types)

## Out of Scope

- `GroundedGoal` changes (S33OPPSCOGOAIDE-002)
- Exhaustion cache re-keying (S33OPPSCOGOAIDE-004)
- `PlannedPlan` changes (S33OPPSCOGOAIDE-006)
- Save/load changes (S33OPPSCOGOAIDE-008)
- Any changes to `worldwake-ai` or `worldwake-sim`
- `Route { from, to }` variant (YAGNI per spec)

## Acceptance Criteria

### Tests That Must Pass

1. `OpportunityAnchor::Place(id)`, `OpportunityAnchor::Entity(id)`, `OpportunityAnchor::None` construct and compare via `Eq`/`Ord`.
2. `OpportunityKey { goal_key, anchor }` round-trips through `serde_json` (or bincode) serialize/deserialize.
3. `BTreeMap<OpportunityKey, ()>` can be constructed and iterated deterministically (proves `Ord` works correctly).
4. Existing suite: `cargo test -p worldwake-core`
5. Existing suite: `cargo clippy --workspace`

### Invariants

1. `OpportunityAnchor` and `OpportunityKey` must derive exactly: `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize`.
2. `OpportunityKey.goal_key` field is of type `GoalKey` (same type used throughout the goal system).
3. No `HashMap`/`HashSet` usage is introduced — the new types support `BTreeMap`/`BTreeSet` via `Ord`, which matches the live deterministic collection policy.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — `opportunity_goal_identity_types_satisfy_value_bounds` — verifies the new types match the live `GoalKey` value-bound contract so downstream crates can use them like existing goal identity values.
2. `crates/worldwake-core/src/goal.rs` — `opportunity_anchor_ordering_is_stable` — verifies `Ord` across all variants so deterministic `BTreeMap`/`BTreeSet` usage is explicit rather than assumed.
3. `crates/worldwake-core/src/goal.rs` — `opportunity_key_bincode_roundtrip_preserves_anchor_and_goal` — verifies serialize/deserialize fidelity with the repository's existing binary serialization path that later S33 runtime/save-load tickets will rely on.
4. `crates/worldwake-core/src/goal.rs` — `opportunity_key_btreemap_iteration_is_deterministic` — verifies deterministic iteration and that `OpportunityKey` composes `GoalKey` plus anchor ordering correctly.

### Commands

1. `cargo test -p worldwake-core opportunity`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-28
- Added `OpportunityAnchor` and `OpportunityKey` to `crates/worldwake-core/src/goal.rs` and exported both from `crates/worldwake-core/src/lib.rs`.
- Strengthened focused core tests for value bounds, stable ordering, bincode round-trip, and deterministic `BTreeMap` iteration.
- Deviation from original plan: the ticket was corrected to mirror the live `GoalKey` trait contract instead of forcing `Hash` into this slice of work. That kept the change additive and core-scoped rather than widening the existing goal identity stack without a current architectural need.
- Verification results:
  - `cargo test -p worldwake-core opportunity` ✅
  - `cargo test -p worldwake-core` ✅
  - `cargo clippy --workspace` ✅
