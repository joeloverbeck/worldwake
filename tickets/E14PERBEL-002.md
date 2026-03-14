# E14PERBEL-002: Define Core Belief and Perception Types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new types in worldwake-core
**Deps**: E14PERBEL-001 (FactId removal clears namespace)

## Problem

E14 requires a state-snapshot belief model where each agent stores its own perceived view of the world. The foundational types — `AgentBeliefStore`, `BelievedEntityState`, `PerceptionSource`, `PerceptionProfile`, `SocialObservation`, `SocialObservationKind` — must be defined in `worldwake-core` before they can be registered as components or used by systems.

## Assumption Reassessment (2026-03-14)

1. `worldwake-core` is the correct crate for all authoritative types — confirmed by spec and existing patterns (`AgentData`, `CombatProfile`, etc. are all in core).
2. `Permille` newtype exists in `crates/worldwake-core/src/numerics.rs` — needed for `PerceptionProfile.observation_fidelity`.
3. `Tick` type exists in `crates/worldwake-core/src/ids.rs` — needed for `observed_tick` and `memory_retention_ticks`.
4. `EntityId` exists in `crates/worldwake-core/src/ids.rs` — needed throughout.
5. `CommodityKind` and `Quantity` exist in `crates/worldwake-core/src/items.rs` and `crates/worldwake-core/src/numerics.rs` — needed for `BelievedEntityState.last_known_inventory`.
6. Wound types exist in `crates/worldwake-core/src/wounds.rs` — needed for `BelievedEntityState.wounds`. Check actual wound storage type used in `WoundList` component.
7. All components use `BTreeMap`/`BTreeSet` for deterministic ordering — enforced invariant.

## Architecture Check

1. Types are pure data with `Serialize`/`Deserialize` — follows existing component patterns.
2. Confidence is derived (not stored) per Principle 3 — `PerceptionSource` + `observed_tick` are stored, confidence computed at query time.
3. No backward-compatibility needed — these are entirely new types.

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
    pub wounds: Vec<WoundRecord>,  // use whatever wound representation matches WoundList
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

All types derive: `Clone, Debug, Serialize, Deserialize`. Enums also derive `Eq, PartialEq`. Structs derive `Eq, PartialEq` where wound representation allows.

### 2. Create `crates/worldwake-core/src/perception_profile.rs`

```rust
pub struct PerceptionProfile {
    pub memory_capacity: u32,
    pub memory_retention_ticks: u32,
    pub observation_fidelity: Permille,
}
```

Derives: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 3. Register modules in `crates/worldwake-core/src/lib.rs`

Add `pub mod belief;` and `pub mod perception_profile;` declarations. Re-export key types.

### 4. Implement `AgentBeliefStore` methods

- `new()` — empty belief store
- `update_entity(&mut self, id: EntityId, state: BelievedEntityState)` — insert or replace
- `get_entity(&self, id: &EntityId) -> Option<&BelievedEntityState>` — lookup
- `record_social_observation(&mut self, obs: SocialObservation)` — append
- `enforce_capacity(&mut self, profile: &PerceptionProfile, current_tick: Tick)` — evict oldest entries beyond capacity, forget entries beyond retention

### 5. Implement `Component` trait

Both `AgentBeliefStore` and `PerceptionProfile` implement the `Component` trait (defined in `crates/worldwake-core/src/traits.rs`).

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (new)
- `crates/worldwake-core/src/perception_profile.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module declarations and re-exports)

## Out of Scope

- Registering these types in `ComponentTables` (that's E14PERBEL-003)
- Implementing `PerAgentBeliefView` (that's E14PERBEL-004)
- Implementing `perception_system()` (that's E14PERBEL-005)
- Modifying any existing components or relations
- Adding confidence derivation logic (lives in `PerAgentBeliefView`, E14PERBEL-004)
- Modifying `BeliefView` trait

## Acceptance Criteria

### Tests That Must Pass

1. `AgentBeliefStore::new()` creates empty store
2. `update_entity()` inserts new entity state and overwrites existing with newer observation
3. `get_entity()` returns `None` for unknown entities
4. `enforce_capacity()` evicts oldest entries when store exceeds `memory_capacity`
5. `enforce_capacity()` removes entries older than `memory_retention_ticks`
6. `record_social_observation()` appends to social observations list
7. `BelievedEntityState` roundtrip serialization (bincode)
8. `PerceptionSource` variant equality and serialization
9. `SocialObservationKind` variant equality and serialization
10. `PerceptionProfile` serialization roundtrip
11. `cargo test -p worldwake-core`
12. `cargo clippy --workspace`

### Invariants

1. All collection types use `BTreeMap`/`BTreeSet` — no `HashMap`/`HashSet` (determinism)
2. No `f32`/`f64` fields — `Permille` for fidelity (spec drafting rules)
3. No stored confidence score — only `PerceptionSource` + `observed_tick` stored (Principle 3)
4. All types are `Serialize + Deserialize` for save/load compatibility

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — unit tests for store operations, serialization
2. `crates/worldwake-core/src/perception_profile.rs` — unit tests for serialization

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
