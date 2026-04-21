# S114PLASTGUA-008: Plan adoption writes ExpectationRecords + clear_plan_step_expectations

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — plan-adoption path writes `ExpectationRecord`s into the agent's `ExpectationStore`; new `clear_plan_step_expectations` helper invoked on plan replacement / step completion.
**Deps**: S114PLASTGUA-002, S114PLASTGUA-003, S114PLASTGUA-004, S114PLASTGUA-006

## Problem

S114 D4 persists plan-step expectations across tick boundaries by reusing the existing `ExpectationStore` infrastructure. When a plan is adopted, the AI crate must write one `ExpectationRecord` per `PlanExpectation` with `basis = PlanStepCompletion { step_index, kind_tag }`. When the plan is replaced or each step completes successfully, the adoption-time records must be explicitly resolved (`Fulfilled`) or expired (`Expired`) so the AI-side overdue tick step (ticket 009) doesn't fire spurious mismatches against stale plan context.

## Assumption Reassessment (2026-04-21)

1. `AgentDecisionRuntime::current_plan` is at `crates/worldwake-ai/src/decision_runtime.rs:152` (`Option<PlannedPlan>`). Plan adoption happens when a new `PlannedPlan` is assigned to `current_plan` — the exact call site lives in the agent-tick planning phase (`crates/worldwake-ai/src/agent_tick/planning.rs` area; verify at implementation time).
2. `ExpectationStore` lives at `crates/worldwake-core/src/expectation.rs:73` as a universal agent component with `records: BTreeMap<ExpectationId, ExpectationRecord>` and `next_expectation_id: ExpectationId`. Records are inserted via an existing store API (e.g. `insert_record`) — confirm the exact fn name at implementation time.
3. S114 spec D4 at `specs/S114-plan-step-guards.md:236-266` defines the `ExpectationRecord` fields to populate at plan adoption:
   - `owner` = acting agent
   - `subject` = step's primary target (or the agent itself if untargeted)
   - `expected_place` = step's target place (or the agent's current place)
   - `deadline_tick` = `step.expected_complete_tick(adoption_tick)` adjusted by `expectation_tolerance_ticks`
   - `grace_ticks` = derived from profile tolerance (the spec says `grace_ticks` maps to `profile.expectation_tolerance_ticks`)
   - `basis` = `ExpectationBasis::PlanStepCompletion { step_index, kind_tag }`
   - `state` = `ExpectationState::Active`
   - `created_tick` = `adoption_tick`
4. Shared boundary under audit: the `current_plan`-assignment seam. Every place `current_plan` is mutated (set to `Some(new_plan)`, set to `None`, replaced mid-execution) must trigger the expectation-record side effect. Verify every assignment site at implementation time via `rg 'current_plan\s*=' crates/worldwake-ai`.
5. `clear_plan_step_expectations(agent, plan_id)` is the AI-crate helper. "plan_id" is whatever disambiguator prevents clearing records from a *different* plan that happens to have the same `step_index` — confirm via `PlannedPlan`'s existing identity fields at implementation time. If no such disambiguator exists, the helper clears all `PlanStepCompletion`-basis records for the agent (safe because ticket 009 only reads `Overdue` records, and pre-transition records are `Active`).
6. Step-completion clearing: when the active step completes successfully (the action handler fires), its adoption-time records should transition to `Resolved { outcome: Fulfilled }`. Identify the step-completion site in `tick_step.rs` or action-handler commit logic at implementation time.
7. No existing test exercises `ExpectationStore` through the plan-adoption path — current `ExpectationStore` coverage is `RoutineReturn`-basis in `search_actions.rs`, `report_actions.rs`, `ask_about_person_actions.rs`. A new focused test is required.

## Architecture Check

1. Plan adoption writes symmetric to how sim's `check_overdue_expectations` state-transitions records: both paths operate on the agent's own `ExpectationStore`. No cross-agent write. `clear_plan_step_expectations` is the AI-side inverse of the adoption write — symmetric lifecycle.
2. Storing `step_index` + `kind_tag` on the record (and keeping the rich `PlanExpectation` on the runtime `PlannedStep`) respects `Copy` on `ExpectationRecord` and avoids replicating the richer predicate on every record.
3. FND-26 (systems interact through state): sim's `ExpectationCheck` transitions records to `Overdue`; AI writes and resolves them. Sim and AI coordinate through the shared store, never through direct calls.

## Verification Layers

1. Plan-adoption write correctness (N expectations on a plan → N records in store with matching `step_index`, `kind_tag`, `deadline_tick`) → focused unit test with a synthetic plan + profile.
2. Grace-tick derivation from profile (`expectation_tolerance_ticks = 4` on profile → record's `grace_ticks = 4`) → focused unit test.
3. Clear-on-replacement (replacing `current_plan` clears all prior `PlanStepCompletion`-basis records) → focused unit test.
4. Step-completion resolution (successful step handler fires → record transitions to `Resolved { Fulfilled }`) → focused unit test driving a minimal action through the tick machinery, asserting post-tick record state.
5. No false negatives when a plan has no expectations (empty `step.expectations` → no records written) → focused unit test.
6. Single-layer behavior (AI crate only; sim-side `check_overdue_expectations` is unchanged by this ticket).

## What to Change

### 1. Write expectation records at plan adoption

At the single assignment site of `AgentDecisionRuntime::current_plan = Some(plan)` (or wherever plan adoption is centralized — prefer a helper fn over inline assignment), after the assignment:

```rust
fn write_plan_step_expectations(
    agent: EntityId,
    plan: &PlannedPlan,
    adoption_tick: Tick,
    profile: &CognitiveProfile,
    store: &mut ExpectationStore,
) {
    let grace = profile.expectation_tolerance_ticks as u64;
    for (idx, step) in plan.steps.iter().enumerate() {
        let step_index = idx as u16;
        let deadline = step.expected_complete_tick(adoption_tick);
        for expectation in &step.expectations {
            let kind_tag = kind_tag_of(&expectation.kind);
            let subject = step.primary_target().unwrap_or(agent);
            let expected_place = step.target_place().unwrap_or_else(|| current_place_of(agent));
            let record = ExpectationRecord {
                id: store.next_expectation_id(),
                owner: agent,
                subject,
                expected_place,
                deadline_tick: expectation.observe_by.unwrap_or(deadline),
                grace_ticks: grace,
                basis: ExpectationBasis::PlanStepCompletion { step_index, kind_tag },
                state: ExpectationState::Active,
                created_tick: adoption_tick,
            };
            store.insert_record(record);
        }
    }
}
```

`kind_tag_of(&ExpectationKind) -> ExpectationKindTag` is a trivial mapper lives alongside in the same module.

### 2. Add `clear_plan_step_expectations` helper

```rust
pub fn clear_plan_step_expectations(
    store: &mut ExpectationStore,
    at_tick: Tick,
    outcome: ClearOutcome,
) {
    let to_update: Vec<_> = store.records.iter()
        .filter(|(_, r)| matches!(r.basis, ExpectationBasis::PlanStepCompletion { .. }))
        .filter(|(_, r)| matches!(r.state, ExpectationState::Active | ExpectationState::Overdue))
        .map(|(id, _)| *id)
        .collect();
    for id in to_update {
        if let Some(record) = store.records.get_mut(&id) {
            record.state = match outcome {
                ClearOutcome::Fulfilled => ExpectationState::Resolved {
                    outcome: ExpectationOutcome::Fulfilled,
                },
                ClearOutcome::Replaced => ExpectationState::Expired,
            };
        }
    }
}

pub enum ClearOutcome {
    Fulfilled,
    Replaced,
}
```

### 3. Invoke `clear_plan_step_expectations` at plan-lifecycle transitions

- Plan replacement (new plan adopted mid-execution, old plan discarded): `ClearOutcome::Replaced`, before writing new-plan records.
- Plan completion (final step commits successfully): `ClearOutcome::Fulfilled`.
- Step-by-step completion (each step's records transition to `Resolved { Fulfilled }` when that step's action commits): narrower variant — clear only records matching `step_index == completed_step`. Factor the helper to accept an optional filter.

Verify exact step-completion site at implementation time (likely `crates/worldwake-sim/src/tick_step.rs` or AI-side commit-handling in `agent_tick/active_action.rs`).

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` OR `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — plan-adoption write; verify exact site at implementation time)
- `crates/worldwake-ai/src/plan_step_expectations.rs` (new — helper module with `clear_plan_step_expectations` + `write_plan_step_expectations`)
- `crates/worldwake-ai/src/lib.rs` (modify — module declaration)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` or `crates/worldwake-sim/src/tick_step.rs` (modify — step-completion resolution; verify exact site)

## Out of Scope

- The AI-side tick step that reads `Overdue` records, emits mismatch events, and classifies discrepancies (ticket 009 — that is the *consumer* side).
- Sim-side changes to `check_overdue_expectations` (none required — it state-transitions generically regardless of basis).
- Populating `guard_template`/`expectation_template` on additional actions beyond `trade` (ticket 006 already covers `trade`; later phases widen).

## Acceptance Criteria

### Tests That Must Pass

1. `plan_adoption_writes_one_record_per_expectation` — synthetic `PlannedPlan` with 2 steps × 2 expectations each adopted at `Tick(10)` produces 4 records in the agent's `ExpectationStore`, each with correct `step_index`, `kind_tag`, `deadline_tick`, `basis = PlanStepCompletion`.
2. `plan_adoption_sets_grace_ticks_from_profile` — profile with `expectation_tolerance_ticks: 4` produces records with `grace_ticks: 4`.
3. `plan_adoption_with_empty_expectations_writes_no_records` — a plan whose steps have `expectations: vec![]` leaves the store untouched.
4. `plan_replacement_expires_prior_records` — adopting plan B when plan A was already adopted transitions plan-A's records to `Expired` and writes plan-B's records fresh.
5. `step_completion_resolves_record_as_fulfilled` — driving a step's action to successful commit transitions its record(s) to `Resolved { outcome: Fulfilled }`.
6. Existing `ExpectationStore` tests in `expectation.rs` continue to pass (no change to non-plan bases).
7. Existing golden suites (`golden_survival_*`, `golden_planner_pathology`, `golden_portfolio_planning`) stay green — the additive store writes do not break any pre-S114 agent behavior.

### Invariants

1. `ExpectationRecord`s written at plan adoption use `ExpectationState::Active` and never skip ahead to `Overdue` at write time.
2. `clear_plan_step_expectations` touches only `PlanStepCompletion`-basis records — never mutates `DutyAssignment`, `DeliveryCommitment`, `RoutineReturn`, `EscortObligation`, or `SocialPromise`.
3. Every `current_plan = Some(new_plan)` assignment is paired with exactly one `write_plan_step_expectations` call. Every `current_plan = None` or replacement is paired with a corresponding `clear_plan_step_expectations` call.
4. `grace_ticks` = `profile.expectation_tolerance_ticks as u64` — the `u32 → u64` widening happens at the record write site, not the profile field.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_step_expectations.rs` tests module (new) — acceptance tests 1-5 above.
2. Integration test in `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — drive a minimal tick that adopts a plan with expectations and asserts store state post-tick.

### Commands

1. `cargo test -p worldwake-ai plan_step_expectations`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-ai` (full AI-crate suite — confirms no regression in planning/revalidation flow)
4. `scripts/verify.sh`
