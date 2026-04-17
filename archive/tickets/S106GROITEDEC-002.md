# S106GROITEDEC-002: Decay infrastructure — types, EventTag, SystemId, ScenarioDef

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `CommodityDecayMap` type, `EventTag::ItemDecay` variant, `SystemId::ItemDecay`, `ScenarioDef` field, world config storage
**Deps**: None

## Problem

The `ItemDecay` system (ticket 003) requires infrastructure that spans multiple crates: a type for the decay configuration, an event tag for traceability, a system ID for scheduling, a scenario field for configuration, and world-level storage. This ticket provides all of that infrastructure so ticket 003 can focus purely on the system logic.

## Assumption Reassessment (2026-04-16)

1. `EventTag` enum is at `crates/worldwake-core/src/event_tag.rs:7-33` with 25 variants. `ItemDecay` does not exist yet. The live exhaustive ownership is the enum plus the local `ALL_EVENT_TAGS` test inventory in the same file; there is no broader workspace `match` fallout today.
2. `define_system_ids!` macro is at `crates/worldwake-sim/src/system_manifest.rs:60-74` with 13 variants. `SYSTEM_COUNT` is derived from this macro. Adding a variant increments the dense dispatch array size in `crates/worldwake-sim/src/system_dispatch.rs`, requires a matching handler entry in `crates/worldwake-systems/src/lib.rs`, and requires canonical-order test updates in `system_manifest.rs`.
3. Canonical system order is at `system_manifest.rs:108-124`. Current order ends `...Perception → ExpectationCheck → EvidenceDecay → Patrol → Compaction`, so `ItemDecay` belongs between `EvidenceDecay` and `Patrol`. The active spec's `SystemFn Integration` sentence that says “between `EvidenceDecay` and `ExpectationCheck`” is stale and must be corrected in this ticket.
4. `ScenarioDef` is at `crates/worldwake-cli/src/scenario/types.rs:22-40` with 8 fields today. Repo-wide `ScenarioDef { ... }` sweeps show exhaustive manual literals across `worldwake-cli` tests and scenario helpers; these will need `commodity_decay: None` once the field lands.
5. `CommodityKind` enum at `crates/worldwake-core/src/items.rs:8-30` includes `Waste` and `Apple` — the two commodities with default decay values. `items.rs` already owns other serde/default helper tests, making it the right home for the default map type/function plus focused bounds/roundtrip proof.
6. `SystemExecutionContext` already exposes `pub world: &mut World` (`crates/worldwake-sim/src/system_dispatch.rs:13-23`), and `SimulationState` persists `World` directly (`crates/worldwake-sim/src/simulation_state.rs:6-27`). The smallest lawful storage seam is therefore a direct `World` field plus accessor/mutator methods, not a singleton config entity.
7. Because `World` is part of the serialized save payload, adding a persisted `commodity_decay` field changes the current save shape. Per repo policy there is no backward-compat layer, so this ticket must update `SAVE_FORMAT_VERSION` and the focused save/load proof in `crates/worldwake-sim/src/save_load.rs`.

## Architecture Check

1. Separating infrastructure from logic (this ticket vs. 003) keeps each diff focused. A stub `item_decay_system` (returns `Ok(())`) satisfies the dispatch table requirement while deferring logic to 003.
2. No backward-compatibility shims. `commodity_decay` field uses `#[serde(default)]` so existing RON scenarios deserialize without changes, while the persisted `World` format is updated in place through the current save version. The default provides Waste: 200, Apple: 720.

## Verification Layers

1. EventTag::ItemDecay variant exists and compiles → focused enum inventory / serde roundtrip test
2. SystemId::ItemDecay in canonical order → existing ordering tests in `system_manifest.rs`
3. dispatch_table routes ItemDecay to stub → focused systems test (call `dispatch_table().get(SystemId::ItemDecay)` and verify it returns `Ok(())`)
4. ScenarioDef deserializes with and without commodity_decay → focused `scenario::types` tests
5. World stores and save-load roundtrips commodity decay config → focused `worldwake-core` / `save_load.rs` tests
6. Single-layer ticket (infrastructure only, no runtime behavior) — additional layer mapping not applicable

## What to Change

### 1. CommodityDecayMap type

In `crates/worldwake-core/src/items.rs`:

```rust
pub type CommodityDecayMap = BTreeMap<CommodityKind, NonZeroU32>;
```

Add a `pub fn default_commodity_decay_map() -> CommodityDecayMap` that returns `{Waste: 200, Apple: 720}`.

Re-export from `crates/worldwake-core/src/lib.rs`.

### 2. EventTag::ItemDecay variant

In `crates/worldwake-core/src/event_tag.rs`, add `ItemDecay` variant to the `EventTag` enum. Grep all exhaustive match sites for `EventTag` across the workspace and add arms.

### 3. SystemId::ItemDecay

In `crates/worldwake-sim/src/system_manifest.rs`:
- Add `(ItemDecay, "item_decay")` to `define_system_ids!` macro — position it after `EvidenceDecay` in the definition order.
- Update `canonical()` to insert `SystemId::ItemDecay` between `SystemId::EvidenceDecay` and `SystemId::Patrol`.
- Update the existing ordering test `canonical_manifest_matches_fixed_scheduler_order` (and `canonical_manifest_places_expectation_check_between_perception_and_evidence_decay` if it asserts adjacency).

### 4. Stub item_decay_system

In `crates/worldwake-systems/src/item_decay.rs` (new file):

```rust
pub fn item_decay_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    // Stub — real implementation in S106GROITEDEC-003
    Ok(())
}
```

Register in `dispatch_table()` at `crates/worldwake-systems/src/lib.rs` at the ordinal matching `SystemId::ItemDecay`.

Add `mod item_decay;` to `crates/worldwake-systems/src/lib.rs`.

### 5. ScenarioDef commodity_decay field

In `crates/worldwake-cli/src/scenario/types.rs`, add to `ScenarioDef`:

```rust
#[serde(default)]
pub commodity_decay: Option<CommodityDecayMap>,
```

Where `None` means "use default_commodity_decay_map()".

Update all 15+ `ScenarioDef {` construction sites in worldwake-cli tests to add `commodity_decay: None`.

### 6. World-level storage for CommodityDecayMap

Store the resolved `CommodityDecayMap` as a direct field on `World`, with a read accessor for systems and a setter used during scenario bootstrap and focused tests. This keeps the config in the persisted authoritative world state already carried by `SimulationState` and avoids inventing a singleton config entity.

Load the decay map during scenario initialization in `crates/worldwake-cli/src/scenario/mod.rs`: if `commodity_decay` is `None`, use `default_commodity_decay_map()`; otherwise use the provided map.

### 7. Persisted-shape fallout

Update `crates/worldwake-sim/src/save_load.rs` for the new persisted `World` field:
- bump `SAVE_FORMAT_VERSION`
- extend the focused roundtrip proof so the non-default save payload includes a non-default `CommodityDecayMap`
- keep older save versions rejected (no compatibility shim)

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — add `CommodityDecayMap` type alias and default fn)
- `crates/worldwake-core/src/event_tag.rs` (modify — add `ItemDecay` variant)
- `crates/worldwake-core/src/lib.rs` (modify — re-export new types)
- `crates/worldwake-core/src/world.rs` (modify — store/read/write commodity decay config)
- `crates/worldwake-sim/src/system_manifest.rs` (modify — add SystemId, update canonical order, update tests)
- `crates/worldwake-sim/src/system_dispatch.rs` (compile fallout only if dense dispatch tests/assertions need updates)
- `crates/worldwake-sim/src/save_load.rs` (modify — current-format save version + roundtrip proof)
- `crates/worldwake-systems/src/item_decay.rs` (new — stub system function)
- `crates/worldwake-systems/src/lib.rs` (modify — add mod, update dispatch_table)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — add field to ScenarioDef)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — load and store decay map)
- `crates/worldwake-cli/src/display.rs` (modify — ScenarioDef construction sites in tests)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify — ScenarioDef construction site)
- `crates/worldwake-cli/src/handlers/actions.rs` (modify — ScenarioDef construction sites)
- `crates/worldwake-cli/src/handlers/events.rs` (modify — ScenarioDef construction site)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify — ScenarioDef construction sites)
- `crates/worldwake-cli/src/handlers/control.rs` (modify — ScenarioDef construction sites)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify — ScenarioDef construction sites)
- EventTag match sites (identify via grep — likely `event_tag.rs` display impl and any filtering/classification code)

## Out of Scope

- The actual decay logic in `item_decay_system` (ticket 003)
- `GroundSince` component (ticket 001)
- Golden E2E tests (ticket 004)
- Decay for carried or stored items (spec non-goal)

## Acceptance Criteria

### Tests That Must Pass

1. `canonical_manifest_matches_fixed_scheduler_order` — updated to include ItemDecay in correct position
2. Dispatch table routes `SystemId::ItemDecay` to stub function that returns `Ok(())`
3. ScenarioDef with `commodity_decay: None` deserializes correctly
4. ScenarioDef with explicit `commodity_decay` map deserializes correctly
5. Non-default `CommodityDecayMap` roundtrips through the current save format
6. Existing suite: `cargo test --workspace` — all existing tests pass

### Invariants

1. `SYSTEM_COUNT` equals the number of entries in `define_system_ids!` (enforced by array sizing in dispatch_table).
2. `canonical()` contains exactly one entry per SystemId (enforced by duplicate check).
3. `EventTag::ItemDecay` is handled in all exhaustive matches.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/system_manifest.rs` (test module) — update canonical-order tests to include `ItemDecay`
2. `crates/worldwake-systems/src/item_decay.rs` (test module) — dispatch table routing test for stub
3. `crates/worldwake-cli/src/scenario/types.rs` (test module) — omitted-field and explicit-value serde tests for `commodity_decay`
4. `crates/worldwake-sim/src/save_load.rs` (test module) — non-default commodity decay save/load roundtrip proof

### Commands

1. `cargo test -p worldwake-sim system_manifest` — targeted manifest tests
2. `cargo test -p worldwake-systems item_decay` — targeted stub tests
3. `cargo test --workspace` — full workspace
4. `cargo clippy --workspace --all-targets -- -D warnings` — lint

## Outcome

Completion date: 2026-04-16

Implemented the full infrastructure substrate for S106 item decay without adding runtime decay behavior yet. `CommodityDecayMap` and default decay config now live in `worldwake-core`, `World` persists the resolved commodity-decay map directly, `EventTag::ItemDecay` and `SystemId::ItemDecay` are registered, and the canonical scheduler/dispatch table now includes a stub `item_decay_system` between `EvidenceDecay` and `Patrol`.

Scenario bootstrap now accepts an optional `commodity_decay` override and otherwise resolves to the spec defaults (`Waste: 200`, `Apple: 720`). Save/load format versioning was updated for the new persisted `World` field, and the focused roundtrip proof now covers a non-default decay map. The active spec sentence about system ordering was also corrected to match the live canonical order.

## Deviations

1. `crates/worldwake-sim/src/system_dispatch.rs` did not require a source edit after reassessment. The dense dispatch array size adjusted automatically through `SYSTEM_COUNT`; the owned code change remained in `crates/worldwake-systems/src/lib.rs`, where the new stub handler was registered at the matching ordinal.

## Verification Result

Passed:

1. `cargo test --workspace --no-run`
2. `cargo test -p worldwake-sim system_manifest::tests::canonical_manifest_matches_fixed_scheduler_order --lib -- --exact`
3. `cargo test -p worldwake-sim system_manifest::tests::canonical_manifest_places_item_decay_between_evidence_decay_and_patrol --lib -- --exact`
4. `cargo test -p worldwake-sim save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state --lib -- --exact`
5. `cargo test -p worldwake-systems item_decay::tests::dispatch_table_routes_item_decay_to_stub --lib -- --exact`
6. `cargo test -p worldwake-systems item_decay::tests::item_decay_stub_returns_ok --lib -- --exact`
7. `cargo test -p worldwake-cli scenario::types::tests::test_scenario_def_commodity_decay_omitted_field_stays_none --lib -- --exact`
8. `cargo test -p worldwake-cli scenario::types::tests::test_scenario_def_commodity_decay_deserializes_when_present --lib -- --exact`
9. `cargo test -p worldwake-cli scenario::tests::test_spawn_minimal_scenario_uses_default_commodity_decay --lib -- --exact`
10. `cargo test -p worldwake-cli scenario::tests::test_spawn_scenario_applies_explicit_commodity_decay_override --lib -- --exact`
11. `cargo test -p worldwake-core`
12. `cargo test -p worldwake-sim`
13. `cargo test -p worldwake-systems`
14. `cargo test -p worldwake-cli`
15. `cargo test --workspace`
16. `cargo clippy --workspace --all-targets -- -D warnings`
