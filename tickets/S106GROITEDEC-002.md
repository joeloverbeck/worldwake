# S106GROITEDEC-002: Decay infrastructure — types, EventTag, SystemId, ScenarioDef

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `CommodityDecayMap` type, `EventTag::ItemDecay` variant, `SystemId::ItemDecay`, `ScenarioDef` field, world config storage
**Deps**: None

## Problem

The `ItemDecay` system (ticket 003) requires infrastructure that spans multiple crates: a type for the decay configuration, an event tag for traceability, a system ID for scheduling, a scenario field for configuration, and world-level storage. This ticket provides all of that infrastructure so ticket 003 can focus purely on the system logic.

## Assumption Reassessment (2026-04-16)

1. `EventTag` enum is at `crates/worldwake-core/src/event_tag.rs:7-33` with 25 variants. `ItemDecay` does not exist yet. Exhaustive match sites must be identified and updated.
2. `define_system_ids!` macro is at `crates/worldwake-sim/src/system_manifest.rs:60-74` with 13 variants. `SYSTEM_COUNT` is derived from this macro. Adding a variant increments the count and requires a matching entry in `dispatch_table()` (worldwake-systems/src/lib.rs:91-107).
3. Canonical system order is at `system_manifest.rs:108-124`. Current order ends: `...ExpectationCheck → EvidenceDecay → Patrol → Compaction`. `ItemDecay` goes between `EvidenceDecay` and `Patrol`.
4. `ScenarioDef` is at `crates/worldwake-cli/src/scenario/types.rs:22-40` with 8 fields. 15+ struct literal construction sites in `worldwake-cli` test code, all exhaustive (no `..Default::default()`). Each needs `commodity_decay: None` added.
5. `CommodityKind` enum at `crates/worldwake-core/src/items.rs:10-21` includes `Waste` and `Apple` — the two commodities with default decay values.
6. `dispatch_table()` at `crates/worldwake-systems/src/lib.rs:91-107` uses `SystemDispatchTable::from_handlers([...])` — array indexed by SystemId ordinal. Adding SystemId::ItemDecay requires a matching handler entry at the correct ordinal position.

## Architecture Check

1. Separating infrastructure from logic (this ticket vs. 003) keeps each diff focused. A stub `item_decay_system` (returns `Ok(())`) satisfies the dispatch table requirement while deferring logic to 003.
2. No backward-compatibility shims. `commodity_decay` field uses `#[serde(default)]` so existing RON scenarios deserialize without changes. The default provides Waste: 200, Apple: 720.

## Verification Layers

1. EventTag::ItemDecay variant exists and compiles → compilation success
2. SystemId::ItemDecay in canonical order → existing ordering test `canonical_manifest_matches_fixed_scheduler_order` updated and passing
3. dispatch_table routes ItemDecay to stub → focused test (call dispatch_table().get(SystemId::ItemDecay) and verify it returns Ok)
4. ScenarioDef deserializes with and without commodity_decay → focused test
5. Single-layer ticket (infrastructure only, no runtime behavior) — additional layer mapping not applicable

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

Store the resolved `CommodityDecayMap` as a world-level resource accessible during system execution. Follow existing world configuration patterns — either a dedicated field on the `World` struct or a component on a singleton config entity. The decay map must be readable by `item_decay_system` via `SystemExecutionContext`.

Load the decay map during scenario initialization in `crates/worldwake-cli/src/scenario/mod.rs`: if `commodity_decay` is `None`, use `default_commodity_decay_map()`; otherwise use the provided map.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — add `CommodityDecayMap` type alias and default fn)
- `crates/worldwake-core/src/event_tag.rs` (modify — add `ItemDecay` variant)
- `crates/worldwake-core/src/lib.rs` (modify — re-export new types)
- `crates/worldwake-sim/src/system_manifest.rs` (modify — add SystemId, update canonical order, update tests)
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
5. Existing suite: `cargo test --workspace` — all existing tests pass

### Invariants

1. `SYSTEM_COUNT` equals the number of entries in `define_system_ids!` (enforced by array sizing in dispatch_table).
2. `canonical()` contains exactly one entry per SystemId (enforced by duplicate check).
3. `EventTag::ItemDecay` is handled in all exhaustive matches.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/system_manifest.rs` (test module) — update `canonical_manifest_matches_fixed_scheduler_order` to include `ItemDecay`
2. `crates/worldwake-systems/src/item_decay.rs` (test module) — dispatch table routing test for stub

### Commands

1. `cargo test -p worldwake-sim system_manifest` — targeted manifest tests
2. `cargo test -p worldwake-systems item_decay` — targeted stub tests
3. `cargo test --workspace` — full workspace
4. `cargo clippy --workspace --all-targets -- -D warnings` — lint
