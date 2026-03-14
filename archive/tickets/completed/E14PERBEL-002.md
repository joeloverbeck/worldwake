# E14PERBEL-002: Define Core Belief and Perception Types

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new authoritative types in `worldwake-core`
**Deps**: E14PERBEL-001 (completed; fact scaffolding already removed)

## Problem

E14 requires a state-snapshot belief model where each agent stores its own perceived view of the world. The foundational data types — `AgentBeliefStore`, `BelievedEntityState`, `PerceptionSource`, `PerceptionProfile`, `SocialObservation`, `SocialObservationKind` — must exist in `worldwake-core` before later tickets can register them as components, populate them from events, or read them through `PerAgentBeliefView`.

## Assumption Reassessment (2026-03-14)

1. `worldwake-core` is the correct crate for all authoritative types — confirmed by spec and existing patterns (`AgentData`, `CombatProfile`, etc. are all in core).
2. `Permille` newtype exists in `crates/worldwake-core/src/numerics.rs` — needed for `PerceptionProfile.observation_fidelity`.
3. `Tick` and `EntityId` both exist in `crates/worldwake-core/src/ids.rs` and are re-exported from `worldwake-core` — needed throughout the belief types.
4. `EntityId` exists in `crates/worldwake-core/src/ids.rs` — needed throughout.
5. `CommodityKind` and `Quantity` exist in `crates/worldwake-core/src/items.rs` and `crates/worldwake-core/src/numerics.rs` — needed for `BelievedEntityState.last_known_inventory`.
6. Wound types exist in `crates/worldwake-core/src/wounds.rs`, and the authoritative component is `WoundList { wounds: Vec<Wound> }` — the belief snapshot should store `Vec<Wound>`, not invent a parallel wound record type.
7. Component registration is generated from `component_schema.rs` and belongs to `E14PERBEL-003`, not this ticket.
8. All authoritative collections use deterministic containers (`BTreeMap`, `BTreeSet`, `Vec`) — enforced invariant.

## Architecture Check

1. These types should live in a single belief-domain module in `worldwake-core`. Splitting `PerceptionProfile` into a second file would scatter one cohesive domain for little benefit.
2. Confidence remains derived, not stored, per Principle 3 — `PerceptionSource` and `observed_tick` are authoritative inputs; any confidence interpretation is a read-model concern for later tickets.
3. `AgentBeliefStore` should own only deterministic, data-local behavior. Visibility resolution, event interpretation, and component registration belong to later tickets.
4. No backward-compatibility or alias types are needed — these are entirely new authoritative types.

## What to Change

### 1. Create `crates/worldwake-core/src/belief.rs`

Define the following types:

```rust
/// Per-agent belief store — each agent's subjective view of the world.
pub struct AgentBeliefStore {
    pub known_entities: BTreeMap<EntityId, BelievedEntityState>,
    pub social_observations: Vec<SocialObservation>,
}

/// Snapshot of what an agent believes about a specific entity.
pub struct BelievedEntityState {
    pub last_known_place: Option<EntityId>,
    pub last_known_inventory: BTreeMap<CommodityKind, Quantity>,
    pub alive: bool,
    pub wounds: Vec<Wound>,
    pub observed_tick: Tick,
    pub source: PerceptionSource,
}

/// How the agent learned this information.
pub enum PerceptionSource {
    DirectObservation,
    Report { from: EntityId, chain_len: u8 },
    Rumor { chain_len: u8 },
    Inference,
}

/// A concrete social event the agent witnessed.
pub struct SocialObservation {
    pub kind: SocialObservationKind,
    pub subjects: (EntityId, EntityId),
    pub place: EntityId,
    pub observed_tick: Tick,
    pub source: PerceptionSource,
}

pub enum SocialObservationKind {
    WitnessedCooperation,
    WitnessedConflict,
    WitnessedObligation,
    CoPresence,
}
```

pub struct PerceptionProfile {
    pub memory_capacity: u32,
    pub memory_retention_ticks: u64,
    pub observation_fidelity: Permille,
}

All types derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 2. Register module in `crates/worldwake-core/src/lib.rs`

Add `pub mod belief;` and re-export the key belief/perception types.

### 3. Implement `AgentBeliefStore` methods

- `new()` — empty belief store
- `update_entity(&mut self, id: EntityId, state: BelievedEntityState)` — insert or replace only when the incoming snapshot is at least as recent as the stored one
- `get_entity(&self, id: &EntityId) -> Option<&BelievedEntityState>` — lookup
- `record_social_observation(&mut self, obs: SocialObservation)` — append
- `enforce_capacity(&mut self, profile: &PerceptionProfile, current_tick: Tick)` — forget stale entity snapshots and stale social observations beyond retention, then evict the oldest known-entity snapshots beyond capacity using deterministic ordering (`observed_tick`, then `EntityId`)

### 4. Implement `Component` trait

`AgentBeliefStore` and `PerceptionProfile` should implement the existing `Component` marker trait in `crates/worldwake-core/src/traits.rs`. No registration work is part of this ticket.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module declaration and re-exports)

## Out of Scope

- Registering these types in `ComponentTables`, `World`, or `WorldTxn` (that's E14PERBEL-003)
- Implementing `PerAgentBeliefView` (that's E14PERBEL-004)
- Implementing `perception_system()` (that's E14PERBEL-005)
- Seeding new agents with default belief components
- Modifying `World`/`WorldTxn` creation paths
- Adding confidence derivation logic (lives in `PerAgentBeliefView`, E14PERBEL-004)
- Modifying `BeliefView` trait
- Interpreting `EventRecord` deltas into beliefs

## Acceptance Criteria

### Tests That Must Pass

1. `AgentBeliefStore::new()` creates empty store
2. `update_entity()` inserts new entity state and overwrites existing only with an equal-or-newer observation
3. `get_entity()` returns `None` for unknown entities
4. `enforce_capacity()` evicts oldest entries when store exceeds `memory_capacity`
5. `enforce_capacity()` removes entries older than `memory_retention_ticks`
6. `record_social_observation()` appends to social observations list
7. `enforce_capacity()` also prunes stale social observations by retention
8. `BelievedEntityState` roundtrip serialization (bincode)
9. `PerceptionSource` variant equality and serialization
10. `SocialObservationKind` variant equality and serialization
11. `PerceptionProfile` serialization roundtrip
12. `AgentBeliefStore` and `PerceptionProfile` satisfy `Component` trait bounds
13. `cargo test -p worldwake-core`
14. `cargo clippy --workspace`

### Invariants

1. All collection types use `BTreeMap`/`BTreeSet` — no `HashMap`/`HashSet` (determinism)
2. No `f32`/`f64` fields — `Permille` for fidelity (spec drafting rules)
3. No stored confidence score — only `PerceptionSource` + `observed_tick` stored (Principle 3)
4. All types are `Serialize + Deserialize` for save/load compatibility

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — unit tests for store operations, retention/capacity behavior, trait bounds, and serialization

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed: added `crates/worldwake-core/src/belief.rs` with `AgentBeliefStore`, `BelievedEntityState`, `PerceptionSource`, `SocialObservation`, `SocialObservationKind`, and `PerceptionProfile`; re-exported the new types from `worldwake-core`.
- Deviations from original plan: kept the domain in a single `belief.rs` module instead of splitting `PerceptionProfile` into a separate file; corrected wound storage to `Vec<Wound>`; changed `memory_retention_ticks` to `u64` to align with `Tick`; removed component-registration work from this ticket because it belongs to `E14PERBEL-003`.
- Verification results: `cargo test -p worldwake-core`, `cargo clippy --workspace`, and `cargo test --workspace` all passed after adding coverage for stale-overwrite behavior, deterministic capacity eviction, retention pruning, serialization, and trait bounds.
