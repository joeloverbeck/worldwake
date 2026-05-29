# S176SANFACDEG-001: WashBasinState effective-dirtiness field + scenario contract

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `WashBasinState` core component (new field), serialized `SimulationState` format (`SAVE_FORMAT_VERSION` bump), scenario contract (`WashBasinStateDef`)
**Deps**: None (must land before S176SANFACDEG-002 for the `SAVE_FORMAT_VERSION` cascade order; see Merge-Order Constraints)

## Problem

`WashBasinState.dirtiness_level` is incremented on every wash but never read to gate or scale wash effectiveness (S176 Summary). Before any consequence can be wired (003), the basin needs a scenario-authored effective-dirtiness threshold (`max_effective_dirtiness`). This ticket adds the data field and its scenario contract only — no behavior change yet.

## Assumption Reassessment (2026-05-29)

1. `WashBasinState` lives at `crates/worldwake-core/src/place_dirtiness.rs:44-67` (6 fields: `clean_water_units`, `max_clean_water`, `refill_per_tick`, `units_per_full_wash`, `dirtiness_level`, `dirtiness_per_use`; derives `Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize`). It is registered on `EntityKind::Facility` (`component_schema.rs:2087`), NOT Place. There are 26 literal construction sites workspace-wide; 20 use spread/`..Default`-style syntax, leaving ~6 explicit-field sites that must add the new field.
2. The scenario wrapper `WashBasinStateDef` is at `crates/worldwake-cli/src/scenario/types.rs:544-567` with `#[serde(default)]` and a `From<WashBasinStateDef> for WashBasinState` impl; spawning is conditional on the `WashBasin` workstation tag at `crates/worldwake-cli/src/scenario/mod.rs:414` via `.map(Into::into).unwrap_or_default()`. No existing scenario authors `wash_basin_state` explicitly — all rely on defaults.
3. Shared boundary under audit: the bincode-serialized `WashBasinState` shape inside `SimulationState`. `SAVE_FORMAT_VERSION` is `108` (`crates/worldwake-sim/src/save_load.rs:7`). Adding a field breaks positional bincode reads of old saves → version bump required (prototype: no migration path, `load_current_format` only).
4. The new field's `Default` must leave existing scenarios behaviorally unchanged until they author dirt: `dirtiness_level` defaults to `Permille::ZERO`, so `effective_fraction = (max_effective_dirtiness - 0) / max_effective_dirtiness = 1` for any non-zero default, and the threshold gate (003) never blocks until a scenario authors dirtiness. Choose a `Default` of `Permille::new(1000)` (full scale) so a clean basin is fully effective and unblocked under current goldens.

## Architecture Check

1. Concrete per-facility carrier (`Permille`), not a derived `sanitation_score` (FND-3). The threshold is authored on the facility's own state, consistent with the existing `WashBasinState` field set.
2. FND-28: the format bump replaces the prior version outright — no dual-format shim, no migration alias. Existing construction sites are updated in this ticket so the workspace compiles after it lands.

## Verification Layers

1. Field persists on the component → focused unit (component insert/get/remove roundtrip).
2. Scenario authoring + default application → scenario-load focused test (basin with no authored `max_effective_dirtiness` gets the default).
3. Serialized-format compatibility → `save_load` version test asserting the bumped `SAVE_FORMAT_VERSION` round-trips a `WashBasinState` carrying the new field.

## What to Change

### 1. Add the field

Add `pub max_effective_dirtiness: Permille` to `WashBasinState` and set it in the struct's `Default` impl (and any `WashBasinState::new`-style constructor if present).

### 2. Update construction sites

Update all 26 literal construction sites so the workspace compiles. Spread-based sites are unaffected; explicit-field sites add `max_effective_dirtiness`.

### 3. Scenario contract

Add `max_effective_dirtiness: Permille` (with `#[serde(default = "…")]`) to `WashBasinStateDef` and map it in the `From<WashBasinStateDef> for WashBasinState` impl.

### 4. Format bump

Bump `SAVE_FORMAT_VERSION` `108 → 109` in `crates/worldwake-sim/src/save_load.rs`.

## Files to Touch

- `crates/worldwake-core/src/place_dirtiness.rs` (modify)
- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump + construction)
- `crates/worldwake-core/src/delta.rs` (modify — construction site)
- `crates/worldwake-core/src/belief.rs` (modify — construction site)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — construction site)
- `crates/worldwake-sim/src/belief_view.rs` (modify — construction site)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — construction site)
- `crates/worldwake-sim/src/action_validation.rs` (modify — construction site)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — construction site)
- `crates/worldwake-systems/src/item_decay.rs` (modify — construction site)
- Remaining `WashBasinState { … }` literal sites (26 total, 20 via spread) — confirm full set via `rg -n 'WashBasinState\s*\{' crates/` during implementation

## Out of Scope

- Wash effectiveness scaling and the `TargetWashBasinNotTooDirty` precondition — owned by S176SANFACDEG-003.
- Observer display of basin condition — owned by S176SANFACDEG-007 (D10 split).
- Cleaning actions and `clean_water_units` consumption — owned by S176SANFACDEG-004.

## Acceptance Criteria

### Tests That Must Pass

1. Component roundtrip: a `WashBasinState` carrying `max_effective_dirtiness` inserts/gets/removes on a Facility entity unchanged.
2. Scenario default: a `WashBasin` facility authored without `max_effective_dirtiness` spawns with the chosen default.
3. Existing suite: `cargo test -p worldwake-core && cargo test -p worldwake-cli`

### Invariants

1. `WashBasinState` remains registered on `EntityKind::Facility` (not Place).
2. No `sanitation_score` or derived aggregate is introduced; the threshold is a concrete authored field.
3. A clean basin (`dirtiness_level == 0`) yields `effective_fraction == 1` under the chosen default (no behavior change until dirt is authored).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/place_dirtiness.rs` (or `world.rs` component roundtrip test) — extend to assert the new field roundtrips.
2. `crates/worldwake-cli/src/scenario/mod.rs` tests — assert default application when `max_effective_dirtiness` is unauthored.
3. `crates/worldwake-sim/src/save_load.rs` tests — assert the bumped version round-trips the widened struct.

### Commands

1. `cargo test -p worldwake-core place_dirtiness`
2. `cargo test -p worldwake-cli && cargo test -p worldwake-sim save_load`
3. `scripts/verify.sh`
