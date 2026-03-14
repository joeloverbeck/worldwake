# E15RUMWITDIS-002: Add Social EventTag, ActionDomain, and SocialObservationKind::WitnessedTelling

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new enum variants in core and sim
**Deps**: None (pure additive enum changes)

## Problem

E15 introduces the Tell action which requires a `Social` event tag, a `Social` action domain, and a `WitnessedTelling` social observation kind. These are foundational enum variants that all subsequent E15 tickets depend on.

## Assumption Reassessment (2026-03-14)

1. `EventTag` lives in `crates/worldwake-core/src/event_tag.rs` — confirmed. Currently has 19 variants, no `Social` or `Discovery`.
2. `ActionDomain` lives in `crates/worldwake-sim/src/action_domain.rs` — confirmed. Currently has 10 variants (Generic, Needs, Production, Trade, Travel, Transport, Combat, Care, Corpse), no `Social`.
3. `SocialObservationKind` lives in `crates/worldwake-core/src/belief.rs` — confirmed. Currently has 4 variants (WitnessedCooperation, WitnessedConflict, WitnessedObligation, CoPresence), no `WitnessedTelling`.
4. The `social_kind()` function in `crates/worldwake-systems/src/perception.rs` maps `EventTag::Trade` → `WitnessedCooperation` and `EventTag::Combat` → `WitnessedConflict`. It will need a new arm for `EventTag::Social` → `WitnessedTelling`.

## Architecture Check

1. Pure additive changes to existing enums. No restructuring needed.
2. No backwards-compatibility shims — just new variants added to existing `#[derive(…)]` enums.

## What to Change

### 1. Add `EventTag::Social` and `EventTag::Discovery`

In `crates/worldwake-core/src/event_tag.rs`, add two new variants to the `EventTag` enum:
- `Social` — tags Tell action events and future social interaction events
- `Discovery` — tags belief mismatch discovery events

### 2. Add `ActionDomain::Social`

In `crates/worldwake-sim/src/action_domain.rs`, add a `Social` variant to the `ActionDomain` enum.

### 3. Add `SocialObservationKind::WitnessedTelling`

In `crates/worldwake-core/src/belief.rs`, add a `WitnessedTelling` variant to the `SocialObservationKind` enum.

### 4. Extend `social_kind()` in perception

In `crates/worldwake-systems/src/perception.rs`, add a match arm in `social_kind()` so that events tagged with `EventTag::Social` produce `SocialObservationKind::WitnessedTelling`.

## Files to Touch

- `crates/worldwake-core/src/event_tag.rs` (modify)
- `crates/worldwake-sim/src/action_domain.rs` (modify)
- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify)

## Out of Scope

- Tell action definition, payload, or handler
- TellProfile component
- MismatchKind enum or mismatch detection logic
- belief_confidence() derivation function
- Any AI/planner changes
- Any new modules or files

## Acceptance Criteria

### Tests That Must Pass

1. `EventTag::Social` exists and is serializable/deserializable
2. `EventTag::Discovery` exists and is serializable/deserializable
3. `ActionDomain::Social` exists and is serializable/deserializable
4. `SocialObservationKind::WitnessedTelling` exists
5. Perception `social_kind()` maps `EventTag::Social` → `SocialObservationKind::WitnessedTelling`
6. Existing suite: `cargo test --workspace`
7. `cargo clippy --workspace`

### Invariants

1. All existing EventTag, ActionDomain, and SocialObservationKind variants remain unchanged
2. Existing perception tests continue to pass without modification
3. Serialization roundtrip works for all new variants (Serialize + Deserialize derives already present)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs::tests` — add test: emit a Social-tagged event and verify bystanders record `WitnessedTelling` social observation

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-systems`
4. `cargo clippy --workspace`
5. `cargo test --workspace`
