# E13DECARC-004: Goal model types (GoalKind, GoalKey, GoalPriorityClass, GroundedGoal)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — shared goal identity in worldwake-core, read-model types in worldwake-ai
**Deps**: E13DECARC-001

## Problem

The decision architecture needs a concrete goal model that enumerates exactly the Phase 2 goals, provides normalized identity keys for planning/switching/failure-memory, and defines priority classes. These types are used by nearly every subsequent ticket.

## Assumption Reassessment (2026-03-11)

1. `CommodityKind`, `EntityId`, `RecipeId` exist in `worldwake-core` — confirmed.
2. `BlockedIntentMemory` needs a persisted normalized goal identity in `worldwake-core`, so `CommodityPurpose`, `GoalKind`, and `GoalKey` cannot remain AI-only types.
3. No existing goal model in `worldwake-ai` — confirmed (crate is empty).
4. `BTreeSet` is required for determinism — confirmed.

## Architecture Check

1. Shared normalized goal identity (`CommodityPurpose`, `GoalKind`, `GoalKey`) lives in `worldwake-core`, because it is used by an authoritative core component (`BlockedIntentMemory`) and by AI planning logic.
2. `GoalPriorityClass` and `GroundedGoal` remain AI-layer read-model types in `worldwake-ai`.
3. `GoalKey` normalizes goal identity for comparisons across planning, switching, and failure memory.
4. `GroundedGoal` is a transient read-model struct, never stored as authoritative state.
5. Only Phase 2 goals are included — no law/policing/succession/rumor goals.

## What to Change

### 1. Consume shared goal identity from `worldwake-core`

This ticket no longer defines `CommodityPurpose`, `GoalKind`, or `GoalKey` in `worldwake-ai`.
Those shared types are owned by `worldwake-core` so both authoritative state and AI logic use one canonical definition.

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

Re-export `CommodityPurpose`, `GoalKind`, and `GoalKey` from `worldwake-core` through `worldwake-ai` if that improves call-site ergonomics, but do not create aliases or duplicate shadow types.

### 4. Implement Ord/PartialOrd for GoalPriorityClass

`Critical > High > Medium > Low > Background` — needed for deterministic sorting.

### 5. Derive traits

All AI-owned types: `Clone`, `Debug`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`.
`GoalPriorityClass`: additionally `Copy`, `Ord`, `PartialOrd`.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — was empty stub)
- `crates/worldwake-ai/src/lib.rs` (modify — re-exports)

## Out of Scope

- Defining `CommodityPurpose`, `GoalKind`, or `GoalKey` in `worldwake-ai`
- Candidate generation logic that produces `GroundedGoal` — E13DECARC-007
- Priority/motive scoring logic — E13DECARC-008
- Any goal kinds from Phase 3+ (law, policing, succession, rumor, party, expedition, persuasion, crime investigation, escort/raid/camp)
- Registering these types as components (they are NOT authoritative world state)

## Acceptance Criteria

### Tests That Must Pass

1. `worldwake-ai` consumes the shared `GoalKey` / `GoalKind` / `CommodityPurpose` definitions from `worldwake-core` without duplicating them
2. `GoalPriorityClass::Critical > GoalPriorityClass::High` (Ord)
3. `GroundedGoal` round-trips through bincode
4. Existing suite: `cargo test --workspace`

### Invariants

1. No Phase 3+ goal kinds present
2. `GroundedGoal` is never stored as a component
3. No duplicate `GoalKey` type exists in `worldwake-ai`
4. `BTreeSet<EntityId>` for evidence, not `HashSet`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — tests for `GoalPriorityClass` ordering, `GroundedGoal` serialization, and shared-type consumption

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
