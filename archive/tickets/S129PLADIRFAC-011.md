# S129PLADIRFAC-011: Scenario authoring — hygiene Defs and spawn integration

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — adds three new `*Def` wrapper types in `worldwake-cli`; extends `PlaceDef` and `FacilityDef`; updates 57 PlaceDef and ~6 FacilityDef construction sites
**Deps**: archive/tickets/S129PLADIRFAC-001.md

## Problem

Without scenario-side authoring of `PlaceDirtiness`, `LatrineFullness`, `WashBasinState`, scenario writers cannot tune hygiene topology — every place gets the universal `Default` (decay_per_tick=2, dirtiness_per_use=80) and every basin/latrine gets the role-specific defaults. PR-9's depth ask (Fertile Fields recovers slowly because no drainage; Hillside Shelter recovers quickly) is impossible to express without per-place authoring. This ticket lands the `*Def` wrapper structs, extends `PlaceDef` / `FacilityDef`, and wires spawn-time integration in `scenario/mod.rs`. The 57 PlaceDef + 6 FacilityDef construction sites currently in the codebase **do not use spread syntax**, so each must be updated to add the new `Option<*Def>` fields explicitly.

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PlaceDef` at `crates/worldwake-cli/src/scenario/types.rs:319–327` (verified during reassessment) currently carries `name: String`, `tags: Vec<PlaceTag>`, `visibility_profile: Option<PlaceVisibilityProfile>`, `sleep_quality: Option<SleepQualityProfileDef>`. The pattern for adding new fields: `Option<*Def>` with `#[serde(default)]`, mirroring `visibility_profile` and `sleep_quality`.
2. `FacilityDef` at `crates/worldwake-cli/src/scenario/types.rs:509–518` carries `name`, `workstation`, `location`, `merchant_storage`, `contention_policy`. Add `wash_basin_state: Option<WashBasinStateDef>` with the same conditional-spawning pattern.
3. `SleepQualityProfileDef` at `types.rs:330–351` is the precedent for a `*Def` wrapper with `into_profile()` conversion. Verified during reassessment. The new three Defs follow the same shape: each field is `Option<T>` so authors can override only what they care about; `into_profile()` (or `From<*Def>`) returns the runtime component with `Default` fallbacks.
4. `spawn_place` at `crates/worldwake-cli/src/scenario/mod.rs:282–290` is the universal-on-Place spawn precedent: `let profile: SleepQualityProfile = place_def.sleep_quality.map(types::SleepQualityProfileDef::into_profile).transpose()?.unwrap_or_default(); txn.set_component_sleep_quality_profile(place_id, profile)?;`. `PlaceDirtiness` follows this universal pattern; `LatrineFullness` follows a tag-conditional variant: `if place_def.tags.contains(&PlaceTag::Latrine) { ... }`.
5. Construction-site count per spot-check (d): **57 PlaceDef sites + ~6 FacilityDef sites with zero spread-syntax occurrences**. The new fields are `Option<X>` with `#[serde(default)]` — RON deserialization handles them automatically (existing `.ron` scenario files don't need updating), but Rust source-code construction sites must explicitly add `place_dirtiness: None,` `latrine_fullness: None,` (PlaceDef) and `wash_basin_state: None,` (FacilityDef). This is the load-bearing reason for Medium effort. Most sites are in `crates/worldwake-cli/src/scenario/mod.rs` (default scenario builders + tests), with some in test fixtures across the workspace.
6. The shared abstraction boundary under audit is the scenario authoring contract — `PlaceDef`/`FacilityDef` are the user-authored shape; `into_profile()` materializes runtime components; `spawn_place`/facility spawn applies them. New fields must round-trip through RON serde defaults so existing scenarios continue to load with `None` and the runtime fills in `Default::default()`.
7. Heuristic Removal Discipline (precision-rules §12): N/A — this is purely additive scenario authoring. No filter or heuristic is being removed.

## Architecture Check

1. Adding three sibling `*Def` types alongside `SleepQualityProfileDef` keeps the scenario authoring surface uniform — every per-place / per-facility profile uses the same shape (Optional wrapper, `into_profile()` conversion, default fallback). Splitting these into a separate scenario submodule (e.g., `scenario/hygiene_defs.rs`) would obscure the symmetry without reducing file size meaningfully.
2. Universal-on-Place spawn for `PlaceDirtiness` and tag-conditional spawn for `LatrineFullness` / `WashBasinState` directly mirrors the schema-registration kind filters from ticket 001. The scenario layer cannot violate the schema (kind filter prevents wrong-kind entities), but it deliberately reflects the same role-specificity for cleaner authoring (RON authors get `Some(...)` only on entities that actually carry the state).
3. No backward-compat shim. Existing scenarios load with `None` for the new fields (via `#[serde(default)]`) and runtime applies `Default`. The 57 PlaceDef Rust-source construction sites get explicit `None` additions — no implicit-default magic.

## Verification Layers

1. RON deserialization round-trip with absent new fields → focused unit test loading a minimal RON scenario without the new fields and asserting the runtime spawned places carry `PlaceDirtiness::default()` and (for latrine-tagged places) `LatrineFullness::default()`.
2. RON deserialization with explicit values → focused unit test loading a RON scenario with `place_dirtiness: Some(PlaceDirtinessDef { value: Some(500), decay_per_tick: Some(5), dirtiness_per_use: Some(100) })` and asserting the runtime component matches.
3. `LatrineFullness` is set only on latrine-tagged places → focused unit test seeding two places, one with `PlaceTag::Latrine`, one without; both authored with `latrine_fullness: Some(...)`. Assert the latrine-tagged place carries the component, the non-latrine-tagged place does not (or carries `Default::default()`, depending on implementation choice — the spec text suggests the latter via `unwrap_or_default()`, but the validation pattern note recommends gating on tag).
4. `WashBasinState` is set only on `WashBasin` workstation facilities → analogous focused unit test.
5. All 57 PlaceDef construction sites + ~6 FacilityDef sites compile after the field additions → `cargo build --workspace` (the canonical proof that no construction site was missed).

## What to Change

### 1. New `*Def` wrapper types in `crates/worldwake-cli/src/scenario/types.rs`

Add three structs after `SleepQualityProfileDef`:

```rust
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct PlaceDirtinessDef {
    pub value: Option<Permille>,
    pub decay_per_tick: Option<Permille>,
    pub dirtiness_per_use: Option<Permille>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct LatrineFullnessDef {
    pub fill: Option<Permille>,
    pub fill_per_use: Option<Permille>,
    pub critical_threshold: Option<Permille>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct WashBasinStateDef {
    pub clean_water_units: Option<u16>,
    pub max_clean_water: Option<u16>,
    pub refill_per_tick: Option<u16>,
    pub units_per_full_wash: Option<u16>,
    pub dirtiness_level: Option<Permille>,
    pub dirtiness_per_use: Option<Permille>,
}
```

Implement `From<PlaceDirtinessDef> for PlaceDirtiness` (and the two analogs) returning the runtime component with field-level fallback to `Component::default()`'s field values.

### 2. Extend `PlaceDef` with two new fields

```rust
pub struct PlaceDef {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<PlaceTag>,
    #[serde(default)]
    pub visibility_profile: Option<PlaceVisibilityProfile>,
    #[serde(default)]
    pub sleep_quality: Option<SleepQualityProfileDef>,
    #[serde(default)]
    pub place_dirtiness: Option<PlaceDirtinessDef>,    // NEW
    #[serde(default)]
    pub latrine_fullness: Option<LatrineFullnessDef>,  // NEW (ignored unless tags include PlaceTag::Latrine)
}
```

### 3. Extend `FacilityDef` with one new field

```rust
pub struct FacilityDef {
    #[serde(default)]
    pub name: Option<String>,
    pub workstation: WorkstationTag,
    pub location: String,
    #[serde(default)]
    pub merchant_storage: Option<MerchantStorageDef>,
    #[serde(default)]
    pub contention_policy: Option<ContentionPolicy>,
    #[serde(default)]
    pub wash_basin_state: Option<WashBasinStateDef>,  // NEW (ignored unless workstation == WashBasin)
}
```

### 4. Spawn integration in `scenario/mod.rs`

In `spawn_place` (around line 282–290 for the existing `sleep_quality` block), add:

```rust
// Universal: every place gets a PlaceDirtiness
let dirtiness: PlaceDirtiness = place_def
    .place_dirtiness
    .as_ref()
    .map(|def| def.clone().into())
    .unwrap_or_default();
txn.set_component_place_dirtiness(place_id, dirtiness)?;

// Tag-conditional: only latrine-tagged places get LatrineFullness
if place_def.tags.contains(&PlaceTag::Latrine) {
    let latrine: LatrineFullness = place_def
        .latrine_fullness
        .as_ref()
        .map(|def| def.clone().into())
        .unwrap_or_default();
    txn.set_component_latrine_fullness(place_id, latrine)?;
}
```

In the facility spawn loop, add:

```rust
if facility_def.workstation == WorkstationTag::WashBasin {
    let basin: WashBasinState = facility_def
        .wash_basin_state
        .as_ref()
        .map(|def| def.clone().into())
        .unwrap_or_default();
    txn.set_component_wash_basin_state(facility_id, basin)?;
}
```

### 5. Update 57 PlaceDef construction sites

Every Rust-source `PlaceDef { ... }` literal must add `place_dirtiness: None,` and `latrine_fullness: None,`. Sites are predominantly in `crates/worldwake-cli/src/scenario/mod.rs` (test fixtures and default scenario builders); a few may live in `tests/` directories across the workspace. Use `rg "PlaceDef\s*\{" crates/` to enumerate during implementation; the count per spot-check (d) is 57.

### 6. Update ~6 FacilityDef construction sites

Every Rust-source `FacilityDef { ... }` literal must add `wash_basin_state: None,`. Use `rg "FacilityDef\s*\{" crates/` to enumerate.

### 7. Existing `.ron` scenario files

No update needed — `#[serde(default)]` handles missing fields. Optionally, update `scenarios/cli-evaluation.ron` and any survival-testing scenario in a follow-up rebalance ticket (out of scope here).

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — three new `*Def` structs + `From` impls; PlaceDef + FacilityDef field additions)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — spawn integration block; PlaceDef construction-site updates)
- All other workspace files with `PlaceDef { ... }` or `FacilityDef { ... }` Rust-source literals (~57 + ~6 sites; enumerate via `rg` during implementation)

## Out of Scope

- Per-place rebalance authoring (Fertile Fields slow recovery, Hillside Shelter fast recovery) — explicitly deferred per spec D11 to a follow-up rebalance ticket once goldens (ticket 012) confirm the baseline behavior.
- RON scenario file updates — not strictly necessary because `#[serde(default)]` handles missing fields. Authors may update specific scenarios in follow-up.
- Schema validation that rejects authoring `latrine_fullness` on non-latrine-tagged places — the current implementation pattern silently ignores the wrong-tag combination (mirrors how `merchant_storage` on a non-stall facility behaves today).

## Acceptance Criteria

### Tests That Must Pass

1. New focused test `place_def_loads_with_default_when_dirtiness_field_absent` — RON without `place_dirtiness:` produces a runtime `PlaceDirtiness::default()`.
2. New focused test `place_def_loads_explicit_dirtiness_values` — RON with explicit values produces matching runtime component.
3. New focused test `latrine_fullness_only_set_on_latrine_tagged_places` — non-latrine place authored with `latrine_fullness` does NOT carry the component (or carries default — confirm semantics during implementation; both are valid per the conditional-spawn precedent).
4. New focused test `wash_basin_state_only_set_on_wash_basin_facilities` — analogous.
5. `cargo build --workspace` succeeds — proves all 57 PlaceDef + 6 FacilityDef sites were updated.
6. Existing scenario-loading tests in `crates/worldwake-cli/src/scenario/` continue to pass (RON deserialization back-compat via serde defaults).
7. Existing suite: `cargo test --workspace`.

### Invariants

1. Every spawned `EntityKind::Place` carries a `PlaceDirtiness` component (universal-on-Place semantics enforced at scenario layer). `world.entities_with_place_dirtiness().count() == world.entities_with_kind(EntityKind::Place).count()` holds for every spawned scenario.
2. `LatrineFullness` is present on a Place if and only if that Place's tags include `PlaceTag::Latrine`.
3. `WashBasinState` is present on a Facility if and only if that Facility's workstation is `WorkstationTag::WashBasin`.
4. No existing `.ron` scenario file requires manual update — RON serde defaults handle the new fields.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/mod.rs` test module — four new focused tests covering RON round-trip, default fallback, and tag-conditional spawn.
2. Updated PlaceDef and FacilityDef construction sites across the workspace (no test changes per se, but each test that constructs these types compiles only after the field additions).

### Commands

1. `cargo test -p worldwake-cli scenario`
2. `cargo test --workspace`
3. `cargo build --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-01.

- Added scenario-authored `PlaceDirtinessDef`, `LatrineFullnessDef`, and `WashBasinStateDef` wrappers with field-level fallback to the runtime component defaults.
- Extended `PlaceDef` and `FacilityDef` with defaulted optional hygiene authoring fields.
- Wired scenario spawn so every place receives `PlaceDirtiness`, only latrine-tagged places receive `LatrineFullness`, and only `WashBasin` facilities receive `WashBasinState`.
- Updated all 57 live `PlaceDef` Rust literals and all 6 live `FacilityDef` Rust literals, including downstream CLI handlers and `golden_survival_baseline`.
- Updated `scenario_coverage` to classify authored `place_dirtiness` and `latrine_fullness` fields instead of reporting them as unmapped authored input.

## Deviations

- The live implementation uses `Copy`/`Into` for the three lightweight `*Def` wrappers instead of the ticket sketch's `.as_ref().map(|def| def.clone().into())` shape.
- No `.ron` scenario files were changed; omitted-field compatibility is handled by `#[serde(default)]` and is covered by focused spawn tests.

## Verification Result

- Passed `cargo test --workspace --no-run`.
- Passed `cargo test -p worldwake-cli --lib scenario::tests:: -- --list` and confirmed the four new focused test selectors exist.
- Passed `cargo test -p worldwake-cli --lib scenario::tests::place_def_loads_with_default_when_dirtiness_field_absent -- --exact`.
- Passed `cargo test -p worldwake-cli --lib scenario::tests::place_def_loads_explicit_dirtiness_values -- --exact`.
- Passed `cargo test -p worldwake-cli --lib scenario::tests::latrine_fullness_only_set_on_latrine_tagged_places -- --exact`.
- Passed `cargo test -p worldwake-cli --lib scenario::tests::wash_basin_state_only_set_on_wash_basin_facilities -- --exact`.
- Passed `cargo test -p worldwake-cli scenario`.
- Passed `cargo build --workspace`.
- Passed `cargo test --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh`, whose live gates are `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
- After the final `scenario_coverage` feature-mapping correction, passed `cargo test -p worldwake-cli --bin scenario-coverage`, `cargo run -p worldwake-cli --bin scenario-coverage -- --check`, and `cargo clippy -p worldwake-cli --bin scenario-coverage -- -D warnings`.
