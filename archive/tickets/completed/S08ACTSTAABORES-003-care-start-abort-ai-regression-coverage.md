# S08ACTSTAABORES-003: Prove Care Start Abort Flows Through AI Failure Handling

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` golden regression coverage and golden E2E docs alignment; production code only if the regression exposes a real remaining bug
**Deps**: `S08ACTSTAABORES-001`, `S08ACTSTAABORES-002`, `S08ACTSTAABORES-004`, `specs/S08-action-start-abort-resilience.md`

## Problem

S08 still needs one missing proof layer: a real AI-driven care scenario where `TreatWounds` is lawfully selected, authoritative world state changes before `heal` can start, and the resulting recoverable pre-start failure is handled as structured start failure plus normal blocker recording instead of crashing or silently disappearing.

## Assumption Reassessment (2026-03-19)

1. `crates/worldwake-sim/src/tick_step.rs::is_best_effort_start_failure()` already treats `ActionError::AbortRequested(_)` as recoverable, and focused coverage already exists:
   - `tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick`
   - `tick_step::tests::strict_request_propagates_abort_requested_start_failure`
2. `crates/worldwake-systems/src/combat.rs` has already landed the heal lifecycle changes the older ticket text implied were still pending:
   - `start_heal()` is validation-only and initializes `ActionState::Heal { medicine_spent: false }`
   - `tick_heal()` spends Medicine on the first successful treatment tick, not on start
   - focused coverage already exists in `combat::tests::heal_lifecycle_consumes_medicine_and_reduces_bleeding_and_severity` and `combat::tests::heal_abort_before_first_treatment_tick_preserves_medicine`
3. `crates/worldwake-ai/src/agent_tick.rs::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons` already proves the AI runtime consumes structured scheduler start-failure records, clears stale in-flight state, and persists blocked intent memory. That test is not the missing end-to-end proof because it injects the scheduler failure directly rather than producing it through the real care action path.
4. `crates/worldwake-ai/src/failure_handling.rs::handle_plan_failure()` remains the relevant blocker-recording surface, and `failure_handling::tests::handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty` is still the focused baseline for the blocker/dirty-runtime path.
5. Candidate-generation coverage for care already exists and should not be duplicated:
   - `candidate_generation::tests::directly_observed_wounded_other_emits_treat_wounds`
   - `candidate_generation::tests::self_wounded_emits_treat_wounds_with_medicine`
6. Existing golden care coverage in `crates/worldwake-ai/tests/golden_care.rs` proves normal third-party care, self-care, acquisition, the indirect-report gate, and care-goal invalidation, but none of those scenarios force a lawful wound disappearance after AI intent generation and before authoritative `heal` start.
7. Existing comments and setup in `crates/worldwake-ai/tests/golden_emergent.rs` still correctly explain why unrelated goldens use `no_recovery_combat_profile()` for scenario isolation. This ticket should not broaden scope into changing those scenarios unless the new regression proves an actual masking problem.
8. Additional current-architecture nuance: when wounds disappear before authoritative input drain, the real engine rejects the queued `heal` at the shared affordance/precondition layer as `PreconditionFailed("TargetHasWounds(0)")`. That is cleaner than forcing the race deeper into `start_heal()`. `AbortRequested(TargetHasNoWounds | TargetLacksWounds)` remains a valid lower-layer failure surface and is already covered by focused runtime tests, but the new golden should assert the real pre-start failure surface the engine now uses.
9. Mismatch found and corrected: the old ticket scope still treated `worldwake-sim` start-failure classification, `worldwake-systems` heal medicine timing, and AI structured failure handoff as missing work. Those are already implemented and covered. The remaining gap is one golden/runtime regression that drives the full care pre-start failure path through the real AI loop.

## Architecture Check

1. Adding one narrow golden regression is cleaner than reopening already-solved production code or duplicating focused unit coverage. The architecture is already in the right shape: recoverable authoritative start failure in `worldwake-sim`, canonical AI failure handoff in `worldwake-ai`, and first-effect medicine spending in `worldwake-systems`.
2. The current shared precondition rejection is architecturally better than forcing this race through a care-specific handler abort. If the world has already invalidated `TargetHasWounds(0)` before input drain, rejecting at the shared affordance gate keeps the contract generic and extensible.
3. The right proof is mixed-layer:
   - decision trace for lawful `TreatWounds` generation/selection,
   - action trace for `StartFailed`,
   - authoritative blocked-intent/world state for failure persistence.
   A broader emergent scenario would be weaker because it could pass through unrelated lawful branches.
4. No backwards-compatibility shim, alias path, or care-specific failure workaround should be introduced. If the new regression exposes a real bug, fix the shared runtime path directly.

## Verification Layers

1. Care goal exists before the race -> decision trace in the new golden regression
2. Planned `heal` step is selected through the real action registry -> decision trace in the new golden regression
3. Wounds disappear before authoritative action start and `heal` fails to start recoverably -> action trace `StartFailed` in the new golden regression
4. Failure is routed through normal blocker recording rather than crash or silent drop -> authoritative `BlockedIntentMemory` assertion in the new golden regression
5. Existing structured-failure plumbing remains intact at focused layers -> existing `worldwake-sim`, `worldwake-systems`, `agent_tick`, and `failure_handling` focused tests named below

## What to Change

### 1. Add one real AI care start-abort regression

Add a targeted golden/runtime test that:
- runs the real AI loop,
- lets the healer lawfully generate/select `TreatWounds`,
- removes the patient's wounds after input production but before authoritative input drain,
- proves the resulting `heal` request becomes a recoverable pre-start failure at the current authoritative boundary,
- proves the tick completes without crashing,
- proves blocker state is recorded and the agent is left in a normal replannable state.

### 2. Assert the strongest mixed-layer surfaces

The new regression should assert all of the following directly:
- `TreatWounds { patient }` appears in decision-trace candidates or the selected plan
- the selected next step is `heal` (or equivalent care-domain `PlannerOpKind::Heal` proof)
- action trace records `ActionTraceKind::StartFailed`
- blocked intent memory for the healer is populated after the failed start

Do not rely only on downstream symptoms such as "later the healer does nothing" or "the world keeps running."

### 3. Update golden E2E docs if the new scenario lands

Because this ticket adds a new `golden_*` care scenario, review and update the golden E2E docs so inventory counts and care-scenario descriptions remain current:
- `docs/golden-e2e-scenarios.md`
- `docs/golden-e2e-coverage.md`

## Files to Touch

- `tickets/S08ACTSTAABORES-003-care-start-abort-ai-regression-coverage.md` (update first, then complete/archive)
- `crates/worldwake-ai/tests/golden_care.rs` (modify)
- `docs/golden-e2e-scenarios.md` (modify if golden scenario is added)
- `docs/golden-e2e-coverage.md` (modify if golden scenario is added)
- `crates/worldwake-ai/src/agent_tick.rs` (modify only if the new regression exposes a real remaining runtime bug)
- `crates/worldwake-sim/src/tick_step.rs` (modify only if the new regression exposes a real remaining start-failure bug)
- `crates/worldwake-systems/src/combat.rs` (modify only if the new regression exposes a real remaining heal lifecycle bug)

## Out of Scope

- Re-implementing the already-landed recoverable start-failure classification
- Reworking heal input consumption timing that is already aligned with the current S08 spec
- Blanket edits to unrelated emergent goldens using `no_recovery_combat_profile()`
- Candidate-generation, ranking, or planner redesign unrelated to this specific regression surface

## Acceptance Criteria

### Tests That Must Pass

1. New `worldwake-ai` golden/runtime regression proving a real care start abort is recorded as `StartFailed` and persists blocked intent memory without crashing
2. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick`
3. `cargo test -p worldwake-sim tick_step::tests::strict_request_propagates_abort_requested_start_failure`
4. `cargo test -p worldwake-systems combat::tests::heal_lifecycle_consumes_medicine_and_reduces_bleeding_and_severity`
5. `cargo test -p worldwake-systems combat::tests::heal_abort_before_first_treatment_tick_preserves_medicine`
6. `cargo test -p worldwake-ai agent_tick::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons`
7. `cargo test -p worldwake-ai failure_handling::tests::handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty`
8. `cargo test -p worldwake-ai golden_healing_wounded_agent`
9. `cargo test -p worldwake-ai golden_self_care_with_medicine`
10. `cargo test -p worldwake-ai golden_care_goal_invalidation_when_patient_heals`
11. `cargo test --workspace`
12. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-19
- **What actually changed**: Added a new `golden_care` regression plus deterministic replay companion covering the real AI-driven care race where a lawful `TreatWounds` plan is selected, the patient's wounds disappear before authoritative input drain, the queued `heal` request records a recoverable `StartFailed`, and the next AI tick persists blocked intent memory. Updated `docs/golden-e2e-scenarios.md` and `docs/golden-e2e-coverage.md` to include the new care scenario and revised golden counts.
- **Deviations from original plan**: No production code changes were needed. The most important reassessment result was architectural: the real engine rejects this race at the shared affordance/precondition boundary as `PreconditionFailed("TargetHasWounds(0)")`, not as a care-handler `AbortRequested(...)`. That is cleaner and more extensible than forcing the rejection deeper into `start_heal()`, so the finished regression asserts the actual current authoritative boundary while retaining the existing focused tests for the lower-layer `AbortRequested(...)` path.
- **Verification results**: `cargo test -p worldwake-sim tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick -- --exact`, `cargo test -p worldwake-sim tick_step::tests::strict_request_propagates_abort_requested_start_failure -- --exact`, `cargo test -p worldwake-systems combat::tests::heal_lifecycle_consumes_medicine_and_reduces_bleeding_and_severity -- --exact`, `cargo test -p worldwake-systems combat::tests::heal_abort_before_first_treatment_tick_preserves_medicine -- --exact`, `cargo test -p worldwake-ai agent_tick::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons -- --exact`, `cargo test -p worldwake-ai failure_handling::tests::handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty -- --exact`, `cargo test -p worldwake-ai --test golden_care`, `cargo test --workspace`, and `cargo clippy --workspace` all passed.

### Invariants

1. The care AI path still lawfully emits and selects `TreatWounds` before the authoritative rejection.
2. A pre-start wound disappearance is handled as recoverable start failure plus blocker persistence, not as a simulation crash.
3. The current shared `PreconditionFailed("TargetHasWounds(0)")` surface is accepted as the authoritative boundary for this race when invalidation happens before input drain; focused tests continue to cover the deeper handler `AbortRequested(...)` path.
4. Existing focused start-failure and heal-lifecycle coverage remains the source of truth for those lower-layer contracts; this ticket adds the missing end-to-end proof instead of duplicating them.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_care.rs` — add a focused golden regression for "planned heal, wounds disappear before input drain, `StartFailed` recorded, blocked intent persisted"

### Rationale

1. `crates/worldwake-ai/tests/golden_care.rs` — closes the one missing mixed-layer gap between already-covered focused plumbing and existing broader care goldens by proving the real AI loop reaches the canonical start-failure path at the actual current authoritative boundary.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick`
3. `cargo test -p worldwake-sim tick_step::tests::strict_request_propagates_abort_requested_start_failure`
4. `cargo test -p worldwake-systems combat::tests::heal_lifecycle_consumes_medicine_and_reduces_bleeding_and_severity`
5. `cargo test -p worldwake-systems combat::tests::heal_abort_before_first_treatment_tick_preserves_medicine`
6. `cargo test -p worldwake-ai agent_tick::tests::planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons`
7. `cargo test -p worldwake-ai failure_handling::tests::handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty`
8. `cargo test -p worldwake-ai golden_healing_wounded_agent`
9. `cargo test -p worldwake-ai golden_self_care_with_medicine`
10. `cargo test -p worldwake-ai golden_care_goal_invalidation_when_patient_heals`
11. `cargo test --workspace`
12. `cargo clippy --workspace`
