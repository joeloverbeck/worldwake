# S177WATSRCQUA-001: `WaterQuality` enum + `ResourceSource.quality` + scenario contract

**Status**: COMPLETED
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
6. ResourceSource construction sites surveyed during implementation: 179 explicit literals or type sites across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, and `worldwake-cli`. The earlier estimate of ~10 covered only a narrow first-order sample. The shared-field fallout was mechanical: every explicit `ResourceSource`/`ResourceSourceDef` literal now sets `quality: None` unless the focused test intentionally exercises a non-`None` value.
7. The new `WaterQuality` enum is consumed in subsequent tickets (002, 003, 004, 005, 006, 008). Forward-declaring it here is FND-28-compliant — the enum is fully defined and live; downstream tickets only add new consumers.

## Architecture Check

1. Field-on-existing-component (vs. sibling-component) is the FND-26-aligned choice because quality is intrinsic to the source — every consumer that reads `ResourceSource` (drink, basin refill, ranking, perception) gets it through the existing accessor with no new lookup chain. A sibling component would split the read surface and require a second accessor at every site.
2. `Option<WaterQuality>` (vs. required `WaterQuality`) is the FND-28-compliant migration shape because non-water `ResourceSource`s (apple, grain) legitimately have no quality concept — `None` is semantically meaningful, not a backcompat placeholder. Scenario authors set `quality: Some(Clean)` for water sources explicitly.
3. `#[serde(default)]` plus `SAVE_FORMAT_VERSION` bump preserves both RON-authored scenarios (existing scenarios without `quality:` still deserialize) and bincode save-load (the version bump is the gate; deserialization treats the missing field as `None`).

## Verified Layers

1. Field addition compiles — full workspace build (cargo check).
2. ResourceSource serialization roundtrip — `experience.rs`-style bincode roundtrip test in `crates/worldwake-core/src/production.rs` test module.
3. SAVE_FORMAT_VERSION migration — `crates/worldwake-sim/src/save_load.rs` version-gate test confirms old-version payloads fail-fast (no silent corruption).
4. RON scenario backcompat — focused test loads an existing scenario file (`scenarios/survival-basin-competition-1440.ron`) and confirms the water source's `quality` is `None` (carries the `#[serde(default)]` default).

## Landed Changes

### 1. New `WaterQuality` enum

Added in `crates/worldwake-core/src/water_quality.rs` and re-exported from `lib.rs`:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WaterQuality {
    Clean,
    Stale,
    Muddy,
}
```

Re-exported from `crates/worldwake-core/src/lib.rs`.

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

Updated explicit `ResourceSource` construction sites with `quality: None`, except the focused positive-path tests that intentionally use `Some(WaterQuality::*)`.

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

Updated `spawn_scenario`'s resource-source insertion at `crates/worldwake-cli/src/scenario/mod.rs` to read `source_def.quality` and write it into the constructed `ResourceSource`:

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

`crates/worldwake-sim/src/save_load.rs` now reports version `111`, and the focused version test was updated to the S177 ticket name.

## Landed Files

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

## Acceptance Result

### Tests Passed

1. Added `water_quality_serialization_roundtrip` in `crates/worldwake-core/src/water_quality.rs` — bincode roundtrip of each variant.
2. Added `resource_source_bincode_roundtrip_includes_extraction_fields` coverage for `Some(WaterQuality::Muddy)` and `resource_source_bincode_roundtrip_preserves_absent_quality` coverage for `None`.
3. Added `scenario_def_resource_source_deserializes_quality` and extended `scenario_def_resource_source_defaults_to_one_slot` for omitted-field defaulting.
4. Passed `cargo test --workspace` after the final source diff.

### Invariants

1. No `ResourceSource` construction site exists in the workspace that elides the `quality` field without spread syntax or an explicit `quality:` line.
2. `SAVE_FORMAT_VERSION` is monotonic — never decremented; 110 → 111 is the only delta in this ticket.
3. Existing scenarios (`scenarios/*.ron`) without `quality:` deserialize unchanged via `#[serde(default)]`.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/water_quality.rs` — variant ordering and serialization roundtrip.
2. `crates/worldwake-core/src/production.rs` (test module extension) — `ResourceSource` roundtrip with `Some(quality)` and `None`.
3. `crates/worldwake-cli/src/scenario/types.rs` (test module extension) — `ResourceSourceDef` RON deserialization with/without `quality:`.
4. `crates/worldwake-cli/src/scenario/mod.rs` (test module extension) — scenario spawn propagates authored source quality.

### Commands Run

1. `cargo test -p worldwake-core water_quality` — targeted enum tests.
2. `cargo test -p worldwake-core resource_source` — targeted ResourceSource tests.
3. `cargo test -p worldwake-cli scenario_def_resource_source` — targeted scenario-types tests.
4. `cargo test -p worldwake-cli spawn_scenario_resource_source_explicit_extraction_fields` — targeted scenario spawn propagation test.
5. `cargo test -p worldwake-sim save_format_version_is_111_after_resource_source_quality` — targeted save-version test.
6. `cargo test --workspace --no-run` — workspace compile-only constructor fallout check.
7. `cargo test --workspace` — full workspace test gate.
8. `./scripts/verify.sh` — waived for this ticket iteration; the harness final branch phase owns the full pre-push verify gate.

Merge note: Ticket 001 bumped SAVE_FORMAT_VERSION 110→111. Tickets 002 (111→112), 003 (112→113), 004 (113→114), and 006 (114→115) carry subsequent bumps in dependency order — landing two tickets out of order produces a value collision.

## Outcome

Completed on 2026-05-31.

- Added `WaterQuality` in `worldwake-core`, re-exported it from the crate root, and added focused ordering + bincode roundtrip coverage.
- Added `ResourceSource.quality: Option<WaterQuality>` with serde defaulting and updated explicit resource-source literals across the workspace to preserve current behavior with `quality: None`.
- Added `ResourceSourceDef.quality` to the RON scenario schema and propagated authored quality through `spawn_scenario`.
- Bumped `SAVE_FORMAT_VERSION` from 110 to 111 and retitled the focused version test.
- Truth-synced S177 status in the active spec and implementation-order roadmap from draft to in-progress.

## Deviations

- The constructor fallout was broader than the draft file list. Implementation updated all explicit `ResourceSource` and `ResourceSourceDef` literals found by the workspace sweep, not just the first-order files listed above.
- `./scripts/verify.sh` was not run for this ticket iteration; the `implement-spec-tickets` harness owns the final pre-push verify gate for the whole S177 branch. The per-ticket broad proof ran `cargo test --workspace` after the final source diff.

## Verification Result

- Passed `cargo test -p worldwake-core water_quality`.
- Passed `cargo test -p worldwake-core resource_source`.
- Passed `cargo test -p worldwake-cli scenario_def_resource_source`.
- Passed `cargo test -p worldwake-cli spawn_scenario_resource_source_explicit_extraction_fields`.
- Passed `cargo test -p worldwake-sim save_format_version_is_111_after_resource_source_quality`.
- Passed `cargo test --workspace --no-run`.
- Passed `cargo test --workspace`.
- Passed constructor zero-match scan: `rg -n "ResourceSource \\{[^}]*extraction_duration_ticks: [^\\n]+,\\n\\s*}" crates -U` returned zero matches.
- Waived `./scripts/verify.sh` for this ticket iteration because the harness final branch phase runs the full pre-PR gate before push.
