# S164BELVIEKIN-004: `facility_controller_at` remote-change regression guard

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (test-only, unless the test reveals an actual leak)
**Deps**: None

## Problem

`facility_controller_at` (`per_agent_belief_view.rs:385-401`) resolves seller/
controller identity by calling `world.can_exercise_control(entity, facility)` for
each agent in `self.entities_at(place)`. The candidate set is belief-filtered (only
agents the observer believes are present), so this is defensible as local observation
of who is staffing a believed-present facility — but it is borderline and currently
**untested for the remote-control-change case**. This ticket adds a confirming test
proving a remote control change does not alter the resolved controller for a distant
actor. No behavior change unless the test fails.

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

## Verification Layers

1. Controller stability under remote control change → focused unit test on
   `facility_controller_at`: transfer control of a facility to an agent the distant
   observer does not believe is present; assert the resolved controller is unchanged
   (the believed-present staff, or `None`).
2. Single-layer ticket (belief-view accessor focused coverage): no action-trace or
   event-log surface applies; the authoritative control transfer is set up directly in
   the test world, and the accessor result is the contract under test.

## What to Change

### 1. Add the confirming focused test

In `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]`, add a test that
sets up a facility with a believed-present controller for the observer, then transfers
`can_exercise_control` to a remote agent the observer does not believe is present (no
carrier). Assert `facility_controller_at` resolves the believed-present controller (or
`None`), not the remote one.

### 2. (Conditional) Gate the probe on belief-presence

Only if the test fails: restrict the `can_exercise_control` probe to agents the
observer believes are present, and document the tightening here.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — new `#[cfg(test)]` test; conditionally `:385-401`)

## Out of Scope

- `entity_kind`, the last-seen carrier, and the bandit gate (tickets 001/002/003).
- Any behavior change unless the new test fails.

## Acceptance Criteria

### Tests That Must Pass

1. A remote control change with no carrier does not alter `facility_controller_at`'s
   resolved controller for a distant observer.
2. Existing suite: `cargo test -p worldwake-sim`.

### Invariants

1. The resolved controller is drawn only from agents the observer believes are present
   at the facility's place.
2. No omniscient resolution of a control transfer the observer has no carrier for.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — new focused test for
   controller stability under remote control change.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-sim`
3. `./scripts/verify.sh`
