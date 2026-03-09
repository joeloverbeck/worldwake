# E06EVELOG-001: CauseRef, EventTag, and VisibilitySpec Enums

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new types in `worldwake-sim`
**Deps**: E05 complete (relation/ownership mutations exist in worldwake-core)

## Problem

The event log needs three foundational enum types before any event records can be constructed. `CauseRef` replaces ambiguous `Option<EventId>` cause semantics with explicit cause references. `EventTag` classifies events for indexing. `VisibilitySpec` defines graph-friendly visibility semantics for event perception.

## Assumption Reassessment (2026-03-09)

1. `EventId` and `Tick` already exist in `worldwake-core::ids` — confirmed
2. `EntityId` already exists in `worldwake-core::ids` — confirmed
3. `worldwake-sim` crate exists with `Cargo.toml` depending on `worldwake-core`, `serde`, `bincode` — confirmed
4. `worldwake-sim/src/lib.rs` is currently empty (just a doc comment) — confirmed
5. No event-related types exist yet in `worldwake-sim` — confirmed

## Architecture Check

1. These are pure data types with no logic beyond derives; placing them in `worldwake-sim` matches the crate assignment in the spec (E06 → `worldwake-sim`)
2. `CauseRef` makes root causes explicit (Bootstrap, SystemTick) instead of encoding them as `None`, which is critical for cause-chain traversal correctness
3. `VisibilitySpec` uses graph hops (`AdjacentPlaces { max_hops: u8 }`) rather than Euclidean distance, matching the place-graph world model
4. All enums must derive the determinism-required trait set: `Clone, Eq, Ord, Hash, Debug, Serialize, Deserialize`

## What to Change

### 1. Create `crates/worldwake-sim/src/cause.rs`

Define `CauseRef` enum:
- `Event(EventId)` — caused by a prior event
- `SystemTick(Tick)` — caused by a system-level tick (e.g. decay, need progression)
- `Bootstrap` — world initialization, no prior cause
- `ExternalInput(u64)` — human or external input with a stable input id

### 2. Create `crates/worldwake-sim/src/event_tag.rs`

Define `EventTag` enum with at minimum:
- `WorldMutation`, `Inventory`, `Transfer`, `Reservation`
- `ActionStarted`, `ActionCommitted`, `ActionAborted`
- `Travel`, `Trade`, `Crime`, `Combat`, `Political`, `Control`, `System`

### 3. Create `crates/worldwake-sim/src/visibility.rs`

Define `VisibilitySpec` enum:
- `ParticipantsOnly`
- `SamePlace`
- `AdjacentPlaces { max_hops: u8 }`
- `PublicRecord`
- `Hidden`

### 4. Register modules in `crates/worldwake-sim/src/lib.rs`

Declare and re-export the three new modules.

## Files to Touch

- `crates/worldwake-sim/src/cause.rs` (new)
- `crates/worldwake-sim/src/event_tag.rs` (new)
- `crates/worldwake-sim/src/visibility.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — register modules, re-export types)

## Out of Scope

- WitnessData struct (E06EVELOG-002)
- Delta types (E06EVELOG-002)
- EventRecord struct (E06EVELOG-003)
- EventLog storage (E06EVELOG-004)
- Any mutation or query logic

## Acceptance Criteria

### Tests That Must Pass

1. `CauseRef` variants construct correctly and are distinguishable via pattern match
2. `EventTag` has all 14 required variants and they are `Ord`-ordered
3. `VisibilitySpec::AdjacentPlaces { max_hops: 2 }` stores hop count correctly
4. All three enums satisfy `Clone + Eq + Ord + Hash + Debug + Serialize + Deserialize`
5. All three enums survive bincode round-trip for every variant
6. `CauseRef::Bootstrap` and `CauseRef::SystemTick(Tick(0))` are distinct (no None-encoding)
7. Existing suite: `cargo test --workspace`

### Invariants

1. Every cause reference is explicit — no `None`-as-root encoding (spec 9.3)
2. All types use deterministic serialization (no HashMap, no floats)
3. `EventTag` ordering is stable across serialization

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/cause.rs` — trait bound assertions, bincode round-trip per variant, distinctness of root causes
2. `crates/worldwake-sim/src/event_tag.rs` — variant completeness, Ord stability, bincode round-trip
3. `crates/worldwake-sim/src/visibility.rs` — construction, hop storage, bincode round-trip

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`
