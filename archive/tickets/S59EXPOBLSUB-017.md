# S59EXPOBLSUB-017: EscortToSafety goal model integration, candidate generation, and golden E2E

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — candidate generation, goal model, golden E2E test
**Deps**: S59EXPOBLSUB-001 through -016 (all completed and archived)

## Problem

`GoalKind::EscortToSafety` exists (`crates/worldwake-core/src/goal.rs:53-56`) and the action handler is fully implemented (`crates/worldwake-systems/src/escort_actions.rs`), but the goal is inert: `is_satisfied()` always returns `false` (goal_model.rs:1228-1236), `build_payload_override()` returns `UnsupportedGoal` (goal_model.rs:644), `apply_planner_step()` is a no-op (goal_model.rs:1009), and no candidate generation function emits this goal. Agents who observe wounded co-located entities cannot autonomously decide to escort them to safety.

Per FOUNDATIONS P20, "Goals name desired world conditions" — getting a wounded entity to a safe care-capable place is a desired world condition, making EscortToSafety a legitimate standalone goal.

## Assumption Reassessment (2026-04-07)

1. **GoalKind::EscortToSafety exists** — Confirmed at `crates/worldwake-core/src/goal.rs:53-56` with fields `{ subject: EntityId, destination: EntityId }`.
2. **Goal dispatch declaration exists** — Confirmed at `crates/worldwake-ai/src/goal_dispatch_decl.rs:262-268` with `ESCORT_TO_SAFETY_OPS = &[PlannerOpKind::Travel, PlannerOpKind::EscortToSafety]` (lines 95-96).
3. **Action handler is complete** — `start_escort_to_safety`, `tick_escort_to_safety`, `commit_escort_to_safety`, `abort_escort_to_safety` all implemented in `escort_actions.rs:351-519` with co-location binding, multi-leg travel, and care handoff.
4. **Affordance enumeration exists** — `enumerate_escort_payloads` at `escort_actions.rs:178-199` checks for wounded entities via `view.has_wounds(subject)` and reachable destinations via `reachable_destinations(view, origin)`.
5. **Live GoalKind under test**: `GoalKind::EscortToSafety`. Current operator surface: `ESCORT_TO_SAFETY_OPS = [Travel, EscortToSafety]`. The goal model for this kind is stub-only — `is_satisfied` returns false, `build_payload_override` returns UnsupportedGoal, `apply_planner_step` returns state unchanged.
6. **No candidate generation exists** — `emit_search_candidates` (candidate_generation.rs:3241) only emits `SearchForMissing` and `ReportMissing`. No `emit_escort_candidates` function exists anywhere in the codebase.
7. **GoalKey mapping exists** — `GoalDispatchKey::EscortToSafety` at goal_dispatch_decl.rs:464 maps to the dispatch declaration, so the planner dispatch infrastructure is already wired.
8. **Golden scenario numbering** — Scenarios 120 and 121 are taken (golden_expectation.rs). Scenario 122 is free.
9. **Existing golden test patterns** — Scenarios 120/121 in `crates/worldwake-ai/tests/golden_expectation.rs` establish the assertion style: decision traces for candidate emission + plan selection, action traces for commit verification, world state assertions for component mutations.
10. **No adjacent contradictions** — EscortToSafety is independent of the other two gap tickets (018, 019).
11. **Ranking returns 0** — `ranking.rs:650` has `GoalKind::EscortToSafety { .. } => 0`. Must implement real ranking using `care_weight` and wound severity, matching the TreatWounds pattern at line 616-622.
12. **`goal_relevant_places` already returns `[destination]`** — Confirmed at `goal_model.rs:1281`. No change needed; A* heuristic guidance is wired.
13. **`is_terminal_target_match` already wired** — `goal_model.rs:1558-1564` checks `authoritative_targets.contains(subject) || authoritative_targets.contains(destination)`. No change needed.
14. **`EscortToSafetyActionPayload` has complex fields** — `route_places`, `route_edges`, `intended_heal_action` in `action_payload.rs:393-399`. These are filled at runtime by `start_escort_to_safety`, not during planning. Payload override should build with empty routes and placeholder `ActionDefId(u32::MAX)`, matching `build_escort_payload` at `escort_actions.rs:152-164`.
15. **No care-capability concept in GoalBeliefView** — `enumerate_escort_payloads` checks wounds and route existence only, not care-capability of destination. Candidate generation should follow same pattern: any reachable destination from a wounded co-located entity.

## Architecture Check

1. The approach follows the established pattern: candidate generation emits the goal, goal model provides payload synthesis and satisfaction logic, golden E2E validates the full cycle. This matches exactly how SearchForMissing and ReportMissing were integrated.
2. No backwards-compatibility shims — stub implementations in goal_model.rs will be replaced with real logic, not wrapped.

## Verification Layers

1. Candidate emission (EscortToSafety appears when wounded entity observed) -> decision trace in golden E2E
2. Planner selects EscortToSafety and produces valid plan -> planning trace in golden E2E
3. escort_to_safety action commits and moves both actor + charge -> action trace in golden E2E
4. Charge arrives at destination and enters care -> authoritative world state assertions in golden E2E
5. is_satisfied returns true when charge is at destination -> focused unit test or golden E2E state check

## What to Change

### 1. Candidate generation — emit EscortToSafety

Add `emit_escort_candidates()` function in `crates/worldwake-ai/src/candidate_generation.rs`, called from `generate_candidates_with_travel_horizon()`. Triggers when:
- Agent perceives a co-located wounded/incapacitated entity
- Agent's beliefs include a reachable care-capable destination
- Emits `GoalKind::EscortToSafety { subject, destination }`

### 2. Goal model — build_payload_override for EscortToSafety

In `crates/worldwake-ai/src/goal_model.rs`, replace the `UnsupportedGoal` return for `PlannerOpKind::EscortToSafety` with real payload synthesis:
- Synthesize `EscortToSafetyActionPayload` with subject from goal and destination from goal
- For `PlannerOpKind::Travel`, synthesize travel payload toward the subject's location (if not co-located) or toward the destination (if co-located with subject)

### 3. Goal model — is_satisfied for EscortToSafety

Replace the `false` return for `GoalKind::EscortToSafety` with: charge (subject) is at the destination place in the agent's beliefs.

### 4. Goal model — apply_planner_step for EscortToSafety

Replace the no-op for `PlannerOpKind::EscortToSafety` with state transition modeling: after the escort step, the planning state should reflect that the subject is now at the destination.

### 5. Ranking — compute_motive for EscortToSafety

In `crates/worldwake-ai/src/ranking.rs`, replace the `=> 0` return for `GoalKind::EscortToSafety` with real ranking logic: `care_weight * wound_severity`, matching the TreatWounds other-patient pattern.

### 6. Golden E2E test — scenario 122

Add `golden_escort_to_safety_after_finding_wounded` (+ deterministic replay variant) to `crates/worldwake-ai/tests/golden_expectation.rs`. Scenario setup:
- Agent A at Place1 with a wounded entity W also at Place1
- Care-capable Place2 reachable from Place1
- Agent A has perception to observe W's wounded state
- Expected trace: EscortToSafety candidate emitted -> plan selected -> travel (if needed) -> escort_to_safety commit -> W at Place2

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add `emit_escort_candidates`)
- `crates/worldwake-ai/src/goal_model.rs` (modify — EscortToSafety goal model methods)
- `crates/worldwake-ai/src/ranking.rs` (modify — EscortToSafety motive computation)
- `crates/worldwake-ai/tests/golden_expectation.rs` (modify — add scenario 122)
- `crates/worldwake-ai/tests/scenarios/` (new — scenario 122 RON file if scenario-driven)

## Out of Scope

- Candidate generation for EscortToSafety triggered by search outcomes (finding wounded during search_place) — that's a cross-goal chaining concern for a future ticket
- Multi-agent escort coordination (only one escort per charge)
- Care system behavior after handoff (E12 scope)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_escort_to_safety_after_finding_wounded` — full decision-to-action cycle
2. `golden_escort_to_safety_after_finding_wounded_replays_deterministically` — determinism invariant
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. EscortToSafety goal is only emitted when a wounded entity is co-located and a care destination is reachable
2. The escort action moves both actor and charge to the destination (co-location binding)
3. `is_satisfied` returns true only when charge is at destination in agent's beliefs
4. Deterministic replay produces identical traces

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_expectation.rs::golden_escort_to_safety_after_finding_wounded` — validates full EscortToSafety goal cycle from candidate generation through action commit
2. `crates/worldwake-ai/tests/golden_expectation.rs::golden_escort_to_safety_after_finding_wounded_replays_deterministically` — determinism invariant

### Commands

1. `cargo test -p worldwake-ai golden_escort_to_safety`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-07.

- Added `emit_escort_candidates()` in `candidate_generation.rs` — emits `EscortToSafety` when agent observes wounded co-located entity via beliefs and has `care_weight > 0`. Suppressed when TreatWounds already covers the same patient (prevents goal-switching conflicts).
- Implemented `build_payload_override` for `PlannerOpKind::EscortToSafety` — synthesizes `EscortToSafetyActionPayload` with empty routes and placeholder `ActionDefId` (filled at runtime by `start_escort_to_safety`).
- Implemented `is_satisfied` for `GoalKind::EscortToSafety` — checks subject is at destination in agent's beliefs.
- Implemented `apply_planner_step` for `PlannerOpKind::EscortToSafety` — models actor moving to destination.
- Added `is_progress_barrier` entry for EscortToSafety.
- Added `synthesized_root_candidate_targets` entry for EscortToSafety — provides subject as target for colocated terminal action.
- Implemented ranking for `GoalKind::EscortToSafety` — `care_weight * subject_pain / 4` (lower than TreatWounds to prefer in-place healing).
- Added golden E2E scenario 122 (escort_to_safety_after_finding_wounded) + deterministic replay.

## Deviations

- Ticket originally proposed checking wound state in `build_payload_override`; removed this guard since candidate generation already validates wounds and the planning state may not include wound data for evidence entities.
- Escort motive is ranked at 1/4 of TreatWounds motive to prevent goal-switching conflicts when both candidates exist for the same patient.
- EscortToSafety candidates are suppressed when TreatWounds candidates already exist for the same entity — this prevents DuplicateActor runtime errors from goal switching.
- Golden scenario uses Report-sourced beliefs (not DirectObservation) to isolate EscortToSafety from TreatWounds candidate emission.
- Three additional planner integration points were needed beyond what the ticket described: `is_progress_barrier`, `synthesized_root_candidate_targets`, and ranking. These were identified during reassessment and added to the ticket.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_expectation` (21 tests)
- Passed `cargo test -p worldwake-ai --test golden_care` (33 tests, no regressions)
- Passed `cargo test -p worldwake-ai` (1582+ tests, 0 failures)
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
