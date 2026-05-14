# S143STABELVIE-003A: Add ownership and holder belief-claim substrate

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `AgentBeliefStore` entity-claim schema gains explicit ownership and holder claim lanes; save format bumps for the persisted belief-store shape.
**Deps**: archive/tickets/S143STABELVIE-001.md, archive/tickets/S143STABELVIE-002.md, archive/specs/S143-static-belief-view-trait-separation.md

## Problem

`S143STABELVIE-003` must migrate authority reads to `BelievedAuthorityView`, but the live `AgentBeliefStore::entity_claims` schema has no ownership or holder claim aspect. `docs/FOUNDATIONS.md` FND-14A forbids deriving ownership, custody, rights, institutional claims, or jurisdiction from co-location alone, so the migration needs an explicit belief carrier before the new authority trait can provide real reads.

## Assumption Reassessment (2026-05-13)

1. Live `EntityBeliefAspect` has physical and evidence aspects (`Location`, `Inventory`, `Alive`, `Wounded`, `Activity`, `WorkstationPresent`, `ResourceAvailable`, `ContentionState`, `WashBasinState`, `Artifact`, `Courage`, `Evidence`) but no owner or holder/custody aspect (`crates/worldwake-core/src/entity_belief_claim.rs`).
2. Live `ClaimValue` can store places, quantities, booleans, activities, workstation/resource state, contention, wash-basin state, artifacts, courage, wounds, and evidence, but has no generic entity-id value suitable for owner/holder belief claims.
3. `AgentBeliefStore::entity_claims` is persisted inside `AgentBeliefStore` and diffed through `BeliefStoreDiff`; adding enum variants changes current-format save shape and requires `SAVE_FORMAT_VERSION` bump under the repo's no-backward-compatibility rule.
4. This ticket owns only the storage/projection substrate for explicit ownership and holder beliefs. It does not make perception infer owner/holder from co-location, and it does not migrate `believed_owner_of` or `believed_holder_of` off legacy traits; `S143STABELVIE-003` owns that trait migration once this substrate exists.
5. Information path: before this ticket, the same authority fact had no lawful `entity_claims` transport path. After this ticket, explicit `EntityBeliefAspect::Owner` and `EntityBeliefAspect::Holder` claims with `ClaimValue::Entity(Option<EntityId>)` are the canonical per-agent belief carriers for item/entity ownership and custody identity. No duplicate transport path is removed in this ticket because the legacy `ControlBeliefView::believed_owner_of` path is removed by `S143STABELVIE-003`.

## Architecture Check

1. The change aligns with FND-14/14A and FND-24 by representing ownership and custody as explicit agent beliefs rather than deriving them from co-location or authoritative world state.
2. No backwards-compatibility shim is introduced. The current save format is bumped; older saves remain rejected by the existing loader.

## Verified Layers

1. Persisted claim schema accepts owner/holder facts -> focused `worldwake-core` entity-claim test and save/load roundtrip with non-default owner/holder claims.
2. `BelievedAuthorityView` can read explicit owner/holder claims without authoritative world reads -> focused `worldwake-sim` `PerAgentBeliefView` tests using synthetic belief claims.
3. Missing owner/holder belief remains unknown -> existing and updated `PerAgentBeliefView` tests with empty `AgentBeliefStore`.

## Landed Changes

### 1. Entity belief claim schema

Add `EntityBeliefAspect::Owner` and `EntityBeliefAspect::Holder`, plus `ClaimValue::Entity(Option<EntityId>)`. These are explicit social/relational belief lanes and are not populated by ordinary co-located physical observation snapshots.

### 2. Authority read substrate

Add a small claim-value helper in `worldwake-sim::belief_view` and update `PerAgentBeliefView`'s `BelievedAuthorityView` impl so `believed_owner_of` and `believed_holder_of` project the best explicit entity claim into `BeliefRead<EntityId>`. `None`, missing claims, contradictions, and disputed non-entity values collapse to `BeliefRead::Unknown`; stale entity claims produce `BeliefRead::Stale`.

### 3. Save format

Bump `SAVE_FORMAT_VERSION` and update the non-default save/load fixture to include owner and holder claims so the persisted current-format path proves the new variants by value.

## Landed Files

- `archive/tickets/S143STABELVIE-003.md` (modify — dependency truth-sync)
- `archive/specs/S143-static-belief-view-trait-separation.md` (modify — substrate correction)
- `crates/worldwake-core/src/entity_belief_claim.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify)

## Out of Scope

- Moving `believed_owner_of` / `believed_office_holder` off legacy traits — now-archived `archive/tickets/S143STABELVIE-003.md`.
- Access-right and jurisdiction belief storage beyond `Unknown` defaults.
- Automatic perception of ownership or holder identity from co-location.
- Tell/report propagation policy for owner/holder claims beyond preserving the generic `entity_claims` carrier.

## Acceptance Result

### Tests Passed

1. Focused test coverage proves `BelievedAuthorityView::believed_owner_of` on `PerAgentBeliefView` returns `BeliefRead::Known` for an explicit owner claim and `Unknown` without one.
2. Focused test coverage proves `BelievedAuthorityView::believed_holder_of` on `PerAgentBeliefView` returns `BeliefRead::Known` for an explicit holder claim and `Unknown` without one.
3. Existing save/load suite preserves a non-default `AgentBeliefStore` containing owner/holder claims.
4. Existing suite: `cargo test -p worldwake-sim believed_authority`.
5. Existing suite: `cargo test -p worldwake-sim save`.

### Invariants

1. No owner/holder claim is generated from co-location alone.
2. Owner and holder are distinct aspects so ownership and custody cannot collapse into one scalar.
3. Current-format save files use version 85; older save versions remain rejected.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — focused authority-view reads from explicit owner/holder claims.
2. `crates/worldwake-sim/src/save_load.rs` — existing full non-default roundtrip fixture carries the owner/holder claim variants.

### Commands Passed

1. `cargo test -p worldwake-sim believed_authority`
2. `cargo test -p worldwake-sim save`
3. `cargo test -p worldwake-sim`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-13.

- Added distinct `EntityBeliefAspect::Owner` and `EntityBeliefAspect::Holder` lanes plus `ClaimValue::Entity(Option<EntityId>)` for explicit per-agent authority beliefs.
- Added a shared entity-claim projection helper and wired `PerAgentBeliefView`'s `BelievedAuthorityView` implementation to read explicit owner/holder claims without consulting authoritative world ownership or possession.
- Bumped the current save format from 84 to 85 and extended the full non-default save/load fixture to round-trip owner and holder claims by value.
- Updated the now-archived `archive/tickets/S143STABELVIE-003.md` and `archive/specs/S143-static-belief-view-trait-separation.md` so the original trait migration depends on this substrate instead of assuming it already existed.

## Deviations

- The original S143 spec said the belief-store data layout would not change. FOUNDATIONS reassessment rejected that premise for authority reads: FND-14A requires explicit owner/holder belief carriers before `BelievedAuthorityView` can provide real reads.
- Access-right and jurisdiction storage stayed out of scope; those methods still use the existing `Unknown` defaults until a later ticket defines lawful carriers.

## Verification Result

- Passed `cargo test -p worldwake-sim believed_authority`
- Passed `cargo test -p worldwake-sim save`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
