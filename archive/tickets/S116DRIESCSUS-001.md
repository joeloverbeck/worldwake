# S116DRIESCSUS-001: Core extensions — HomeostaticNeedId::ALL, DeprivationExposure.dirtiness, EventTag::Escalation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `HomeostaticNeedId`, `HomeostaticNeeds`, `DriveThresholds`, `DeprivationExposure`, `EventTag`
**Deps**: archive/specs/S116-drive-escalation-sustained-critical.md (D1, D5)

## Problem

Spec S116 extends motive scoring with a per-need ticks-above-critical multiplier. The multiplier reads from an authoritative counter. Today `DeprivationExposure` tracks ticks-above-critical for 4 of 5 homeostatic needs (no dirtiness) and there is no keyed accessor on `HomeostaticNeeds`, `DriveThresholds`, or `DeprivationExposure`. Adding a second tracker component would violate FND-28. This ticket extends the existing `DeprivationExposure` to cover dirtiness, adds keyed accessors, introduces `HomeostaticNeedId::ALL`, and adds the `EventTag::Escalation` unit variant that later tickets emit.

## Assumption Reassessment (2026-04-17)

1. `HomeostaticNeedId` at `crates/worldwake-core/src/needs.rs:19-29` has `VARIANT_COUNT: usize = 5` but no `::ALL` constant. Existing pattern: `CommodityKind::ALL`, `SystemId::ALL`. Existing tests: `homeostatic_need_id_variant_count_matches_enum` at needs.rs:265.
2. `DeprivationExposure` at `needs.rs:72-80` holds 4 `*_critical_ticks` fields; dirtiness intentionally absent. `#[derive(Default)]` means `dirtiness_critical_ticks: u32` defaults to 0 with no change to literal construction sites that use `..Default::default()`.
3. Shared boundary under audit: `EventTag` unit-variant enum in `crates/worldwake-core/src/event_tag.rs` is consumed by `worldwake-sim` via `EventPayload.tags: BTreeSet<EventTag>`. Existing tests: `event_tag_includes_all_required_variants` (26 entries), `event_tag_bincode_roundtrip_covers_every_variant`, `event_tag_order_is_declaration_stable` all at event_tag.rs:75-101. Live invariant: `ALL_EVENT_TAGS` must stay in declaration order, not alphabetical order.
4. Existing needs-system tests that assert counter increments: `needs_system_increments_deprivation_exposure_at_critical_thresholds` at `crates/worldwake-systems/src/needs.rs:768`; and `needs_system_resets_deprivation_exposure_when_pressure_drops_below_critical` at needs.rs:849. Both hard-code the 4-field pattern and will need a dirtiness row after this ticket.
5. Live `DeprivationExposure { ... }` fallout is broader than the drafted systems-only list. In addition to the named `worldwake-systems` tests and the `e09_needs_integration` destructure, full literals also exist in `crates/worldwake-core/src/needs.rs`, `component_tables.rs`, `world_txn.rs`, `delta.rs`, and `world.rs`. Production `worldwake-systems/src/needs.rs::update_exposure` also constructs a full `DeprivationExposure`; for this ticket it must preserve `dirtiness_critical_ticks` unchanged rather than begin maintaining it.

## Architecture Check

1. Extending existing authoritative state preserves FND-28 — no second "ticks-above-critical per need" counter component is introduced. The counter's consumer set widens from wound generation alone to wound generation + motive escalation.
2. Keyed accessors (`HomeostaticNeeds::value(id)`, `DriveThresholds::critical(id)`, `DeprivationExposure::ticks_at_critical(id)`) centralize enum→field dispatch, avoiding duplicated match blocks across future consumers.
3. `EventTag::Escalation` is a unit variant, consistent with every existing variant. No payload-embedded fields — the escalation event's need id and multiplier are carried via the emitting `EventPayload` (action_name / state_deltas) in ticket 003.

## Verification Layers

1. Struct extension correctness → focused serde round-trip test for `DeprivationExposure` including the new field.
2. Keyed accessor correctness → focused unit tests per accessor covering all 5 `HomeostaticNeedId` variants.
3. `EventTag` surface invariant → existing `event_tag_includes_all_required_variants` test count updated to 27; existing `event_tag_bincode_roundtrip_covers_every_variant` exercises the new variant via `ALL_EVENT_TAGS`.
4. Shared additive ticket with narrow production impact. Verification still stays below full AI-pipeline scope because this ticket does not change authoritative validation or planner behavior, but same-crate and cross-crate constructor fallout must be covered honestly.

## What to Change

### 1. `HomeostaticNeedId::ALL`

Add `pub const ALL: [Self; 5] = [Self::Hunger, Self::Thirst, Self::Fatigue, Self::Bladder, Self::Dirtiness];` in the `impl HomeostaticNeedId` block in `needs.rs`.

### 2. Extend `DeprivationExposure`

Add `pub dirtiness_critical_ticks: u32,` as the fifth field. Update the existing `deprivation_exposure_default_is_zeroed` test to include the new field explicitly.

### 3. Keyed accessor on `DeprivationExposure`

```rust
impl DeprivationExposure {
    pub fn ticks_at_critical(&self, need: HomeostaticNeedId) -> u32 {
        match need {
            HomeostaticNeedId::Hunger => self.hunger_critical_ticks,
            HomeostaticNeedId::Thirst => self.thirst_critical_ticks,
            HomeostaticNeedId::Fatigue => self.fatigue_critical_ticks,
            HomeostaticNeedId::Bladder => self.bladder_critical_ticks,
            HomeostaticNeedId::Dirtiness => self.dirtiness_critical_ticks,
        }
    }
}
```

### 4. Keyed accessor on `HomeostaticNeeds`

```rust
impl HomeostaticNeeds {
    pub fn value(&self, need: HomeostaticNeedId) -> Permille {
        match need {
            HomeostaticNeedId::Hunger => self.hunger,
            HomeostaticNeedId::Thirst => self.thirst,
            HomeostaticNeedId::Fatigue => self.fatigue,
            HomeostaticNeedId::Bladder => self.bladder,
            HomeostaticNeedId::Dirtiness => self.dirtiness,
        }
    }
}
```

### 5. Keyed accessor on `DriveThresholds`

```rust
impl DriveThresholds {
    pub fn critical(&self, need: HomeostaticNeedId) -> Permille {
        match need {
            HomeostaticNeedId::Hunger => self.hunger.critical(),
            HomeostaticNeedId::Thirst => self.thirst.critical(),
            HomeostaticNeedId::Fatigue => self.fatigue.critical(),
            HomeostaticNeedId::Bladder => self.bladder.critical(),
            HomeostaticNeedId::Dirtiness => self.dirtiness.critical(),
        }
    }
}
```

### 6. `EventTag::Escalation`

Add the unit variant to `EventTag` in `crates/worldwake-core/src/event_tag.rs`. Update `ALL_EVENT_TAGS` (line 46) to include `EventTag::Escalation` and bump the length assertion (line 82) from 26 to 27. Variants must remain in declaration order with `ALL_EVENT_TAGS` matching the enum exactly; add `Escalation` adjacent to `Discovery` and preserve the existing `event_tag_order_is_declaration_stable` proof.

### 7. Update construction sites

Update all live `DeprivationExposure { ... }` full literals touched by the new field, including the drafted systems/test sites and the shared core test fixtures (`needs.rs`, `component_tables.rs`, `world_txn.rs`, `delta.rs`, `world.rs`). Keep the existing style at each site. In `worldwake-systems/src/needs.rs::update_exposure`, carry the existing dirtiness counter through unchanged so this ticket does not start dirtiness-counter maintenance early.

## Files to Touch

- `crates/worldwake-core/src/needs.rs` (modify — `HomeostaticNeedId::ALL`, `DeprivationExposure.dirtiness_critical_ticks`, `ticks_at_critical`, `value`, plus tests)
- `crates/worldwake-core/src/drives.rs` (modify — `DriveThresholds::critical(HomeostaticNeedId)` accessor + test)
- `crates/worldwake-core/src/event_tag.rs` (modify — add `Escalation`; bump test count; preserve declaration order)
- `crates/worldwake-core/src/component_tables.rs` (modify — shared `DeprivationExposure` test fixture)
- `crates/worldwake-core/src/world_txn.rs` (modify — shared `DeprivationExposure` delta test fixture)
- `crates/worldwake-core/src/delta.rs` (modify — shared `ComponentValue::DeprivationExposure` fixture)
- `crates/worldwake-core/src/world.rs` (modify — shared `sample_deprivation_exposure()` fixture)
- `crates/worldwake-systems/src/needs.rs` (modify — production `update_exposure` carry-through + test construction sites)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify — `DeprivationExposure` destructure)

## Out of Scope

- `needs_system` dirtiness counter maintenance — ticket 003.
- Escalation event emission — ticket 003.
- `DriveEscalationProfile` component — ticket 002.
- Ranking integration — ticket 004.

## Acceptance Criteria

### Tests That Must Pass

1. `needs::tests::deprivation_exposure_default_is_zeroed` — extended to include `dirtiness_critical_ticks: 0`.
2. `needs::tests::homeostatic_need_id_variant_count_matches_enum` — unchanged; `VARIANT_COUNT == 5`.
3. `needs::tests::physiology_types_roundtrip_through_bincode` — extended to roundtrip the new field.
4. `event_tag::tests::event_tag_includes_all_required_variants` — count updated 26 → 27.
5. `event_tag::tests::event_tag_bincode_roundtrip_covers_every_variant` — exercises `Escalation`.
6. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-systems needs`.

### Invariants

1. `DeprivationExposure::default()` zeroes all 5 fields.
2. No behavioral change to `needs_system` — dirtiness counter is declared but not yet written in this ticket.
3. `EventTag::ALL` ordering stays declaration-sorted (existing `event_tag_order_is_declaration_stable` test passes).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/needs.rs` — new focused tests for `HomeostaticNeedId::ALL` length/ordering, `ticks_at_critical`, `value` covering all 5 variants.
2. `crates/worldwake-core/src/drives.rs` — new focused test for `DriveThresholds::critical(HomeostaticNeedId)` covering all 5 variants.
3. `crates/worldwake-core/src/event_tag.rs` — update `ALL_EVENT_TAGS` (length 27).
4. Existing `needs::tests::*` updated to include dirtiness field in literal construction.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-systems needs`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-17.

- Added `HomeostaticNeedId::ALL`, `HomeostaticNeeds::value(HomeostaticNeedId)`, `DeprivationExposure::ticks_at_critical(HomeostaticNeedId)`, and `DriveThresholds::critical(HomeostaticNeedId)`.
- Extended `DeprivationExposure` with `dirtiness_critical_ticks` and updated the real shared constructor / fixture fallout across `worldwake-core`, `worldwake-systems`, and the E09 integration schema assertion.
- Added `EventTag::Escalation` and updated the declaration-order coverage so the tag surface now includes 27 variants.
- Preserved current `needs_system` behavior by carrying `dirtiness_critical_ticks` through `update_exposure` unchanged; dirtiness-counter maintenance remains owned by ticket `S116DRIESCSUS-003`.
- The `crates/worldwake-systems/tests/e09_needs_integration.rs` change was compile-surface fallout only (shared destructure/schema coverage), not a new runtime behavior proof surface.

## Deviations

- Reassessment widened the drafted constructor fallout beyond the original systems-only file list. Shared core fixtures in `component_tables.rs`, `world_txn.rs`, `delta.rs`, `world.rs`, and `needs.rs` also required updates.
- The draft claimed `EventTag` ordering was alphabetical. Live code proves a declaration-order invariant instead, so `Escalation` was inserted adjacent to `Discovery` and `ALL_EVENT_TAGS` was kept in enum declaration order.
- The existing `needs_system_resets_deprivation_exposure_when_pressure_drops_below_critical` assertion had to be updated to preserve `dirtiness_critical_ticks` rather than zero it, which is the intended no-behavior-change contract for this ticket.

## Verification Result

- Passed `cargo test --workspace --no-run`
- `cargo test --workspace --no-run` was the proof surface that validated shared constructor/destructure fallout, including `crates/worldwake-systems/tests/e09_needs_integration.rs`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-systems needs`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
