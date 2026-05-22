# S164BELVIEKIN-004: `facility_controller_at` remote-change regression guard

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (test-only, unless the test reveals an actual leak)
**Deps**: None

## Problem

Before this ticket, `facility_controller_at` (`per_agent_belief_view.rs:385-401`) resolved seller/
controller identity by calling `world.can_exercise_control(entity, facility)` for
each agent in `self.entities_at(place)`. The candidate set is belief-filtered (only
agents the observer believes are present), so this is defensible as local observation
of who is staffing a believed-present facility — but it was borderline and
**untested for the remote-control-change case**. This ticket added a confirming test
proving a hidden remote controller does not become the resolved controller for a
distant actor. No production behavior change was required.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `facility_controller_at` (`crates/worldwake-sim/src/per_agent_belief_view.rs:385-401`)
   gates the candidate set through `self.entities_at(place)` (belief-filtered) and the
   per-candidate checks `entity_kind(*entity) == Some(Agent)`, `is_alive`, and
   `world.can_exercise_control(*entity, facility).is_ok()`. The facility location read
   is authoritative (`world.effective_place(facility)`), justified because facilities
   are physical infrastructure always present at their place.
2. No existing focused test isolates the remote-control-change case for this accessor
   (grep of the `#[cfg(test)]` block, `:2295`+, found related last-seen/local-visibility
   tests such as `remote_facility_discovery_requires_believed_entity_snapshot` (`:4075`)
   but none asserting controller stability under a remote control transfer). This is
   missing focused/unit coverage, not missing golden coverage.
3. Boundary under audit: the belief-filtered candidate set vs. the authoritative
   `can_exercise_control` dispatch check among that set. The intended invariant: a
   controller the observer does not believe is present (or a control transfer with no
   carrier reaching the distant actor) must not become the resolved controller/seller.
4. Conditional scope (per spec D4): if the test reveals an actual leak — a controller
   resolved for a non-believed-present agent — gate the `can_exercise_control` probe on
   belief-presence and reclassify this ticket as a behavior fix (Engine Changes: Yes).
   If the test confirms lawfulness, it stands as a regression guard with no production
   change.

## Architecture Check

1. The confirming test pins the borderline accessor's contract at the focused-unit
   layer, where the belief-filtered candidate set and the dispatch check can be
   asserted directly — cleaner than relying on a downstream golden to catch a future
   regression.
2. No backward-compat shim introduced. If a fix becomes necessary, it tightens the
   candidate gate (belief-presence) rather than adding an alias path.

## Verified Layers

1. Controller stability under remote control change → focused unit test on
   `facility_controller_at`: transfer control of a facility to an agent the distant
   observer does not believe is present; assert the resolved controller is unchanged
   (the believed-present staff, or `None`).
2. Single-layer ticket (belief-view accessor focused coverage): no action-trace or
   event-log surface applies; the authoritative control transfer is set up directly in
   the test world, and the accessor result is the contract under test.

## Landed Changes

### 1. Added the confirming focused test

In `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]`, added
`remote_facility_controller_change_without_carrier_stays_hidden`. It sets up a
remote facility known to the observer, keeps the authoritative hidden controller
outside the observer's believed-present entity set, transfers authoritative control
to that hidden controller with no carrier, and asserts `facility_controller_at`
still resolves `None` rather than the hidden remote controller.

### 2. Production accessor unchanged

The focused test confirmed the existing candidate-set gate: `facility_controller_at`
only evaluates authoritative control for entities returned by the observer's
belief-filtered `entities_at(place)` set. No production change was required.

## Landed Files

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modified — added focused
  `#[cfg(test)]` regression guard only)

## Out of Scope

- `entity_kind`, the last-seen carrier, and the bandit gate (tickets 001/002/003).
- Any production behavior change; the focused test passed against the existing
  belief-filtered candidate set.

## Acceptance Result

### Passed Criteria

1. A hidden remote controller with live authoritative control does not become
   `facility_controller_at`'s resolved controller for a distant observer.
2. Existing `worldwake-sim` tests passed.

### Verified Invariants

1. The resolved controller is drawn only from agents the observer believes are present
   at the facility's place.
2. No omniscient resolution of a control transfer the observer has no carrier for.

## Test Plan Result

### Added Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — new focused test for
   controller stability under remote control change.

### Commands Run

1. Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::remote_facility_controller_change_without_carrier_stays_hidden -- --exact`.
2. Passed `cargo test -p worldwake-sim`.
3. Passed `./scripts/verify.sh`.

## Outcome

Completed on 2026-05-22.

- Added a focused `PerAgentBeliefView` regression guard for `facility_controller_at`
  proving an authoritative controller hidden from the observer's believed-present
  set does not surface as a remote seller/controller.
- The test confirmed the existing belief-filtered candidate boundary, so no
  production behavior change was needed.

## Verification Result

- Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::remote_facility_controller_change_without_carrier_stays_hidden -- --exact`.
- Passed `cargo test -p worldwake-sim`.
- Passed `./scripts/verify.sh`.
