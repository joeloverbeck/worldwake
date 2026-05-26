# S173SELCARINT-006: Candidate-emitter occupancy filter (wash + relieve)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes - wash + relieve candidate emitters filter known other-occupant self-care targets through the belief-view surface
**Deps**: `archive/tickets/S173SELCARINT-001.md` (uses `SelfCareOccupancy`), `archive/specs/S173-self-care-interruption-occupancy.md` (D6, distributed D5 emitter-time read)

## Problem

Before this ticket, the Wash and Relieve candidate emitters in `crates/worldwake-ai/src/candidate_generation.rs` filtered on physical facility state but did not consult `SelfCareOccupancy`. The planner could emit wash or latrine candidates targeting a facility/place already occupied by another actor, leaving the first rejection to action start and forcing avoidable replan churn.

## Assumption Reassessment (2026-05-26)

1. The live goal kinds under test are `GoalKind::Wash` and `GoalKind::Relieve`. The owned integration points are `wash_access_opportunities` and the latrine branch inside `emit_relieve_goal` in `crates/worldwake-ai/src/candidate_generation.rs`.
2. The shared abstraction boundary is the `GoalBeliefView` / `FacilityBeliefView` read surface. Reassessment confirmed the spec's D5 note that no new accessor was needed was false on this branch: `facility_wash_basin_state` carried basin water state, but no existing method exposed known self-care occupancy to candidate emission.
3. The landed read surface is `self_care_occupant(entity) -> Option<EntityId>`, not the drafted `facility_self_care_occupancy_observed(actor, facility) -> Option<bool>` sketch. This keeps candidate generation asking the narrow question it needs: is this self-care target known to be held by someone other than the actor?
4. `PerAgentBeliefView::self_care_occupant` reads authoritative `SelfCareOccupancy` only for self/current-place/co-located targets. Remote reads are belief-backed through the existing `BelievedContentionState::grant_holder` carrier; remote world `SelfCareOccupancy` without that belief returns `None`.
5. The start-gate reservation contract from ticket 004 remains the authoritative final defense. This ticket only removes candidates whose occupancy is already known at emit time.
6. `GoalKind::Wash` and `GoalKind::Relieve` planner roots, relevant op lists, and queue operators remain unchanged. Per `docs/planner-contracts.md`, this is a planner-visible field-source change, not a new root operator or snapshot-completeness change.

## Architecture Check

1. The filter lives beside the existing physical-state gates, preserving emit-vs-rank separation: known occupied targets are not emitted; ranking still orders emitted candidates.
2. The read surface preserves FND-14A/FND-14B. Co-located occupancy can be observed from current world state; remote occupancy only affects planning when a belief carrier says a grant/occupant exists.
3. Self-occupancy is not treated as a blocker, so an actor already holding the target is not prevented from continuing a lawful same-target self-care plan.

## Verified Layers

1. Emitter-time co-located occupancy read -> focused candidate-generation tests for Wash and Relieve filter known other-occupants.
2. Emitter-time remote belief-backed occupancy read -> focused Wash test filters a remote basin only when the belief-view output reports an occupant.
3. Remote no-leak boundary -> focused `PerAgentBeliefView` test proves remote authoritative `SelfCareOccupancy` alone is not surfaced.
4. Self-occupancy exception -> focused Wash test proves the actor's own occupancy does not self-block.

## Landed Changes

1. Added `self_care_occupant` to `GoalBeliefView` and `FacilityBeliefView`.
2. Implemented `PerAgentBeliefView::self_care_occupant` with local authoritative reads and remote `BelievedContentionState::grant_holder` reads.
3. Added `self_care_target_occupied_by_other` in `candidate_generation.rs` and applied it to Wash basin enumeration and Relieve latrine enumeration. Wilderness relief remains unfiltered.
4. Added focused candidate-generation and belief-view tests covering known other-occupancy, self-occupancy, remote belief-backed occupancy, and remote world-state non-leakage.

## Landed Files

- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-sim/src/belief_view.rs`
- `crates/worldwake-sim/src/per_agent_belief_view.rs`
- `archive/specs/S173-self-care-interruption-occupancy.md`

## Out of Scope

- Start-gate reservation enforcement stayed owned by ticket 004.
- Action-side occupancy write/remove stayed owned by ticket 004.
- Atomic-action emitter changes stayed out of scope.
- New belief-write mechanics stayed out of scope; the remote read uses the existing contention belief carrier.
- Golden scenarios A-E remain owned by tickets 007-009.

## Acceptance Result

1. Wash and Relieve candidates are not emitted for targets known to be occupied by another actor.
2. Remote authoritative `SelfCareOccupancy` is not read on behalf of the planner.
3. Wilderness-relief candidates remain available because they have no occupancy target.
4. Self-occupied Wash candidates remain available.

## Outcome

Completed on 2026-05-26.

- Landed the emitter-time self-care occupancy filter for Wash basins and Relieve latrines.
- Corrected the spec/ticket path-level discrepancy by adding the narrow `self_care_occupant` read surface.
- Preserved FND-14A/FND-14B by making remote occupancy planner-visible only through an existing belief-backed contention grant carrier.
- Waived per-ticket `./scripts/verify.sh` because this run is inside `$implement-spec-tickets`; the harness final branch phase still owns the full pre-PR gate before push.

## Deviations

- The drafted `facility_self_care_occupancy_observed(actor, facility) -> Option<bool>` sketch did not land. The implemented API is `self_care_occupant(entity) -> Option<EntityId>`, which preserves the occupant identity needed to avoid self-blocking.
- No `SelfCareOccupancy` field was added to `BelievedEntityState`; the remote belief-backed branch uses the existing `BelievedContentionState::grant_holder` carrier.

## Verification Result

- Passed `cargo test -p worldwake-ai emit_wash_goal_skips_basin_with_known_self_care_occupancy_by_other_actor`
- Passed `cargo test -p worldwake-ai emit_wash_goal_emits_when_actor_is_the_occupant`
- Passed `cargo test -p worldwake-ai emit_wash_goal_skips_remote_basin_with_belief_of_occupancy`
- Passed `cargo test -p worldwake-ai emit_relieve_goal_skips_latrine_with_known_occupancy_by_other_actor`
- Passed `cargo test -p worldwake-sim self_care_occupant`
- Passed `cargo test -p worldwake-ai candidate_generation`
- Passed `cargo build --workspace`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
- Passed `cargo clippy -p worldwake-sim --all-targets -- -D warnings`
- Waived `./scripts/verify.sh` for this ticket because the harness final branch phase owns the full pre-push verification gate.
