# S162BELVIESOU-007: Restore lawful office/record information flow for the gated survival golden families (ask_consult, justice, offices)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Landed in passive local perception.
**Deps**: S162 belief-view source-gate hardening on this branch. Sibling fix `project_self_produced_lot_belief` already closed the `survival_production` regression and remains out of scope.

## Problem

S162 correctly gated `office_data`, `record_data`, `believed_rights`, `can_control`, `has_control`, `loyalty_to`, and contention belief-view reads behind lawful belief carriers. The non-ignored suite missed the ignored survival goldens, so `golden-survival.yml` later exposed that several survival families could no longer acquire enough local institutional substrate to keep their existing trajectories:

- `survival_ask_consult_lands_row_six` did not commit `ask_about_person`.
- `survival_justice_*` lost `accuse`, `fine`, bounty posting, and the search/report branch.
- `survival_offices_*` stopped selecting the force-claim office path.

The production regression listed in the original draft was already fixed by a sibling ticket and was not part of this implementation.

## Reassessment Result

The S162 accessor gates stayed correct. The missing contract was the acquisition side: co-located passive perception already writes belief-backed entity snapshots, but it did not project the institutional metadata snapshots that `record_data(...)` and `office_data(...)` now require. `consult_record` still projects those snapshots after a completed consultation; this ticket adds the same source-scoped snapshot shape for direct local observation of visible local records and offices.

The justice family exposed one extra live detail: under survival pressure, need-boosted item lots could outrank local records in the observation budget. Merchant Sera retained a lawful office-holder belief, but the local `CrimeRegister` was over-budget omitted, so `known_authority_crime_registers` had no believed record-data carrier and emitted no `Accuse` candidate. Institutional record/office carriers now outrank need-boosted item lots without bypassing the observation budget.

This preserves the S162 belief-only contract: candidate generation still reads only `PerAgentBeliefView` snapshots, and distant actors do not regain live-world office/record access.

## Outcome

Completed: 2026-05-21.

The gated ask-consult, justice, and offices survival families are restored through lawful local perception of institutional record/office carriers. The final diff is engine-only and keeps S162's accessor gates intact.

One accidental zero-test command was observed during verification: `cargo test -p worldwake-systems --lib passive_ -- --exact`. It is not counted as proof; the two exact perception test commands below are the focused proof runs.

## Landed Changes

- `collect_direct_local_observation_batch` now captures `BelievedRecordDataSnapshot` and `BelievedOfficeDataSnapshot` for observed local `Record` and `Office` entities.
- `apply_direct_local_observation_batch` records those snapshots into the observer's `AgentBeliefStore`.
- Passive observation priority now keeps local institutional `Office` and `Record` carriers above need-boosted item lots while still applying the configured observation budget.
- Added focused perception tests for local record/office snapshot projection and for record/office retention under urgent need pressure.

## Landed Files

- `crates/worldwake-systems/src/perception.rs`

## Acceptance Result

- `survival_ask_consult_lands_row_six` passes.
- All five `survival_justice` ignored tests pass.
- Both `survival_offices` ignored tests pass.
- The full ignored `scenarios::survival_` golden filter passes, covering the previously green survival families as regression proof.

## Verification Result

- Passed: `cargo test -p worldwake-systems --lib perception::tests::passive_perception_projects_local_record_and_office_snapshots -- --exact`
- Passed: `cargo test -p worldwake-systems --lib perception::tests::passive_local_observation_keeps_institutional_records_under_need_pressure -- --exact`
- Passed: `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_ask_consult::survival_ask_consult_lands_row_six -- --ignored --exact --test-threads=1`
- Passed: `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_offices:: -- --ignored --test-threads=1`
- Passed: `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_justice:: -- --ignored --test-threads=1`
- Passed: `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_ -- --ignored --test-threads=1` (46 passed)
- Passed: `cargo fmt --all`
