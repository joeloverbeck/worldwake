# S173SELCARINT-004: Wash + Toilet interruption contract (start/commit/abort + reservation)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `reservation_requirements` on wash/toilet action defs; new `abort_release_self_care_occupancy` handler; `SelfCareOccupancy` writes/removes in start, commit, abort flows
**Deps**: S173SELCARINT-001 (uses `SelfCareOccupancy`, `SelfCareUseKind`), S173SELCARINT-002 (uses `ActionTraceDetail::SelfCareInterrupted`), S173SELCARINT-003 (`PromotableContentionKind::SelfCareWash`/`SelfCareLatrine` queue classification), `specs/S173-self-care-interruption-occupancy.md` (D2 wash/toilet rows, D4, distributed D5 start-gate read)

## Problem

Wash and Toilet actions register with `reservation_requirements: Vec::new()` (`crates/worldwake-systems/src/needs_actions.rs` registration block at L23-58, via the shared `register_def` helper at L141-198 with default empty `reservation_requirements` at L170). Their abort handler is `abort_noop` (L51 wash, L45 toilet). Two consequences: (a) two co-located dirty agents can both start a Wash action on the same basin in the same tick because nothing gates the start, and (b) when a Wash is aborted, no facility state is released and no structured trace records "this agent was washing at this basin when interrupted". This ticket wires the full self-care interruption contract for wash and toilet: start writes `SelfCareOccupancy`, commit removes it (in `commit_wash`/`commit_toilet`), the new `abort_release_self_care_occupancy` handler replaces `abort_noop` and both removes the occupancy and populates `ActionTraceDetail::SelfCareInterrupted` on the action-trace event.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing tests in `crates/worldwake-systems/src/needs_actions.rs` (#[cfg(test)] from L1058) that exercise wash/toilet behavior — these must continue to pass without scope creep:
   - Toilet: `toilet_reduces_bladder_and_creates_waste:1709`, `toilet_overflow_emits_waste_created_with_overcapacity_source:1791`, `toilet_under_threshold_does_not_emit_waste_created:1851`, `toilet_already_over_threshold_emits_waste_created_each_tick:1880`, `toilet_latrine_fullness_saturates_at_max:1927`, `toilet_affordance_requires_latrine_tagged_place:1955`
   - Wash: `wash_full_success_consumes_basin_water_and_clears_dirtiness:1983`, `wash_partial_success_when_basin_water_below_full_wash:2042`, `wash_rejects_basin_with_zero_clean_water:2209`, `wash_accepts_basin_with_sufficient_clean_water:2222`, `wash_race_condition_basin_emptied_between_affordance_and_start_returns_precondition_failed:2235`
   - Registration: `register_needs_actions_adds_all_six_defs_and_handlers:1299`
2. `register_def` helper at L141-198 currently defaults `reservation_requirements: Vec::new()` for all six self-care actions. Either pass a `reservation_requirements` parameter into `register_def` or call sites override the field per-action. Verify the helper signature at implementation time — the simplest path is likely a per-action `register_def_with_reservation` variant or threading `reservation_requirements` through the helper. Document the choice in the implementation.
3. `commit_wash` and `commit_toilet` exist in `needs_actions.rs` — confirmed by the existing test `wash_full_success_consumes_basin_water_and_clears_dirtiness` (which asserts commit-side state changes) and the commit handler registration at L37-39 (wash) and L41-43 (toilet). The commit functions need a new step: `remove_self_care_occupancy(facility_id)` (or the corresponding `set_component_self_care_occupancy(None)` via the macro-generated accessor).
4. `abort_sleep_episode` (`needs_actions.rs:552-567`) is the precedent abort handler that does meaningful cleanup work. New abort handler `abort_release_self_care_occupancy` follows its signature (`ActionDef`, `ActionInstance`, `ActionExecutionContext`, `AbortReason`, `EventLog`, `DeterministicRng`, `WorldTxn`) but performs occupancy removal + trace-detail population.
5. Shared abstraction boundary: the action start-gate path. Reservation requirements are checked during start; if the target facility carries `SelfCareOccupancy`, start fails with `ActionError::PreconditionFailed`. The planner then replans next tick via existing `handle_plan_failure` machinery. This is the FND-14A/14B-split start-gate revalidation path for the actor reading the facility's `SelfCareOccupancy` directly (FND-14A — actor is co-located with the facility at start time).
6. For start-gate occupancy read: actor and target facility must be co-located at action start (action-start invariant: `TargetSpec::EntityAtActorPlace` for wash and toilet). Reading the facility's `SelfCareOccupancy` from world state at start is FND-14A-compliant (same-tick co-located physical observation).
7. Distributed deliverable D5: this ticket covers the start-gate read side (FND-14A co-located observation at action start). The emitter-time read (FND-14B belief-backed for remote candidates) lands in ticket 006. Both share the source-class table from spec D5 but are implemented at different layers.
8. Authoritative-to-AI Impact Rule applies (per spec's Authoritative-to-AI Impact Analysis section, points 1, 4, 5, 6): adding a reservation requirement modifies action preconditions. `BestEffort` action start now has a new failure mode (`SelfCareOccupancy` present at start time → precondition rejection); `handle_plan_failure` must replan correctly when the new rejection fires. Verify at implementation that the existing replan machinery covers this case without special-casing.

## Architecture Check

1. Single source of truth for occupancy: the `SelfCareOccupancy` component on the facility/place. Start writes it; commit and abort remove it. No parallel intent-tracking, no planner-reserved "soft hold" — intent is not entitlement (FND-21).
2. FND-28-driven combining: this ticket bundles D2's wash/toilet rows + D4's reservation requirements because splitting them would leave a transient intermediate state (e.g., start writes occupancy but commit doesn't remove it → leaked occupancy that never clears). The combined ticket guarantees the workspace compiles AND the live authority path is consistent at every commit point.
3. The new `abort_release_self_care_occupancy` is a fresh handler, not a wrapper or shim around `abort_noop`. Per FND-28 there is no parallel "old" path retained.

## Verification Layers

1. Start-gate reservation enforcement → focused authoritative runtime test: two agents attempt wash on the same basin same tick; only one succeeds. Use the existing `wash_race_condition_*` test (L2235) as the structural precedent.
2. Commit-side occupancy removal → focused unit test on `commit_wash` and `commit_toilet`: assert `SelfCareOccupancy` is removed from the target facility after a successful commit. Verifiable via authoritative world-state read.
3. Abort-side occupancy removal → focused unit test on `abort_release_self_care_occupancy`: instantiate a wash with `SelfCareOccupancy` written, fire abort, assert the component is removed and `ActionTraceEvent.detail` carries `Some(ActionTraceDetail::SelfCareInterrupted { kind: Wash, basin: Some(basin_id) })`.
4. Authoritative event-log surface unchanged: `EventTag::ActionAborted` continues to fire from the engine (`tick_action.rs:96, 188, 220, 265` / `interrupt_abort.rs:147`). No new `EventTag` variant added. → event-log delta assertion in scenarios.
5. Cross-system trace mapping: action-trace (`detail` field) carries the typed payload; event-log carries the generic `ActionAborted` record. The two layers are distinct proof surfaces per `docs/precision-rules.md` Rule 5.

## What to Change

### 1. Extend `register_def` (or equivalent) to accept reservation requirements for wash and toilet

In `crates/worldwake-systems/src/needs_actions.rs`, modify the registration call sites for `wash` and `toilet` so they pass a non-empty `reservation_requirements` Vec. The reservation requirement gates start on the target facility being reservable (no current `SelfCareOccupancy`). The exact shape of the reservation requirement entry depends on the existing `ReservationRequirement` struct/enum — verify at implementation time. Likely shape: a per-target check function `|world, target| world.get_component_self_care_occupancy(target).is_none()`, or a declarative tag matching the existing reservation-requirement vocabulary.

Eat, drink, sleep, and relieve_wilderness registrations retain `reservation_requirements: Vec::new()` — none of those actions write occupancy.

### 2. Modify `commit_wash` and `commit_toilet` to remove occupancy

In `commit_wash` and `commit_toilet` (existing handlers in `needs_actions.rs`), add a step to remove `SelfCareOccupancy` from the target facility/place via the macro-generated `remove_self_care_occupancy(target)` or `set_component_self_care_occupancy(target, None)` accessor (verify exact name at implementation time per ticket 001's generated accessors). Place the removal alongside the existing commit-side state mutations (water consumption, dirtiness reduction, latrine fullness update).

### 3. Implement `abort_release_self_care_occupancy` handler

Add a new handler function in `needs_actions.rs` following the `abort_sleep_episode` signature pattern (L552-567):

```rust
fn abort_release_self_care_occupancy(
    _def: &ActionDef,
    instance: &ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    // 1. Identify the target facility/place from instance.targets[0].
    // 2. Read SelfCareUseKind from the facility's SelfCareOccupancy (if present)
    //    OR infer from the action def name (wash → Wash; toilet → LatrineRelief).
    //    The latter is more robust if the occupancy was already removed by a
    //    prior step in the abort flow.
    // 3. Remove the SelfCareOccupancy component from the target.
    // 4. Populate context.action_trace_sink (or equivalent) with
    //    ActionTraceEvent { detail: Some(ActionTraceDetail::SelfCareInterrupted { kind, basin: Some(target) }), ... }
    //    The exact trace-sink emission API is verified at implementation time
    //    via the ActionExecutionContext fields.
    Ok(())
}
```

Verify at implementation time how trace-sink writes from inside an abort handler are wired — `ActionExecutionContext` may carry the sink directly or via an intermediate dispatcher.

### 4. Wire the new abort handler into wash and toilet registrations

In `needs_actions.rs:35-46`, replace `abort_noop` with `abort_release_self_care_occupancy` on the wash and toilet `ActionHandler::new(...)` calls. Leave eat (L23-28), drink (L29-34), and relieve_wilderness (L53-58) with `abort_noop` for now — ticket 005 replaces those.

### 5. Update `register_needs_actions_adds_all_six_defs_and_handlers` test

The existing registration test at L1299 asserts handler identity for all six actions. If wash/toilet abort handlers are now `abort_release_self_care_occupancy` and eat/drink/wilderness retain `abort_noop`, the assertion shape changes — verify and update at implementation time.

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modify — handler additions, registration changes, commit-side occupancy removal, possibly `register_def` helper extension)

If reservation-requirement plumbing requires extending shared sim infrastructure (e.g., the `ReservationRequirement` type lives in `worldwake-sim`), additional sim-crate files may be touched — verify at implementation time. The likely path is that `ReservationRequirement` already supports a closure-based check that reads facility state; no sim-crate change needed.

## Out of Scope

- Atomic-action abort handlers (eat, drink, relieve_wilderness, sleep enrichment) — owned by ticket 005.
- Candidate-emitter occupancy filtering (emitter-time read of `SelfCareOccupancy`) — owned by ticket 006.
- `PromotableContentionKind` extension — owned by ticket 003 (prerequisite).
- Belief-view accessor for `SelfCareOccupancy` if needed for remote queries — verified by ticket 006; this ticket reads only co-located occupancy at start time (FND-14A path).
- Scenario goldens — owned by ticket 007 (A/B/C), 008 (D), 009 (E).
- New `EventTag` variant — explicitly rejected per spec Non-Goals; `EventTag::ActionAborted` reused.

## Acceptance Criteria

### Tests That Must Pass

1. New unit test: `wash_start_writes_self_care_occupancy_on_basin` — start a wash, assert `SelfCareOccupancy { use_kind: Wash, occupant: agent, started_tick, goal_key }` is present on the target facility.
2. New unit test: `wash_commit_removes_self_care_occupancy` — full commit removes the component.
3. New unit test: `wash_abort_releases_occupancy_and_populates_trace_detail` — abort removes the component AND `ActionTraceDetail::SelfCareInterrupted { kind: Wash, basin: Some(basin_id) }` is captured in the trace sink.
4. New unit test: `wash_start_fails_when_basin_already_occupied` — pre-write `SelfCareOccupancy` on a basin, attempt a second wash on the same basin, assert `ActionError::PreconditionFailed` and no second occupancy written.
5. Symmetric tests for toilet on a latrine-tagged Place.
6. All existing wash/toilet tests pass: `toilet_reduces_bladder_and_creates_waste`, `wash_full_success_consumes_basin_water_and_clears_dirtiness`, `wash_partial_success_when_basin_water_below_full_wash`, etc. (named in Assumption Reassessment item 1).
7. `register_needs_actions_adds_all_six_defs_and_handlers` — updated to expect `abort_release_self_care_occupancy` for wash/toilet and `abort_noop` (unchanged) for the others.
8. Existing suite: `cargo test -p worldwake-systems needs_actions`.

### Invariants

1. After a successful wash commit, no `SelfCareOccupancy` remains on the basin facility.
2. After a wash abort, no `SelfCareOccupancy` remains on the basin facility, AND the action-trace sink carries a `SelfCareInterrupted` detail entry with the correct `kind` and `basin`.
3. Two co-located agents cannot both start a wash on the same basin in the same tick — exactly one succeeds; the other fails at the start-gate with `ActionError::PreconditionFailed`.
4. Symmetric invariants for toilet on a latrine-tagged Place.
5. Eat, drink, sleep, and relieve_wilderness abort behavior is unchanged by this ticket (verified by existing tests passing).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` inline tests (existing `#[cfg(test)]` from L1058) — 6 new tests (5 from Acceptance Criteria plus the registration-test update).

### Commands

1. `cargo test -p worldwake-systems needs_actions`
2. `cargo test -p worldwake-sim --test save_load` (sanity: SAVE_FORMAT_VERSION 107 round-trips with `SelfCareOccupancy` instances created by wash/toilet)
3. `cargo build --workspace -- -D warnings`
4. `./scripts/verify.sh` before commit.
