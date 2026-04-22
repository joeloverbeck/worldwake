# S114PLASTGUA-008: Plan adoption wires `PlanStepCompletion` expectation lifecycle

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI plan adoption now writes `ExpectationRecord`s into the agent's `ExpectationStore`, and plan clear / failure / completion paths now expire or fulfill `PlanStepCompletion` records against the live single-active-plan runtime seam.
**Deps**: `archive/tickets/S114PLASTGUA-002.md`, `archive/tickets/S114PLASTGUA-003.md`, `archive/tickets/S114PLASTGUA-004.md`, `archive/tickets/S114PLASTGUA-006.md`

## Problem

S114 D4 persists plan-step expectations across tick boundaries by reusing the existing `ExpectationStore` infrastructure. The missing live slice was no longer the shared `ExpectationBasis` substrate; that basis already existed. The remaining gap was AI runtime lifecycle wiring: when a plan is adopted, the AI crate must write one `ExpectationRecord` per `PlanExpectation` with `basis = PlanStepCompletion { step_index, kind_tag }`, then explicitly fulfill or expire those records when the active plan advances, is replaced, or is discarded so ticket 009 does not read stale plan context.

## Assumption Reassessment (2026-04-22)

1. `ExpectationBasis::PlanStepCompletion { step_index, kind_tag }` was already present in `crates/worldwake-core/src/expectation.rs`, and the downstream candidate-generation / ranking exclusions for this basis were already landed. The ticket draft's earlier "add the basis to core" work was stale.
2. `AgentDecisionRuntime::current_plan` remains the authoritative AI seam: adoption and replacement happen in `crates/worldwake-ai/src/agent_tick/planning.rs`, while downstream invalidation or completion paths live across `agent_tick` execution / observation / runtime maintenance code.
3. `ExpectationStore` still lives in `crates/worldwake-core/src/expectation.rs` as the universal agent component that both sim and AI mutate through state, preserving the shared-store architecture required by the spec.
4. S114 spec D4 at `specs/S114-plan-step-guards.md:236-266` still defines the `ExpectationRecord` payload written at plan adoption:
   - `owner` = acting agent
   - `subject` = step's primary target (or the agent itself if untargeted)
   - `expected_place` = step's target place (or the agent's current place)
   - `deadline_tick` = `step.expected_complete_tick(adoption_tick)` adjusted by `expectation_tolerance_ticks`
   - `grace_ticks` = derived from profile tolerance (the spec says `grace_ticks` maps to `profile.expectation_tolerance_ticks`)
   - `basis` = `ExpectationBasis::PlanStepCompletion { step_index, kind_tag }`
   - `state` = `ExpectationState::Active`
   - `created_tick` = `adoption_tick`
5. Shared boundary under audit: every mutation of the single active `current_plan` lifecycle. The live code still runs only one active plan at a time, so no separate plan-id discriminator was necessary; replacement safely expires all active `PlanStepCompletion` records before writing the next plan's records.
6. Step-completion fulfillment belongs on the AI reconciliation side where a committed step advances `current_step_index`, not in sim's generic overdue check. That is the strongest live seam for "this step completed successfully."
7. No existing test exercised `ExpectationStore` through the plan-adoption lifecycle, so the required proof remained new focused helper tests plus one `agent_tick` reconciliation test.

## Architecture Check

1. Plan adoption writes remain symmetric to sim's `check_overdue_expectations`: both paths operate on the agent's own `ExpectationStore`. No cross-agent write was introduced.
2. The live contract stays minimal: records store only `step_index` + `kind_tag` via `ExpectationBasis::PlanStepCompletion`, while the richer `PlanExpectation` continues to live on runtime plan steps.
3. FND-26 still holds: sim transitions records to `Overdue`, AI writes / expires / fulfills them through the shared store, and the crates coordinate only through persisted state.

## Verification Layers

1. Plan-adoption write correctness (N expectations on a plan -> N records in store with matching `step_index`, `kind_tag`, `deadline_tick`) -> focused helper tests with a synthetic plan + profile.
2. Grace-tick derivation from profile (`expectation_tolerance_ticks = 4` -> record `grace_ticks = 4`) -> focused helper test.
3. Clear-on-replacement (adopting plan B after plan A expires prior `PlanStepCompletion` records before writing plan B) -> focused helper test.
4. Step-completion fulfillment (successful committed-step reconciliation transitions matching records to `Resolved { Fulfilled }`) -> focused `agent_tick` test.
5. No false positives when a plan has no expectations (`step.expectations.is_empty()`) -> focused helper test.
6. Single-layer behavior (AI crate only; sim-side `check_overdue_expectations` remains unchanged).

## What Changed

### 1. Added AI-local expectation lifecycle helpers

`crates/worldwake-ai/src/plan_step_expectations.rs` now owns the store mutations for this ticket:

- `write_plan_step_expectations(...)` writes one active record per runtime `PlanExpectation`
- `expire_plan_step_expectations(...)` expires active / overdue `PlanStepCompletion` records when the active plan is discarded
- `fulfill_plan_step_expectations(...)` resolves the completed step's matching records as `Fulfilled`
- `persist_expectation_store_update(...)` commits AI-side store mutations back through `WorldTxn`

### 2. Wired plan adoption and replacement

`crates/worldwake-ai/src/agent_tick/planning.rs` now centralizes the side effect at the real `current_plan` seam:

- adopting a new plan expires any prior `PlanStepCompletion` records, then writes the new plan's records
- clearing `current_plan` also expires those records
- the planning entrypoints now take `&mut World` so the store mutation can be persisted before runtime state changes

### 3. Wired non-adoption plan invalidation paths

The AI runtime now expires stale plan-step expectations whenever the active plan is abandoned outside normal replacement:

- current-step failure in `agent_tick/active_action.rs`
- recoverable travel blockage paths that clear the plan in `agent_tick/execution.rs`
- dead-agent, assumption-failure, patience-exhaustion, and pursuit-invalidation paths in `agent_tick/mod.rs`

### 4. Wired committed-step fulfillment

`crates/worldwake-ai/src/agent_tick/observation.rs` now fulfills the current step's `PlanStepCompletion` records immediately before advancing the runtime step index when committed-step reconciliation proves success.

## Files Touched

- `crates/worldwake-ai/src/plan_step_expectations.rs`
- `crates/worldwake-ai/src/lib.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/agent_tick/active_action.rs`
- `crates/worldwake-ai/src/agent_tick/execution.rs`
- `crates/worldwake-ai/src/agent_tick/observation.rs`
- `crates/worldwake-ai/src/agent_tick/mod.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`

## Out of Scope

- The AI-side overdue consumer step that reads `Overdue` records and classifies mismatches (ticket 009).
- Sim-side changes to `check_overdue_expectations` (none required).
- Any wider action-template expansion beyond the plan-step lifecycle seam already covered by sibling S114 tickets.

## Acceptance Criteria

### Tests That Must Pass

1. `plan_adoption_writes_one_record_per_expectation` passed.
2. `plan_adoption_sets_grace_ticks_from_profile` passed.
3. `plan_adoption_with_empty_expectations_writes_no_records` passed.
4. `plan_replacement_expires_prior_records` passed.
5. `step_completion_resolves_record_as_fulfilled` passed.
6. `committed_step_fulfills_matching_plan_step_expectations_in_world_store` passed.
7. `cargo test -p worldwake-ai`, `cargo clippy -p worldwake-ai --all-targets -- -D warnings`, and `./scripts/verify.sh` all passed.

### Invariants

1. Records are still written as `ExpectationState::Active`.
2. Expire / fulfill helpers touch only `ExpectationBasis::PlanStepCompletion` records.
3. The live single-active-plan seam means replacement safely expires prior plan-step records before writing the next plan; no additional plan-id field was required.
4. `grace_ticks` still widens from `profile.expectation_tolerance_ticks` at record-write time.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_step_expectations.rs` tests module (new) — plan-adoption, grace-tick, empty-plan, replacement-expiry, and step-fulfillment helper coverage.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` (modified) — committed-step reconciliation now proves fulfilled store state in the live `agent_tick` pipeline.

### Commands

1. `cargo test -p worldwake-ai plan_step_expectations`
2. `cargo test -p worldwake-ai`
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
4. `./scripts/verify.sh`

## Outcome

Completed: 2026-04-22. The ticket landed the missing AI runtime lifecycle wiring for `PlanStepCompletion` expectation records without changing the already-landed core substrate.

## Deviations

1. The draft ticket assumed new core `ExpectationBasis` work and broader downstream exhaustive-match fallout. Reassessment showed that substrate was already implemented, so the truthful boundary narrowed to AI lifecycle wiring only.
2. No explicit plan-id discriminator was added. The live runtime keeps a single active plan, and replacement now expires existing `PlanStepCompletion` records before writing the next plan, which is sufficient for this seam.

## Verification Result

1. Focused helper tests passed.
2. Live `agent_tick` reconciliation fulfillment coverage passed.
3. `cargo test -p worldwake-ai` passed.
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` passed.
5. `./scripts/verify.sh` passed.
