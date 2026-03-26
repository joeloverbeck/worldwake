# E17CRITHEJUS-010: Implement theft candidate generation at the live AI boundary

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate admission in `worldwake-ai`
**Deps**: `specs/E17-crime-theft-justice.md`

## Problem

The live engine already has the theft domain in core, planner, and action layers, but `worldwake-ai` still never emits `GoalKind::StealItem`. The missing boundary is candidate generation from `GoalBeliefView` into grounded goals, so theft never reaches ranking, feasibility, search, or start-failure handling.

## Assumption Reassessment (2026-03-26)

Shared abstraction boundary under audit: `worldwake_ai::candidate_generation` consuming `worldwake_sim::GoalBeliefView` and emitting `worldwake_core::GoalKind::StealItem`.

1. `GoalKind::StealItem` already exists in [`crates/worldwake-core/src/goal.rs`](../crates/worldwake-core/src/goal.rs), so this ticket is not introducing a new goal kind.
2. Theft planner support already exists in [`crates/worldwake-ai/src/goal_model.rs`](../crates/worldwake-ai/src/goal_model.rs), [`crates/worldwake-ai/src/planner_ops.rs`](../crates/worldwake-ai/src/planner_ops.rs), and associated search tests. The original dependency note saying planner support was still missing is stale.
3. Theft action validation and commit support already exist in [`crates/worldwake-systems/src/transport_actions.rs`](../crates/worldwake-systems/src/transport_actions.rs), so the live gap is not authoritative execution.
4. `candidate_generation.rs` currently dispatches `emit_need_candidates`, `emit_production_candidates`, `emit_enterprise_candidates`, `emit_combat_candidates`, `emit_social_candidates`, `emit_political_candidates`, `emit_recorded_violation_candidates`, and `emit_expectation_violation_candidates`. There is no theft or crime candidate group today.
5. The live belief-view surface is broader than the original ticket assumed. The relevant queries are `locally_observed_entities_at`, `believed_owner_of`, `can_control`, `direct_container`, `direct_possessor`, `carry_capacity`, `load_of_entity`, `effective_place`, and `theft_disposition_profile`.
6. `emit_candidate_with_trace()` is still the right emission surface. `GroundedGoal` no longer carries motive or priority class at candidate-generation time, so the old ticket text prescribing a motive-bearing `GroundedGoal` was wrong for the live architecture.
7. `TheftDispositionProfile` already exists in [`crates/worldwake-core/src/crime.rs`](../crates/worldwake-core/src/crime.rs) with `theft_motive_weight` and `witness_risk_penalty`. In the live architecture, this ticket should use those values as an emission gate, not invent a parallel grounded-goal scoring struct.
8. The direct local information path is canonical for this ticket: theft candidates should come from locally observed item lots at the actor's current place. There is no second lawful transport path for this fact in-scope, and this ticket should not add one.
9. The live theft action rejects targets inside containers in [`crates/worldwake-systems/src/transport_actions.rs`](../crates/worldwake-systems/src/transport_actions.rs). The original ticket missed this precondition; candidate generation must mirror it to avoid emitting obviously invalid terminal goals.
10. `GoalBeliefView` deliberately excludes reservation helpers, while `RuntimeBeliefView` includes them. During implementation that boundary proved real: candidate admission can and should use theft disposition, but reservation filtering would require widening the trait against its documented scope. Reservation-based theft rejection is therefore deferred to downstream planning/start-failure handling rather than forced into this ticket.
11. The live goal policy and ranking layers already know about `GoalKind::StealItem`, but they currently treat crime goals as `Low` priority with constant motive `1` in [`crates/worldwake-ai/src/ranking.rs`](../crates/worldwake-ai/src/ranking.rs). That is an adjacent architectural limitation, but this ticket stays scoped to candidate admission; it should not silently widen into a crime-ranking redesign.
12. The live test inventory already includes targeted `candidate_generation` tests and broader `worldwake-ai` suites, verified with `cargo test -p worldwake-ai -- --list`. There was no existing theft candidate coverage before this change.
13. Corrected motive gate for this ticket: emit only when `theft_motive_weight > witness_risk_penalty * observed_non_self_agent_count`. Example: `300 - 200 > 0` emits; `300 - 300 <= 0` does not.

## Architecture Check

1. The cleanest implementation is a dedicated crime candidate hook in `candidate_generation.rs`, with `emit_theft_candidates()` living beside the other domain emitters and wired through a small `emit_crime_candidates()` dispatch point. This keeps theft out of combat/social buckets and leaves a natural extension point for `E17CRITHEJUS-011` justice emitters.
2. The live architecture already centralizes priority and motive scoring in `ranking.rs`, so this ticket should not backslide into stuffing scores into `GroundedGoal`. Candidate generation should only decide whether the goal is lawfully and evidentially grounded enough to exist.
3. Mirroring the authoritative steal preconditions that are knowable from `GoalBeliefView` (`owned by another`, `not lawfully controllable`, `not possessed`, `not inside container`, `has carry capacity`) is more robust than emitting broad theft roots and letting search/start-failure absorb obvious invalidity.
4. No backwards-compatibility aliasing or shim paths are needed.

## Verification Layers

1. Theft candidate admission from local observable state -> focused `candidate_generation` unit tests
2. Theft candidate exclusion for ownership/control/possession/container/capacity gates -> focused `candidate_generation` unit tests
3. Witness-risk emission gate -> focused `candidate_generation` unit tests
4. Direct-observation provenance on emitted theft candidate -> focused `candidate_generation` diagnostics assertion on `CandidateEvidenceTrace.knowledge_path`
5. No regressions to broader AI candidate/ranking/planning surfaces -> `cargo test -p worldwake-ai`
6. Lint cleanliness for touched code -> `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## What to Change

### 1. Add theft candidate emission in `candidate_generation.rs`

Guard on both:
1. Actor is alive and has a current place through existing outer generation flow
2. Actor has `TheftDispositionProfile`

Algorithm:
1. Use `locally_observed_entities_at(actor, place)` as the candidate source, not a new global query path.
2. Filter to `EntityKind::ItemLot`.
3. Require `believed_owner_of(item)` to be `Some(owner)` where `owner != actor`.
4. Reject items the actor can already control through `can_control(actor, item)`.
5. Reject items inside containers (`direct_container(item).is_some()`).
6. Reject items already possessed (`direct_possessor(item).is_some()`).
7. Reject items that do not fit within remaining carry capacity.
8. Count observed non-self living agents at the same place as witness pressure.
9. Apply the motive gate `theft_motive_weight > witness_risk_penalty * witness_count`; emit nothing when the result is non-positive.
10. Emit `GoalKind::StealItem { target_item }` through `emit_candidate_with_trace()`, with the item and current place as evidence.
11. When tracing is enabled, record direct local provenance for the observed item belief. The trace should show direct observation, not an invented knowledge source.

### 2. Wire theft into the live candidate dispatch

Add a crime-domain dispatch hook in the main candidate generation orchestration and call the theft emitter there.

## Files to Touch

- `tickets/E17CRITHEJUS-010.md`
- `crates/worldwake-ai/src/candidate_generation.rs`

## Out of Scope

- Justice candidate generation (`E17CRITHEJUS-011`)
- Reworking crime-goal ranking from the current constant-score placeholder
- Steal action handler or authoritative validation
- Planner-operator changes, binding changes, or goal-policy redesign
- Golden theft scenarios
- Any remote or rumor-based theft-candidate path

## Acceptance Criteria

### Tests That Must Pass

1. Actor with `TheftDispositionProfile` and locally observed owned stealable item emits `StealItem`
2. Actor without `TheftDispositionProfile` emits no theft candidate
3. Actor does not emit theft for self-owned items
4. Actor does not emit theft for unowned items
5. Actor does not emit theft for items they can already control
6. Actor does not emit theft for items already possessed by another entity
7. Actor does not emit theft for items inside containers
8. Actor does not emit theft when the item exceeds remaining carry capacity
9. Actor does not emit theft when witness-risk reduces the motive gate to zero
10. Emitted theft candidate carries direct-observation provenance in diagnostics
11. `cargo test -p worldwake-ai`

### Invariants

1. Theft candidates only arise from local observable state at the actor's current place
2. Theft candidates only exist for actors with `TheftDispositionProfile`
3. Theft candidates mirror belief-visible authoritative steal preconditions closely enough to avoid obvious dead roots without widening `GoalBeliefView` into runtime reservation queries
4. Witness deterrence remains profile-driven; no hardcoded crime constants are introduced
5. Candidate generation stays deterministic and uses ordered collections only

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — `local_owned_item_emits_theft_goal`
2. `crates/worldwake-ai/src/candidate_generation.rs` — `theft_candidate_respects_preconditions_and_witness_gate`
3. `crates/worldwake-ai/src/candidate_generation.rs` — `theft_candidate_knowledge_path_records_direct_local_observation`

### Commands

1. `cargo test -p worldwake-ai local_owned_item_emits_theft_goal`
2. `cargo test -p worldwake-ai theft_candidate_respects_preconditions_and_witness_gate`
3. `cargo test -p worldwake-ai theft_candidate_knowledge_path_records_direct_local_observation`
4. `cargo test -p worldwake-ai`
5. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-26
- Actual changes:
  - Added a dedicated crime candidate dispatch and `emit_theft_candidates()` in [`crates/worldwake-ai/src/candidate_generation.rs`](../crates/worldwake-ai/src/candidate_generation.rs)
  - Extended [`crates/worldwake-sim/src/belief_view.rs`](../crates/worldwake-sim/src/belief_view.rs) so `GoalBeliefView` exposes `theft_disposition_profile`, matching the live candidate-formation boundary
  - Added focused theft candidate tests for emission, exclusion gates, and direct-observation provenance
- Deviations from original plan:
  - Did not add reservation filtering to candidate generation because `GoalBeliefView` intentionally excludes reservation helpers; widening that trait would have been a broader architectural change than this ticket justified
  - Kept crime-goal ranking unchanged; theft admission now works, but crime goals still use the existing low/constant-score ranking path
- Verification:
  - `cargo test -p worldwake-ai local_owned_item_emits_theft_goal`
  - `cargo test -p worldwake-ai theft_candidate_respects_preconditions_and_witness_gate`
  - `cargo test -p worldwake-ai theft_candidate_knowledge_path_records_direct_local_observation`
  - `cargo test -p worldwake-ai`
  - `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
