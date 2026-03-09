# E06EVELOG-001: CauseRef, EventTag, and VisibilitySpec Enums

**Status**: COMPLETED
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
4. `worldwake-sim/src/lib.rs` currently contains only crate docs and no public modules or re-exports — confirmed
5. No event-related types exist yet in `worldwake-sim`, and `cargo test -p worldwake-sim` currently runs zero tests — confirmed
6. `worldwake-core` re-exports `EventId`, `Tick`, and `EntityId` from the crate root, so `worldwake-sim` can use either `worldwake_core::{...}` or `worldwake_core::ids::{...}` without adding aliases — confirmed

## Architecture Check

1. These are pure data types with no logic beyond derives; placing them in `worldwake-sim` matches the crate assignment in the spec (E06 → `worldwake-sim`)
2. `CauseRef` makes root causes explicit (Bootstrap, SystemTick) instead of encoding them as `None`, which is critical for cause-chain traversal correctness
3. `VisibilitySpec` uses graph hops (`AdjacentPlaces { max_hops: u8 }`) rather than Euclidean distance, matching the place-graph world model
4. These value types should follow the same lightweight value semantics as `worldwake-core` ID newtypes: derive `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`
5. `EventTag` should remain a closed ordered enum, not a stringly-typed tag wrapper; adding variants later is cleaner than weakening determinism and exhaustiveness now

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

- WitnessData struct (`tickets/E06EVELOG-002-witness-data-and-delta-types.md`)
- Delta types (`tickets/E06EVELOG-002-witness-data-and-delta-types.md`)
- EventRecord struct (E06EVELOG-003)
- EventLog storage (E06EVELOG-004)
- Any mutation or query logic

## Acceptance Criteria

### Tests That Must Pass

1. `CauseRef` variants construct correctly and are distinguishable via pattern match
2. `EventTag` has all 14 required variants and they are `Ord`-ordered
3. `VisibilitySpec::AdjacentPlaces { max_hops: 2 }` stores hop count correctly
4. All three enums satisfy `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + Deserialize`
5. All three enums survive bincode round-trip for every variant
6. `CauseRef::Bootstrap` and `CauseRef::SystemTick(Tick(0))` are distinct (no None-encoding)
7. Existing suite: `cargo test --workspace`

### Invariants

1. Every cause reference is explicit — no `None`-as-root encoding (spec 9.3)
2. All types use deterministic serialization (no HashMap, no floats)
3. `EventTag` ordering is stable across serialization

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/cause.rs` — trait bound assertions, constructor/pattern-match coverage, bincode round-trip per variant, distinctness of root causes
2. `crates/worldwake-sim/src/event_tag.rs` — variant completeness, deterministic ordering, bincode round-trip
3. `crates/worldwake-sim/src/visibility.rs` — construction, hop storage, deterministic ordering, bincode round-trip

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo fmt --check`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `CauseRef`, `EventTag`, and `VisibilitySpec` as new public modules in `worldwake-sim`
  - Re-exported the three types from `crates/worldwake-sim/src/lib.rs`
  - Added colocated unit tests covering trait bounds, variant coverage, deterministic ordering where relevant, and bincode round-trip behavior
- Deviations from original plan:
  - Tightened the ticket and implementation to align these enums with existing core value-type conventions by deriving `Copy`, `PartialEq`, and `PartialOrd` in addition to the originally listed traits
  - Clarified assumptions to reflect the real crate state (`lib.rs` had docs only, not a fully empty file) and the adjacent E06EVELOG-002 ticket filename
- Verification results:
  - `cargo test -p worldwake-sim` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
  - `cargo fmt --all --check` reported pre-existing formatting drift in unrelated `worldwake-core` files; the modified `worldwake-sim` files were formatted and pass `rustfmt --check`
