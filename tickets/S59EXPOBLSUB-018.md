# S59EXPOBLSUB-018: AskAboutPerson planner integration within SearchForMissing and golden E2E

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — goal model planner integration, golden E2E test
**Deps**: S59EXPOBLSUB-001 through -016 (all completed and archived)

## Problem

`AskAboutPerson` is declared in `SEARCH_FOR_MISSING_OPS` (goal_dispatch_decl.rs:89-93) as a valid mid-plan step for the SearchForMissing goal, and the action handler is fully implemented (`crates/worldwake-systems/src/ask_about_person_actions.rs`). However, `build_payload_override()` returns `UnsupportedGoal` (goal_model.rs:643) and `apply_planner_step()` is a no-op (goal_model.rs:1007), so the planner can never synthesize a payload or reason about the effects of asking a witness. The action is declared relevant but unreachable by the planner.

Per FOUNDATIONS P20, AskAboutPerson is a tactic (not a desired world condition), so it correctly remains a mid-plan step within SearchForMissing rather than a standalone goal. The gap is in the planner's ability to use it as such.

## Assumption Reassessment (2026-04-07)

1. **AskAboutPerson is in SEARCH_FOR_MISSING_OPS** — Confirmed at `crates/worldwake-ai/src/goal_dispatch_decl.rs:89-93`: `&[PlannerOpKind::Travel, PlannerOpKind::AskAboutPerson, PlannerOpKind::SearchPlace]`.
2. **Action handler is complete** — `start_ask_about_person`, `tick_ask_about_person`, `commit_ask_about_person`, `abort_ask_about_person` all implemented in `ask_about_person_actions.rs:316-456`.
3. **Affordance enumeration exists** — `enumerate_ask_about_person_payloads` at `ask_about_person_actions.rs:112-159` filters for overdue expectations and checks ask-memory retention.
4. **Payload struct** — `AskAboutPersonActionPayload { target: EntityId, subject: EntityId }` in `worldwake-sim/src/action_payload.rs:371-374`.
5. **Live GoalKind under test**: `GoalKind::SearchForMissing { subject, last_seen }`. Current operator surface: `[Travel, AskAboutPerson, SearchPlace]`. AskAboutPerson is listed but `build_payload_override` returns UnsupportedGoal (goal_model.rs:643) and `apply_planner_step` returns state unchanged (goal_model.rs:1007).
6. **commit_ask_about_person world mutations** — Records asked-witness interaction, relays last-seen records from target to actor with hearsay provenance tracking (`ask_about_person_actions.rs:394-443`).
7. **Planner semantics** — `PlannerOpKind::AskAboutPerson` has `may_appear_mid_plan=false`, `is_materialization_barrier=false`, `transition_kind=GoalModelFallback` (planner_ops.rs:276). The GoalModelFallback transition kind means the goal model's `apply_planner_step` is the authority — which currently returns state unchanged.
8. **No AskAboutPerson GoalKind** — Confirmed: no `GoalKind::AskAboutPerson` variant exists in `goal.rs`. This is correct per FOUNDATIONS P20 — it's a tactic, not a goal.
9. **Golden scenario numbering** — Scenario 123 is free.
10. **Scenario isolation consideration** — The golden test must ensure the planner prefers AskAboutPerson as a mid-plan step before SearchPlace when a witness is co-located. This requires careful scenario setup: the searcher must be co-located with a potential witness, and the witness must have relevant last-seen information.

## Architecture Check

1. AskAboutPerson as a mid-plan step follows FOUNDATIONS P20 (goals are world conditions, tactics are plan steps) and P15 (knowledge travels physically — asking is an explicit knowledge transfer mechanism). The planner should reason about the epistemic value of asking before searching blindly.
2. No backwards-compatibility shims — the UnsupportedGoal stub in `build_payload_override` and the no-op in `apply_planner_step` will be replaced with real logic.

## Verification Layers

1. Planner includes AskAboutPerson as mid-plan step within SearchForMissing -> planning trace in golden E2E
2. AskAboutPerson payload synthesized correctly (target = co-located agent, subject = missing entity) -> decision trace showing payload
3. ask_about_person action commits and transfers last-seen record -> action trace in golden E2E
4. Actor receives last-seen memory with hearsay provenance -> authoritative world state assertions
5. Subsequent search uses updated last-seen information -> planning/action trace showing search at last-seen place

## What to Change

### 1. Goal model — build_payload_override for AskAboutPerson within SearchForMissing

In `crates/worldwake-ai/src/goal_model.rs`, replace the `UnsupportedGoal` return for `PlannerOpKind::AskAboutPerson` when the active goal is `SearchForMissing`:
- Synthesize `AskAboutPersonActionPayload` with `target` = a co-located agent from beliefs and `subject` = the missing entity from the goal
- If no co-located agent exists in beliefs, return `UnsupportedGoal` (planner skips this step)

### 2. Goal model — apply_planner_step for AskAboutPerson within SearchForMissing

Replace the no-op for `PlannerOpKind::AskAboutPerson` with epistemic state modeling:
- After AskAboutPerson step, the planning state should reflect that the agent may have acquired last-seen information about the subject
- This models the epistemic gain that makes subsequent SearchPlace more targeted

### 3. Golden E2E test — scenario 123

Add `golden_ask_about_person_during_search` (+ deterministic replay variant) to `crates/worldwake-ai/tests/golden_expectation.rs`. Scenario setup:
- Agent A at Place1 with overdue expectation for entity S
- Agent B (witness) also at Place1, with LastSeenMemory containing a record for S at Place2
- Place2 and Place3 both reachable from Place1
- Expected trace: SearchForMissing candidate emitted -> plan includes AskAboutPerson step -> ask_about_person commits -> A receives last-seen record for S at Place2 with hearsay provenance -> A searches at Place2

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — AskAboutPerson payload override and planner step)
- `crates/worldwake-ai/tests/golden_expectation.rs` (modify — add scenario 123)
- `crates/worldwake-ai/tests/scenarios/` (new — scenario 123 RON file if scenario-driven)

## Out of Scope

- AskAboutPerson as a standalone goal (per FOUNDATIONS P20, it's a tactic)
- Multi-witness chaining (asking multiple witnesses in sequence)
- Hearsay degradation tracking beyond what `commit_ask_about_person` already implements

## Acceptance Criteria

### Tests That Must Pass

1. `golden_ask_about_person_during_search` — AskAboutPerson appears as mid-plan step, transfers last-seen info
2. `golden_ask_about_person_during_search_replays_deterministically` — determinism invariant
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. AskAboutPerson only appears in plans when a co-located agent exists in beliefs
2. Last-seen transfer uses hearsay provenance (not direct observation)
3. Planner reasons about epistemic gain from asking (search targets last-seen place, not random)
4. Deterministic replay produces identical traces

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_expectation.rs::golden_ask_about_person_during_search` — validates AskAboutPerson as mid-plan step within SearchForMissing goal
2. `crates/worldwake-ai/tests/golden_expectation.rs::golden_ask_about_person_during_search_replays_deterministically` — determinism invariant

### Commands

1. `cargo test -p worldwake-ai golden_ask_about_person`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
