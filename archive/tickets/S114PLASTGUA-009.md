# S114PLASTGUA-009: AI-side plan-step mismatch tick step — emission + discrepancy classification

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium-Large
**Engine Changes**: Yes — the AI agent tick now consumes `PlanStepCompletion`-basis `Overdue` records before planning, emits `EventTag::ExpectationMismatch`, records same-tick discrepancy memory, and transitions records to `Resolved { ReturnedLate }` / `Expired`; `classify_discrepancy` is now `pub(crate)` and shares a new reusable failure-recording helper.
**Deps**: `archive/tickets/S114PLASTGUA-003.md`, `archive/tickets/S114PLASTGUA-004.md`, `archive/tickets/S114PLASTGUA-005.md`, `archive/tickets/S114PLASTGUA-008.md`

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
10. `ExpectationMismatchPayload` widening from ticket 005 is already live in `worldwake-core`; this ticket does not touch core schema. The live owned surface is the AI overdue-record consumer path plus same-crate discrepancy-recording reuse.

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

### 2. Add the overdue-record consumer at the live agent-tick seam

Implement `process_overdue_plan_step_expectations` directly in `crates/worldwake-ai/src/agent_tick/mod.rs`, immediately after in-flight reconciliation and before the read/planning phase. The helper should:

- scan the agent's `ExpectationStore` for `PlanStepCompletion` + `Overdue` records
- resolve the current step from `AgentDecisionRuntime::current_plan`
- emit `ExpectationMismatch` through the existing `emit_expectation_mismatch` helper with populated `expectation_kind` and derived overdue-path `MismatchDetail`
- map `ExpectationKindTag` to the S114 discrepancy class, record that through the shared AI failure-recording helper, and mark the runtime dirty for same-tick replanning
- batch-transition processed records to `Resolved { outcome: ReturnedLate }` and stale records to `Expired`

### 3. Wire into the agent tick entry

Invoke `process_overdue_plan_step_expectations` in `crates/worldwake-ai/src/agent_tick/mod.rs` between in-flight reconciliation and the read/planning phase so the fresh discrepancy is visible to same-tick replanning.

### 4. Cover the live seam with focused tests

Add focused `agent_tick` tests for:

- populated mismatch emission on an overdue state expectation
- stale `step_index` expiry with no emission
- all four `ExpectationKindTag -> Discrepancy` mappings
- sim `check_overdue_expectations` handoff into the AI overdue consumer at the post-grace tick

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (modify — promote `classify_discrepancy`)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — overdue consumer wiring + helper)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — focused overdue consumer coverage)

## Out of Scope

- Guard-breach emission from the revalidation path (`archive/tickets/S114PLASTGUA-011.md` owns the landed `GuardInvalidator` `MismatchDetail` case on the AI execution/start-failure path).
- Changing sim-side `check_overdue_expectations` (unchanged — the whole point of F1's resolution).
- New golden scenarios (ticket 010).
- New `ExpectationOutcome` variant beyond `ReturnedLate`.

## Acceptance Criteria

### Tests That Must Pass

1. `overdue_plan_step_expectation_emits_mismatch_and_records_discrepancy` — inject an overdue state expectation, run the helper, assert the emitted payload, discrepancy memory entry, and `Resolved { ReturnedLate }` state.
2. `overdue_plan_step_expectation_expires_when_plan_moved_on` — inject an overdue record whose `step_index` is no longer valid, run the helper, assert `Expired` with no emission.
3. `overdue_plan_step_expectation_classifies_discrepancy_per_kind` — cover the `ExpectationKindTag -> Discrepancy` mapping for `Immediate`, `State`, `Informed`, and `Regression`.
4. `overdue_plan_step_expectation_processes_after_sim_marks_record_overdue` — prove the sim `check_overdue_expectations` `Active -> Overdue` handoff and the same-tick AI consumer after grace elapses.
6. Existing `check_overdue_expectations` tests at `expectation_check.rs:157,218,232,271,312` stay green.
7. Existing goldens (`golden_survival_*`, `golden_planner_pathology`, `golden_portfolio_planning`) stay green — agents without `PlanStepCompletion`-basis records see no behavioral change.

### Invariants

1. AI tick step only reads/writes `PlanStepCompletion`-basis records — never touches other bases.
2. Every `Overdue` → `Resolved` / `Expired` transition corresponds to at most one emitted event (never double-emitted; never emit on `Expired` path).
3. `classify_discrepancy` remains an AI-crate-internal helper (`pub(crate)`, not `pub`).
4. The tick step respects FND-26: all cross-system communication happens through `ExpectationStore` state + the event log, never a direct sim call.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — all four overdue-consumer focused tests above.
2. Existing `crates/worldwake-systems/src/expectation_check.rs` tests — no changes; must stay green.

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::tests::overdue_plan_step_expectation_emits_mismatch_and_records_discrepancy -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::tests::overdue_plan_step_expectation_expires_when_plan_moved_on -- --exact`
3. `cargo test -p worldwake-ai --lib agent_tick::tests::overdue_plan_step_expectation_classifies_discrepancy_per_kind -- --exact`
4. `cargo test -p worldwake-ai --lib agent_tick::tests::overdue_plan_step_expectation_processes_after_sim_marks_record_overdue -- --exact`
5. `cargo test -p worldwake-systems expectation_check`
6. `cargo test -p worldwake-ai`
7. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-22.

- Added `process_overdue_plan_step_expectations` to the live `agent_tick` pipeline so `PlanStepCompletion` overdue records now emit `DecisionEventPayload::ExpectationMismatch`, map to the S114 discrepancy classes, mark the runtime dirty for replanning, and transition to `Resolved { outcome: ReturnedLate }` or `Expired`.
- Promoted `classify_discrepancy` to `pub(crate)` and extracted `record_failure_classification` so the overdue consumer can reuse the existing blocker/discrepancy recording path without duplicating S109 memory semantics.
- Added focused `agent_tick` coverage for emission, stale-step expiry, class mapping, and the sim-to-AI overdue handoff after grace elapses.

## Deviations

- The overdue consumer landed in `crates/worldwake-ai/src/agent_tick/mod.rs` instead of a new `agent_tick/plan_step_expectations.rs` module. The crate already had a top-level `plan_step_expectations.rs` for expectation-store persistence helpers, and reusing the live `agent_tick` event/runtime seams kept the patch smaller and avoided splitting one concern across two similarly named modules.
- Ticket 005's `ExpectationMismatchPayload` widening was already present on the live branch, so this ticket did not modify `worldwake-core`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::overdue_plan_step_expectation_emits_mismatch_and_records_discrepancy -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::overdue_plan_step_expectation_expires_when_plan_moved_on -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::overdue_plan_step_expectation_classifies_discrepancy_per_kind -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::overdue_plan_step_expectation_processes_after_sim_marks_record_overdue -- --exact`
- Passed `cargo test -p worldwake-systems expectation_check`
- Passed `cargo test -p worldwake-ai`
- Passed `./scripts/verify.sh`
