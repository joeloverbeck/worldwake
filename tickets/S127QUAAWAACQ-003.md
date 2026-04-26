# S127QUAAWAACQ-003: ResourceSource extraction_slots and extraction_duration_ticks

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends `ResourceSource` with two new fields, extends `ResourceSourceDef`, updates the scenario translator, bumps `SAVE_FORMAT_VERSION`
**Deps**: None

## Problem

S127 makes per-source extraction concurrency and per-extraction time cost concrete world state (D4). `ResourceSource.extraction_slots: NonZeroU8` makes occupancy explicit (a one-bucket well vs. a five-actor river bank); `extraction_duration_ticks: NonZeroU32` makes the time cost a waiting agent sees explicit, replacing the invisible reservation-conflict cooldown with concrete world time. This ticket also lands D10's first slice — adding the fields to `ResourceSourceDef` with backward-compatible serde defaults so existing `scenarios/*.ron` files require no change — and updates the scenario translator that constructs `ResourceSource` from `ResourceSourceDef`. Per FND-28 the bincode format breaks; bump `SAVE_FORMAT_VERSION` accordingly.

## Assumption Reassessment (2026-04-26)

1. `crates/worldwake-core/src/production.rs:74-83` defines `ResourceSource { commodity: CommodityKind, available_quantity: Quantity, max_quantity: Quantity, regeneration_ticks_per_unit: Option<NonZeroU32>, last_regeneration_tick: Option<Tick> }` — confirmed during reassessment. The struct derives `Component` (line 83) and `Default` (`#[derive(... Default ...)]` at line 41 of the surrounding scope per spot-check).
2. `specs/S127-quantity-aware-acquisition.md` D4 prescribes the field additions; D10 prescribes the `ResourceSourceDef` mirror with `#[serde(default = "default_extraction_slots")]` and `default_extraction_slots() -> u8 { 1 }`.
3. Construction-site spot-check: `grep -rn "ResourceSource {" crates/ | wc -l` → 163 sites. `ResourceSource` derives `Default`, but `NonZeroU8` does not have a stdlib `Default` impl — adding `extraction_slots: NonZeroU8` and `extraction_duration_ticks: NonZeroU32` to a `Default`-deriving struct will break the derive unless we either (a) drop the `Default` derive and write a manual `Default` impl returning `NonZeroU8::new(1).unwrap()`, or (b) use a wrapper type. Option (a) is cleanest and matches the field semantics (single-slot, one-tick extraction is the legacy default).
4. `crates/worldwake-cli/src/scenario/types.rs:500` defines `ResourceSourceDef` with fields `commodity, location, facility, regeneration_ticks_per_unit, capacity` — confirmed during reassessment.
5. `crates/worldwake-cli/src/scenario/mod.rs:417` constructs `ResourceSource { … }` from `ResourceSourceDef` in `spawn_scenario` (the second site is `crates/worldwake-cli/src/bin/observer.rs:4669` in test fixtures). Both must add the two new fields with `NonZeroU8::new(def.extraction_slots).unwrap_or(NonZeroU8::MIN)` and `NonZeroU32::new(def.extraction_duration_ticks).unwrap_or(NonZeroU32::MIN)`.
6. `SAVE_FORMAT_VERSION` is at `crates/worldwake-sim/src/save_load.rs:6`. After ticket 002 it lands at `49`; this ticket bumps it to `50` (sequential bumps because both tickets break the format).
7. Component registration in `crates/worldwake-core/src/component_schema.rs` already covers `ResourceSource`; field additions don't require a new registration entry. Per `tickets/README.md` check #13, no macro-expansion-site imports change because `ResourceSource` itself is not new.
13. Adjacent contradictions: `extraction_slots` is read by ticket 005 (queues registration) and ticket 007 (multi-slot start handler); both depend on this ticket landing first. No contradiction — they are sequential consumers.

## Architecture Check

1. `extraction_slots` and `extraction_duration_ticks` are concrete entity state, not derived "throughput score" abstractions (FND-3). The waiting agent's projected delay is now `extraction_duration_ticks * queue_position` — concrete world time, not an opaque blocker cooldown.
2. `serde(default)` on `ResourceSourceDef` keeps existing `scenarios/*.ron` files compiling without edit, satisfying the spec's backward-compat scenario claim. This is a boundary-only compat (RON parsing), not a live-authority shim — the runtime always reads the resolved `NonZeroU8`/`NonZeroU32` value, never the optional default.
3. Dropping the `Default` derive on `ResourceSource` and writing a manual `impl Default` is cleaner than wrapping fields in `Option<NonZero…>` — the wrapper would propagate `None`-handling through every consumer for a default that is never `None` in practice (single-slot, one-tick extraction is the universal legacy baseline).

## Verification Layers

1. `ResourceSource` field additions round-trip via bincode → focused unit test in `production.rs` `#[cfg(test)]`.
2. `ResourceSourceDef` deserializes existing scenarios without the new fields → focused test loading a representative `scenarios/*.ron` and confirming the resolved `ResourceSource` has `extraction_slots = 1` and `extraction_duration_ticks = 1`.
3. New `ResourceSourceDef` authoring with explicit `extraction_slots = 5` produces a `ResourceSource` with `NonZeroU8::new(5).unwrap()` → focused test in `scenario/mod.rs`.
4. Save format rejects version `49` saves after bump → existing save-load infrastructure handles this when `SAVE_FORMAT_VERSION` increments.
5. Single-layer ticket (data shape addition with translator wiring) — no action trace, decision trace, or world-state ordering proof surface applies; correctness is type-level + bincode round-trip.

## What to Change

### 1. Extend `ResourceSource` in `crates/worldwake-core/src/production.rs:74-83`

Add `extraction_slots: NonZeroU8` and `extraction_duration_ticks: NonZeroU32` per spec D4. Drop the `Default` derive (will fail because `NonZeroU8` has no `Default`) and replace with a manual `impl Default for ResourceSource` returning all fields at their legacy defaults: `available_quantity: Quantity(0)`, `max_quantity: Quantity(0)`, `regeneration_ticks_per_unit: None`, `last_regeneration_tick: None`, `extraction_slots: NonZeroU8::new(1).unwrap()`, `extraction_duration_ticks: NonZeroU32::new(1).unwrap()`, `commodity: CommodityKind::default()` (assuming `CommodityKind` derives `Default` — confirm during implementation; if not, the manual `Default` impl drops too and call sites use explicit construction).

### 2. Update all `ResourceSource { … }` construction sites

163 sites across the workspace; most are test fixtures. Systematic update:

- `crates/worldwake-cli/src/scenario/mod.rs:417` (production scenario translator) — read `def.extraction_slots` and `def.extraction_duration_ticks` and construct with `NonZeroU8::new(def.extraction_slots).unwrap_or(NonZeroU8::MIN)`, `NonZeroU32::new(def.extraction_duration_ticks).unwrap_or(NonZeroU32::MIN)`.
- `crates/worldwake-cli/src/bin/observer.rs:4669` (test fixture) — add explicit `extraction_slots: NonZeroU8::new(1).unwrap()`, `extraction_duration_ticks: NonZeroU32::new(1).unwrap()`.
- All other test fixtures — same pattern, or use `..ResourceSource::default()` if the manual `Default` impl is preserved.

### 3. Extend `ResourceSourceDef` in `crates/worldwake-cli/src/scenario/types.rs:500`

Per spec D10:

```rust
pub struct ResourceSourceDef {
    // … existing fields …
    #[serde(default = "default_extraction_slots")]
    pub extraction_slots: u8,
    #[serde(default = "default_extraction_duration_ticks")]
    pub extraction_duration_ticks: u32,
}

fn default_extraction_slots() -> u8 { 1 }
fn default_extraction_duration_ticks() -> u32 { 1 }
```

### 4. Bump `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs:6` — change from `49` (after ticket 002) to `50`.

### 5. Add focused tests

In `production.rs` `#[cfg(test)]`: bincode round-trip preserving the two new fields. In `scenario/mod.rs` `#[cfg(test)]`: load a synthetic `ResourceSourceDef` without the new fields and confirm the resolved `ResourceSource` has `extraction_slots = NonZeroU8::new(1).unwrap()`; load with explicit `extraction_slots = 5` and confirm round-trip.

## Files to Touch

- `crates/worldwake-core/src/production.rs` (modify — field additions, manual `Default` impl, focused test)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — `ResourceSourceDef` extension, default helpers)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — translator at line 417)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — test fixture at line 4669)
- All test fixture files constructing `ResourceSource { … }` (modify — workspace-wide grep covers these; ~160 sites)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump SAVE_FORMAT_VERSION)

## Out of Scope

- `LastHarvestTrace` component — ticket 004.
- `ResourceExtractionQueues` component and queues registration in scenario translator — ticket 005.
- `ScenarioDef.harvest_trace_retention_ticks` — ticket 004 (lives with the LastHarvestTrace component).
- Reading `extraction_slots` in the harvest action handler (multi-slot start) — ticket 007.
- Updating decision/action traces to surface the new fields — out of scope for this ticket; the fields are observable via component-level perception reads (FND-14A) without explicit trace surfacing.

## Acceptance Criteria

### Tests That Must Pass

1. `resource_source_bincode_roundtrip_includes_extraction_fields` — round-trip preserves `extraction_slots` and `extraction_duration_ticks`.
2. `scenario_def_resource_source_defaults_to_one_slot` — RON without the new fields resolves to `extraction_slots == 1, extraction_duration_ticks == 1`.
3. `scenario_def_resource_source_explicit_slots` — RON with `extraction_slots: 5` resolves to `extraction_slots == 5`.
4. Existing scenario load tests pass without modification (proves backward-compat).
5. Existing suite: `cargo test --workspace`.

### Invariants

1. `ResourceSource.extraction_slots.get() >= 1` always (NonZeroU8 enforces this at the type level).
2. `ResourceSource.extraction_duration_ticks.get() >= 1` always.
3. Existing `scenarios/*.ron` files deserialize identically to pre-ticket behavior (resolved `ResourceSource` has `extraction_slots = 1, extraction_duration_ticks = 1`).
4. `SAVE_FORMAT_VERSION = 50`; bincode saves at version `49` fail to load.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` `#[cfg(test)]` — bincode round-trip test for the new fields.
2. `crates/worldwake-cli/src/scenario/mod.rs` `#[cfg(test)]` — defaults test and explicit-value test.

### Commands

1. `cargo test -p worldwake-core resource_source_bincode_roundtrip`
2. `cargo test -p worldwake-cli scenario_def_resource_source`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh`
