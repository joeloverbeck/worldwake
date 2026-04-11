# S91PLAPATGOL-001: Budget exhaustion blocks cross-location water acquisition

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — test-only
**Deps**: None

## Problem

The GOAP planner budget-exhausts on `AcquireCommodity{Water}` when water is at a remote location. Candidate explosion (1400–2600+ candidates per expansion) prevents multi-location plans (travel→queue→drink). No existing golden test reproduces this pathology in isolation. Without a targeted reproduction test, fixes cannot be verified and regressions cannot be prevented.

## Assumption Reassessment (2026-04-11)

1. **Harness infrastructure exists**: `GoldenHarness` at `crates/worldwake-ai/tests/golden_harness/mod.rs:1089`, `step_once()` at line 1196, `seed_actor_beliefs()` at line 158, and the custom-topology override pattern already used by `golden_exploration.rs`. All confirmed present.
2. **Trace types exist**: `PlanSearchOutcome::BudgetExhausted` at `crates/worldwake-ai/src/decision_trace.rs:897`, `PlanAttemptTrace` at line 860, `PlanningPipelineTrace` at line 227, `DecisionOutcome::Planning` at line 94. All confirmed present.
3. **File boundary restored to the live pathology family**: Reassessment against the motivating evidence in `reports/simulation-observer-report.md` showed this ticket needs an exact Dusty Trail / Thornwall Village slice, not the prototype-world remote-scarcity suite. `crates/worldwake-ai/tests/golden_planner_pathology.rs` is the honest boundary because it can own a custom topology and scenario-distilled planner pathology without mutating the generic prototype-world scarcity goldens.
4. **GoalKind surface**: `GoalKind::AcquireCommodity { commodity, purpose }` at `crates/worldwake-core/src/goal.rs:22` remains the live planner root under audit. The relevant proof boundary is the decision-trace `PlanAttemptTrace` outcome for `AcquireCommodity { commodity: Water, purpose: SelfConsume }`, consistent with `docs/planner-contracts.md` guidance to name the exact goal/operator surface rather than a vague planner failure.
5. **Scenario substrate corrected to the motivating report**: The observer report explicitly ties this pathology to `Dusty Trail` in `scenarios/cli-evaluation.ron`, with `Thornwall Village` and its `Village Well` two travel ticks away. The landed golden mirrors that place graph instead of substituting `VILLAGE_SQUARE` / `ORCHARD_FARM`.
6. **Isolation boundary**: The test distills the Dusty Trail condition to the smallest faithful slice that still reproduces the failure: the `cli-evaluation.ron` place graph, Dusty Trail origin, Thornwall Village well, and a guard-style profile boundary matching the report's “remote water from Dusty Trail” failure mode. This keeps the proof about planner budget exhaustion, not unrelated prototype-world scarcity behavior.

## Architecture Check

1. Pure test addition — no production code changes. Uses existing `GoldenHarness` patterns from `conformance_execution_budget.rs` and `golden_expectation.rs`. Decision-trace assertions are the strongest proof surface for planner pathologies (precision rule §6).
2. No backwards-compatibility shims introduced.

## Verification Layers

1. AcquireCommodity{Water} attempted -> decision trace (`PlanAttemptTrace.goal.kind`)
2. Budget exhaustion on that attempt -> decision trace (`PlanAttemptTrace.outcome == BudgetExhausted`)
3. No committed `drink` after the failed Dusty Trail water search -> action trace
4. Thirst rising -> authoritative world state (`HomeostaticNeeds.thirst`)
5. Single-layer ticket (golden E2E with trace inspection). Additional layer mapping not applicable — no production code changes.

## What to Change

### 1. Add a dedicated planner-pathology golden file

Create `crates/worldwake-ai/tests/golden_planner_pathology.rs` and reuse `GoldenHarness` plus the custom-topology override pattern already present in the golden suite.

### 2. Implement `budget_exhaustion_blocks_cross_location_water_acquisition`

Build a Dusty Trail slice distilled from `scenarios/cli-evaluation.ron`:
- places: `Thornwall Village`, `Dusty Trail`, `Eldergrove Forest`, `Hearthstone Inn`, `Golden Fields`
- edges matching the scenario graph, including `Thornwall Village <-> Dusty Trail` at `travel_ticks: 2`
- one `WorkstationTag::Well` with a water `ResourceSource` at `Thornwall Village`
- one AI guard agent at `Dusty Trail` with the report-relevant patrol/utility/perception boundary and elevated thirst so the remote-water search is exercised immediately
- local Dusty Trail beliefs plus inferred beliefs about `Thornwall Village` and the remote well

Run 60 ticks. Assert Phase 1 (bug reproduction):
1. At tick 0, planning trace contains a `PlanAttemptTrace` with `goal.kind` matching `AcquireCommodity { commodity: Water, .. }`
2. That attempt's outcome is `PlanSearchOutcome::BudgetExhausted { .. }`
3. No committed `drink` occurs during the run
4. After 60 ticks, the agent's thirst is higher than the initial value

Include Phase 2 assertions as commented-out code blocks with clear labels for when the fix lands.

## Files to Touch

- `crates/worldwake-ai/tests/golden_planner_pathology.rs`
- `specs/S91-planner-pathology-golden-tests.md`
- `tickets/S91PLAPATGOL-001.md`

## Out of Scope

- Fixing the budget exhaustion bug (separate spec/ticket)
- Phase 2 assertion activation (deferred until fix lands)
- Tests for 0-step plan loop or missing survival goals (S91PLAPATGOL-002, -003)
- Any production code changes

## Acceptance Criteria

### Tests That Must Pass

1. `budget_exhaustion_blocks_cross_location_water_acquisition` passes — confirms the pathology exists (Phase 1 assertions)
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No production code modified — test-only change
2. Deterministic reproduction: same seed produces same trace outcome every run

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs::budget_exhaustion_blocks_cross_location_water_acquisition` — reproduces Dusty Trail `AcquireCommodity{Water}` budget exhaustion against the `cli-evaluation.ron` place graph

### Commands

1. `cargo test -p worldwake-ai budget_exhaustion_blocks_cross_location_water_acquisition`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

Completed on 2026-04-11.

- Implemented `crates/worldwake-ai/tests/golden_planner_pathology.rs::budget_exhaustion_blocks_cross_location_water_acquisition` as a Dusty Trail / Thornwall Village reproduction distilled from `scenarios/cli-evaluation.ron` and `reports/simulation-observer-report.md`.
- Added a dedicated `golden_planner_pathology.rs` file with Scenario 142 metadata and generated-doc coverage for the planner-pathology family.
- Regenerated the golden inventory/docs so the new planner-pathology scenario appears in `docs/generated/`, including `docs/generated/golden-scenario-details/planner-pathology.md`.

The landed golden uses a custom topology matching the motivating place-graph slice, seeds a Guard Theron-style agent at `Dusty Trail`, places the `Village Well` water source at `Thornwall Village`, and proves the current failure at the strongest available surface:

1. `AcquireCommodity { commodity: Water, .. }` is attempted
2. the attempt reaches `PlanSearchOutcome::BudgetExhausted`
3. no committed `drink` occurs
4. thirst keeps rising over the run

No production code changed.

## Deviations

- The initial prototype-world `VillageSquare` approximation was rejected during implementation because it did not reproduce the motivating observer condition. Reassessment narrowed the ticket back to the exact Dusty Trail / Thornwall Village slice from `cli-evaluation.ron`.
- The landed file boundary is `crates/worldwake-ai/tests/golden_planner_pathology.rs` rather than an existing prototype-world scarcity file, because the faithful reproduction required a dedicated custom-topology golden.

## Verification Result

1. `cargo test -p worldwake-ai budget_exhaustion_blocks_cross_location_water_acquisition --test golden_planner_pathology -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo clippy --workspace --all-targets -- -D warnings`
