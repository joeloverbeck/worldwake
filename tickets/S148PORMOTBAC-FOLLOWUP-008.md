# S148PORMOTBAC-FOLLOWUP-008: Split self-care pressure admission from durable source-failure accounting

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — likely `crates/worldwake-ai/src/agent_tick/mod.rs`, source reliability/ranking surfaces, and self-care acquire feasibility/planning surfaces.
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-007.md`, `archive/tickets/S148PORMOTBAC-FOLLOWUP-002.md`, `archive/tickets/S148PORMOTBAC-FOLLOWUP-003.md`

## Problem

`archive/tickets/S148PORMOTBAC-FOLLOWUP-007.md` proved the remaining S148 contracts are not one clean implementation seam. Two attempted narrow fixes each satisfied a local premise but broke required survival goldens:

1. Removing pressure-only self-care `AcquireCommodity` probe admission rejected ungrounded self-care acquire goals, but regressed baseline survival and still did not make `golden_survival_trade` healthy.
2. Preventing search-only same-goal sibling failures from persisting as durable `SourceReliability.failed_attempts` fixed the preferences familiar-failure premise locally, but regressed baseline survival because the durable search-failure signal still participates in ranking/replanning.

The next implementation must separate the internal planning/ranking memory needed for survival from the public/familiar source-failure accounting asserted by preferences, while preserving lawful pressure-driven self-care planning until a concrete replacement feasibility substrate exists.

## Assumption Reassessment (2026-05-18)

1. The shared boundary under audit is not generic source reliability. It is the contract between `OpportunityExpectationFailureIncident`, durable `SourceReliability` records, source-composite ranking, and golden-visible familiar source-failure accounting.
2. `ExpectationFailurePhase::Search` plus `ExpectationFailureCause::SameGoalSearchInfeasibleWhileSiblingSucceeded` is a transient same-goal search/ranking signal. Treating it as a public failed source attempt makes `golden_survival_preferences` report `familiar_failed_attempts == 1`.
3. Removing that same durable record outright regresses `golden_survival_baseline`: `Agent A` thirst exceeds critical for 550 ticks and enters a stuck idle window from ticks 1086-1326. The signal is therefore still behaviorally meaningful to survival planning.
4. Probe-only self-care tightening is not a complete substitute. Removing the pressure-only self-care acquire admission regresses baseline (`Agent C` no `Eat`; `Agent B` critical hunger for 1179 ticks) and leaves trade unhealthy (`Buyer Nila` critical hunger in the substitute market branch).
5. The live goal family under test remains `GoalKind::AcquireCommodity { purpose: SelfConsume, .. }` competing with `GoalKind::ConsumeOwnedCommodity { .. }` and trade/resource acquisition branches under per-tick candidate caps.
6. Adjacent contradiction classification: durable search-failure memory vs preferences accounting is required in-scope; broader probe/feasibility tightening is separate and must not be changed until the memory/accounting split has a green baseline.
7. Mismatch + correction: do not implement another probe-only or source-reliability-only deletion. The required substrate is a split representation or reporting rule that preserves ranking behavior while excluding search-only sibling failures from familiar failed-attempt accounting.

## Architecture Check

1. A split memory/accounting design is cleaner than deleting the search-failure incident because baseline proves the incident still carries useful planning information.
2. It is also cleaner than preserving the status quo because preferences proves the same incident is not a truthful durable failed source attempt from the agent's perspective.
3. No backward-compatibility shim should be introduced; update the current memory/reporting contract directly.

## Verification Layers

1. Search-only same-goal sibling failure remains available to ranking/replanning -> focused unit/runtime test around source-composite or agent tick behavior.
2. Search-only same-goal sibling failure is excluded from familiar failed-source accounting -> focused unit test and `golden_survival_preferences`.
3. Pressure-driven self-care baseline remains healthy -> `golden_survival_baseline`.
4. Remote loose-lot acquisition remains admitted -> `acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`.
5. Trade substitute branch remains healthy -> `golden_survival_trade`.

## What to Change

### 1. Split source-failure representation or reporting

Introduce a distinction between durable planning/ranking search-failure memory and golden-visible familiar failed-attempt accounting, or change the reporting layer so search-only same-goal sibling failures no longer count as familiar failed attempts while preserving their planning effect.

### 2. Preserve pressure-driven self-care admission

Do not remove the current pressure-only self-care acquire admission unless this ticket also adds a replacement feasibility/ranking substrate that passes baseline and trade in the same run.

### 3. Add focused coverage before golden iteration

Write the focused test that proves search-only sibling failure remains behaviorally available to planning but no longer increments familiar failed-attempt accounting.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify, likely)
- `crates/worldwake-ai/src/source_composite.rs` or related source ranking/reporting surface (modify, if needed)
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (regenerate if diagnostics output changes)

## Out of Scope

- Reintroducing FOLLOWUP-003's world-state `can_control` filters.
- Removing pressure-only self-care acquire admission without replacement proof.
- Scenario `.ron` changes that mask the planner/accounting contradiction.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --lib`
2. `cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1`
3. `cargo test --release -p worldwake-ai --test golden_survival_scattered -- --ignored --test-threads=1`
4. `cargo test --release -p worldwake-ai --test golden_survival_preferences -- --ignored --test-threads=1`
5. `cargo test --release -p worldwake-ai --test golden_survival_trade -- --ignored --test-threads=1`
6. `cargo test --release -p worldwake-ai --test golden_survival_ask_consult -- --ignored --test-threads=1`
7. `cargo test --release -p worldwake-ai --test golden_survival_patrol -- --ignored --test-threads=1`

### Invariants

1. Search-only same-goal sibling failure is not counted as a familiar failed source attempt unless it corresponds to observed local absence/depletion or another concrete source failure.
2. The planning/ranking signal currently needed by baseline survival is preserved or replaced by a focused-proven equivalent.
3. Remote loose unowned `ItemLot` acquisition remains lawful over the travel horizon.

## Test Plan

### New/Modified Tests

1. New focused source-failure accounting/ranking test — proves the split between planning memory and familiar failed-attempt accounting.
2. Existing focused remote acquisition/search tests remain unchanged.

### Commands

1. `cargo test -p worldwake-ai --lib`
2. `cargo test --release -p worldwake-ai --test golden_survival_preferences -- --ignored --test-threads=1`
3. Full golden set listed in Acceptance Criteria.
