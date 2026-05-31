# S177WATSRCQUA-001: `WaterQuality` enum + `ResourceSource.quality` + scenario contract

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` (new enum, field addition to `ResourceSource`), `worldwake-cli/scenario` (`ResourceSourceDef.quality` + `spawn_scenario` propagation), `worldwake-sim/save_load` (SAVE_FORMAT_VERSION bump)
**Deps**: `specs/S177-water-source-quality-depletion-reliability.md`

## Problem

`ResourceSource` (`crates/worldwake-core/src/production.rs:75-83`) has no quality field. Clean, stale, and muddy water are indistinguishable to drink, basin refill, and ranking. The spec's D1 deliverable adds a concrete `WaterQuality` enum on water sources as the foundation every other quality-axis deliverable consumes. Without this foundation, no downstream consequence (relief scaling, basin preference, quality observation, forensic record) is possible.

## Assumption Reassessment (2026-05-31)

1. `ResourceSource` at `crates/worldwake-core/src/production.rs:75-83` carries `commodity`, `available_quantity`, `max_quantity`, `regeneration_ticks_per_unit`, `last_regeneration_tick`, `extraction_slots`, `extraction_duration_ticks` — no quality. Derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` (not `Copy/Hash/Ord`). `Option<WaterQuality>` is derive-compatible.
2. `ResourceSource` is registered on `EntityKind::Facility || EntityKind::Place` at `crates/worldwake-core/src/component_schema.rs:1787` — no registration change required.
3. Shared abstraction boundary: the `ResourceSource` component's serialized shape. This boundary is read by `crates/worldwake-sim/src/per_agent_belief_view.rs::resource_source` (live + belief-mediated) and reconstructed on save/load. Adding a non-`Option` field would break bincode positional reads — `Option<WaterQuality>` with `#[serde(default)]` plus a `SAVE_FORMAT_VERSION` bump is the FND-28-compliant migration.
4. `ResourceSourceDef` at `crates/worldwake-cli/src/scenario/types.rs:820-831` has no `quality` field; `spawn_scenario` block at `crates/worldwake-cli/src/scenario/mod.rs:496-507` does not insert quality. Both need the new field. `#[serde(default)]` preserves existing RON deserialization.
5. `SAVE_FORMAT_VERSION` is 110 at `crates/worldwake-sim/src/save_load.rs:7`. Cascade for S177: this ticket bumps to 111; tickets 002, 003, 004, 006 carry subsequent bumps per their respective format-breaking changes (see Merge-Order Constraints in the Step 6 summary).
6. ResourceSource construction sites surveyed: ~10 sites across `worldwake-core/src/conservation.rs` (3), `world.rs`, `world_txn.rs`, `belief.rs`; `worldwake-sim/src/action_validation.rs`, `per_agent_belief_view.rs` (4); `worldwake-ai/src/failure_handling.rs`, `survival_forensics.rs`, `planning_state.rs`, `agent_tick/observation.rs`. Most are test bodies. Construction-site count is informational because `Option<WaterQuality>::None` is a meaningful default — sites can opt to spread `..Default::default()` or set `quality: None` explicitly without changing semantic behavior.
7. The new `WaterQuality` enum is consumed in subsequent tickets (002, 003, 004, 005, 006, 008). Forward-declaring it here is FND-28-compliant — the enum is fully defined and live; downstream tickets only add new consumers.

## Architecture Check

1. Field-on-existing-component (vs. sibling-component) is the FND-26-aligned choice because quality is intrinsic to the source — every consumer that reads `ResourceSource` (drink, basin refill, ranking, perception) gets it through the existing accessor with no new lookup chain. A sibling component would split the read surface and require a second accessor at every site.
2. `Option<WaterQuality>` (vs. required `WaterQuality`) is the FND-28-compliant migration shape because non-water `ResourceSource`s (apple, grain) legitimately have no quality concept — `None` is semantically meaningful, not a backcompat placeholder. Scenario authors set `quality: Some(Clean)` for water sources explicitly.
3. `#[serde(default)]` plus `SAVE_FORMAT_VERSION` bump preserves both RON-authored scenarios (existing scenarios without `quality:` still deserialize) and bincode save-load (the version bump is the gate; deserialization treats the missing field as `None`).

## Verification Layers

1. Field addition compiles — full workspace build (cargo check).
2. ResourceSource serialization roundtrip — `experience.rs`-style bincode roundtrip test in `crates/worldwake-core/src/production.rs` test module.
3. SAVE_FORMAT_VERSION migration — `crates/worldwake-sim/src/save_load.rs` version-gate test confirms old-version payloads fail-fast (no silent corruption).
4. RON scenario backcompat — focused test loads an existing scenario file (`scenarios/survival-basin-competition-1440.ron`) and confirms the water source's `quality` is `None` (carries the `#[serde(default)]` default).

## What to Change

### 1. New `WaterQuality` enum

Add to `crates/worldwake-core/src/production.rs` (or a new sibling file `crates/worldwake-core/src/water_quality.rs` re-exported from `lib.rs`; prefer sibling file for clean module ownership):

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WaterQuality {
    Clean,
    Stale,
    Muddy,
}
```

Re-export from `crates/worldwake-core/src/lib.rs`.

### 2. Add `quality` field to `ResourceSource`

```rust
pub struct ResourceSource {
    pub commodity: CommodityKind,
    pub available_quantity: Quantity,
    pub max_quantity: Quantity,
    pub regeneration_ticks_per_unit: Option<NonZeroU32>,
    pub last_regeneration_tick: Option<Tick>,
    pub extraction_slots: NonZeroU8,
    pub extraction_duration_ticks: NonZeroU32,
    #[serde(default)]
    pub quality: Option<WaterQuality>,
}
```

Update existing ResourceSource construction sites (see Files to Touch). Sites that spread from `Default` already work without edit; sites that explicitly enumerate fields need `quality: None` added.

### 3. Add `quality` field to `ResourceSourceDef`

`crates/worldwake-cli/src/scenario/types.rs:820-831`:

```rust
pub struct ResourceSourceDef {
    pub commodity: CommodityKind,
    pub location: String,
    pub facility: Option<String>,
    pub regeneration_ticks_per_unit: Option<NonZeroU32>,
    pub capacity: Quantity,
    pub extraction_slots: u8,
    pub extraction_duration_ticks: u32,
    #[serde(default)]
    pub quality: Option<WaterQuality>,
}
```

Update `spawn_scenario`'s resource-source insertion at `crates/worldwake-cli/src/scenario/mod.rs:496-507` to read `source_def.quality` and write it into the constructed `ResourceSource`:

```rust
ResourceSource {
    commodity: source_def.commodity,
    available_quantity: source_def.capacity,
    max_quantity: source_def.capacity,
    regeneration_ticks_per_unit: source_def.regeneration_ticks_per_unit,
    last_regeneration_tick: None,
    extraction_slots,
    extraction_duration_ticks: NonZeroU32::new(source_def.extraction_duration_ticks)
        .ok_or(/* … */)?,
    quality: source_def.quality,
},
```

### 4. Bump `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs:7`: change `110` to `111`. Update any version-gate test fixtures that assert the current version.

## Files to Touch

- `crates/worldwake-core/src/water_quality.rs` (new — `WaterQuality` enum, focused tests for ordering / hash / serialization roundtrip)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `WaterQuality`)
- `crates/worldwake-core/src/production.rs` (modify — add `quality` field to `ResourceSource`)
- `crates/worldwake-core/src/conservation.rs` (modify — 3 ResourceSource construction sites; add `quality: None` or spread)
- `crates/worldwake-core/src/belief.rs` (modify — 1 site at line 3096)
- `crates/worldwake-core/src/world.rs` (modify — 1 site at line 980)
- `crates/worldwake-core/src/world_txn.rs` (modify — 1 site at line 2179)
- `crates/worldwake-sim/src/action_validation.rs` (modify — 1 site)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — 4 sites)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — 1 site at line 4225)
- `crates/worldwake-ai/src/survival_forensics.rs` (modify — 1 site at line 1640)
- `crates/worldwake-ai/src/planning_state.rs` (modify — 1 site at line 3314)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — 1 site at line 1449)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — add `quality` field to `ResourceSourceDef`)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — propagate quality through resource-source spawning at lines 496-507)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump `SAVE_FORMAT_VERSION` 110→111)

## Out of Scope

- Drink relief scaling, dirtiness penalty, basin refill preference, quality observation, forensic record, CLI gating — all in subsequent tickets.
- Authoring concrete quality values in existing scenarios (`scenarios/*.ron`) — left as `None` to preserve current behavior; downstream tickets 009-010 author quality values in new scenarios.
- `ItemLot.quality` — owned by ticket 002.
- `WaterToleranceProfile` — owned by ticket 003.
- Replacing existing `ResourceSource` construction sites with `..Default::default()` spread syntax where they currently enumerate fields — out of scope; only minimal `quality: None` additions where needed.

## Acceptance Criteria

### Tests That Must Pass

1. New: `water_quality_serialization_roundtrip` in `crates/worldwake-core/src/water_quality.rs` — bincode roundtrip of each variant.
2. New: `resource_source_with_quality_roundtrip` in `crates/worldwake-core/src/production.rs` test module — `ResourceSource { …, quality: Some(WaterQuality::Muddy) }` and `quality: None` both roundtrip through bincode.
3. New: `resource_source_def_quality_defaults_to_none_in_ron` in `crates/worldwake-cli/src/scenario/types.rs` test module — RON without `quality:` deserializes with `quality: None`.
4. Existing: `cargo test --workspace` — full suite passes (construction-site spread-syntax + explicit `quality: None` updates preserve all existing semantics).

### Invariants

1. No `ResourceSource` construction site exists in the workspace that elides the `quality` field without spread syntax or an explicit `quality:` line.
2. `SAVE_FORMAT_VERSION` is monotonic — never decremented; 110 → 111 is the only delta in this ticket.
3. Existing scenarios (`scenarios/*.ron`) without `quality:` deserialize unchanged via `#[serde(default)]`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/water_quality.rs` (new test module) — variant ordering, hash stability, serialization roundtrip.
2. `crates/worldwake-core/src/production.rs` (test module extension) — `ResourceSource` roundtrip with `Some(quality)` and `None`.
3. `crates/worldwake-cli/src/scenario/types.rs` (test module extension) — `ResourceSourceDef` RON deserialization with/without `quality:`.

### Commands

1. `cargo test -p worldwake-core water_quality` — targeted enum tests.
2. `cargo test -p worldwake-core resource_source` — targeted ResourceSource tests.
3. `cargo test -p worldwake-cli scenario` — targeted scenario-types tests.
4. `./scripts/verify.sh` — full workspace (fmt + clippy + tests).

Merge note: Ticket 001 bumps SAVE_FORMAT_VERSION 110→111. Tickets 002 (111→112), 003 (112→113), 004 (113→114), and 006 (114→115) carry subsequent bumps in dependency order — landing two tickets out of order produces a value collision.
