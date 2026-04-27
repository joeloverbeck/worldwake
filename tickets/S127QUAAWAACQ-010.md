# S127QUAAWAACQ-010: Integrate `ResourceExtractionQueues` with AI blocker-clearing

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: `worldwake-sim` (belief-view extension), `worldwake-ai` (clearing baseline)
**Deps**: S127QUAAWAACQ-008

## Problem

When an agent's harvest start fails with `extraction_slots_full`, `record_harvest_start_failure` enqueues the actor in the new per-slot `ResourceExtractionQueues` substrate, AND the AI's `failure_handling::record_failure_classification` writes a `BlockingFact::ReservationConflict` blocker with clearing condition `BlockerClearingCondition::ContentionChanged { facility }` and baseline `ClearingBaseline::ContentionPosition(view.facility_queue_position(facility, agent))`.

The clearing baseline is read via `PerAgentBeliefView::facility_queue_position` (`crates/worldwake-sim/src/per_agent_belief_view.rs:814`), which inspects the legacy per-facility `ContentionQueue` component — not the new `ResourceExtractionQueues`. Since harvest workstations only carry `ResourceExtractionQueues` (not `ContentionQueue`), `facility_queue_position` always returns `None` for both baseline and current observations, so `is_blocker_cleared` never detects a position change. The blocker only expires via TTL (`CognitiveProfile.transient_block_ticks`, default 20).

This means FOUNDATIONS Section VI Scenario E ("Competing Claimants → Queue or Race → Expiry/Prune → Next Actor Acts") only resolves through TTL today — when a slot frees, the next queued actor cannot replan into it immediately even though the world state has changed. S127QUAAWAACQ-008 Golden 5 (`golden_scenario_e_queue_abandonment_promotes_next_actor`) had to set `transient_block_ticks: 2` on test agents to land within a small tick budget, which is acceptable for goldens but masks the production behavior gap.

## Architecture Check

1. The split between extraction-state (`ResourceSource`) and reservation-state (`ResourceExtractionQueues`) on the source entity is correct per FND-26 (per spec D6 design intent).
2. The AI's blocker-clearing surface (`PerAgentBeliefView` accessors) needs to also read from `ResourceExtractionQueues` so it can detect grant-state changes and queue-position changes.
3. Two viable shapes for the new accessor:
   - **Option A (extend existing accessors)**: Make `facility_queue_position` and `facility_grant` also inspect `ResourceExtractionQueues` (returning the lowest-index slot's grant/position when no `ContentionQueue` exists).
   - **Option B (parallel accessors)**: Add new `extraction_slot_grant(entity)` and `extraction_slot_queue_position(entity, actor)` methods, and update `derive_clearing_condition` for `ReservationConflict` to use these when the facility has `ResourceExtractionQueues`.
   - Option A is simpler but conflates two distinct semantics; Option B is cleaner but touches more sites. Recommend Option B.

## What to Change

1. Extend `RuntimeBeliefView` / `PerAgentBeliefView` with new accessors that surface `ResourceExtractionQueues` grant-state and position-state to the AI (see Option B above).
2. Update `derive_clearing_condition` in `crates/worldwake-ai/src/failure_handling.rs:929-941` so the `ReservationConflict` baseline reads from the new accessors when the facility carries `ResourceExtractionQueues` rather than `ContentionQueue`.
3. Update `is_blocker_cleared` to compare against the new baseline.

## Out of Scope

- Auto-promotion from waiting list when grant frees (current "next requester wins" semantics is correct per FND).
- Architectural unification of `ContentionQueue` and `ResourceExtractionQueues` — separate work.
- Changes to harvest action handlers themselves.

## Acceptance Criteria

1. After implementation, `golden_scenario_e_queue_abandonment_promotes_next_actor` (in `golden_quantity_aware_acquisition.rs`) and `golden_single_slot_queue_forms_with_concrete_wait` can drop their `transient_block_ticks: 2` overrides and rely on the default profile. Update those goldens in this ticket.
2. Add a focused test in `crates/worldwake-ai/src/failure_handling.rs` proving the `ReservationConflict` blocker clears when the actor's slot position changes in `ResourceExtractionQueues`.
3. `cargo test -p worldwake-ai` passes.
4. `./scripts/verify.sh` passes.

## Test Plan

1. Focused unit tests for the new accessors and the updated `is_blocker_cleared` path.
2. Existing `golden_quantity_aware_acquisition.rs` Goldens 1 and 5 with `transient_block_ticks` overrides removed.
3. `python3 scripts/golden_inventory.py --write --check-docs` — refresh if scenario metadata changes.

## References

- S127QUAAWAACQ-008 Outcome §"Follow-up Gaps Identified" item 2.
- `crates/worldwake-sim/src/per_agent_belief_view.rs:814` — `facility_queue_position` legacy implementation.
- `crates/worldwake-ai/src/failure_handling.rs:929-941` — `derive_clearing_condition` for `ReservationConflict`.
- `crates/worldwake-systems/src/production_actions.rs:927-962` — `record_harvest_start_failure` (the producer of these blockers).
