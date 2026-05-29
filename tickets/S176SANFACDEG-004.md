# S176SANFACDEG-004: clean_wash_basin & empty_latrine maintenance actions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — 2 new actions (systems), `WasteSource::LatrineEmptied` (core), `SelfCareUseKind::{CleanWashBasin, EmptyLatrine}` (core + 8-file exhaustive-match arms)
**Deps**: S176SANFACDEG-001 (`WashBasinState` reset), S176SANFACDEG-002 (`MetabolismDurationKind::{CleanBasin, EmptyLatrine}` + profile durations)

## Problem

There is no cleaning or emptying affordance — basin dirtiness and latrine fill rise monotonically and never recover through labor (S176 D5). This ticket adds two duration-bearing, occupancy-bearing maintenance actions that reset the degradation state and emit concrete `Waste` aftermath.

## Assumption Reassessment (2026-05-29)

1. `register_needs_actions` is at `crates/worldwake-systems/src/needs_actions.rs:23`; actions register via the `register_def` helper (`:147`) or direct `defs.register(ActionDef { … })` (the `relieve_wilderness` precedent at `:111-144` shows the full-struct form with explicit abort handler). `SelfCareOccupancy` is at `crates/worldwake-core/src/self_care_occupancy.rs:7` with `use_kind: SelfCareUseKind` (variants `Wash, LatrineRelief, Eat, Drink, WildernessRelief, Sleep`); `start_self_care_occupancy()` (`needs_actions.rs:348`) enforces exclusivity; the S173 abort discipline (explicit handler, no `abort_noop`) applies.
2. `WasteSource` is at `crates/worldwake-core/src/decision_event_payload.rs:73` (`WildernessRelief`, `OvercapacityLatrine`); use sites are construction-only (`decision_event_payload.rs`, `save_load.rs`, `observer.rs`) — appending `LatrineEmptied` is bincode-read-safe (no format bump). `WasteCreatedPayload` carries `{ creator, place, waste_lot, source, place_dirtiness_delta }`. `CommodityKind::Waste` exists (`crates/worldwake-core/src/items.rs:20`).
3. `SelfCareUseKind` is matched in **8** non-test files: `component_schema.rs`, `needs_actions.rs`, `save_load.rs`, `tick_step.rs`, `per_agent_belief_view.rs`, `action_trace.rs`, `self_care_occupancy.rs`, `delta.rs`. The two new variants need arms at each exhaustive site (several are serialization/delta mirrors taking pass-through arms). `ActionTraceDetail::SelfCareInterrupted { kind: SelfCareUseKind, basin: Option<EntityId> }` (`action_trace.rs:66`) is reused for cleaning interruption.
4. Conserved quantities (FND-4): `clean_wash_basin` consumes `clean_water_units` and resets `dirtiness_level` toward `Permille::ZERO`, emitting a `Waste` lot + `PlaceDirtiness`. `empty_latrine` resets `LatrineFullness.fill` toward `Permille::ZERO` and creates a `Waste` lot proportional to the emptied fill, emitting `WasteCreated { source: LatrineEmptied }`. State the concrete reset cadence (single-commit full reset) and the lot-quantity formula (proportional to emptied fill) at implementation.
5. Durations come from `DurationExpr::ActorMetabolism { kind: CleanBasin | EmptyLatrine }` (the kinds + profile fields land in S176SANFACDEG-002); `clean_wash_basin` targets the co-located `WashBasin` facility (as `wash`), `empty_latrine` targets `TargetSpec::ActorPlace` (as `toilet`).

## Architecture Check

1. Recovery is concrete labor with duration, cost (time), and exclusive occupancy reusing the existing `SelfCareOccupancy` substrate — no new contention queue, no "facility resets itself" (FND-8, FND-11 dampener).
2. FND-4: the emptied fill / cleaned grime materializes as a `Waste` lot with provenance rather than vanishing; FND-28: new `SelfCareUseKind`/`WasteSource` variants are appended, not aliased.

## Verification Layers

1. State reset on commit → authoritative world state (`dirtiness_level`/`fill` toward zero).
2. Waste aftermath + provenance → event-log delta (`WasteCreated` with `LatrineEmptied`/place dirtiness delta) and authoritative `Waste` lot.
3. Occupancy + interruption → action trace (`SelfCareInterrupted` on abort) and `SelfCareOccupancy` release.

## What to Change

### 1. WasteSource + SelfCareUseKind variants

Add `WasteSource::LatrineEmptied`; add `SelfCareUseKind::{CleanWashBasin, EmptyLatrine}` and the arms at the 8 exhaustive-match sites.

### 2. clean_wash_basin action

Register the action (target = co-located `WashBasin` facility, `DurationExpr::ActorMetabolism { kind: CleanBasin }`, `SelfCareUseKind::CleanWashBasin` occupancy, explicit abort handler). Commit: reset `dirtiness_level`, consume `clean_water_units`, emit `Waste` lot + raise `PlaceDirtiness`.

### 3. empty_latrine action

Register the action (target = `ActorPlace`, `DurationExpr::ActorMetabolism { kind: EmptyLatrine }`, `SelfCareUseKind::EmptyLatrine` occupancy, explicit abort handler). Commit: reset `LatrineFullness.fill`, create proportional `Waste` lot, emit `WasteCreated { source: LatrineEmptied }`.

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modify — register both actions + handlers + SelfCareUseKind arms)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify — `WasteSource::LatrineEmptied`)
- `crates/worldwake-core/src/self_care_occupancy.rs` (modify — `SelfCareUseKind` variants)
- `crates/worldwake-core/src/component_schema.rs` (modify — `SelfCareUseKind` arm)
- `crates/worldwake-core/src/delta.rs` (modify — `SelfCareUseKind` arm)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SelfCareUseKind` arm)
- `crates/worldwake-sim/src/tick_step.rs` (modify — `SelfCareUseKind` arm)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — `SelfCareUseKind` arm)
- `crates/worldwake-sim/src/action_trace.rs` (modify — `SelfCareUseKind` arm in `SelfCareInterrupted` rendering)

## Out of Scope

- Planner insertion of these actions as prerequisites — S176SANFACDEG-005 (named owner of `classify_action_def` + op mapping).
- The duration kinds + profile fields these actions consume — S176SANFACDEG-002.

## Acceptance Criteria

### Tests That Must Pass

1. `clean_wash_basin` commit resets `dirtiness_level`, consumes `clean_water_units`, and emits a `Waste` lot + `PlaceDirtiness`.
2. `empty_latrine` commit resets `fill` and emits `WasteCreated { source: LatrineEmptied }` with a proportional `Waste` lot.
3. Both actions occupy the facility exclusively and release occupancy on abort (`SelfCareInterrupted`).
4. Existing suite: `cargo test -p worldwake-systems && cargo test -p worldwake-core`

### Invariants

1. Conservation: emptied fill / cleaned grime becomes a concrete `Waste` lot — no quantity created from nothing (FND-4).
2. Cleaning/emptying reuse `SelfCareOccupancy`; no new contention queue.
3. Facility state only resets through a committed maintenance action or the existing decay/refill — never spontaneously.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` — new: clean_wash_basin reset+aftermath, empty_latrine reset+waste provenance, occupancy + abort release for both.

### Commands

1. `cargo test -p worldwake-systems needs_actions`
2. `cargo test -p worldwake-core && cargo test -p worldwake-sim`
3. `scripts/verify.sh`
