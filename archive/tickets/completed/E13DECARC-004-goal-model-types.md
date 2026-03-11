# E13DECARC-004: Goal model types (GoalKind, GoalKey, GoalPriorityClass, GroundedGoal)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — shared goal identity in worldwake-core, read-model types in worldwake-ai
**Deps**: E13DECARC-001

## Problem

The decision architecture needs a concrete goal model that enumerates exactly the Phase 2 goals, provides normalized identity keys for planning/switching/failure-memory, and defines priority classes. These types are used by nearly every subsequent ticket.

## Assumption Reassessment (2026-03-11)

1. `CommodityKind`, `EntityId`, `RecipeId` exist in `worldwake-core` — confirmed.
2. `BlockedIntentMemory` already persists normalized goal identity from `worldwake-core`, and `CommodityPurpose`, `GoalKind`, and `GoalKey` already exist there — confirmed.
3. No AI-layer goal read-model exists in `worldwake-ai` yet — confirmed (`worldwake-ai` still only has the E13 dependency smoke test).
4. `BTreeSet` is required for determinism — confirmed.
5. `worldwake-ai/Cargo.toml` already has the required `worldwake-sim` dependency from E13DECARC-001, so this ticket does not need Cargo wiring.

## Architecture Check

1. Shared normalized goal identity (`CommodityPurpose`, `GoalKind`, `GoalKey`) lives in `worldwake-core`, because it is used by an authoritative core component (`BlockedIntentMemory`) and by AI planning logic.
2. `GoalPriorityClass` and `GroundedGoal` remain AI-layer read-model types in `worldwake-ai`.
3. `GoalKey` normalizes goal identity for comparisons across planning, switching, and failure memory.
4. `GroundedGoal` is a transient read-model struct, never stored as authoritative state.
5. Only Phase 2 goals are included — no law/policing/succession/rumor goals.

## Scope Correction

This ticket should:

1. Add the missing AI-layer goal read-model types in `worldwake-ai`.
2. Re-export the canonical shared goal-identity types from `worldwake-ai` only if that keeps call sites cleaner without introducing duplicate ownership or alias behavior.
3. Add focused tests for deterministic ordering, serialization, and canonical shared-type usage.

This ticket should not:

1. Re-implement `CommodityPurpose`, `GoalKind`, or `GoalKey` in any crate.
2. Re-open completed `worldwake-core` work from E13DECARC-003.
3. Add runtime planner state, candidate generation, motive scoring, or authoritative component registration.
4. Introduce compatibility wrappers, shadow types, or alias types around the canonical goal identity.

## What to Change

### 1. Consume shared goal identity from `worldwake-core`

This ticket does not define `CommodityPurpose`, `GoalKind`, or `GoalKey`.
Those shared types are already implemented in `worldwake-core` and must remain the single canonical definition used by both authoritative memory and AI logic.

### 2. Define AI-layer read-model types in `worldwake-ai/src/goal_model.rs`

```rust
pub enum GoalPriorityClass {
    Critical,
    High,
    Medium,
    Low,
    Background,
}

pub struct GroundedGoal {
    pub key: GoalKey,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
    pub evidence_entities: BTreeSet<EntityId>,
    pub evidence_places: BTreeSet<EntityId>,
}
```

### 3. Re-export shared goal identity from `worldwake-ai`

Re-export `CommodityPurpose`, `GoalKind`, and `GoalKey` directly from `worldwake-core` through `worldwake-ai` if that improves call-site ergonomics, but do not create aliases, wrappers, or duplicate shadow types.

### 4. Implement Ord/PartialOrd for GoalPriorityClass

`Critical > High > Medium > Low > Background` — needed for deterministic sorting.

### 5. Derive traits

All AI-owned types: `Clone`, `Debug`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`.
`GoalPriorityClass`: additionally `Copy`, `Ord`, `PartialOrd`.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — module + re-exports)

## Out of Scope

- Defining `CommodityPurpose`, `GoalKind`, or `GoalKey` in `worldwake-ai`
- Modifying `worldwake-core/src/goal.rs` or `worldwake-core/src/blocked_intent.rs`
- Candidate generation logic that produces `GroundedGoal` — E13DECARC-007
- Priority/motive scoring logic — E13DECARC-008
- Any goal kinds from Phase 3+ (law, policing, succession, rumor, party, expedition, persuasion, crime investigation, escort/raid/camp)
- Registering these types as components (they are NOT authoritative world state)

## Acceptance Criteria

### Tests That Must Pass

1. `worldwake-ai` consumes the shared `GoalKey` / `GoalKind` / `CommodityPurpose` definitions from `worldwake-core` without duplicating them
2. `GoalPriorityClass::Critical > GoalPriorityClass::High` (Ord)
3. `GroundedGoal` round-trips through bincode
4. `GoalPriorityClass` / `GroundedGoal` satisfy the required trait bounds
5. Existing suite: `cargo test --workspace`

### Invariants

1. No Phase 3+ goal kinds present
2. `GroundedGoal` is never stored as a component
3. No duplicate `GoalKey` type exists in `worldwake-ai`
4. `BTreeSet<EntityId>` for evidence, not `HashSet`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — tests for `GoalPriorityClass` ordering, trait bounds, `GroundedGoal` serialization, and canonical shared-type consumption

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Outcome amended: 2026-03-11

- Completion date: 2026-03-11
- What actually changed:
  - Corrected the ticket scope to reflect that the canonical shared goal identity (`CommodityPurpose`, `GoalKind`, `GoalKey`) was already implemented in `worldwake-core`.
  - Added `crates/worldwake-ai/src/goal_model.rs` with the transient AI read-model types `GoalPriorityClass` and `GroundedGoal`.
  - Re-exported `CommodityPurpose`, `GoalKind`, and `GoalKey` directly from `worldwake-core` through `worldwake-ai`, alongside the new AI-owned goal-model types.
  - Added focused unit tests for deterministic priority ordering, trait bounds, `GroundedGoal` bincode roundtrip, and canonical shared-type consumption through the `worldwake-ai` crate surface.
  - Added the minimum `worldwake-ai` dependencies needed for the declared serialization contract: `serde` in dependencies and `bincode` in dev-dependencies.
- Deviations from original plan:
  - Did not re-implement or modify the shared goal identity in `worldwake-core`; that work was already completed by E13DECARC-003 and reopening it would have been redundant architectural churn.
  - Created `goal_model.rs` as a new module instead of modifying a nonexistent stub file.
  - Kept the `worldwake-ai` crate surface thin and direct: simple re-exports only, with no wrapper types or compatibility aliasing.
  - A later 2026-03-11 architecture refinement narrowed `GroundedGoal` to evidence-only data and introduced `RankedGoal` as the separate ranking-layer read model. This preserved the ticket's layering goal while removing cross-phase coupling between grounding and scoring.
- Verification results:
  - `cargo test -p worldwake-ai` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
