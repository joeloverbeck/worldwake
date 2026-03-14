# E15RUMWITDIS-002: Add Social EventTag, ActionDomain, and SocialObservationKind::WitnessedTelling

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new enum variants in core and sim
**Deps**: None (pure additive enum changes)

## Problem

E15 introduces two new event classifications with different purposes:
- `EventTag::Social` for Tell commits and future social interactions.
- `EventTag::Discovery` for belief-mismatch discovery events emitted by perception.

It also requires:
- `ActionDomain::Social` so Tell and future social actions have a first-class domain.
- `SocialObservationKind::WitnessedTelling` so bystanders can record that they saw a telling interaction.

These are foundational enum additions for the later E15 work, but this ticket should stay limited to the classification surfaces and the current perception mapping.

## Assumption Reassessment (2026-03-14)

1. `EventTag` lives in `crates/worldwake-core/src/event_tag.rs` — confirmed. It currently has 18 variants, not 19. There is no `Social` or `Discovery` variant today.
2. `ActionDomain` lives in `crates/worldwake-sim/src/action_domain.rs` — confirmed. It currently has 9 variants, not 10: `Generic`, `Needs`, `Production`, `Trade`, `Travel`, `Transport`, `Combat`, `Care`, `Corpse`. There is no `Social` variant today.
3. `SocialObservationKind` lives in `crates/worldwake-core/src/belief.rs` — confirmed. It currently has 4 variants: `WitnessedCooperation`, `WitnessedConflict`, `WitnessedObligation`, `CoPresence`. There is no `WitnessedTelling` variant today.
4. `social_kind()` in `crates/worldwake-systems/src/perception.rs` currently recognizes only `EventTag::Trade` and `EventTag::Combat`. It does not yet map a social tag.
5. The enum files already contain bincode roundtrip tests over "all variants" arrays. The right implementation is to extend those existing coverage points, not add duplicate one-off serialization tests elsewhere.
6. There is currently no discovery-emission logic in perception. Adding `EventTag::Discovery` here is still aligned with the E15 spec, but behavioral tests for emitted discovery events are out of scope for this ticket because the emitter is not part of this change.

## Architecture Check

1. The proposed enum additions are still the right architecture. `EventTag`, `ActionDomain`, and `SocialObservationKind` are the existing type-safe classification surfaces for event indexing, action categorization, and belief memory. Extending them is cleaner than introducing side tables, stringly typed tags, or temporary aliases.
2. `EventTag::Discovery` should remain distinct from `EventTag::Social`. Discovery is an epistemic mismatch event produced by perception, not a social interaction, so collapsing them would blur causality and downstream consumers.
3. No restructuring is justified in this ticket. The only code change beyond enum additions should be the existing perception classifier (`social_kind()`), because that is the current single mapping point from event tags into social observations.
4. No backward-compatibility shims or alias variants. Broken callers should be updated in later tickets as the new variants become used.

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
- Emitting Discovery-tagged events
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
6. Existing enum roundtrip/all-variants tests are updated rather than duplicated
7. Relevant crate suites pass before workspace-wide verification
8. Existing suite: `cargo test --workspace`
9. `cargo clippy --workspace`

### Invariants

1. All existing EventTag, ActionDomain, and SocialObservationKind variants remain unchanged
2. Existing perception behavior for `Trade` and `Combat` remains unchanged
3. Serialization roundtrip works for all variants after the new enum members are added

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_tag.rs::tests` — extend the all-variants roundtrip/order coverage to include `Social` and `Discovery`
2. `crates/worldwake-sim/src/action_domain.rs::tests` — extend the all-domains roundtrip/combat-classification coverage to include `Social`
3. `crates/worldwake-core/src/belief.rs::tests` — strengthen the social observation kind roundtrip test to cover `WitnessedTelling`
4. `crates/worldwake-systems/src/perception.rs::tests` — add a social-tagged event test verifying bystanders record `WitnessedTelling`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-systems`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - Added `EventTag::Social` and `EventTag::Discovery`
  - Added `ActionDomain::Social`
  - Added `SocialObservationKind::WitnessedTelling`
  - Extended `perception::social_kind()` so `EventTag::Social` records `WitnessedTelling`
  - Strengthened existing enum coverage tests and added a perception test for social-tagged events
- Deviations from original plan:
  - Corrected the ticket assumptions before implementation: the pre-change enum counts were lower than stated, and discovery-emission behavior was not already present
  - Kept scope intentionally narrow: no Tell action, no Discovery event emission, no payload/schema work
- Verification results:
  - `cargo test -p worldwake-core` passed
  - `cargo test -p worldwake-sim` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
