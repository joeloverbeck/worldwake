# S173SELCARINT-004: Wash + Toilet interruption contract (start/commit/abort + reservation)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `reservation_requirements` on wash/toilet action defs; new `abort_release_self_care_occupancy` handler; `SelfCareOccupancy` writes/removes in start, commit, abort flows
**Deps**: `archive/tickets/S173SELCARINT-001.md` (uses `SelfCareOccupancy`, `SelfCareUseKind`), `archive/tickets/S173SELCARINT-002.md` (uses `ActionTraceDetail::SelfCareInterrupted`), `archive/tickets/S173SELCARINT-003.md` (`PromotableContentionKind::SelfCareWash`/`SelfCareLatrine` queue classification), `archive/specs/S173-self-care-interruption-occupancy.md` (D2 wash/toilet rows, D4, distributed D5 start-gate read)

## Problem

Wash and Toilet actions register with `reservation_requirements: Vec::new()` (`crates/worldwake-systems/src/needs_actions.rs` registration block at L23-58, via the shared `register_def` helper at L141-198 with default empty `reservation_requirements` at L170). Their abort handler is `abort_noop` (L51 wash, L45 toilet). Two consequences: (a) two co-located dirty agents can both start a Wash action on the same basin in the same tick because nothing gates the start, and (b) when a Wash is aborted, no facility state is released and no structured trace records "this agent was washing at this basin when interrupted". This ticket wires the full self-care interruption contract for wash and toilet: start writes `SelfCareOccupancy`, commit removes it (in `commit_wash`/`commit_toilet`), the new `abort_release_self_care_occupancy` handler replaces `abort_noop` and both removes the occupancy and populates `ActionTraceDetail::SelfCareInterrupted` on the action-trace event.

## Assumption Reassessment (2026-05-26)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing tests in `crates/worldwake-systems/src/needs_actions.rs` (#[cfg(test)] from L1058) that exercise wash/toilet behavior — these must continue to pass without scope creep:
   - Toilet: `toilet_reduces_bladder_and_creates_waste:1709`, `toilet_overflow_emits_waste_created_with_overcapacity_source:1791`, `toilet_under_threshold_does_not_emit_waste_created:1851`, `toilet_already_over_threshold_emits_waste_created_each_tick:1880`, `toilet_latrine_fullness_saturates_at_max:1927`, `toilet_affordance_requires_latrine_tagged_place:1955`
   - Wash: `wash_full_success_consumes_basin_water_and_clears_dirtiness:1983`, `wash_partial_success_when_basin_water_below_full_wash:2042`, `wash_rejects_basin_with_zero_clean_water:2209`, `wash_accepts_basin_with_sufficient_clean_water:2222`, `wash_race_condition_basin_emptied_between_affordance_and_start_returns_precondition_failed:2235`
   - Registration: `register_needs_actions_adds_all_six_defs_and_handlers:1299`
2. `register_def` helper now threads a `reservation_requirements: Vec<ReservationReq>` argument. Wash passes `ReservationReq { target_index: 0 }` for the basin target. Toilet now binds `TargetSpec::ActorPlace` and passes `ReservationReq { target_index: 0 }` so the latrine place can be reserved and occupied directly.
3. `commit_wash` and `commit_toilet` exist in `needs_actions.rs` and now call `clear_component_self_care_occupancy` after successful effect-schema application. The generated accessor name is `clear_component_self_care_occupancy`, not the drafted `remove_self_care_occupancy` sketch.
4. `abort_sleep_episode` remains the precedent for handler-owned cleanup. The new `abort_release_self_care_occupancy` follows the same abort-handler signature but only clears occupancy. Live reassessment corrected the drafted trace-detail placement: `ActionExecutionContext` has no action-trace sink, so abort trace detail is populated in `crates/worldwake-sim/src/tick_step.rs::abort_trace_detail_for_instance` when the existing action-trace emission records an abort.
5. Shared abstraction boundary: the action start-gate path plus handler start callback. `ReservationReq` reserves the target during the action lifetime; `start_self_care_occupancy` rejects a target that already carries `SelfCareOccupancy` and writes the component otherwise. The planner then replans next tick via existing start-failure machinery. This is the FND-14A start-gate revalidation path for co-located self-care targets.
6. For start-gate occupancy read: Wash uses `TargetSpec::EntityAtActorPlace`; Toilet uses `TargetSpec::ActorPlace`. Reading the target's `SelfCareOccupancy` from world state at start is FND-14A-compliant because the actor is at that facility/place.
7. Distributed deliverable D5: this ticket covers the start-gate read side (FND-14A co-located observation at action start). The emitter-time read (FND-14B belief-backed for remote candidates) lands in ticket 006. Both share the source-class table from spec D5 but are implemented at different layers.
8. Authoritative-to-AI Impact Rule applies (per spec's Authoritative-to-AI Impact Analysis section, points 1, 4, 5, 6): adding a reservation requirement modifies action preconditions. `BestEffort` action start now has a new failure mode (`SelfCareOccupancy` present at start time → precondition rejection); `handle_plan_failure` must replan correctly when the new rejection fires. Verify at implementation that the existing replan machinery covers this case without special-casing.

## Architecture Check

1. Single source of truth for occupancy: the `SelfCareOccupancy` component on the facility/place. Start writes it; commit and abort remove it. No parallel intent-tracking, no planner-reserved "soft hold" — intent is not entitlement (FND-21).
2. FND-28-driven combining: this ticket bundles D2's wash/toilet rows + D4's reservation requirements because splitting them would leave a transient intermediate state (e.g., start writes occupancy but commit doesn't remove it → leaked occupancy that never clears). The combined ticket guarantees the workspace compiles AND the live authority path is consistent at every commit point.
3. The new `abort_release_self_care_occupancy` is a fresh handler, not a wrapper or shim around `abort_noop`. Per FND-28 there is no parallel "old" path retained.

## Verified Layers

1. Start-gate reservation enforcement → focused authoritative runtime test: two agents attempt wash on the same basin same tick; only one succeeds. Use the existing `wash_race_condition_*` test (L2235) as the structural precedent.
2. Commit-side occupancy removal → focused unit test on `commit_wash` and `commit_toilet`: assert `SelfCareOccupancy` is removed from the target facility after a successful commit. Verifiable via authoritative world-state read.
3. Abort-side occupancy removal → focused unit test on `abort_release_self_care_occupancy`: instantiate a wash with `SelfCareOccupancy` written, fire abort, assert the component is removed and `ActionTraceEvent.detail` carries `Some(ActionTraceDetail::SelfCareInterrupted { kind: Wash, basin: Some(basin_id) })`.
4. Authoritative event-log surface unchanged: `EventTag::ActionAborted` continues to fire from the engine (`tick_action.rs:96, 188, 220, 265` / `interrupt_abort.rs:147`). No new `EventTag` variant added. → event-log delta assertion in scenarios.
5. Cross-system trace mapping: action-trace (`detail` field) carries the typed payload; event-log carries the generic `ActionAborted` record. The two layers are distinct proof surfaces per `docs/precision-rules.md` Rule 5.

## Landed Changes

### 1. Extend `register_def` (or equivalent) to accept reservation requirements for wash and toilet

In `crates/worldwake-systems/src/needs_actions.rs`, the `register_def` helper now accepts `reservation_requirements`. Wash and Toilet pass one `ReservationReq { target_index: 0 }`. Wash already targeted the basin; Toilet now binds the current latrine place with `TargetSpec::ActorPlace` and uses `BindingStrictness::AnyLegalTarget`.

Eat, drink, sleep, and relieve_wilderness registrations retain `reservation_requirements: Vec::new()` — none of those actions write occupancy.

### 2. Modify `commit_wash` and `commit_toilet` to remove occupancy

`commit_wash` and `commit_toilet` now clear `SelfCareOccupancy` from the basin/latrine place after successful commit effects.

### 3. Implement `abort_release_self_care_occupancy` handler

Added `abort_release_self_care_occupancy` in `needs_actions.rs` following the `abort_sleep_episode` cleanup pattern:

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
    // Identifies the wash target or latrine place and clears SelfCareOccupancy.
    Ok(())
}
```

The trace-sink write is not owned by the systems handler because the live `ActionExecutionContext` carries no sink. The typed abort detail is attached by `tick_step::abort_trace_detail_for_instance` at the existing action-trace emission boundary for cancel, tick abort, and dead-actor abort paths.

### 4. Wire the new abort handler into wash and toilet registrations

`abort_noop` was replaced with `abort_release_self_care_occupancy` on the Wash and Toilet handler registrations. Eat, Drink, and Relieve Wilderness still use `abort_noop` and remain ticket 005's owner.

### 5. Update `register_needs_actions_adds_all_six_defs_and_handlers` test

`register_needs_actions_adds_all_six_defs_and_handlers` now asserts Wash and Toilet reservation requirements and Toilet's `ActorPlace` target.

## Landed Files

- `crates/worldwake-systems/src/needs_actions.rs` — handler additions, registration changes, commit/abort occupancy cleanup, focused tests.
- `crates/worldwake-sim/src/tick_step.rs` — abort trace-detail helper for Wash/Toilet action-trace events.
- `crates/worldwake-ai/tests/scenarios/place_dirtiness.rs` — updated the scripted Toilet request in the latrine-overflow golden to bind the current latrine place now that Toilet has a target.

## Out of Scope

- Atomic-action abort handlers (eat, drink, relieve_wilderness, sleep enrichment) — owned by ticket 005.
- Candidate-emitter occupancy filtering (emitter-time read of `SelfCareOccupancy`) — owned by ticket 006.
- `PromotableContentionKind` extension — landed in `archive/tickets/S173SELCARINT-003.md` (prerequisite).
- Belief-view accessor for `SelfCareOccupancy` if needed for remote queries — verified by ticket 006; this ticket reads only co-located occupancy at start time (FND-14A path).
- Scenario goldens — owned by ticket 007 (A/B/C), 008 (D), 009 (E).
- New `EventTag` variant — explicitly rejected per spec Non-Goals; `EventTag::ActionAborted` reused.

## Acceptance Result

### Tests Passed Or Substituted

1. Added `wash_start_writes_self_care_occupancy_on_basin`.
2. Covered Wash commit removal in `wash_full_success_consumes_basin_water_and_clears_dirtiness`.
3. Split the drafted trace+abort proof across two honest seams: `wash_abort_releases_occupancy` proves handler cleanup, and `tick_step::tests::abort_trace_detail_for_self_care_actions_uses_instance_target` proves the trace detail attached at the live trace-emission boundary.
4. Added `wash_start_fails_when_basin_already_occupied`.
5. Added symmetric Toilet tests: `toilet_start_writes_self_care_occupancy_on_latrine_place`, `toilet_start_fails_when_latrine_place_already_occupied`, and `toilet_abort_releases_occupancy`. Commit removal is covered in `toilet_reduces_bladder_and_creates_waste`.
6. Existing wash/toilet tests pass under `cargo test -p worldwake-systems needs_actions`.
7. `register_needs_actions_adds_all_six_defs_and_handlers` now asserts reservation/target registration for Wash and Toilet.
8. Existing suite `cargo test -p worldwake-systems needs_actions` passed.
9. AI/golden fallout from Toilet's new `ActorPlace` target was fixed in `latrine_overflow_creates_waste_at_place_and_increments_place_dirtiness`; `cargo test -p worldwake-ai` passed afterward.

### Invariants

1. After a successful wash commit, no `SelfCareOccupancy` remains on the basin facility.
2. After a wash abort, no `SelfCareOccupancy` remains on the basin facility, AND the action-trace sink carries a `SelfCareInterrupted` detail entry with the correct `kind` and `basin`.
3. Two co-located agents cannot both start a wash on the same basin in the same tick — exactly one succeeds; the other fails at the start-gate with `ActionError::PreconditionFailed`.
4. Symmetric invariants for toilet on a latrine-tagged Place.
5. Eat, drink, sleep, and relieve_wilderness abort behavior is unchanged by this ticket (verified by existing tests passing).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` inline tests (existing `#[cfg(test)]` from L1058) — 6 new tests (5 from Acceptance Criteria plus the registration-test update).
2. `crates/worldwake-sim/src/tick_step.rs` inline test — proves abort trace detail for Wash uses `ActionTraceDetail::SelfCareInterrupted`.

### Commands And Results

1. Passed `cargo test -p worldwake-systems needs_actions`.
2. Passed `cargo test -p worldwake-sim abort_trace_detail_for_self_care_actions_uses_instance_target`.
3. Draft command `cargo test -p worldwake-sim --test save_load` was invalid because `save_load` is a module, not an integration-test target. Passed substituted command `cargo test -p worldwake-sim save_load`.
4. Passed `cargo build --workspace`.
5. Passed `cargo test -p worldwake-ai`.
6. Waived per-ticket `./scripts/verify.sh` because this ticket is running inside `implement-spec-tickets`; the harness final branch phase still owns the full pre-push verification gate.

## Outcome

Completed on 2026-05-26.

- Wash and Toilet now reserve their self-care target on start.
- Wash writes `SelfCareOccupancy` on the basin; Toilet writes it on the latrine place.
- Successful Wash/Toilet commits and explicit aborts clear occupancy.
- Wash/Toilet abort traces get `ActionTraceDetail::SelfCareInterrupted` at the live `tick_step` action-trace emission boundary.
- The existing latrine-overflow golden now sends the current latrine place as the scripted Toilet target, matching the landed action binding.
- No new `EventTag` was added; the authoritative causal record remains `EventTag::ActionAborted`.

## Deviations

- The drafted handler-local trace write was corrected. Systems abort handlers cannot populate `ActionTraceEvent.detail` directly because `ActionExecutionContext` has no trace sink. The landed trace detail is attached in `worldwake-sim` when abort traces are recorded.
- Toilet gained an explicit `TargetSpec::ActorPlace` target so the latrine place can be reserved and occupied; its binding strictness is `AnyLegalTarget` rather than workstation-tag equivalence.

## Verification Result

- Passed `cargo test -p worldwake-systems needs_actions`.
- Passed `cargo test -p worldwake-sim abort_trace_detail_for_self_care_actions_uses_instance_target`.
- Passed `cargo test -p worldwake-sim save_load`.
- Passed `cargo build --workspace`.
- Passed `cargo test -p worldwake-ai`.
- Waived `./scripts/verify.sh` for this per-ticket closeout because `implement-spec-tickets` owns the final full pre-push gate for the S173 branch.
