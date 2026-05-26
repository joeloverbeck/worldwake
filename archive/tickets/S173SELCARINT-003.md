# S173SELCARINT-003: `PromotableContentionKind` self-care classification

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — crate-private enum variant additions in `worldwake-systems` + new match arms in `promotable_contention_kind` and `contention_target_matches_kind`
**Deps**: `specs/S173-self-care-interruption-occupancy.md` (D3)

## Problem

Before this ticket, the contention substrate (`S44` `ContentionQueue`, `S142` queue-grant promotion) was already in place, but `promotable_contention_kind` (`crates/worldwake-systems/src/facility_queue.rs:463-473`) classified only `(Corpse, "loot"|"bury")`, `(Care, "heal")`, and the `exclusive_facility_workstation_tag` auto-promotion path for `ActionPayload::Harvest`/`Craft`. Wash and Toilet actions use `ActionDomain::Needs` + `ActionPayload::None`, so neither flowed through any classification — meaning the contention queue did not promote grants for self-care actions and two dirty agents at the same basin could not lawfully contend through the queue substrate. This ticket extended `PromotableContentionKind` with two new unit variants and added the corresponding classifier + downstream match arms so wash and toilet can participate in the existing queue/grant flow once ticket 004 wires reservation requirements onto them.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PromotableContentionKind` is defined at `crates/worldwake-systems/src/facility_queue.rs:29-33` as `enum { FacilityExclusive(WorkstationTag), Corpse, Care }` — crate-private (no `pub`). `promotable_contention_kind` at L463-473 returns `Option<PromotableContentionKind>`. `contention_target_matches_kind` at L478+ exhaustively matches the enum (lines 481, 490, 494). Both call sites must gain new arms for `SelfCareWash` and `SelfCareLatrine`.
2. `exclusive_facility_workstation_tag` (`crates/worldwake-systems/src/facility_queue_actions.rs:152-172`) matches only on `ActionPayload::Harvest` and `ActionPayload::Craft` payloads. Wash and Toilet use `ActionPayload::None`, so neither flows through the `FacilityExclusive(WashBasin)` auto-promotion path today. Both `SelfCareWash` and `SelfCareLatrine` are genuine net-new variants — confirmed by Step 2 mini-investigation. No subsumption into `FacilityExclusive`.
3. Shared abstraction boundary: the crate-private `PromotableContentionKind` enum and the `(ActionDomain, action_name)` classifier pattern. Cross-crate consumers DO NOT exist — the enum is not `pub`-exported and is not referenced outside `facility_queue.rs` and `facility_queue_actions.rs`. No Core-Side Mirror Enum pattern applies (the enum is sim/systems-crate-local, not core-resident).
4. `ContentionPolicy` (`crates/worldwake-core/src/contention.rs:52-56`) is per-facility-entity, not per-kind. Spec D3 commits explicitly to no per-kind policy routing: the same policy applies to self-care facilities as to other exclusive workstations. This resolves the original Open Question #1 from the spec draft.
5. Existing tests in `facility_queue.rs` (#[cfg(test)] from L598): `dead_actor_is_pruned_from_queue` (L920), `departed_actor_is_pruned_from_queue` (L951), `deallocated_actor_is_pruned_from_queue` (L980), and others. Adding variants without altering the classifier's existing arms preserved behavior for all existing actions; the focused and crate suites passed unchanged.

## Architecture Check

1. The enum and classifier remain crate-private to `worldwake-systems`. Per spec D3, no relocation to `worldwake-core` is needed because no core-resident type references the enum. This honors the smallest-blast-radius FND-28 path — relocation would have a much larger blast radius for no architectural benefit.
2. Two new unit variants (no payload) keep the enum compact and preserve the existing `#[derive(Clone, Copy)]` on `PromotableContentionKind`. No risk of `#[allow(clippy::large_enum_variant)]` triggering.
3. Self-care contention reuses the existing queue/grant machinery (`ContentionQueue`, `EventTag::ContentionResolved`, `EventTag::QueueGrantPromoted`) rather than introducing a parallel queue — fully FND-28-aligned.

## Verified Layers

1. Classifier output → passed focused unit coverage: `promotable_contention_kind` returns `Some(PromotableContentionKind::SelfCareWash)` for `wash`, `Some(PromotableContentionKind::SelfCareLatrine)` for `toilet`, and preserved the existing harvest/craft/corpse/care classifications.
2. Target matcher coverage → passed focused unit coverage: `contention_target_matches_kind` accepts only a `WashBasin`-tagged `Facility` for `SelfCareWash` and only a `PlaceTag::Latrine` place for `SelfCareLatrine`.
3. Exhaustive match coverage → passed `cargo build --workspace`; the workspace compiles with the added `contention_target_matches_kind` arms.
4. Single-layer ticket (classification only): downstream queue behavior (grant promotion, contention resolution) remains owned by ticket 007's Scenario B golden. This ticket's landed contract is solely the classifier and target-matcher output.

## Landed Changes

### 1. Extended `PromotableContentionKind`

In `crates/worldwake-systems/src/facility_queue.rs`, added two crate-private unit variants:

```rust
enum PromotableContentionKind {
    FacilityExclusive(WorkstationTag),
    Corpse,
    Care,
    SelfCareWash,
    SelfCareLatrine,
}
```

### 2. Added classifier match arms

In `promotable_contention_kind`, added `ActionDomain::Needs` name-based classification for the two self-care actions:

```rust
(ActionDomain::Needs, "wash") => Some(PromotableContentionKind::SelfCareWash),
(ActionDomain::Needs, "toilet") => Some(PromotableContentionKind::SelfCareLatrine),
```

### 3. Added downstream match arms in `contention_target_matches_kind`

The `SelfCareWash` arm matches a live `EntityKind::Facility` carrying `WorkstationTag::WashBasin`. The `SelfCareLatrine` arm matches an `EntityKind::Place` where `World::place_has_tag(entity, PlaceTag::Latrine)` is true.

## Landed Files

- `crates/worldwake-systems/src/facility_queue.rs` (modified — enum variant additions, two classifier match arms, two `contention_target_matches_kind` match arms, and inline focused tests)

The change is contained to a single file because `PromotableContentionKind` is crate-private. No other workspace crate references it.

## Out of Scope

- Writing `SelfCareOccupancy` on action start — owned by ticket 004.
- Adding `reservation_requirements` to wash/toilet action registrations — owned by ticket 004.
- `ContentionPolicy` per-kind routing — explicitly rejected per spec Non-Goals; same per-facility policy applies.
- Promotion of `PromotableContentionKind` to a `pub` export — not needed; no core-resident or cross-crate consumer.

## Acceptance Result

### Tests Passed Or Deferred

1. Passed `promotable_contention_kind_classifies_wash_action_as_self_care_wash`.
2. Passed `promotable_contention_kind_classifies_toilet_action_as_self_care_latrine`.
3. Passed `promotable_contention_kind_unchanged_for_existing_actions`.
4. Passed `self_care_contention_kinds_match_only_their_eligible_targets` to cover the new downstream matcher arms.
5. Passed existing focused suite: `cargo test -p worldwake-systems facility_queue`.
6. Passed affected crate suite: `cargo test -p worldwake-systems`.

### Invariants

1. `promotable_contention_kind` output for all pre-existing actions remains unchanged in focused tests.
2. `contention_target_matches_kind` exhaustively covers all five variants; `cargo build --workspace` passed after the new arms landed.
3. No core crate depends on `PromotableContentionKind`; it remains crate-private inside `worldwake-systems`.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-systems/src/facility_queue.rs` inline `#[cfg(test)]` — added focused classifier tests for wash, toilet, existing action classes, and eligible target matching for the two self-care variants.

### Verification Commands

1. Passed `cargo test -p worldwake-systems facility_queue`.
2. Passed `cargo build --workspace`. This replaces the drafted `cargo build --workspace -- -D warnings` command because that command is not valid Cargo syntax; exhaustive match coverage is compile-time proof here.
3. Passed `cargo test -p worldwake-systems`.
4. Waived `./scripts/verify.sh` for this ticket iteration because the `implement-spec-tickets` harness owns the full pre-push verification gate after the S173 family lands.

## Outcome

Completed on 2026-05-26.

- Added `PromotableContentionKind::SelfCareWash` and `PromotableContentionKind::SelfCareLatrine` as crate-private systems variants.
- Classified `ActionDomain::Needs` actions named `wash` and `toilet` into those variants without changing harvest, craft, corpse, or care classification.
- Added target matching for `WashBasin`-tagged facilities and `Latrine`-tagged places so ticket 004 can wire reservation requirements onto the existing contention substrate.
- Added focused inline unit coverage for the classifier and matcher.

## Deviations

- The drafted workspace build command used invalid Cargo syntax: `cargo build --workspace -- -D warnings`. The observed compile proof was `cargo build --workspace`; CI-shaped warning denial remains covered by the family-level `./scripts/verify.sh` gate before push.
- Added one extra focused matcher test because the implementation touched `contention_target_matches_kind`, not only `promotable_contention_kind`.

## Verification Result

- Passed `cargo test -p worldwake-systems facility_queue`
- Passed `cargo fmt --all`
- Passed `cargo build --workspace`
- Passed `cargo test -p worldwake-systems`
- Waived `./scripts/verify.sh` for this ticket iteration because the S173 harness finalization owns the full pre-push gate.
