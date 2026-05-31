# S178PERFOOSPO-001: Perishable foundation types and CommodityPerishProfile

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new ECS component (`PerishableState`), new world-authored profile map (`CommodityPerishProfile` + `StorageRateMultipliers`), new `EventTag::ItemSpoiled` variant, new `Freshness` derived enum, `SAVE_FORMAT_VERSION` 115→116.
**Deps**: spec `specs/S178-perishable-food-spoilage.md`

## Problem

`LotOperation::Spoiled` exists in the item-lot lineage enum but is never constructed anywhere; food spoilage today is binary archive-only and ignores per-lot freshness. Lay the foundation types — per-lot mutable freshness component, per-commodity perish profile with storage-context multipliers, event tag, derived band enum, world-authored profile map — on which the decay system (003), Eat handler (004), belief view (005), candidate generation (006), and forensics (007) build. Bumps `SAVE_FORMAT_VERSION` 115→116 to cover the new component, the new event-tag variant, and the new world-authored profile map.

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `LotOperation::Spoiled` exists at `crates/worldwake-core/src/items.rs:232` (variant declared, 0 emission sites workspace-wide via `rg "LotOperation::Spoiled" crates/`). `ItemLot` at `crates/worldwake-core/src/items.rs:317` carries `commodity`, `quantity`, `provenance`, `quality: Option<WaterQuality>` (S177-added) — `PerishableState` is a separate sparse component, not a field on `ItemLot`. `CommodityDecayMap` at `crates/worldwake-core/src/items.rs:338` is `BTreeMap<CommodityKind, NonZeroU32>` with defaults `Apple=720`, `Waste=200` (verified via `default_commodity_decay_map_contains_expected_defaults` test). Existing item-lot component registration precedent: `GroundSince` at `crates/worldwake-core/src/component_schema.rs:2469-2493` with kind filter `|kind| kind == EntityKind::ItemLot`. `EventTag` enum at `crates/worldwake-core/src/event_tag.rs:7-57` (48 variants), canonical `ALL` array at `event_tag.rs:69-118`. No exhaustive match sites on `EventTag` in production code; 2 test-side use sites (`golden_harness/mod.rs`, `simulation_gaps.rs`) are filter/tag operations, not exhaustive matches. Adding `ItemSpoiled` is forward-declared-safe per the spec-to-tickets forward-declared-enum-variants rule — emission lands in ticket 003 and no exhaustive match breaks.
2. Spec D1, D2 verified against current `specs/S178-perishable-food-spoilage.md` (post-reassessment). FOUNDATIONS Alignment row for FND-3 mandates `Permille` for [0,1000] range values; D1 uses `Permille` for `condition`. Section H stored-state list classifies `PerishableState`, `CommodityPerishProfile`, `LotOperation::Spoiled`, `EventTag::ItemSpoiled` as authoritative; `Freshness` band as derived. The spec's Component Registration section at lines 187-189 specifies the item-lot entity-kind filter.
3. Shared abstraction boundary: the item-lot ECS-component surface (kind filter `EntityKind::ItemLot`) and the world-authored `BTreeMap<CommodityKind, _>` profile surface (parallel to `CommodityDecayMap`'s scenario integration). Both are core-resident; no cross-crate type movement required (spot-check (i) confirmed all field types resolve to `worldwake-core` symbols — `Permille` (numerics.rs), `Tick` (ids.rs), `NonZeroU32` (std)).
4. Authoritative arithmetic for D2 defaults pinned by S178 reassessment: `Apple.fresh_to_spoiled_ticks = nz(720)`, `stale_threshold = Permille::new_unchecked(667)`, `spoiled_threshold = Permille::new_unchecked(333)`, `storage_rates: { ground: 1000, container: 500, possessed: 750 }`. Per-tick condition delta at ground baseline: `1000 / 720 ≈ 1.39 permille/tick` (Fresh band ≈ 240 ticks; Stale ≈ 240; Spoiled ≈ 240 to reach 0; lot persists at condition 0 thereafter). The equal-thirds split matches the spec's pinning paragraph; verified internally consistent against `Fresh = condition >= 667`, `Stale = 667 > condition >= 333`, `Spoiled = condition < 333`.

## Architecture Check

1. `PerishableState` as a separate ECS component (not a field on `ItemLot`) keeps the perishable population sparse — non-perishable lots carry no component, item-decay's iteration is `query_with(PerishableState)` rather than "every lot", and lots can opt in/out without touching the `ItemLot` struct. Mirrors the `GroundSince` precedent (mutable system-driven tick state as a separate component) rather than the S177 `ItemLot.quality` precedent (immutable creation-time enum). The spec's D1 Architecture-rationale paragraph documents this choice explicitly.
2. No backwards-compatibility shim. The `CommodityDecayMap.Apple` entry stays for back-compat with non-perishable tests during transition; ticket 003 adds a `world.has_component_perishable_state(lot)` filter in `collect_decay_targets` so perishable lots never hit the archive-at-duration path even though they remain map-listed. Once ticket 003 lands, the perishable-Apple archive path is structurally unreachable (FND-28 compliance — one live path per fact). The map entry itself becomes dead config in a future cleanup ticket, not part of this spec's scope.

## Verification Layers

1. `PerishableState` component round-trips through save/load (post-`SAVE_FORMAT_VERSION` 115→116) → focused save-load equivalence test in `crates/worldwake-sim/src/save_load.rs` test module.
2. `CommodityPerishProfile` default map contains the pinned `Apple` entry with the spec-pinned thresholds and multipliers → focused unit test in `crates/worldwake-core/src/items.rs` test module.
3. `EventTag::ItemSpoiled` is in the canonical `ALL` array and its discriminant addition does not perturb existing variants' relative `Ord` ordering → focused unit test asserting `ALL` array length grew by exactly 1 and the new variant's position is at the end.
4. `Freshness::derive_from(condition, profile)` returns the correct band for boundary cases (`condition == stale_threshold` → `Fresh`; `condition == spoiled_threshold` → `Stale`; below → `Spoiled`) → focused unit test.
5. Single-layer ticket: no cross-system invariants to map; emission and downstream effects land in tickets 003+.

## What to Change

### 1. `PerishableState` component on item lots

In `crates/worldwake-core/src/items.rs`, define:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerishableState {
    pub condition: Permille,
    pub last_advanced_tick: Tick,
}

impl Component for PerishableState {}
```

No `Default` impl — the component is always constructed with the explicit creation tick. Ticket 003 attaches it at perishable-commodity lot spawn (`PerishableState { condition: Permille::new_unchecked(1000), last_advanced_tick: creation_tick }`).

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

In `crates/worldwake-sim/src/save_load.rs`, bump `SAVE_FORMAT_VERSION` from 115 to 116 to cover (a) the new `PerishableState` ECS component in `SimulationState`, (b) the new `EventTag::ItemSpoiled` variant in serialized event payloads, (c) the new `CommodityPerishProfileMap` in world substrate. Add load/store handling for the new component. Tickets 002, 003, 004, 005, 006, 007, 008 ride this bump via `#[serde(default)]` on any incrementally added fields where appropriate.

## Files to Touch

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

## Acceptance Criteria

### Tests That Must Pass

1. `default_commodity_perish_profile_map_contains_pinned_apple_entry` — asserts the Apple entry has `fresh_to_spoiled_ticks=nz(720)`, `stale_threshold=Permille(667)`, `spoiled_threshold=Permille(333)`, and the three storage multipliers (1000/500/750).
2. `freshness_derive_from_matches_band_thresholds` — asserts `condition >= stale_threshold → Fresh`, between thresholds → `Stale`, below `spoiled_threshold → Spoiled`, including the inclusive-equality boundary cases.
3. `perishable_state_round_trips_through_save_load` — asserts a `SimulationState` carrying an item lot with `PerishableState` serializes at `SAVE_FORMAT_VERSION=116` and deserializes byte-for-byte equivalent.
4. `event_tag_item_spoiled_is_in_all_array` — asserts `EventTag::ALL.contains(&EventTag::ItemSpoiled)` and the array length grew by one.
5. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-sim`.

### Invariants

1. `PerishableState` is registered only on `EntityKind::ItemLot` (kind filter); attempts to attach it to other entity kinds produce a registration error.
2. `EventTag::ItemSpoiled`'s discriminant addition preserves the relative ordering of pre-existing variants (no perturbation of derived `Ord`).
3. `default_commodity_perish_profile_map` returns a `BTreeMap` (deterministic iteration order per CLAUDE.md Determinism invariant).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` `#[cfg(test)]` — add 2 new unit tests (default-map content; `Freshness` band derivation).
2. `crates/worldwake-core/src/event_tag.rs` `#[cfg(test)]` — extend the existing `ALL`-array test to include `ItemSpoiled`.
3. `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` — add 1 round-trip test for `PerishableState`.

### Commands

1. `cargo test -p worldwake-core -- items::tests::default_commodity_perish_profile_map_contains_pinned_apple_entry items::tests::freshness_derive_from_matches_band_thresholds`
2. `cargo test -p worldwake-core` and `cargo test -p worldwake-sim`
3. `./scripts/verify.sh`
