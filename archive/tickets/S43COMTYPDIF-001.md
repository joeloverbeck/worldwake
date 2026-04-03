# S43COMTYPDIF-001: Core types — CommunicationClass, classify_communication, CommunicationProfile

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new enum, new function, new ECS component + registration
**Deps**: None

## Problem

All social communication routes through a single undifferentiated Tell action. There is no type-level distinction between a panicked alarm, first-hand testimony, and idle gossip. This blocks class-aware suppression, acceptance, and ranking (S43 deliverables 4–7) which all need the classification types to exist first.

## Assumption Reassessment (2026-04-03)

1. `TellTopic` enum exists at `crates/worldwake-core/src/belief.rs:704` with variants `EntityBelief`, `SocialObservation`, `InstitutionalClaim` — confirmed via Grep.
2. `SocialObservationDetail` enum at `belief.rs:1214` has variants: `WitnessedCooperation`, `WitnessedConflict`, `WitnessedObligation`, `WitnessedTelling`, `CoPresence`, `WitnessedAbsence`, `SuspectedTheft` — confirmed.
3. `PerceptionSource` enum at `belief.rs:1133` has variants: `DirectObservation`, `Report { from, chain_len }`, `Rumor { chain_len }`, `Inference` — confirmed.
4. `InstitutionalKnowledgeSource` at `institutional.rs:219` has variants including `RecordConsultation`, `SelfDeclaration`, `DirectObservation`, `WitnessedEvent`, `Report`, `Rumor` — confirmed.
5. `AgentBeliefStore` at `belief.rs` has `known_entities: BTreeMap<EntityId, BelievedEntityState>` — `classify_communication` needs this to check "entity believed dead" for EntityBelief topics.
6. `Permille` at `numerics.rs` confirmed as `struct Permille(u16)` with `new()`, `new_unchecked()`.
7. `Component` trait at `traits.rs` requires `'static + Send + Sync + Clone + Debug + Serialize + DeserializeOwned`.
8. Component registration via `with_component_schema_entries!` macro expands in `delta.rs`, `component_tables.rs`, and `world.rs` with the bare type in scope. `world_txn.rs` uses the schema's qualified `crate::Type` setter type path via `select_txn_simple_set_components`, so registration affects generated setters there without requiring a new top-level import.
9. `TellProfile` registered at `component_schema.rs:609` on `EntityKind::Agent` — `CommunicationProfile` follows the same pattern.
10. No existing `CommunicationClass` or `CommunicationProfile` in the codebase — confirmed via Grep.
11. Ticket says `world_txn.rs` needs a new type import at a macro expansion site; live code has `select_txn_simple_set_components` forwarding the schema entry's qualified `crate::Type` as `$txn_component_ty`, so no `world_txn.rs` top-level import is required for this registration change.
12. Correction applied: removed `crates/worldwake-core/src/world_txn.rs` from `Files to Touch` and narrowed the reassessment note to the real import fallout in `delta.rs`, `component_tables.rs`, and `world.rs`.
13. Why safe: this is a mechanical correction derived directly from the live macro signature in `component_schema.rs`, not an architecture change.

## Architecture Check

1. All three types live in `worldwake-core`, the dependency-free bottom crate. `classify_communication()` is a pure function over core types, callable from both `worldwake-ai` and `worldwake-systems` without introducing cross-system coupling. This is cleaner than placing classification logic in either consumer crate.
2. No backwards-compatibility shims. `CommunicationProfile` is a new component, not a wrapper around `TellProfile.acceptance_fidelity` (that migration happens in ticket 004).

## Verification Layers

1. `CommunicationClass` enum completeness -> focused unit tests covering all TellTopic × content combinations
2. `classify_communication` correctness -> focused unit tests: WitnessedConflict → Alarm, DirectObservation entity belief → Testimony, Rumor-sourced belief → Gossip, dead-entity belief → Alarm
3. `CommunicationProfile` registration -> `cargo test -p worldwake-core` passes (macro expansion compiles)
4. Single-crate ticket — no cross-layer verification needed beyond compilation and unit tests

## What to Change

### 1. Add `CommunicationClass` enum

In a new file `crates/worldwake-core/src/communication.rs` (or in `belief.rs` near `TellTopic`):

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CommunicationClass {
    Alarm,
    Testimony,
    Gossip,
}
```

Export from `crates/worldwake-core/src/lib.rs`.

### 2. Add `classify_communication()` function

```rust
pub fn classify_communication(
    topic: &TellTopic,
    speaker_beliefs: &AgentBeliefStore,
) -> CommunicationClass
```

Classification logic per spec Deliverable 2 table:
- `SocialObservation(WitnessedConflict)` → Alarm
- `EntityBelief` where subject believed dead → Alarm
- `SocialObservation(SuspectedTheft | WitnessedAbsence)` → Testimony
- `EntityBelief` with `DirectObservation` or `Report` source → Testimony
- `InstitutionalClaim` with `DirectObservation`, `WitnessedEvent`, `RecordConsultation`, `SelfDeclaration`, or `Report` source → Testimony
- Everything else → Gossip

### 3. Add `CommunicationProfile` component

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommunicationProfile {
    pub alarm_acceptance: Permille,
    pub testimony_acceptance: Permille,
    pub gossip_acceptance: Permille,
}

impl Component for CommunicationProfile {}
```

With `Default` impl: `alarm_acceptance: 950`, `testimony_acceptance: 800`, `gossip_acceptance: 600`.

### 4. Register CommunicationProfile in component schema

Add entry in `component_schema.rs` following the `TellProfile` pattern, registered on `EntityKind::Agent`. Ensure the type is imported at the bare-type macro expansion sites (`delta.rs`, `component_tables.rs`, `world.rs`).

## Files to Touch

- `crates/worldwake-core/src/communication.rs` (new) — or add to `belief.rs`
- `crates/worldwake-core/src/lib.rs` (modify) — export new types
- `crates/worldwake-core/src/component_schema.rs` (modify) — register CommunicationProfile
- `crates/worldwake-core/src/delta.rs` (modify) — import CommunicationProfile
- `crates/worldwake-core/src/component_tables.rs` (modify) — import CommunicationProfile
- `crates/worldwake-core/src/world.rs` (modify) — import CommunicationProfile

## Out of Scope

- Modifying `GoalKind::ShareBelief` (ticket 002)
- Modifying goal policy, ranking, or Tell handler (tickets 003, 004)
- Removing `TellProfile.acceptance_fidelity` (ticket 004)
- Golden tests (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `classify_communication` returns `Alarm` for `SocialObservation(WitnessedConflict)`
2. Unit test: `classify_communication` returns `Alarm` for `EntityBelief` where subject believed dead
3. Unit test: `classify_communication` returns `Testimony` for `EntityBelief` with `DirectObservation` source
4. Unit test: `classify_communication` returns `Gossip` for `EntityBelief` with `Rumor` source
5. Unit test: `classify_communication` returns `Testimony` for `InstitutionalClaim` with `RecordConsultation` source
6. Unit test: `classify_communication` returns `Gossip` for `SocialObservation(CoPresence)`
7. Unit test: `CommunicationProfile::default()` returns expected values (950, 800, 600)
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `CommunicationClass` is `Copy + Serialize + Deserialize` — required for GoalKind embedding (ticket 002) and save/load
2. `classify_communication` is a pure function over core types — no dependency on sim/ai/systems crates
3. `CommunicationProfile` registered on `EntityKind::Agent` only

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/communication.rs` (or `belief.rs`) — unit tests for `classify_communication` covering all TellTopic variants × source combinations
2. `crates/worldwake-core/src/communication.rs` — unit test for `CommunicationProfile::default()`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completed: 2026-04-03

- Added [`CommunicationClass`], `classify_communication()`, and the authoritative per-agent [`CommunicationProfile`] substrate in `worldwake-core`.
- Registered `CommunicationProfile` on `EntityKind::Agent` and updated the explicit schema inventory, component table, world API, and component-delta test surfaces to include it.
- Added focused classification tests for the spec-owned alarm/testimony/gossip boundaries, plus default, registration, and bincode roundtrip coverage for `CommunicationProfile`.
- Corrected the ticket's macro-expansion fallout during reassessment: `world_txn.rs` did not require a new top-level import because the generated setter path already uses the schema entry's qualified `crate::Type`.

Deviation from original plan:

- The owned code surface remained core-only, but the ticket's original `world_txn.rs` import expectation was removed as stale reassessment fallout rather than implemented literally.

Verification:

- `cargo test -p worldwake-core communication`
- `cargo test -p worldwake-core`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
