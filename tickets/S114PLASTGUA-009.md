# S114PLASTGUA-009: AI-side plan-step mismatch tick step — emission + discrepancy classification

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium-Large
**Engine Changes**: Yes — new AI-side tick step reads `PlanStepCompletion`-basis `Overdue` records, emits `EventTag::ExpectationMismatch`, classifies discrepancies, transitions record state; `classify_discrepancy` promoted to `pub(crate)`.
**Deps**: `archive/tickets/S114PLASTGUA-003.md`, `archive/tickets/S114PLASTGUA-004.md`, `archive/tickets/S114PLASTGUA-005.md`, `tickets/S114PLASTGUA-008.md`

## Problem

S114 D6 (post-F1 correction) places plan-step-specific mismatch handling on the AI side because worldwake-systems cannot depend on worldwake-ai (per the one-way `ai → systems → sim → core` crate dependency graph). Sim's `check_overdue_expectations` keeps its generic `Active → Overdue` transition; the AI tick step owns step resolution, event emission, discrepancy classification, and state transition to `Resolved { ReturnedLate }` / `Expired`. Without this consumer, the records written in ticket 008 sit as silent state — no surprise signal (FND-17), no discrepancy learning (S109).

## Assumption Reassessment (2026-04-22)

1. Sim-side `check_overdue_expectations` at `crates/worldwake-systems/src/expectation_check.rs:7` already transitions records `Active → Overdue` generically (confirmed via file read: body filters `state == Active` regardless of `basis`). No widening is required there for plan-step-specific behavior — the AI tick step is the interpretation layer.
2. `classify_discrepancy` is currently private at `crates/worldwake-ai/src/failure_handling.rs:133`. It must be promoted to `pub(crate)` (or moved to a shared AI-crate helper module) so the new tick step can call it. Its existing callers at `failure_handling.rs:53` continue to work; no external crate relies on it.
3. S114 spec D6 at `specs/S114-plan-step-guards.md:322-357` (post-F1 correction) defines the four-step tick-step contract:
   - Resolve step via `AgentDecisionRuntime::current_plan`. If plan absent or `step_index` out of range → transition record to `Expired`, skip emission.
   - Emit `EventTag::ExpectationMismatch` with widened `ExpectationMismatchPayload` carrying `kind_tag`, `step_index`, and `MismatchDetail` (from ticket 005).
   - Route through `classify_discrepancy` by `ExpectationKind` class (mapping per spec).
   - Transition record to `Resolved { outcome: ReturnedLate }` (or a new outcome variant — implementation time decides).
4. Tick placement: after observation gathering, before planning, per-agent. `crates/worldwake-ai/src/agent_tick/mod.rs` sequences the agent tick. Natural site: a new sibling module (`agent_tick/plan_step_expectations.rs`) invoked from the agent tick entry point between `observation` and `planning` phases. Verify exact sequencing at implementation time.
5. Shared boundary under audit: the sim→ai seam on `ExpectationStore`. Sim transitions to `Overdue`; AI reads `Overdue`, emits, classifies, resolves. The store is the shared state surface (FND-26). No direct sim→ai call.
6. `ExpectationOutcome::ReturnedLate` at `expectation.rs:54` already exists. The spec says implementation time may introduce a new outcome if none fits; `ReturnedLate` is semantically appropriate for plan-step overdue and does not require a new variant.
7. Existing tests in `expectation_check.rs` at lines 157, 218, 232, 271, 312 cover `RoutineReturn`-basis overdue behavior — they assert only the `Active → Overdue` transition. They stay green because this ticket does not touch `check_overdue_expectations`. A new test exercising `PlanStepCompletion`-basis transition is already covered by the widened `check_overdue_expectations` generic behavior, but an additional AI-side test covers the emission + classification path.
8. Authoritative-to-AI Impact Rule (CLAUDE.md): this ticket gates no new action preconditions; it observes a state transition and reacts. Items 1-5 of the rule are `pass` / `N/A`. Item 6 (payload revalidation): no payload-override changes here. Item 7 (golden tests): must stay green — ticket 010 provides the positive-case golden.
9. Mismatch + correction: `archive/tickets/S114PLASTGUA-007.md` landed `PlanInvalidationReason::ExpectationMismatch` classification and replan-reason preservation, but it did **not** land guard-breach `DecisionEventPayload::ExpectationMismatch` emission or `MismatchDetail::GuardInvalidator(...)` plumbing. Follow-up ticket `archive/tickets/S114PLASTGUA-011.md` now owns that delivered AI execution/start-failure producer path. This ticket remains the overdue-record consumer path only.

## Architecture Check

1. AI-side placement respects FND-26: sim owns the generic state transition (`Active → Overdue`), AI owns plan-specific interpretation. Symmetric with ticket 008's plan-adoption writes — the full lifecycle is AI-driven on both ends, sim provides only the in-between overdue sweep.
2. Promoting `classify_discrepancy` from `fn` to `pub(crate) fn` is the minimum-surface-change option. No API widening beyond what's needed by the new tick step's intra-crate call.
3. `MismatchDetail::StateUnmet { predicate }` / `MismatchDetail::ObservationMissing { predicate }` is the payload variant when the mismatch fires from the overdue path (as opposed to guard-breach, which is `GuardInvalidator` — ticket 007). Populating the predicate requires reading the runtime `PlanExpectation` on the resolved step — hence the step-resolution step.

## Verification Layers

1. Step resolution (`current_plan` absent or `step_index` out of range → record transitions to `Expired`, no event emitted) → focused unit test in the new tick-step module.
2. Event emission (overdue `PlanStepCompletion`-basis record with valid step resolution → exactly one `EventTag::ExpectationMismatch` event with correct `expectation_kind` + `mismatch_detail`) → event-log delta assertion in focused test.
3. Discrepancy classification mapping (ExpectationKind class → Discrepancy variant per spec D6 step 3) → four focused unit tests, one per class.
4. Record state transition (post-emission state is `Resolved { outcome: ReturnedLate }`) → focused unit test.
5. Immediate-expectation behavior (Immediate expectation with `observe_by = tick+5` fires at tick+6 because grace elapses: sim transitions to `Overdue`, AI emits → this is the integration of the full sim+AI pipeline) → focused runtime test.
6. No regression on existing non-plan bases (`DutyAssignment`, `DeliveryCommitment`, `RoutineReturn`, `EscortObligation`, `SocialPromise` continue to transition to `Overdue` without AI-side emission for them) → existing `expectation_check.rs` tests (lines 157, 218, 232, 271, 312) stay green; no AI-side branching for those bases.

## What to Change

### 1. Promote `classify_discrepancy` to `pub(crate)`

In `crates/worldwake-ai/src/failure_handling.rs:133`, change `fn classify_discrepancy(...)` → `pub(crate) fn classify_discrepancy(...)`.

### 2. New module `crates/worldwake-ai/src/agent_tick/plan_step_expectations.rs`

```rust
use worldwake_core::{
    EntityId, EventTag, ExpectationBasis, ExpectationKindTag, ExpectationMismatchPayload,
    ExpectationOutcome, ExpectationRecord, ExpectationState, ExpectationStore,
    MaterializationTag, MismatchDetail, StatePredicate, Tick, World, WorldTxn,
};

pub(crate) fn tick_plan_step_mismatches(
    agent: EntityId,
    tick: Tick,
    world: &mut World,
    event_log: &mut EventLog,
) -> Result<(), TickError> {
    let store = world.get_component_expectation_store(agent)
        .ok_or(TickError::MissingStore)?;

    let current_plan = world.runtime_of(agent).and_then(|r| r.current_plan.as_ref());

    let overdue_ids: Vec<_> = store.records.iter()
        .filter(|(_, r)| matches!(r.basis, ExpectationBasis::PlanStepCompletion { .. }))
        .filter(|(_, r)| r.state == ExpectationState::Overdue)
        .map(|(id, _)| *id)
        .collect();

    for id in overdue_ids {
        let record = /* fetch from store */;
        let (step_index, kind_tag) = match record.basis {
            ExpectationBasis::PlanStepCompletion { step_index, kind_tag } => (step_index, kind_tag),
            _ => unreachable!(),
        };

        let step = current_plan
            .and_then(|p| p.steps.get(step_index as usize));

        let Some(step) = step else {
            // Plan moved on — expire the record, skip emission
            transition_record(world, agent, id, ExpectationState::Expired)?;
            continue;
        };

        let mismatch_detail = derive_mismatch_detail(step, kind_tag);
        let payload = ExpectationMismatchPayload {
            agent,
            goal_key: current_plan.map(|p| p.goal).unwrap_or_default(),
            step_index,
            expected_materializations: /* from step.expected_materializations */,
            expectation_kind: Some(kind_tag),
            mismatch_detail: Some(mismatch_detail),
        };

        emit_expectation_mismatch(event_log, tick, agent, payload);

        let discrepancy_kind = match kind_tag {
            ExpectationKindTag::Immediate => Discrepancy::PartialExecutionDrift,
            ExpectationKindTag::State => Discrepancy::BeliefContradicted,
            ExpectationKindTag::Informed => Discrepancy::MissingObservation,
            ExpectationKindTag::Regression => Discrepancy::BeliefContradicted,
        };
        crate::failure_handling::classify_discrepancy(
            agent, current_plan.map(|p| p.goal), discrepancy_kind, world, tick,
        )?;

        transition_record(world, agent, id, ExpectationState::Resolved {
            outcome: ExpectationOutcome::ReturnedLate,
        })?;
    }
    Ok(())
}
```

`derive_mismatch_detail` inspects the `PlanExpectation.kind` and produces the corresponding `MismatchDetail` variant (`StateUnmet { predicate }` for `ExpectationKind::State`/`Regression`, `ObservationMissing { predicate }` for `ExpectationKind::Informed`, `GuardInvalidator(...)` deferred to ticket 007's breach path).

### 3. Wire into agent tick entry

In `crates/worldwake-ai/src/agent_tick/mod.rs`, invoke `tick_plan_step_mismatches` between observation and planning phases — exact insertion point depends on the current tick phasing; verify at implementation time.

### 4. Declare the new module

`crates/worldwake-ai/src/agent_tick/mod.rs` gets `mod plan_step_expectations;`.

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (modify — promote `classify_discrepancy`)
- `crates/worldwake-ai/src/agent_tick/plan_step_expectations.rs` (new)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — module declaration + tick-sequence wiring)

## Out of Scope

- Guard-breach emission from the revalidation path (`archive/tickets/S114PLASTGUA-011.md` owns the landed `GuardInvalidator` `MismatchDetail` case on the AI execution/start-failure path).
- Changing sim-side `check_overdue_expectations` (unchanged — the whole point of F1's resolution).
- New golden scenarios (ticket 010).
- New `ExpectationOutcome` variant beyond `ReturnedLate`.

## Acceptance Criteria

### Tests That Must Pass

1. `plan_step_expectation_tick_emits_mismatch_on_overdue_state_record` — setup: inject a `PlanStepCompletion`-basis `Overdue` record into an agent's store; run the tick step; assert exactly one `DecisionEventPayload::ExpectationMismatch` event in the log with `expectation_kind: Some(ExpectationKindTag::State)` and `mismatch_detail: Some(StateUnmet { predicate })`.
2. `plan_step_expectation_tick_expires_record_when_plan_moved_on` — setup: record references `step_index = 5` but `current_plan` has only 3 steps; run tick; assert record transitions to `Expired` and no event is emitted.
3. `plan_step_expectation_tick_classifies_discrepancy_per_kind` — four parameterized tests covering the `ExpectationKindTag → Discrepancy` mapping from spec D6 step 3.
4. `plan_step_expectation_tick_transitions_to_resolved_returned_late` — post-tick state is `Resolved { outcome: ReturnedLate }`.
5. `immediate_expectation_fires_mismatch_at_tick_plus_one_over_grace` — end-to-end: adopt a plan at `Tick(0)` with an `Immediate` expectation `observe_by: Tick(5)`, agent profile `expectation_tolerance_ticks: 1`. At `Tick(7)`, sim's `check_overdue_expectations` transitions to `Overdue`; AI tick emits `ExpectationMismatch`. (Maps to spec test #4, adjusted for grace.)
6. Existing `check_overdue_expectations` tests at `expectation_check.rs:157,218,232,271,312` stay green.
7. Existing goldens (`golden_survival_*`, `golden_planner_pathology`, `golden_portfolio_planning`) stay green — agents without `PlanStepCompletion`-basis records see no behavioral change.

### Invariants

1. AI tick step only reads/writes `PlanStepCompletion`-basis records — never touches other bases.
2. Every `Overdue` → `Resolved` / `Expired` transition corresponds to at most one emitted event (never double-emitted; never emit on `Expired` path).
3. `classify_discrepancy` remains an AI-crate-internal helper (`pub(crate)`, not `pub`).
4. The tick step respects FND-26: all cross-system communication happens through `ExpectationStore` state + the event log, never a direct sim call.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/plan_step_expectations.rs` tests module (new) — acceptance tests 1-4.
2. Integration test in `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — end-to-end tick 5.
3. Existing `crates/worldwake-systems/src/expectation_check.rs` tests — no changes; must stay green.

### Commands

1. `cargo test -p worldwake-ai plan_step_expectations`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-systems expectation_check`
4. `cargo test -p worldwake-ai` (full AI-crate suite)
5. `scripts/verify.sh`
