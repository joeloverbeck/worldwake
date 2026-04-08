# S59EXPOBLSUB-018: AskAboutPerson planner integration within SearchForMissing and golden E2E

**Status**: ✅ COMPLETED
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
5. **Live GoalKind under test**: `GoalKind::SearchForMissing { subject, last_seen }`. Current operator surface: `[Travel, AskAboutPerson, SearchPlace]`. AskAboutPerson is listed but `build_payload_override` returns UnsupportedGoal (goal_model.rs:643) and `apply_planner_step` returns state unchanged (goal_model.rs:1026).
6. **commit_ask_about_person world mutations** — Records asked-witness interaction, relays last-seen records from target to actor with hearsay provenance tracking (`ask_about_person_actions.rs:394-443`).
7. **Planner semantics** — `PlannerOpKind::AskAboutPerson` has `may_appear_mid_plan=false`, `is_materialization_barrier=false`, `transition_kind=GoalModelFallback` (planner_ops.rs:276). With `may_appear_mid_plan=false`, the planner rejects any AskAboutPerson step that is not terminal (`search/transition.rs:124`).
8. **No AskAboutPerson GoalKind** — Confirmed: no `GoalKind::AskAboutPerson` variant exists in `goal.rs`. This is correct per FOUNDATIONS P20 — it's a tactic, not a goal.
9. **Golden scenario numbering** — Scenario 124 is free.
10. **`is_progress_barrier` for SearchForMissing** — Only returns `true` for `SearchPlace` (goal_model.rs:1065-1069). AskAboutPerson is not a progress barrier, so combined with `may_appear_mid_plan=false` the planner can never include it in any SearchForMissing plan.
11. **ARCHITECTURAL CORRECTION — progress barrier, not mid-plan step** — The ticket originally framed AskAboutPerson as a "mid-plan step." This violates FOUNDATIONS P14 (planner may consult only accessible belief state): modeling epistemic gain from asking requires the planner to hypothesize what the witness will say, which is non-local information access at plan time. The correct design is **progress barrier**: the planner produces terminal plans `[Travel? → AskAboutPerson]`, the ask commits and transfers last-seen info, then replanning with updated beliefs directs search to the revealed location. This preserves P14, P7 (locality), P15 (knowledge travels physically), and P1 (emergence through replanning).
12. **`synthesized_root_candidate_targets` not needed** — AskAboutPerson targets come from real affordance enumeration (`enumerate_ask_about_person_payloads`), not from goal-synthesized root candidates. The current `_ => UnsupportedGoalOp` wildcard is correct for AskAboutPerson.
13. **`apply_planner_step` stays as-is** — Terminal progress-barrier ops don't need hypothetical state transitions. The plan ends after AskAboutPerson; the next cycle replans with updated beliefs.

## Architecture Check

1. AskAboutPerson as a progress barrier follows FOUNDATIONS P20 (goals are world conditions, tactics are plan steps) and P15 (knowledge travels physically — asking is an explicit knowledge transfer mechanism). The planner produces terminal ask plans; the epistemic gain from asking emerges through replanning with updated beliefs (P14 compliance — planner never consults witness state).
2. No backwards-compatibility shims — the UnsupportedGoal stub in `build_payload_override` will be replaced with real payload synthesis. `apply_planner_step` remains a no-op (correct for terminal ops).

## Verification Layers

1. Planner selects AskAboutPerson as progress-barrier terminal for SearchForMissing -> decision trace in golden E2E
2. AskAboutPerson payload synthesized correctly (target = co-located agent, subject = missing entity) -> action trace showing committed ask
3. ask_about_person action commits and transfers last-seen record -> action trace + authoritative state
4. Actor receives last-seen memory with hearsay provenance -> authoritative world state assertions
5. Replanning with updated beliefs produces SearchForMissing targeting the revealed location -> decision trace + action trace showing search at last-seen place
6. Search resolves overdue expectation -> authoritative state assertion

## What to Change

### 1. Goal model — build_payload_override for AskAboutPerson within SearchForMissing

In `crates/worldwake-ai/src/goal_model.rs`, replace the `UnsupportedGoal` return for `PlannerOpKind::AskAboutPerson` when the active goal is `SearchForMissing`:
- Synthesize `AskAboutPersonActionPayload` with `target` from the planner-bound targets and `subject` from the goal
- If targets are empty, return `UnsupportedGoal`

### 2. Goal model — is_progress_barrier for AskAboutPerson within SearchForMissing

Add `AskAboutPerson` as a progress barrier for `SearchForMissing` (alongside existing `SearchPlace`). This lets the planner produce terminal `[Travel? → AskAboutPerson]` plans when a co-located witness provides an affordance.

### 3. Golden E2E test — scenario 124

Add `golden_ask_about_person_during_search` (+ deterministic replay variant) to `crates/worldwake-ai/tests/golden_expectation.rs`. Scenario setup:
- Agent A (searcher) at VillageSquare with overdue expectation for entity S
- Agent B (witness) also at VillageSquare, with LastSeenMemory containing a record for S at OrchardFarm
- Entity S actually at OrchardFarm with ControlSource::None
- Expected emergent behavior over two planning cycles:
  - Cycle 1: SearchForMissing candidate → plan terminal is AskAboutPerson → ask_about_person commits → A receives last-seen record for S at OrchardFarm with hearsay provenance
  - Cycle 2: SearchForMissing candidate (with updated last_seen) → plan includes Travel + SearchPlace → search commits → expectation resolved as FoundSafe

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — `build_payload_override` for AskAboutPerson, `is_progress_barrier` for SearchForMissing+AskAboutPerson)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — add `actor_expectation_store` field)
- `crates/worldwake-ai/src/planning_state.rs` (modify — implement `expectation_store()` on RuntimeBeliefView)
- `crates/worldwake-ai/tests/golden_expectation.rs` (modify — add scenario 124)

## Out of Scope

- AskAboutPerson as a standalone goal (per FOUNDATIONS P20, it's a tactic)
- Multi-witness chaining (asking multiple witnesses in sequence)
- Hearsay degradation tracking beyond what `commit_ask_about_person` already implements

## Acceptance Criteria

### Tests That Must Pass

1. `golden_ask_about_person_during_search` — AskAboutPerson appears as progress-barrier terminal, transfers last-seen info
2. `golden_ask_about_person_during_search_replays_deterministically` — determinism invariant
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. AskAboutPerson only appears in plans when a co-located agent exists in beliefs and an overdue expectation provides the subject
2. Last-seen transfer uses hearsay provenance (not direct observation)
3. After AskAboutPerson commits, replanning with updated beliefs directs search to the revealed location (epistemic gain emerges through replanning, not modeled in hypothetical state)
4. Deterministic replay produces identical traces

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_expectation.rs::golden_ask_about_person_during_search` — validates AskAboutPerson as progress-barrier terminal within SearchForMissing goal
2. `crates/worldwake-ai/tests/golden_expectation.rs::golden_ask_about_person_during_search_replays_deterministically` — determinism invariant

### Commands

1. `cargo test -p worldwake-ai --test golden_expectation golden_ask`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-07.

### What Changed

**Production code** (2 files):
- `goal_model.rs`: Added `build_payload_override` for `PlannerOpKind::AskAboutPerson` within `GoalKind::SearchForMissing` — synthesizes `AskAboutPersonActionPayload` from planner-bound targets and the goal's subject. Added `AskAboutPerson` as a progress barrier for SearchForMissing in `is_progress_barrier`.
- `planning_snapshot.rs`: Added `actor_expectation_store: Option<ExpectationStore>` field, populated from `view.expectation_store(actor)` during snapshot construction.
- `planning_state.rs`: Implemented `expectation_store()` on `PlanningState`'s `RuntimeBeliefView` impl, returning the actor's snapshotted expectation store.

**Golden E2E test** (Scenario 124):
- `golden_expectation.rs`: Added `golden_ask_about_person_during_search` + deterministic replay companion. Proves the full multi-cycle chain: overdue expectation → AskAboutPerson as progress-barrier terminal → last-seen hearsay transfer → replan with updated beliefs → travel to revealed location → search_place → expectation resolved as FoundSafe.

### Deviations from Original Ticket

1. **Progress barrier, not mid-plan step**: Ticket originally framed AskAboutPerson as a "mid-plan step." Corrected to progress barrier because modeling epistemic gain in hypothetical state violates FOUNDATIONS P14 (planner may only consult accessible belief state). The emergent multi-cycle behavior (ask → gain info → replan → search) is the FOUNDATIONS-compliant design.
2. **Scenario number 124** (was 123): Scenario 123 was already taken by `golden_production.rs`.
3. **Expected place = OrchardFarm**: Expectation's `expected_place` set to OrchardFarm (not VillageSquare) so `search_place` at OrchardFarm can resolve it. This isolates the AskAboutPerson path more cleanly.
4. **PlanningSnapshot widened**: Added `actor_expectation_store` to enable affordance enumeration for AskAboutPerson during planner search. Without this, `enumerate_ask_about_person_payloads` returned empty payloads because the planning state's `RuntimeBeliefView` defaulted `expectation_store()` to `None`.
5. **`apply_planner_step` stays as-is**: No epistemic state modeling needed — terminal progress-barrier ops don't need hypothetical transitions.
6. **`synthesized_root_candidate_targets` not added**: Real affordance enumeration handles AskAboutPerson target resolution.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_expectation golden_ask` (2 tests)
- Passed `cargo test -p worldwake-ai` (36 tests)
- Passed `cargo test --test golden_expectation -p worldwake-ai` (23 tests)
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `python3 scripts/golden_inventory.py --write --check-docs` (143 scenario blocks)
