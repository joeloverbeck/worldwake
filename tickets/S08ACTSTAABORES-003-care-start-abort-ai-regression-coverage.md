# S08ACTSTAABORES-003: Prove Care Start Abort Flows Through AI Failure Handling

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` targeted regression coverage, with minimal harness support only if required
**Deps**: `S08ACTSTAABORES-001`, `S08ACTSTAABORES-002`, `S08ACTSTAABORES-004`, `specs/S08-action-start-abort-resilience.md`

## Problem

S08 is not complete until the authoritative-to-AI path is proven end to end for care actions. After the authoritative fixes land, AI must still expose heal affordances, generate `TreatWounds`, find a plan, survive a start-time `AbortRequested(TargetHasNoWounds | TargetLacksWounds)` without crashing, and route that failure through normal blocker recording and replanning behavior.

## Assumption Reassessment (2026-03-19)

1. The relevant AI failure surface is still `crates/worldwake-ai/src/failure_handling.rs::handle_plan_failure()`, and existing focused coverage in `failure_handling::tests::handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty` proves the blocker/dirty-plan path directly.
2. `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/search.rs` already have real coverage for care preconditions: `candidate_generation::tests::directly_observed_wounded_other_emits_treat_wounds`, `candidate_generation::tests::self_wounded_emits_treat_wounds_with_medicine`, and multiple search tests establish the broader planning substrate. This ticket should not duplicate that lower-layer coverage unnecessarily.
3. Existing golden care coverage in `crates/worldwake-ai/tests/golden_care.rs` includes `golden_healing_wounded_agent`, `golden_self_care_with_medicine`, and `golden_care_goal_invalidation_when_patient_heals`, but none of those explicitly pin the start-abort path created by an authoritative wound disappearance between planning and action start.
4. Existing comments in `crates/worldwake-ai/tests/golden_emergent.rs` around `no_recovery_combat_profile()` document the natural-recovery race, and the S08 spec explicitly says this ticket must not require blanket removal of that profile from unrelated goldens.
5. This is a runtime `agent_tick` regression, not a candidate-generation-only or pure golden-E2E ticket. The harness boundary should stay as narrow as possible, but it must include the full action registries needed for care action start and failure propagation.
6. The relevant failure reason mapping already exists in `failure_handling.rs`: `ActionAbortRequestReason::TargetLacksWounds` and `TargetHasNoWounds` are already mapped into blocker derivation inputs. This ticket verifies that the runtime path reaches that logic instead of crashing earlier.
7. No mismatch found between the spec and current AI/runtime code. The gap is missing regression coverage for the specific start-abort scenario.

## Architecture Check

1. Adding a targeted AI regression is cleaner than weakening unrelated emergent goldens because it proves the intended failure path directly without re-baselining broader scenarios that currently use zero natural recovery for lawful scenario isolation.
2. No backwards-compatibility shim is introduced; this ticket should validate the current AI failure pipeline rather than adding alternate care-specific recovery logic.

## Verification Layers

1. Heal affordance remains exposed when the agent lawfully observes wounds and controls Medicine -> existing focused coverage named in Assumption Reassessment; no new assertion surface required here
2. `TreatWounds` candidate generation remains present before the failure scenario -> decision trace or focused runtime assertion in the new AI regression
3. Planned care action hits authoritative start rejection without crashing the simulation -> action trace and focused runtime/golden AI regression
4. Start rejection clears the failed plan, records a blocker, and marks runtime dirty -> authoritative runtime state plus blocked-intent assertions in the new AI regression
5. Later world evolution should not be used as the only proof of failure handling; the regression must inspect the failed-start/blocker layer directly.

## What to Change

### 1. Add a targeted care start-abort regression

Add a `worldwake-ai` test that constructs a lawful care plan, removes the target wounds between planning and start, and proves that the next step:
- does not crash,
- records a start failure rather than a hard engine error,
- routes through normal plan-failure handling,
- leaves the agent ready to replan.

Prefer a focused runtime or golden harness test that can inspect blocked memory and traces directly.

### 2. Assert the right layer, not just later symptoms

The new test should assert at least:
- pre-failure `TreatWounds` selection or candidate presence,
- same-tick `ActionTraceKind::StartFailed` or equivalent start-failure evidence,
- blocked-intent memory / runtime-dirty state after failure.

If `S08ACTSTAABORES-004` lands first, this regression should assert against that canonical structured start-failure handoff rather than freezing the current generic missing-in-flight reconciliation path as the intended long-term architecture.

### 3. Preserve existing care goldens intentionally

Do not remove `no_recovery_combat_profile()` from unrelated `golden_emergent.rs` scenarios unless a specific assertion in this ticket demonstrates that the profile is now masking the exact regression under test. If that happens, document the isolated change explicitly.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify or add targeted regression here)
- `crates/worldwake-ai/tests/golden_care.rs` (modify only if this is the narrower harness for asserting the failure path)
- `crates/worldwake-ai/src/agent_tick.rs` (modify only if the regression uncovers a real runtime bug beyond missing coverage)
- `crates/worldwake-ai/src/failure_handling.rs` (modify only if the regression uncovers a real blocker-recording bug beyond missing coverage)

## Out of Scope

- `worldwake-sim` recoverable start-failure classification implementation
- `worldwake-systems` heal lifecycle or Medicine consumption semantics
- Blanket edits to unrelated emergent goldens that currently use zero natural recovery
- Broad planner/ranking changes unrelated to the start-abort failure path

## Acceptance Criteria

### Tests That Must Pass

1. A new targeted `worldwake-ai` regression proving a care start `AbortRequested(TargetHasNoWounds | TargetLacksWounds)` does not crash and records normal plan failure state
2. `cargo test -p worldwake-ai failure_handling::tests::handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty`
3. `cargo test -p worldwake-ai golden_healing_wounded_agent`
4. `cargo test -p worldwake-ai golden_self_care_with_medicine`
5. `cargo test -p worldwake-ai golden_care_goal_invalidation_when_patient_heals`

### Invariants

1. The AI planner still reaches lawful care intent generation and plan search before the authoritative start rejection occurs.
2. A care start-time wound disappearance is handled as plan failure plus replanning input, not as a simulation crash.
3. Existing scenario-isolation choices such as `no_recovery_combat_profile()` remain in place unless this ticket proves a narrower, explicit adjustment is required.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — add a narrow runtime/golden regression for start-time care abort handling, blocked intent recording, and non-crashing continuation.
2. `crates/worldwake-ai/tests/golden_care.rs` — optional only if the care harness is the cleanest place to inspect traces and blocked state without duplicating helpers.
3. `None — production AI code should change only if the regression reveals a real bug after `S08ACTSTAABORES-001` and `S08ACTSTAABORES-002` land.`

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai failure_handling::tests::handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty`
3. `cargo test -p worldwake-ai golden_healing_wounded_agent`
4. `cargo test -p worldwake-ai golden_self_care_with_medicine`
5. `cargo test -p worldwake-ai golden_care_goal_invalidation_when_patient_heals`
