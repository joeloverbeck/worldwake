# S59EXPOBLSUB-001: Core expectation and search types

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new types in worldwake-core
**Deps**: S59 spec (reassessed 2026-04-06)

## Problem

No type substrate exists for expectation tracking, last-seen records, or search results. All downstream tickets (component registration, actions, candidate generation) depend on these types being defined first.

## Assumption Reassessment (2026-04-06)

1. `ExpectationId` follows the manual ID pattern used by `ViolationId` (`violation.rs:20`) and `ClaimId` (`entity_belief_claim.rs:14`) — `#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)] pub struct FooId(pub u64)`. Confirmed 2026-04-06.
2. `CommodityKind` exists at `crates/worldwake-core/src/items.rs:10-21`. Used in `ExpectationBasis::DeliveryCommitment`.
3. `Quantity` newtype exists at `crates/worldwake-core/src/numerics.rs:74` — `pub struct Quantity(pub u32)`.
4. `Tick` exists at `crates/worldwake-core/src/ids.rs:56-57` — `pub struct Tick(pub u64)`. Grace ticks use `u64` to match.
5. `EntityId` exists at `crates/worldwake-core/src/ids.rs:44-47`.
6. `EvidenceKind` exists at `crates/worldwake-core/src/evidence.rs:23` with variants ContainerTampered, BloodTrail, DisturbanceMarker, MovementTrace. Used in `SearchResult::FoundEvidence`.
7. No existing types named `ExpectationId`, `ExpectationRecord`, `ExpectationStore`, `LastSeenRecord`, `LastSeenMemory`, `SearchTarget`, `SearchResult`, `SearchCondition` in the codebase.
8. User-supplied `specs/S59-expectation*` glob resolves to `specs/S59-expectation-obligation-substrate.md`. Safe mechanical correction.
9. `ViolationId`/`ClaimId` establish the derive pattern for manual `u64` IDs, but neither implements `Display`. The ticket's acceptance criteria explicitly require `ExpectationId` display coverage, so `Display` is an additive ticket-owned requirement rather than a pre-existing pattern mismatch.

## Architecture Check

1. All types are plain data structs/enums in worldwake-core with no cross-crate dependencies beyond core itself. This keeps them available to all downstream crates (sim, systems, ai, cli).
2. No backward compatibility shims. These are entirely new types.

## Verification Layers

1. Type definitions compile with correct derives → focused unit test (serde roundtrip, trait bound assertions)
2. Single-layer ticket (type definitions only) — no cross-system verification needed.

## What to Change

### 1. Create expectation types module

Create `crates/worldwake-core/src/expectation.rs` containing:

- `ExpectationId(pub u64)` — derives matching `ViolationId` pattern (includes `Default`)
- `ExpectationBasis` enum — `DutyAssignment { office: EntityId }`, `DeliveryCommitment { commodity: CommodityKind, quantity: Quantity }`, `RoutineReturn`, `EscortObligation { charge: EntityId }`, `SocialPromise`
- `ExpectationState` enum — `Active`, `Overdue`, `Resolved { outcome: ExpectationOutcome }`, `Expired`
- `ExpectationOutcome` enum — `Fulfilled`, `FoundSafe`, `FoundWounded`, `FoundDead` (each with `at_place: EntityId`), `NotFound`, `ReturnedLate`
- `ExpectationRecord` struct — id, owner, subject, expected_place, deadline_tick, grace_ticks (u64), basis, state, created_tick

### 2. Create last-seen types

In the same module or a separate `last_seen.rs`:

- `LastSeenRecord` struct — subject, place, observed_tick, source, provenance
- `LastSeenProvenance` enum — `DirectObservation`, `Hearsay { original_observer: EntityId, chain_depth: u8 }`

### 3. Create search types

In the same module or a separate `search.rs`:

- `SearchTarget` enum — `MissingEntity { entity, last_seen_place: Option<EntityId> }`, `RouteSearch { from, to }`
- `SearchResult` enum — `FoundAlive { entity, condition }`, `FoundDead { entity }`, `FoundEvidence { evidence_kinds: Vec<EvidenceKind> }`, `NothingFound`
- `SearchCondition` enum — `Healthy`, `Wounded`, `Unconscious`

### 4. Export from lib.rs

Add `mod expectation;` (and `mod last_seen; mod search;` if split) to `crates/worldwake-core/src/lib.rs` and re-export all public types.

### 5. Trait bound tests

Add `assert_value_bounds` tests following the pattern in `evidence.rs:96` and `ids.rs:254` to verify serde roundtrip and required trait bounds.

## Files to Touch

- `crates/worldwake-core/src/expectation.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Component definitions (`ExpectationStore`, `LastSeenMemory`) — ticket 002
- Component registration in ECS schema — ticket 002
- GoalKind variants — ticket 005
- Any action definitions — tickets 007-011

## Acceptance Criteria

### Tests That Must Pass

1. `ExpectationId` serde roundtrip and Display
2. `ExpectationRecord` serde roundtrip
3. `LastSeenRecord` serde roundtrip
4. `SearchResult` serde roundtrip
5. Trait bound assertions for all new types (Copy where applicable, Serialize+Deserialize for all)
6. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All types derive `Serialize, Deserialize` for save/load compatibility
2. `ExpectationId` derives `Default` matching ViolationId/ClaimId pattern
3. All uses of `u64` for grace_ticks match Tick arithmetic conventions

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/expectation.rs` (inline tests) — serde roundtrip + trait bound assertions for all new types

### Commands

1. `cargo test -p worldwake-core expectation`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

Completed on 2026-04-06.

- Added `crates/worldwake-core/src/expectation.rs` with the shared expectation, last-seen, and search enums/structs plus `ExpectationId` display support.
- Exported the new public types from `crates/worldwake-core/src/lib.rs`.
- Added inline serde roundtrip and trait-bound tests covering the new type substrate.

## Verification Result

- Passed `cargo test -p worldwake-core expectation`
- Passed `cargo test -p worldwake-core`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
