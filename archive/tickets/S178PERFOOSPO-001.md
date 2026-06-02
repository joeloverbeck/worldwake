# S178PERFOOSPO-001: Perishable foundation types and CommodityPerishProfile

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new ECS component (`PerishableState`), new world-authored profile map (`CommodityPerishProfile` + `StorageRateMultipliers`), new `EventTag::ItemSpoiled` variant, new `Freshness` derived enum, `SAVE_FORMAT_VERSION` 115→116.
**Deps**: spec `specs/S178-perishable-food-spoilage.md`

## Problem

Before this ticket, `LotOperation::Spoiled` existed in the item-lot lineage enum but was never constructed anywhere; food spoilage was binary archive-only and ignored per-lot freshness. This ticket laid the foundation types — per-lot mutable freshness component, per-commodity perish profile with storage-context multipliers, event tag, derived band enum, world-authored profile map — on which the decay system (003), Eat handler (004), belief view (005), candidate generation (006), and forensics (007) build. It bumped `SAVE_FORMAT_VERSION` 115→116 to cover the new component, the new event-tag variant, and the new world-authored profile map.

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `LotOperation::Spoiled` exists at `crates/worldwake-core/src/items.rs:232` (variant declared, 0 emission sites workspace-wide via `rg "LotOperation::Spoiled" crates/`). `ItemLot` at `crates/worldwake-core/src/items.rs:317` carries `commodity`, `quantity`, `provenance`, `quality: Option<WaterQuality>` (S177-added) — `PerishableState` is a separate sparse component, not a field on `ItemLot`. `CommodityDecayMap` at `crates/worldwake-core/src/items.rs:338` is `BTreeMap<CommodityKind, NonZeroU32>` with defaults `Apple=720`, `Waste=200` (verified via `default_commodity_decay_map_contains_expected_defaults` test). Existing item-lot component registration precedent: `GroundSince` at `crates/worldwake-core/src/component_schema.rs:2469-2493` with kind filter `|kind| kind == EntityKind::ItemLot`. `EventTag` enum at `crates/worldwake-core/src/event_tag.rs:7-57` (48 variants), canonical `ALL` array at `event_tag.rs:69-118`. No exhaustive match sites on `EventTag` in production code; 2 test-side use sites (`golden_harness/mod.rs`, `simulation_gaps.rs`) are filter/tag operations, not exhaustive matches. Adding `ItemSpoiled` is forward-declared-safe per the spec-to-tickets forward-declared-enum-variants rule — emission lands in ticket 003 and no exhaustive match breaks.
2. Spec D1, D2 verified against current `specs/S178-perishable-food-spoilage.md` (post-reassessment). FOUNDATIONS Alignment row for FND-3 mandates `Permille` for [0,1000] range values; D1 uses `Permille` for `condition`. Section H stored-state list classifies `PerishableState`, `CommodityPerishProfile`, `LotOperation::Spoiled`, `EventTag::ItemSpoiled` as authoritative; `Freshness` band as derived. The spec's Component Registration section at lines 187-189 specifies the item-lot entity-kind filter.
3. Shared abstraction boundary: the item-lot ECS-component surface (kind filter `EntityKind::ItemLot`) and the world-authored `BTreeMap<CommodityKind, _>` profile surface (parallel to `CommodityDecayMap`'s scenario integration). Both are core-resident; no cross-crate type movement required (spot-check (i) confirmed all field types resolve to `worldwake-core` symbols — `Permille` (numerics.rs), `Tick` (ids.rs), `NonZeroU32` (std)).
4. Authoritative arithmetic for D2 defaults pinned by S178 reassessment: `Apple.fresh_to_spoiled_ticks = nz(720)`, `stale_threshold = Permille::new_unchecked(667)`, `spoiled_threshold = Permille::new_unchecked(333)`, `storage_rates: { ground: 1000, container: 500, possessed: 750 }`. Per-tick condition delta at ground baseline: `1000 / 720 ≈ 1.39 permille/tick` (Fresh band ≈ 240 ticks; Stale ≈ 240; Spoiled ≈ 240 to reach 0; lot persists at condition 0 thereafter). The equal-thirds split matches the spec's pinning paragraph; verified internally consistent against `Fresh = condition >= 667`, `Stale = 667 > condition >= 333`, `Spoiled = condition < 333`.

## Architecture Check

1. `PerishableState` as a separate ECS component (not a field on `ItemLot`) keeps the perishable population sparse — non-perishable lots carry no component, item-decay's iteration is `query_with(PerishableState)` rather than "every lot", and lots can opt in/out without touching the `ItemLot` struct. Mirrors the `GroundSince` precedent (mutable system-driven tick state as a separate component) rather than the S177 `ItemLot.quality` precedent (immutable creation-time enum). The spec's D1 Architecture-rationale paragraph documents this choice explicitly.
2. No backwards-compatibility shim. The `CommodityDecayMap.Apple` entry stays for back-compat with non-perishable tests during transition; ticket 003 adds a `world.has_component_perishable_state(lot)` filter in `collect_decay_targets` so perishable lots never hit the archive-at-duration path even though they remain map-listed. Once ticket 003 lands, the perishable-Apple archive path is structurally unreachable (FND-28 compliance — one live path per fact). The map entry itself becomes dead config in a future cleanup ticket, not part of this spec's scope.

## Verified Layers

1. `PerishableState` component round-trips through save/load (post-`SAVE_FORMAT_VERSION` 115→116) → focused save-load equivalence test in `crates/worldwake-sim/src/save_load.rs` test module.
2. `CommodityPerishProfile` default map contains the pinned `Apple` entry with the spec-pinned thresholds and multipliers → focused unit test in `crates/worldwake-core/src/items.rs` test module.
3. `EventTag::ItemSpoiled` is in the canonical `ALL` array and its discriminant addition does not perturb existing variants' relative `Ord` ordering → focused unit test asserting `ALL` array length grew by exactly 1 and the new variant's position is at the end.
4. `Freshness::derive_from(condition, profile)` returns the correct band for boundary cases (`condition == stale_threshold` → `Fresh`; `condition == spoiled_threshold` → `Stale`; below → `Spoiled`) → focused unit test.
5. Single-layer ticket: no cross-system invariants to map; emission and downstream effects land in tickets 003+.

## Landed Changes

### 1. `PerishableState` component on item lots

In `crates/worldwake-core/src/items.rs`, define:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerishableState {
    pub condition: Permille,
    pub last_advanced_tick: Tick,
    /// Added by ticket 003 to carry integer-only fractional decay across ticks.
    pub decay_remainder: u32,
}

impl Component for PerishableState {}
```

No `Default` impl — the component is always constructed with the explicit creation tick. Ticket 003 attaches it at perishable-commodity lot spawn (`PerishableState { condition: Permille::new_unchecked(1000), last_advanced_tick: creation_tick, decay_remainder: 0 }`) and copies the state when lots split.

### 2. `CommodityPerishProfile`, `StorageRateMultipliers`, `Freshness` enum, default map

In `crates/worldwake-core/src/items.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageRateMultipliers {
    pub ground: Permille,
    pub container: Permille,
    pub possessed: Permille,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommodityPerishProfile {
    pub fresh_to_spoiled_ticks: NonZeroU32,
    pub stale_threshold: Permille,
    pub spoiled_threshold: Permille,
    pub storage_rates: StorageRateMultipliers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Freshness { Fresh, Stale, Spoiled }

impl Freshness {
    pub fn derive_from(condition: Permille, profile: &CommodityPerishProfile) -> Self {
        if condition.value() >= profile.stale_threshold.value() {
            Freshness::Fresh
        } else if condition.value() >= profile.spoiled_threshold.value() {
            Freshness::Stale
        } else {
            Freshness::Spoiled
        }
    }
}

pub type CommodityPerishProfileMap = BTreeMap<CommodityKind, CommodityPerishProfile>;

#[must_use]
pub fn default_commodity_perish_profile_map() -> CommodityPerishProfileMap {
    BTreeMap::from([
        (CommodityKind::Apple, CommodityPerishProfile {
            fresh_to_spoiled_ticks: nz(720),
            stale_threshold: Permille::new_unchecked(667),
            spoiled_threshold: Permille::new_unchecked(333),
            storage_rates: StorageRateMultipliers {
                ground: Permille::new_unchecked(1000),
                container: Permille::new_unchecked(500),
                possessed: Permille::new_unchecked(750),
            },
        }),
    ])
}
```

### 3. `EventTag::ItemSpoiled` variant

In `crates/worldwake-core/src/event_tag.rs`, append `ItemSpoiled` to the enum (after `ItemDecay` at line 31 to keep related variants adjacent) and to the `ALL` array (lines 69-118). No exhaustive match arms exist in production code for `EventTag`; test-side match sites use filter/tag operations that survive variant addition. Forward-declared per the spec-to-tickets rule: emission lands in ticket 003.

### 4. `PerishableState` component registration

In `crates/worldwake-core/src/component_schema.rs`, register `PerishableState` via the `with_component_schema_entries!` macro with kind filter `|kind| kind == EntityKind::ItemLot`, following the `GroundSince` precedent at lines 2469-2493. Per `tickets/README.md` check #13, verify expansion sites `delta.rs`, `world.rs`, `component_tables.rs` import `PerishableState`.

### 5. World-authored `CommodityPerishProfile` map plumbing

In `crates/worldwake-cli/src/scenario/types.rs`, add to `ScenarioDef`:

```rust
#[serde(default)]
pub commodity_perish_profile: Option<CommodityPerishProfileMap>,
```

In `crates/worldwake-cli/src/scenario/mod.rs`, wire the loaded map into the world during scenario spawn (parallel to how `CommodityDecayMap` is currently authored). Missing scenario field → `default_commodity_perish_profile_map()`.

In `crates/worldwake-core/src/world.rs`, alongside the existing `commodity_decay()` accessor (or whichever accessor surfaces `CommodityDecayMap`), add `commodity_perish_profiles() -> &CommodityPerishProfileMap` returning the world-stored map. The map storage uses the same mechanism as `CommodityDecayMap`'s storage in the world's substrate.

### 6. `SAVE_FORMAT_VERSION` bump 115→116

In `crates/worldwake-sim/src/save_load.rs`, bump `SAVE_FORMAT_VERSION` from 115 to 116 to cover (a) the new `PerishableState` ECS component in `SimulationState`, (b) the new `EventTag::ItemSpoiled` variant in serialized event payloads, (c) the new `CommodityPerishProfileMap` in world substrate. Add load/store handling for the new component. Ticket 003 later extends `PerishableState` with `decay_remainder` and bumps the version again to 118; later tickets must reassess their own persisted-shape changes against that live baseline.

## Landed Files

- `crates/worldwake-core/src/items.rs` (modify — add `PerishableState`, `CommodityPerishProfile`, `StorageRateMultipliers`, `Freshness`, `CommodityPerishProfileMap`, `default_commodity_perish_profile_map`, `Freshness::derive_from`)
- `crates/worldwake-core/src/event_tag.rs` (modify — add `ItemSpoiled` variant + `ALL` array entry)
- `crates/worldwake-core/src/component_schema.rs` (modify — register `PerishableState`)
- `crates/worldwake-core/src/world.rs` (modify — add `commodity_perish_profiles()` accessor; verify `PerishableState` imported for macro expansion per `tickets/README.md` check #13)
- `crates/worldwake-core/src/delta.rs` (modify — verify `PerishableState` imported for macro expansion per check #13)
- `crates/worldwake-core/src/component_tables.rs` (modify — verify `PerishableState` imported for macro expansion per check #13)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — add `commodity_perish_profile` field to `ScenarioDef`)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — wire profile map into world spawn)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump `SAVE_FORMAT_VERSION` 115→116; serde for `PerishableState` + `CommodityPerishProfileMap`)

## Out of Scope

- Item-decay system mutation of `PerishableState.condition` (ticket 003).
- Eat-handler relief scaling by condition (ticket 004).
- Belief-view accessors and perception write of `last_observed_condition` (ticket 005).
- Candidate generation / desperation gate (ticket 006).
- Forensic record `SpoiledFoodDiscovery` (ticket 007).
- Goldens (ticket 008).
- Preserving-place tag or component (deferred per spec Non-Goals to a future Cold Storage spec).
- `Grain` / `Bread` perishability (deferred per spec Non-Goals).
- Initialization of `PerishableState` at lot creation — placeholder, replaced by ticket 003's spawn-side attach at harvest/production/scenario lot creation. This ticket only declares the type and registration; runtime attachment happens in ticket 003.

## Acceptance Result

### Tests Passed

1. `default_commodity_perish_profile_map_contains_pinned_apple_entry` — asserts the Apple entry has `fresh_to_spoiled_ticks=nz(720)`, `stale_threshold=Permille(667)`, `spoiled_threshold=Permille(333)`, and the three storage multipliers (1000/500/750).
2. `freshness_derive_from_matches_band_thresholds` — asserts `condition >= stale_threshold → Fresh`, between thresholds → `Stale`, below `spoiled_threshold → Spoiled`, including the inclusive-equality boundary cases.
3. `save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state` — asserted a `SimulationState` carrying an item lot with `PerishableState` and a non-default `CommodityPerishProfileMap` serialized at `SAVE_FORMAT_VERSION=116` and deserialized equivalently for this ticket; ticket 003 updates this witness to version 118 and explicitly asserts `decay_remainder` roundtrip.
4. `event_tag::tests::event_tag_includes_all_required_variants` — asserts `EventTag::ItemSpoiled` is present and the array length grew by one.
5. Existing suites: `cargo test -p worldwake-core`, `cargo test -p worldwake-sim`, and `cargo test -p worldwake-cli --lib`.

### Invariants

1. `PerishableState` is registered only on `EntityKind::ItemLot` (kind filter); attempts to attach it to other entity kinds produce a registration error.
2. `EventTag::ItemSpoiled`'s discriminant addition preserves the relative ordering of pre-existing variants (no perturbation of derived `Ord`).
3. `default_commodity_perish_profile_map` returns a `BTreeMap` (deterministic iteration order per `AGENTS.md` Determinism invariant).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/items.rs` `#[cfg(test)]` — add 2 new unit tests (default-map content; `Freshness` band derivation).
2. `crates/worldwake-core/src/event_tag.rs` `#[cfg(test)]` — extended the existing `ALL`-array test to include `ItemSpoiled`.
3. `crates/worldwake-core/src/world.rs` `#[cfg(test)]` — added the item-lot-only registration rejection test for `PerishableState`.
4. `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` — extended the full non-default save/load round-trip test for `PerishableState` and `CommodityPerishProfileMap`.
5. `crates/worldwake-cli/src/scenario/types.rs` and `crates/worldwake-cli/src/scenario/mod.rs` `#[cfg(test)]` — added RON omitted/explicit field and spawn default/override tests for `commodity_perish_profile`.

## Outcome

Completed on 2026-06-02.

- Added `PerishableState`, `StorageRateMultipliers`, `CommodityPerishProfile`, `CommodityPerishProfileMap`, `Freshness`, and `default_commodity_perish_profile_map()` in `worldwake-core`.
- Registered `PerishableState` as an item-lot-only component and updated macro expansion imports, component tables, component samples, and component-kind inventories.
- Added `EventTag::ItemSpoiled` as staged substrate for ticket 003 emission.
- Added `World::commodity_perish_profiles()` / `set_commodity_perish_profiles()` and initialized worlds with the default perishable profile map.
- Added `ScenarioDef.commodity_perish_profile` authoring and spawn wiring, plus exhaustive `ScenarioDef` constructor fallout across CLI and golden test fixtures.
- Bumped `SAVE_FORMAT_VERSION` from 115 to 116 and extended save/load coverage with a non-default perishable component and perish-profile map.

## Deviations

- The save/load proof reused and extended the existing full non-default roundtrip test instead of adding a narrowly named `perishable_state_round_trips_through_save_load` test. This provides stronger coverage because it proves the new component and world map inside the persisted `SimulationState`.
- The drafted combined focused Cargo command was invalid for Cargo's single-filter syntax, so focused proofs were run as separate valid commands.
- `./scripts/verify.sh` is waived for this per-ticket closeout because the implement-spec-tickets harness owns the final full pre-PR verification after all S178 tickets land.

## Verification Result

- Passed `cargo test -p worldwake-core --lib default_commodity_perish_profile_map_contains_pinned_apple_entry`.
- Passed `cargo test -p worldwake-core --lib freshness_derive_from_matches_band_thresholds`.
- Passed `cargo test -p worldwake-core --lib event_tag_includes_all_required_variants`.
- Passed `cargo test -p worldwake-sim --lib save_to_bytes_roundtrip_preserves_full_nondefault_state`.
- Passed `cargo test -p worldwake-cli --lib commodity_perish_profile`.
- Passed `cargo test -p worldwake-core`.
- Passed `cargo test -p worldwake-sim`.
- Passed `cargo test -p worldwake-cli --lib`.
- Passed `cargo test --workspace --no-run`.
- Waived `./scripts/verify.sh` because the final S178 branch closeout owns the full pre-PR gate after tickets 001-008 land.
