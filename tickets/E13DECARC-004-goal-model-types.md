# E13DECARC-004: Goal model types (GoalKind, GoalKey, GoalPriorityClass, GroundedGoal)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — AI-layer types only
**Deps**: E13DECARC-001

## Problem

The decision architecture needs a concrete goal model that enumerates exactly the Phase 2 goals, provides normalized identity keys for planning/switching/failure-memory, and defines priority classes. These types are used by nearly every subsequent ticket.

## Assumption Reassessment (2026-03-11)

1. `CommodityKind`, `EntityId`, `RecipeId` exist in `worldwake-core` — confirmed.
2. `CommodityPurpose` does not yet exist anywhere — must be created.
3. No existing goal model in `worldwake-ai` — confirmed (crate is empty).
4. `BTreeSet` is required for determinism — confirmed.

## Architecture Check

1. These types live in `worldwake-ai`, not `worldwake-core`, because they are AI-layer concepts.
2. `GoalKey` normalizes goal identity for comparisons across planning, switching, and failure memory.
3. `GroundedGoal` is a transient read-model struct, never stored as authoritative state.
4. Only Phase 2 goals are included — no law/policing/succession/rumor goals.

## What to Change

### 1. Define goal types in `worldwake-ai/src/goal_model.rs`

```rust
pub enum CommodityPurpose {
    SelfConsume,
    Restock,
    RecipeInput(RecipeId),
    Treatment,
}

pub enum GoalKind {
    ConsumeOwnedCommodity { commodity: CommodityKind },
    AcquireCommodity { commodity: CommodityKind, purpose: CommodityPurpose },
    Sleep,
    Relieve,
    Wash,
    ReduceDanger,
    Heal { target: EntityId },
    ProduceCommodity { recipe_id: RecipeId },
    SellCommodity { commodity: CommodityKind },
    RestockCommodity { commodity: CommodityKind },
    MoveCargo { lot: EntityId, destination: EntityId },
    LootCorpse { corpse: EntityId },
    BuryCorpse { corpse: EntityId, burial_site: EntityId },
}

pub struct GoalKey {
    pub kind: GoalKind,
    pub commodity: Option<CommodityKind>,
    pub entity: Option<EntityId>,
    pub place: Option<EntityId>,
}

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

### 2. Implement GoalKey normalization

`GoalKey::from(goal_kind: &GoalKind)` — extracts the canonical commodity/entity/place from each `GoalKind` variant.

### 3. Implement Ord/PartialOrd for GoalPriorityClass

`Critical > High > Medium > Low > Background` — needed for deterministic sorting.

### 4. Implement Eq/Hash-equivalent for GoalKey

`GoalKey` must implement `Eq`, `PartialEq`, `Ord`, `PartialOrd` for use in `BTreeSet` and deterministic sorting. No `Hash` (no HashMap usage).

### 5. Derive traits

All types: `Clone`, `Debug`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`.
`GoalPriorityClass`: additionally `Copy`, `Ord`, `PartialOrd`.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — was empty stub)
- `crates/worldwake-ai/src/lib.rs` (modify — re-exports)

## Out of Scope

- Candidate generation logic that produces `GroundedGoal` — E13DECARC-007
- Priority/motive scoring logic — E13DECARC-008
- Any goal kinds from Phase 3+ (law, policing, succession, rumor, party, expedition, persuasion, crime investigation, escort/raid/camp)
- Registering these types as components (they are NOT authoritative world state)

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKey::from(&GoalKind::AcquireCommodity { commodity: Apple, purpose: SelfConsume })` extracts `commodity = Some(Apple)`
2. `GoalKey::from(&GoalKind::Sleep)` has `commodity = None, entity = None, place = None`
3. `GoalKey::from(&GoalKind::LootCorpse { corpse })` extracts `entity = Some(corpse)`
4. `GoalPriorityClass::Critical > GoalPriorityClass::High` (Ord)
5. `GoalKind` has exactly 13 variants (no Phase 3+ leakage)
6. All types round-trip through bincode
7. Existing suite: `cargo test --workspace`

### Invariants

1. No Phase 3+ goal kinds present
2. `GroundedGoal` is never stored as a component
3. `GoalKey` comparison is deterministic (Ord, not Hash-based)
4. `BTreeSet<EntityId>` for evidence, not `HashSet`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — tests for GoalKey normalization, GoalPriorityClass ordering, trait bounds, bincode roundtrip

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
