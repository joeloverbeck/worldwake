# S116DRIESCSUS-002: Add DriveEscalationProfile universal agent component

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new universal agent component, component schema registration, `World::create_agent` bootstrap, save-format version bump
**Deps**: archive/tickets/S116DRIESCSUS-001.md

## Problem

Spec S116 requires a per-agent profile component `DriveEscalationProfile` carrying `start_after_ticks`, `growth_per_tick`, and a multiplier-scale `max_multiplier` cap per `HomeostaticNeedId`. Universal per CLAUDE.md §5 — every agent needs it for motive scoring. Must round-trip through save/load and be constructed by `World::create_agent` so runtime reads via `expect()` succeed. This ticket also adds the pure `escalation_multiplier(ticks, params) -> MultiplierPermille` helper that the ranking-layer ticket (004) and the needs-system ticket (003) both consume.

## Assumption Reassessment (2026-04-17)

1. Registration pattern reference: `DriveThresholds` macro entry at `crates/worldwake-core/src/component_schema.rs:1032-1054` — 20-item `with_component_schema_entries!` form restricted to `EntityKind::Agent`.
2. `World::create_agent()` at `crates/worldwake-core/src/world.rs:164-207` already seeds 18 universal components/profiles today (name, agent_data, belief store, artifact posting, expectation store, last seen memory, perception, tell, cognitive, acquisition exhaustion tracker, exploration, obligation satiation, disposal, execution budget, epistemic disposition, intention disposition, communication, preference). Adding `DriveEscalationProfile` means seeding one more.
3. `SAVE_FORMAT_VERSION: u32 = 31` at `crates/worldwake-sim/src/save_load.rs:6`. Any new ECS-stored component bumps the version. Loader at save_load.rs:129 pattern-matches on the current version, so old saves intentionally cease to load (FND-28 — no backward compatibility in live authority paths; there is no migration path to maintain).
4. Macro expansion sites that must re-compile under `with_component_schema_entries!`: `crates/worldwake-core/src/delta.rs:34-67`, `crates/worldwake-core/src/component_tables.rs:131-135`, `crates/worldwake-core/src/world.rs` (per README check #13). Registration in `component_schema.rs` propagates to all three.
5. `create_agent` delta assertion test: `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` at `crates/worldwake-core/src/world_txn.rs:2374` enumerates every `ComponentDelta::Set` row in an explicit sequence — one new row needed.
6. Live numeric contract mismatch: `Permille` in `crates/worldwake-core/src/numerics.rs` is hard-bounded to `0..=1000`, so the drafted `max_multiplier: Permille(3000)` / `3x` cap is not representable. To preserve S116's intended behavior without widening a foundational numeric type, this ticket must introduce a dedicated multiplier-scale type (for example `MultiplierPermille`) for `max_multiplier` and `escalation_multiplier` instead of overloading `Permille`.
7. Shared boundary under audit: the `with_component_schema_entries!` macro output plus the new multiplier-scale type exported from `worldwake-core` — adding one entry here implies generated function names `insert_component_drive_escalation_profile`, `get_component_drive_escalation_profile`, `set_component_drive_escalation_profile`, etc., consumed by ticket 003 (needs_system) and ticket 004 (RankingContext via belief views).

## Architecture Check

1. Universal classification per CLAUDE.md §5: `Default` impl at engine-wide defaults (`start_after_ticks: 100`, `growth_per_tick: Permille(10)`, `max_multiplier: MultiplierPermille(3000)`), `unwrap_or_default()` in spawn_agent (ticket 005), `.expect()` on known agents in ranking (ticket 004).
2. No `*Def` wrapper needed — `DriveEscalationProfile` contains no `EntityId` references (same shape as `DriveThresholds`). Direct RON deserialization.
3. `escalation_multiplier` is a pure function — takes `(ticks, params)`, returns a multiplier-scale value distinct from `Permille`. Never persisted; recomputed each ranking pass (FND-27 — derived summaries are caches, never truth).
4. Save-format version bump over migration layer: FND-28 forbids compatibility shims in live authority paths. Existing saves become unloadable after this change, which is acceptable and documented as the canonical handling.

## Verification Layers

1. Component registration completeness → workspace compiles (delta.rs, component_tables.rs, world.rs macro expansions succeed).
2. Bootstrap ordering → `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` passes with the new `ComponentDelta::Set` row added.
3. Save/load round-trip → new bincode round-trip focused test for `DriveEscalationProfile` and `DriveEscalationParams`.
4. Multiplier math purity → focused unit tests covering neutrality below `start_after_ticks`, linear growth above it, and saturation at `max_multiplier`.
5. Single-crate ticket (core-only apart from the save_load version bump) — no cross-system verification needed yet; consumer tickets 003/004 will stress the cross-crate surface.

## What to Change

### 1. New component file

Create `crates/worldwake-core/src/drive_escalation_profile.rs` with:

```rust
use crate::{Component, HomeostaticNeedId, Permille};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DriveEscalationProfile {
    pub per_need: BTreeMap<HomeostaticNeedId, DriveEscalationParams>,
    pub default_per_need: DriveEscalationParams,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DriveEscalationParams {
    pub start_after_ticks: u32,
    pub growth_per_tick: Permille,
    pub max_multiplier: MultiplierPermille,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct MultiplierPermille(u16);

impl Component for DriveEscalationProfile {}

impl MultiplierPermille {
    pub const fn new(value: u16) -> Result<Self, &'static str> {
        if value < 1000 {
            Err("MultiplierPermille value must be >= 1000")
        } else {
            Ok(Self(value))
        }
    }

    pub const fn new_unchecked(value: u16) -> Self {
        assert!(value >= 1000, "MultiplierPermille value must be >= 1000");
        Self(value)
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl Default for DriveEscalationParams {
    fn default() -> Self {
        Self {
            start_after_ticks: 100,
            growth_per_tick: Permille::new_unchecked(10),
            max_multiplier: MultiplierPermille::new_unchecked(3000),
        }
    }
}

impl Default for DriveEscalationProfile {
    fn default() -> Self {
        Self {
            per_need: BTreeMap::new(),
            default_per_need: DriveEscalationParams::default(),
        }
    }
}

impl DriveEscalationProfile {
    pub fn params_for(&self, need: HomeostaticNeedId) -> DriveEscalationParams {
        self.per_need.get(&need).copied().unwrap_or(self.default_per_need)
    }
}

pub fn escalation_multiplier(
    ticks_over_critical: u32,
    params: DriveEscalationParams,
) -> MultiplierPermille {
    if ticks_over_critical <= params.start_after_ticks {
        return MultiplierPermille::new_unchecked(1000);
    }
    let over_start = ticks_over_critical - params.start_after_ticks;
    let raw = 1000u32
        .saturating_add(over_start.saturating_mul(u32::from(params.growth_per_tick.value())));
    let capped = raw.min(u32::from(params.max_multiplier.value())).min(u32::from(u16::MAX));
    MultiplierPermille::new_unchecked(capped as u16)
}
```

### 2. Export from crate root

Add `pub use drive_escalation_profile::{DriveEscalationParams, DriveEscalationProfile, MultiplierPermille, escalation_multiplier};` in `crates/worldwake-core/src/lib.rs` and declare `mod drive_escalation_profile;`.

### 3. Register component in schema

Add a new `with_component_schema_entries!` entry in `crates/worldwake-core/src/component_schema.rs` following the `DriveThresholds` pattern (lines 1032-1054). Restrict to `EntityKind::Agent`. Entry naming should follow the generated-function convention — `drive_escalation_profile` / `DriveEscalationProfile` / `insert_drive_escalation_profile` / etc. / `set_component_drive_escalation_profile` / `clear_component_drive_escalation_profile`.

### 4. Bootstrap in `World::create_agent`

Add before `Ok(())` at `world.rs:205`:

```rust
world.insert_component_drive_escalation_profile(entity, DriveEscalationProfile::default())?;
```

### 5. Update create_agent delta assertion

Insert one `StateDelta::Component(ComponentDelta::Set { entity: agent, component_kind: ComponentKind::DriveEscalationProfile, before: None, after: ComponentValue::DriveEscalationProfile(DriveEscalationProfile::default()) })` row in the `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` expected-delta list at `world_txn.rs:2386-...`. Position must match the canonical emitted delta order for `create_agent`; in the live implementation this places `DriveEscalationProfile` between `ExecutionBudget` and `IntentionDispositionProfile`.

### 6. Bump `SAVE_FORMAT_VERSION`

Change `crates/worldwake-sim/src/save_load.rs:6` from `31` to `32`. No migration shim — existing saves cease to load (intentional per FND-28).

## Files to Touch

- `crates/worldwake-core/src/drive_escalation_profile.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — module declaration + re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — new `with_component_schema_entries!` entry)
- `crates/worldwake-core/src/world.rs` (modify — `create_agent` insert call)
- `crates/worldwake-core/src/world_txn.rs` (modify — create_agent delta assertion test)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` bump to 32)

## Out of Scope

- Scenario-RON integration — ticket 005.
- `needs_system` consumption — ticket 003.
- `ranking.rs` consumption — ticket 004.
- Migration of existing save files to the new version (FND-28: no backward compatibility layer).

## Acceptance Criteria

### Tests That Must Pass

1. New bincode round-trip test for `DriveEscalationProfile` (with a non-empty `per_need` map), `DriveEscalationParams`, and `MultiplierPermille`.
2. New unit tests for `escalation_multiplier`: (a) returns `MultiplierPermille::IDENTITY` when `ticks <= start_after_ticks`, (b) grows linearly by `growth_per_tick` past the threshold, (c) saturates at `max_multiplier`, (d) handles `u32::MAX` ticks via saturation without panic.
3. `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` passes with the new row.
4. Existing suite: `cargo test -p worldwake-core`, `cargo test -p worldwake-sim save_load`.

### Invariants

1. Every spawned agent via `World::create_agent` has `DriveEscalationProfile` set; `get_component_drive_escalation_profile` returns `Some(_)` for all live agents.
2. `escalation_multiplier` never exceeds `params.max_multiplier`.
3. Macro expansion in `delta.rs`, `component_tables.rs`, `world.rs` compiles without imports of `DriveEscalationProfile` being missing (README check #13).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/drive_escalation_profile.rs` — `#[cfg(test)] mod tests` with bincode round-trip, `Default` correctness, `params_for` fallback to default, `escalation_multiplier` math at boundary / linear / saturation / extreme-input cases.
2. `crates/worldwake-core/src/world_txn.rs` — update `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` expected-delta list.
3. `crates/worldwake-sim/src/save_load.rs` — update any version-pinned test fixtures; ensure save/load round-trip covers an agent with `DriveEscalationProfile`.

### Commands

1. `cargo test -p worldwake-core drive_escalation_profile`
2. `cargo test -p worldwake-core`
3. `cargo test -p worldwake-sim save_load`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-17.

- Added [crates/worldwake-core/src/drive_escalation_profile.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/drive_escalation_profile.rs) with `DriveEscalationProfile`, `DriveEscalationParams`, the dedicated `MultiplierPermille` scalar wrapper, and the pure `escalation_multiplier(...)` helper.
- Registered `DriveEscalationProfile` as a universal agent component, exported the new types from `worldwake-core`, and bootstrapped the component in `World::create_agent`.
- Updated shared macro-expansion fallout in `component_schema.rs`, `component_tables.rs`, `delta.rs`, and `world_txn.rs`, including the `create_agent` delta assertion surface and component-kind coverage.
- Bumped `SAVE_FORMAT_VERSION` from `31` to `32` so persisted authoritative state remains truthful after the new ECS component lands.
- Corrected the live S116 contract away from the drafted but impossible `Permille(3000)` cap to `MultiplierPermille(3000)`, and propagated that factual update into the active S116 spec and dependent tickets 004, 005, and 006.

## Deviations

- Reassessment found a foundational numeric mismatch: `Permille` is lawfully bounded to `0..=1000`, so the draft's `max_multiplier: Permille(3000)` was not representable. This ticket therefore introduced `MultiplierPermille` rather than widening `Permille`, preserving FOUNDATIONS alignment and the intended `>1x` escalation behavior.
- The drafted delta-order note assumed the new component would appear last because `create_agent` inserts it last. Live `world_txn` output uses canonical component-delta ordering instead, so the assertion had to place `DriveEscalationProfile` between `ExecutionBudget` and `IntentionDispositionProfile`.
- Shared compile-surface fallout was broader than the original file list. In addition to the drafted core/save-load surfaces, `delta.rs` and `component_tables.rs` required explicit imports / fixture coverage to keep the macro-generated component surface complete.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-core --lib world_txn::tests::create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through -- --exact`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-sim save_load`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
