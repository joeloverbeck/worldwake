# S122FRAASSCOM-001: Commodity-availability evaluator substrate

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — adds `IntentionFrame::expected_commodity` derived helper in `worldwake-core`; adds `assess_commodity_availability` private free function and `AvailabilityVerdict` enum in `worldwake-ai/src/agent_tick/frame.rs`. No new component, no new trait method.
**Deps**: specs/S122-frame-assumption-commodity-availability.md

## Problem

S122 closes the FND-21 / FND-15 / FND-17 gap that prevents agents from revising commodity-acquisition intentions when local observation contradicts belief. The end-to-end wiring (population in `populate_assumptions`, evaluation in `evaluate_assumptions`, trace surfacing) all depends on two pure-read substrate pieces: (a) a way to derive the goal-implied `(commodity, place)` pair from a frame, and (b) a way to assess whether the agent believes that commodity is accessible at that place. This ticket lands both with no consumers yet, so the wiring tickets (002–004) can focus on integration without redoing helper design.

## Assumption Reassessment (2026-04-20)

1. Existing focused/unit coverage in `crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]`: 5 `populate_*` tests (lines 768/788/813/844/864), 6 `evaluate_assumptions` exercises (lines 879/893/909/929/944/952), 1 `commodity_available_at_stubbed_as_pass` (line 949 — slated for deletion in S122FRAASSCOM-003). The helpers added by this ticket are net-new — no existing test covers them. Eight new unit tests cover the four verdict cases plus the three `expected_commodity` cases.
2. Spec deliverables D1 + D4 defined in `specs/S122-frame-assumption-commodity-availability.md` (D1 at lines 86–114, D4 at lines 167–194).
3. Shared abstraction boundary under audit: `IntentionFrame` in `crates/worldwake-core/src/intention_frame.rs` gains a derived accessor; `RuntimeBeliefView` in `crates/worldwake-sim/src/belief_view.rs` is consumed via `&dyn RuntimeBeliefView` without a trait modification. The boundary is the new helper signatures, both of which are pure-read.
6. Intended layer: AI / belief-view / planning-layer logic. Local needs-only harness is sufficient — the helpers are pure-read against a mock view (`MockBeliefView` already present in `agent_tick/frame.rs#[cfg(test)]`).

## Architecture Check

1. The free function in `agent_tick/frame.rs` composes existing trait methods (`SpatialBeliefView::effective_place`, `SpatialBeliefView::entities_at`, `InventoryBeliefView::item_lot_commodity`, `FacilityBeliefView::resource_source`, `FacilityBeliefView::resource_sources_at`, `SocialBeliefView::agent_belief_store`) without adding a new trait method. This avoids the cross-trait composition issue an `EconomicBeliefView` default method would face — `EconomicBeliefView` lacks `FacilityBeliefView`/`SpatialBeliefView`/`SocialBeliefView` supertrait bounds. It also avoids the "default returns false" anti-pattern that would force every test mock to opt in.
2. `IntentionFrame::expected_commodity` reads only stored fields (`self.domain`, `self.goal.kind`). No backwards-compatibility shim; it is a net-new accessor on an existing type.

## Verification Layers

1. `expected_commodity` correctness for Travel/Errand + AcquireCommodity → `Some((commodity, destination))`, other domain/goal combos → `None` -> focused unit tests in `crates/worldwake-core/src/intention_frame.rs#[cfg(test)]`.
2. `assess_commodity_availability` verdict correctness across (co-located item lot, co-located resource source, co-located no-match, not-co-located belief-backed lot, no belief) -> focused unit tests in `crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]` extending the existing `MockBeliefView`.
6. Single-layer ticket — pure-read helpers, no action lifecycle, no event-log delta. Authoritative-state mapping is not applicable; the helpers do not mutate.

## What to Change

### 1. Add `IntentionFrame::expected_commodity` derived helper

- File: `crates/worldwake-core/src/intention_frame.rs`
- Add `GoalKind` to the existing `use crate::{...}` line (alongside `GoalKey`).
- Add an `impl IntentionFrame { ... }` block with:

  ```rust
  pub fn expected_commodity(&self) -> Option<(CommodityKind, EntityId)> {
      let destination = match self.domain {
          IntentionDomain::Travel { destination }
          | IntentionDomain::Errand { destination } => destination,
          _ => return None,
      };
      match self.goal.kind {
          GoalKind::AcquireCommodity { commodity, .. } => Some((commodity, destination)),
          _ => None,
      }
  }
  ```

### 2. Add private `AvailabilityVerdict` enum and `assess_commodity_availability` free function

- File: `crates/worldwake-ai/src/agent_tick/frame.rs`
- Add (above `populate_assumptions`):

  ```rust
  #[derive(Debug, Eq, PartialEq)]
  pub(super) enum AvailabilityVerdict {
      Believed,
      Refuted,
      UnknownOrStale,
  }

  pub(super) fn assess_commodity_availability(
      view: &dyn RuntimeBeliefView,
      agent: EntityId,
      commodity: CommodityKind,
      place: EntityId,
  ) -> AvailabilityVerdict { /* see logic below */ }
  ```

- Co-located case (FND-14A): if `view.effective_place(agent) == Some(place)`, iterate `view.entities_at(place)`. For each entity, accept if either `view.item_lot_commodity(entity) == Some(commodity)` (open ground lot) OR `view.resource_source(entity).map(|s| (s.commodity, s.available_quantity > Quantity(0))) == Some((commodity, true))` (viable resource source). At least one match → `Believed`. Co-located AND no match → `Refuted`.
- Not-co-located case: read `view.agent_belief_store(agent)?.known_entities`. For each `(_id, BelievedEntityState)` where `state.last_known_place == Some(place)`, accept if either (a) `state.resource_source.as_ref().map(|s| (s.commodity, s.available_quantity > Quantity(0))) == Some((commodity, true))` OR (b) `state.last_known_inventory.get(&commodity).copied().unwrap_or(Quantity(0)) > Quantity(0)`. At least one supporting entry → `Believed`. Place has belief entries but none supports → `UnknownOrStale`. No belief entries about the place at all → `UnknownOrStale`.
- Confidence gating via `claim_confidence_threshold` is intentionally deferred (see Out of Scope).

## Files to Touch

- `crates/worldwake-core/src/intention_frame.rs` (modify — import `GoalKind`, add `expected_commodity` impl, add unit tests)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — add `AvailabilityVerdict` enum and `assess_commodity_availability` function, add unit tests)

## Out of Scope

- Wiring the helpers into `populate_assumptions` (S122FRAASSCOM-002) or `evaluate_assumptions` (S122FRAASSCOM-003).
- Confidence gating on belief-backed reads via `claim_confidence_threshold`. The initial implementation accepts any matching `BelievedEntityState` regardless of per-claim confidence; freshness/confidence refinement is future S113-aware work.
- Container-stored or seller-listed commodities (spec Non-Goals).
- Modifying any belief-view trait definition. No new trait method on `EconomicBeliefView`, `FacilityBeliefView`, or `RuntimeBeliefView`.

## Acceptance Criteria

### Tests That Must Pass

1. New unit test `expected_commodity_returns_pair_for_travel_and_acquire_goal` — Travel domain + AcquireCommodity goal returns `Some((commodity, destination))`.
2. New unit test `expected_commodity_returns_none_for_non_acquisition_goal` — Travel + Sleep returns `None`.
3. New unit test `expected_commodity_returns_none_for_non_travel_domain` — Care/Escort/Generic + AcquireCommodity returns `None`.
4. New unit test `assess_commodity_availability_co_located_lot_returns_believed` — agent at place, item lot of commodity present.
5. New unit test `assess_commodity_availability_co_located_resource_source_returns_believed` — agent at place, viable resource source for commodity present.
6. New unit test `assess_commodity_availability_co_located_no_match_returns_refuted` — agent at place, no lot/source for commodity.
7. New unit test `assess_commodity_availability_belief_backed_lot_returns_believed` — agent not co-located, belief store has entry with `last_known_inventory[commodity] > 0`.
8. New unit test `assess_commodity_availability_no_belief_returns_unknown_or_stale` — agent not co-located, no belief about place.
9. Existing suite: `cargo test -p worldwake-core --lib intention_frame` and `cargo test -p worldwake-ai --lib agent_tick::frame` pass.

### Invariants

1. `expected_commodity` reads only stored fields (`self.domain`, `self.goal.kind`); no view dependency, no I/O. (FND-27.)
2. `assess_commodity_availability` reads only the agent's own belief store (`agent_belief_store(agent)`) and FND-14A co-located perception (`entities_at`, `item_lot_commodity`, `resource_source`). It does not read entities not at the agent's current place from authoritative state. (FND-14, FND-14A.)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/intention_frame.rs#[cfg(test)]` — three `expected_commodity_*` unit tests covering the Travel+AcquireCommodity, Travel+other-goal, and non-Travel domain cases.
2. `crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]` — five `assess_commodity_availability_*` unit tests, extending `MockBeliefView` to populate `agent_belief_store`, `entities_at`, `item_lot_commodity`, and `resource_source` accessors as needed for each case.

### Commands

1. `cargo test -p worldwake-core --lib intention_frame`
2. `cargo test -p worldwake-ai --lib agent_tick::frame`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-20.

- Added `IntentionFrame::expected_commodity()` in `crates/worldwake-core/src/intention_frame.rs` as a pure derived helper over `self.domain` and `self.goal.kind`, with focused unit coverage for acquisition vs. non-acquisition and travel vs. non-travel domains.
- Added `AvailabilityVerdict` plus the private `assess_commodity_availability()` helper in `crates/worldwake-ai/src/agent_tick/frame.rs`, with focused helper tests covering co-located lot, co-located resource source, co-located refutation, remote belief-backed inventory, and no-belief cases.
- Kept `populate_assumptions` and `evaluate_assumptions` wiring unchanged in this ticket. That split already belongs to the active sibling tickets `S122FRAASSCOM-002` and `S122FRAASSCOM-003`, so `001` stayed substrate-only as intended.

## Deviations

- Added `#[cfg_attr(not(test), allow(dead_code))]` on the staged `AvailabilityVerdict` enum and `assess_commodity_availability()` helper so `cargo clippy --workspace --all-targets -- -D warnings` stays green before the sibling integration tickets make the new substrate live in production code.
- The active ticket file remains untracked on this branch; close-out evidence for it does not appear in ordinary tracked-file git diffs.

## Verification Result

- Passed `cargo test -p worldwake-core --lib intention_frame`
- Passed `cargo test -p worldwake-ai --lib agent_tick::frame`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
