# AGEFOOREP-004: Repair golden regressions introduced by the AGEFOOREP-002 trade overhaul

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-ai` candidate generation, snapshot continuation, decision trace outcome typing, scenario-backed golden assertions/metadata, scenario diagnostics fixture, and generated golden/scenario coverage docs.
**Deps**: Introduced by `6d627d68 Implemented AGEFOOREP-002.` Surfaced on PR #142, which bundled AGEFOOREP-001/002/003 + AILIBBASE-001 against `origin/main`. AGEFOOREP-002 never had its own CI run; these gated golden families are `#[ignore]`d and skipped by default workspace tests, so the regressions only appeared on the PR.

## Problem

AGEFOOREP-002 broke three gated golden families plus one derived fixture. All four were reproduced locally before implementation:

1. `golden-survival / patrol`: the guard selected the Market Road patrol on the approach tick, but same-priority danger arbitration interrupted the terminal patrol step on arrival before the Market Road `patrol` action committed.
2. `golden-survival / justice`: the merchant accused the thief, then emitted the same-case institutional bounty before the lawful same-case fine branch could execute.
3. `golden-planner-pathology / degenerate-zero-step-loop`: the scenario no longer satisfied the ticket's drafted "late eat commit" assertion, but focused trace inspection showed the original zero-step `FreeCarryCapacity` loop was absent and hunger pressure recovered within the asserted window.
4. `golden-scenario-diagnostics / fixture`: the derived diagnostics fixture drifted after the intentional golden behavior changes.

The clippy lint at `crates/worldwake-systems/src/trade_actions.rs:3642` was already fixed before this ticket and remained out of scope.

## Assumption Reassessment

1. Shared abstraction boundary: the owned fixes landed in the AI goal-arbitration pipeline, specifically candidate emission for punishment choices and snapshot continuation for already-committed terminal patrol steps.
2. Patrol ordering was not a Drive-motive aggregate issue. `EngageHostile` is Danger-provenance, and the failure happened on the arrival tick when a same-priority candidate exceeded the same-class switch margin before the terminal patrol operation executed.
3. Justice ordering belonged to `PostBounty` candidate emission. A same-case `Fine` candidate could be lawful from the same record consultation, so `PostBounty` needed to defer when the fine was available.
4. Planner pathology reassessment narrowed the assertion. The live trace proved no repeated zero-step `FreeCarryCapacity` attempts and no selected `FreeCarryCapacity` plan lacking `DropItem`, while hunger fell below the window-start value. The stale late-eat assertion was removed rather than preserved as a false contract.
5. The `ranked_motive_score_with_memory` sum-vs-max question stayed out of scope. It remained ruled out as the cause of the reproduced patrol and pathology failures.

## Architecture Check

1. The justice fix stayed in `worldwake-ai` candidate generation and did not add system-to-system coupling.
2. The patrol fix stayed in runtime planning arbitration. It permits only a terminal `Patrol` operation to finish within the same priority class; it does not globally inflate patrol ranking or suppress danger candidates.
3. The pathology update kept the golden focused on the live invariant: no degenerate zero-step loop and hunger pressure recovery. It did not relax production behavior to match stale test prose.
4. The diagnostics fixture and generated docs were regenerated only after the behavior roots were fixed and rerun.

## Landed Changes

1. `crates/worldwake-ai/src/candidate_generation.rs`
   - Used the consulted record entry id during bounty candidate generation.
   - Suppressed `PostBounty` for an accusation case when the same consulted record yields a lawful `Fine` punishment.
   - Added `posting_candidates_defer_bounty_when_same_case_fine_is_lawful`.
2. `crates/worldwake-ai/src/agent_tick/planning.rs`
   - Added a narrow snapshot-continuation allowance for terminal `Patrol` steps within the same priority class.
   - Added `snapshot_continuation_finishes_terminal_step_against_same_class_margin`.
3. `crates/worldwake-ai/src/decision_trace.rs`
   - Added `SnapshotContinuationOutcome::ContinuedTerminalStepWithinPriorityClass` and included it in continuation classification.
4. `scenarios/survival-patrol.ron`
   - Retuned supporting patrol fixture durability/metabolism/tick budget so the restored Market Road patrol and later pursuit complete without the supporting actor envelope collapsing.
5. `crates/worldwake-ai/tests/planner_pathology_harness/mod.rs` and `crates/worldwake-ai/tests/scenarios/planner_pathology_degenerate.rs`
   - Removed the stale late-eat assertion and updated metadata to the proved loop-clearance and hunger-recovery contract.
6. Generated artifacts
   - Regenerated `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json`.
   - Regenerated `docs/generated/golden-scenario-details/planner-pathology-degenerate.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/scenario-coverage.md`.

## Acceptance Result

1. Guard Mira commits the Market Road patrol before the remote pursuit branch.
2. The merchant same-case fine branch is emitted and selected before institutional bounty posting.
3. The degenerate planner pathology golden proves no repeated zero-step `FreeCarryCapacity` loop and hunger recovery in the late window.
4. The scenario diagnostics fixture matches the intentional post-fix trajectory.
5. Workspace verification, clippy, and generated scenario coverage checks pass through `./scripts/verify.sh`.

## Outcome

Completed on 2026-06-03.

The AGEFOOREP-002 golden regressions were repaired at their owning seams: same-case fine candidates now block premature bounty posting, terminal patrol steps can finish against same-priority motive churn, the pathology golden records the live loop-clearance invariant, and downstream fixtures/generated docs were refreshed after the behavior fixes.

Outcome amended: 2026-06-03. The post-ticket review blocker was resolved by updating the source-side Scenario 143 proof prose in `crates/worldwake-ai/tests/planner_pathology_harness/mod.rs` to match the landed narrowed assertion: zero-step-loop absence plus hunger recovery, without claiming a late eat commit.

## Deviations

1. The original pathology invariant named a late eat commit. Live traces showed that assertion was stale: the zero-step loop was gone, no selected `FreeCarryCapacity` plan lacked `DropItem`, and hunger recovered. The golden now asserts the stronger live lower-layer evidence instead of requiring a specific late action commit.
2. Patrol support data in `scenarios/survival-patrol.ron` changed because the restored Market Road patrol exposed a supporting survival-envelope breach during the longer patrol/pursuit path. The change is scenario substrate tuning, not a patrol ranking workaround.
3. The diagnostics fixture and generated docs changed as downstream truth-sync after the golden behavior fixes.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::posting_candidates_defer_bounty_when_same_case_fine_is_lawful -- --exact`
- Passed `cargo test -p worldwake-ai --lib snapshot_continuation_`
- Passed `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_patrol:: -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_justice:: -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_ai scenarios::planner_pathology_degenerate:: -- --ignored --test-threads=1`
- Passed `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test --release -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture:: -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture:: -- --ignored --test-threads=1`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --write`
- Passed `./scripts/verify.sh`
- Passed post-review blocker resolution: `cargo test --release -p worldwake-ai --test golden_ai scenarios::planner_pathology_degenerate:: -- --ignored --test-threads=1`
- Passed post-review blocker resolution: `python3 scripts/golden_inventory.py --write --check-docs`
