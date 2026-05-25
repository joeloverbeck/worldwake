# S173SELCARINT-003: `PromotableContentionKind` self-care classification

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — crate-private enum variant additions in `worldwake-systems` + new match arms in `promotable_contention_kind` and `contention_target_matches_kind`
**Deps**: `specs/S173-self-care-interruption-occupancy.md` (D3)

## Problem

The contention substrate (`S44` `ContentionQueue`, `S142` queue-grant promotion) is already in place, but `promotable_contention_kind` (`crates/worldwake-systems/src/facility_queue.rs:463-473`) classifies only `(Corpse, "loot"|"bury")`, `(Care, "heal")`, and the `exclusive_facility_workstation_tag` auto-promotion path for `ActionPayload::Harvest`/`Craft`. Wash and Toilet actions use `ActionDomain::Needs` + `ActionPayload::None`, so neither flows through any classification today — meaning the contention queue does not promote grants for self-care actions and two dirty agents at the same basin cannot lawfully contend through the queue substrate. This ticket extends `PromotableContentionKind` with two new unit variants and adds the corresponding classifier + downstream match arms so wash and toilet can participate in the existing queue/grant flow once ticket 004 wires reservation requirements onto them.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PromotableContentionKind` is defined at `crates/worldwake-systems/src/facility_queue.rs:29-33` as `enum { FacilityExclusive(WorkstationTag), Corpse, Care }` — crate-private (no `pub`). `promotable_contention_kind` at L463-473 returns `Option<PromotableContentionKind>`. `contention_target_matches_kind` at L478+ exhaustively matches the enum (lines 481, 490, 494). Both call sites must gain new arms for `SelfCareWash` and `SelfCareLatrine`.
2. `exclusive_facility_workstation_tag` (`crates/worldwake-systems/src/facility_queue_actions.rs:152-172`) matches only on `ActionPayload::Harvest` and `ActionPayload::Craft` payloads. Wash and Toilet use `ActionPayload::None`, so neither flows through the `FacilityExclusive(WashBasin)` auto-promotion path today. Both `SelfCareWash` and `SelfCareLatrine` are genuine net-new variants — confirmed by Step 2 mini-investigation. No subsumption into `FacilityExclusive`.
3. Shared abstraction boundary: the crate-private `PromotableContentionKind` enum and the `(ActionDomain, action_name)` classifier pattern. Cross-crate consumers DO NOT exist — the enum is not `pub`-exported and is not referenced outside `facility_queue.rs` and `facility_queue_actions.rs`. No Core-Side Mirror Enum pattern applies (the enum is sim/systems-crate-local, not core-resident).
4. `ContentionPolicy` (`crates/worldwake-core/src/contention.rs:52-56`) is per-facility-entity, not per-kind. Spec D3 commits explicitly to no per-kind policy routing: the same policy applies to self-care facilities as to other exclusive workstations. This resolves the original Open Question #1 from the spec draft.
5. Existing tests in `facility_queue.rs` (#[cfg(test)] from L598): `dead_actor_is_pruned_from_queue` (L920), `departed_actor_is_pruned_from_queue` (L951), `deallocated_actor_is_pruned_from_queue` (L980), and others. Adding variants without altering the classifier's existing arms preserves behavior for all existing actions; these tests should pass unchanged.

## Architecture Check

1. The enum and classifier remain crate-private to `worldwake-systems`. Per spec D3, no relocation to `worldwake-core` is needed because no core-resident type references the enum. This honors the smallest-blast-radius FND-28 path — relocation would have a much larger blast radius for no architectural benefit.
2. Two new unit variants (no payload) keep the enum compact and preserve the existing `#[derive(Clone, Copy)]` on `PromotableContentionKind`. No risk of `#[allow(clippy::large_enum_variant)]` triggering.
3. Self-care contention reuses the existing queue/grant machinery (`ContentionQueue`, `EventTag::ContentionResolved`, `EventTag::QueueGrantPromoted`) rather than introducing a parallel queue — fully FND-28-aligned.

## Verification Layers

1. Classifier output → focused unit test: `promotable_contention_kind(wash_def)` returns `Some(PromotableContentionKind::SelfCareWash)`; same for toilet returning `SelfCareLatrine`; other action defs return their existing classifications unchanged.
2. Exhaustive match coverage → compile-time: the workspace builds only when the new arms are added to `contention_target_matches_kind`.
3. Single-layer ticket (classification only): downstream queue behavior (grant promotion, contention resolution) is exercised by ticket 007's Scenario B golden. This ticket's contract is solely the classifier's output.

## What to Change

### 1. Extend `PromotableContentionKind`

In `crates/worldwake-systems/src/facility_queue.rs:29-33`:

```rust
#[derive(Clone, Copy)]
enum PromotableContentionKind {
    FacilityExclusive(WorkstationTag),
    Corpse,
    Care,
    /// Wash action contention — wash at a `WashBasin`-tagged `Facility`.
    /// Action uses `ActionDomain::Needs` + `ActionPayload::None`; classification
    /// goes through `(ActionDomain, action_name)` matching rather than
    /// `exclusive_facility_workstation_tag` (which matches only Harvest/Craft).
    SelfCareWash,
    /// Toilet action contention — relief at a `Latrine`-tagged `Place`.
    /// Same classification mechanism as `SelfCareWash`.
    SelfCareLatrine,
}
```

### 2. Add classifier match arms

In `promotable_contention_kind` at L463-473, add two new arms to the `match (def.domain, def.name.as_str())` block:

```rust
fn promotable_contention_kind(def: &worldwake_sim::ActionDef) -> Option<PromotableContentionKind> {
    if let Some(tag) = exclusive_facility_workstation_tag(def) {
        return Some(PromotableContentionKind::FacilityExclusive(tag));
    }

    match (def.domain, def.name.as_str()) {
        (ActionDomain::Corpse, "loot" | "bury") => Some(PromotableContentionKind::Corpse),
        (ActionDomain::Care, "heal") => Some(PromotableContentionKind::Care),
        (ActionDomain::Needs, "wash") => Some(PromotableContentionKind::SelfCareWash),
        (ActionDomain::Needs, "toilet") => Some(PromotableContentionKind::SelfCareLatrine),
        _ => None,
    }
}
```

### 3. Add downstream match arms in `contention_target_matches_kind`

In the exhaustive `match kind` at L478+, add arms for the two new variants. The behavior should follow `Corpse`/`Care` precedent (target matching by entity identity). Specifically: `SelfCareWash` matches when the target entity is a `Facility` carrying `WorkstationTag::WashBasin` (the wash action's target); `SelfCareLatrine` matches when the target entity is a `Place` carrying `PlaceTag::Latrine` (the toilet action's target). Verify the exact match shape at implementation time by reading the existing `Corpse` and `Care` arm bodies — both follow the same per-entity-target pattern but the target-resolution helper differs.

## Files to Touch

- `crates/worldwake-systems/src/facility_queue.rs` (modify — enum variant additions + two classifier match arms + two `contention_target_matches_kind` match arms)

The change is contained to a single file because `PromotableContentionKind` is crate-private. No other workspace crate references it.

## Out of Scope

- Writing `SelfCareOccupancy` on action start — owned by ticket 004.
- Adding `reservation_requirements` to wash/toilet action registrations — owned by ticket 004.
- `ContentionPolicy` per-kind routing — explicitly rejected per spec Non-Goals; same per-facility policy applies.
- Promotion of `PromotableContentionKind` to a `pub` export — not needed; no core-resident or cross-crate consumer.

## Acceptance Criteria

### Tests That Must Pass

1. New unit test: `promotable_contention_kind_classifies_wash_action_as_self_care_wash` — construct a wash `ActionDef` (or use the existing test helper), assert classifier returns `Some(PromotableContentionKind::SelfCareWash)`.
2. New unit test: `promotable_contention_kind_classifies_toilet_action_as_self_care_latrine` — same shape for toilet.
3. New unit test: `promotable_contention_kind_unchanged_for_existing_actions` — corpse/care/harvest/craft actions retain their existing classifications.
4. Existing suite: `cargo test -p worldwake-systems facility_queue`.

### Invariants

1. `promotable_contention_kind` output for all pre-existing actions is unchanged.
2. `contention_target_matches_kind` exhaustively covers all five variants (compile-time enforced).
3. No core crate depends on `PromotableContentionKind` (verified by `worldwake-core/Cargo.toml` not depending on `worldwake-systems`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/facility_queue.rs` inline `#[cfg(test)]` (from L598) — three new test cases covering the classifier for wash, toilet, and unchanged-existing-actions.

### Commands

1. `cargo test -p worldwake-systems facility_queue`
2. `cargo build --workspace -- -D warnings` (verify no exhaustive-match warnings in `contention_target_matches_kind`)
3. `./scripts/verify.sh` before commit.
