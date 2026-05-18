# S148PORMOTBAC-FOLLOWUP-008: Split self-care pressure admission from durable source-failure accounting

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `crates/worldwake-ai/src/planning_state.rs`, `crates/worldwake-ai/src/failure_handling.rs`, `crates/worldwake-ai/src/candidate_generation.rs`, and `crates/worldwake-ai/tests/golden_survival_preferences.rs`.
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-007.md`, `archive/tickets/S148PORMOTBAC-FOLLOWUP-002.md`, `archive/tickets/S148PORMOTBAC-FOLLOWUP-003.md`

## Problem

`archive/tickets/S148PORMOTBAC-FOLLOWUP-007.md` proved the remaining S148 contracts are not one clean implementation seam. Two attempted narrow fixes each satisfied a local premise but broke required survival goldens:

1. Removing pressure-only self-care `AcquireCommodity` probe admission rejected ungrounded self-care acquire goals, but regressed baseline survival and still did not make `golden_survival_trade` healthy.
2. Preventing search-only same-goal sibling failures from persisting as durable `SourceReliability.failed_attempts` fixed the preferences familiar-failure premise locally, but regressed baseline survival because the durable search-failure signal still participates in ranking/replanning.

This ticket separates the internal planning/ranking memory needed for survival from the public/familiar source-failure accounting asserted by preferences, while preserving lawful pressure-driven self-care planning.

## Assumption Reassessment (2026-05-18)

1. The shared boundary under audit is not generic source reliability. It is the contract between `OpportunityExpectationFailureIncident`, durable `SourceReliability` records, source-composite ranking, and golden-visible familiar source-failure accounting.
2. `ExpectationFailurePhase::Search` plus `ExpectationFailureCause::SameGoalSearchInfeasibleWhileSiblingSucceeded` is a transient same-goal search/ranking signal. Treating it as a public failed source attempt makes `golden_survival_preferences` report `familiar_failed_attempts == 1`.
3. Removing that same durable record outright regresses `golden_survival_baseline`: `Agent A` thirst exceeds critical for 550 ticks and enters a stuck idle window from ticks 1086-1326. The signal is therefore still behaviorally meaningful to survival planning.
4. Probe-only self-care tightening is not a complete substitute. Removing the pressure-only self-care acquire admission regresses baseline (`Agent C` no `Eat`; `Agent B` critical hunger for 1179 ticks) and leaves trade unhealthy (`Buyer Nila` critical hunger in the substitute market branch).
5. The live goal family under test remains `GoalKind::AcquireCommodity { purpose: SelfConsume, .. }` competing with `GoalKind::ConsumeOwnedCommodity { .. }` and trade/resource acquisition branches under per-tick candidate caps.
6. Adjacent contradiction classification: durable search-failure memory vs preferences accounting is required in-scope; broader probe/feasibility tightening is separate and must not be changed unless the memory/accounting split still leaves an executable-planner mismatch.
7. Mismatch + correction: do not implement another probe-only or source-reliability-only deletion. The landed substrate is a reporting rule that preserves ranking behavior while excluding search-only sibling failures from familiar failed-attempt accounting.
8. Live verification exposed a second, adjacent planner/runtime mismatch: initially local loose lots with no ownership belief were considered controllable by the search model even when the authoritative target was other-owned and `pick_up` failed `TargetUnownedOrActorControls(0)`. The completed fix keeps remote loose-lot acquisition lawful after planned travel while requiring initially local loose lots to satisfy the live control predicate.

## Architecture Check

1. A split memory/accounting design is cleaner than deleting the search-failure incident because baseline proves the incident still carries useful planning information.
2. It is also cleaner than preserving the status quo because preferences proves the same incident is not a truthful durable failed source attempt from the agent's perspective.
3. No backward-compatibility shim should be introduced; update the current memory/reporting contract directly.

## Verified Layers

1. Search-only same-goal sibling failure remains available to ranking/replanning because `SourceReliability` durability was preserved; only preferences' public failed-attempt read was narrowed.
2. Search-only same-goal sibling failure is excluded from familiar failed-source accounting by `familiar_failed_attempt_accounting_excludes_search_only_sibling_failures` and `golden_survival_preferences`.
3. Pressure-driven self-care baseline remains healthy via `golden_survival_baseline`.
4. Remote loose-lot acquisition remains admitted via `acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence` and `can_control_ref_allows_remote_loose_lot_after_hypothetical_travel`.
5. Trade substitute branch remains healthy via `golden_survival_trade`.

## Landed Changes

### 1. Split source-failure reporting

The preferences golden now derives familiar failed-attempt accounting from `SourceExpectationFailure` payloads that represent concrete observation/candidate-generation absence or depletion. Search-only same-goal sibling failures remain durable planning/ranking memory but no longer count as public failed-source attempts.

### 2. Preserve pressure-driven self-care admission and legal pickup planning

Pressure-driven self-care acquire admission remains in place. The planner now distinguishes initially local loose lots from remote loose lots after hypothetical travel: initially local loose lots must satisfy the live control predicate, while remote loose lots can still become lawful after planned travel.

### 3. Focused coverage

Focused tests cover public failed-attempt accounting, known other-owned loose-lot candidate suppression, unlawful pickup failure classification, and remote loose-lot planning after travel.

## Landed Files

- `crates/worldwake-ai/tests/golden_survival_preferences.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/failure_handling.rs`
- `crates/worldwake-ai/src/planning_state.rs`

## Out of Scope

- FOLLOWUP-003's broad early world-state `can_control` filter was not reintroduced.
- Pressure-only self-care acquire admission was not removed.
- Scenario `.ron` files were not changed.

## Acceptance Result

### Tests Passed

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

## Test Plan Result

### Added/Modified Tests

1. `familiar_failed_attempt_accounting_excludes_search_only_sibling_failures`
2. `local_other_owned_loose_water_does_not_emit_pickup_acquire_goal`
3. `handle_plan_failure_records_unlawful_pickup_as_exact_target_legal_discrepancy`
4. `can_control_ref_rejects_initially_local_uncontrolled_loose_lot`
5. `can_control_ref_allows_remote_loose_lot_after_hypothetical_travel`

### Commands Run

1. `cargo test -p worldwake-ai --lib`
2. `cargo test --release -p worldwake-ai --test golden_survival_preferences -- --ignored --test-threads=1`
3. Full golden set listed in Acceptance Result.

## Outcome

Completed on 2026-05-18.

- Preserved durable source-reliability planning memory while narrowing preferences' familiar failed-attempt accounting to concrete public source failures.
- Kept pressure-driven self-care acquire admission intact.
- Fixed the adjacent executable-planner mismatch that let initially local, authoritatively other-owned loose lots satisfy `MoveCargo`/`pick_up` planning while runtime rejected `TargetUnownedOrActorControls(0)`.
- Preserved remote loose-lot acquisition after planned travel.

## Deviations

- The drafted storage-level `SourceReliability` split was attempted during reassessment but not landed because it did not preserve the baseline golden. The completed split is at the public accounting/reporting layer.
- The landed source files differ from the draft's expected file list: no `agent_tick/mod.rs`, `source_composite.rs`, or diagnostics fixture edits were needed.

## Verification Result

- Passed `cargo test -p worldwake-ai can_control_ref`
- Passed `cargo test -p worldwake-ai local_unpossessed_water_emits_acquire_goal_when_thirsty`
- Passed `cargo test -p worldwake-ai local_other_owned_loose_water_does_not_emit_pickup_acquire_goal`
- Passed `cargo test -p worldwake-ai acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`
- Passed `cargo test -p worldwake-ai handle_plan_failure_records_unlawful_pickup_as_exact_target_legal_discrepancy`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_preferences -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_scattered -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_trade -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_ask_consult -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_patrol -- --ignored --test-threads=1`
- Passed `cargo test -p worldwake-ai --lib`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1`
