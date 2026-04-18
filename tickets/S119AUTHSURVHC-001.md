# S119AUTHSURVHC-001: Implement authored survival health contracts for survival goldens

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario schema/loader, survival golden harness helpers, authored scenario fixtures, survival golden assertions, golden testing docs
**Deps**: specs/S119-authored-survival-health-contracts.md

## Problem

The existing survival goldens still restate scenario health with file-local constants instead of reading one canonical authored contract. On April 18, 2026, `golden_survival_scattered` and `golden_survival_contested` both failed after the S116 chain not because live survival behavior was clearly regressing, but because those files still enforce a hardcoded `pm(750)` sustained-critical bound and stale self-care expectations that no longer match the authored scenarios or the post-`S116DRIESCSUS-010` wash contract. This leaves `S116DRIESCSUS-006` blocked on a proof-surface contradiction rather than a confirmed escalation-default regression.

## Assumption Reassessment (2026-04-18)

1. The current survival-golden family is split across three existing binaries:
   - `crates/worldwake-ai/tests/golden_survival_baseline.rs`
   - `crates/worldwake-ai/tests/golden_survival_scattered.rs`
   - `crates/worldwake-ai/tests/golden_survival_contested.rs`
   Target names were verified with real cargo invocations during live S116 reassessment.
2. The baseline golden already partially corrected this drift. `golden_survival_baseline.rs` now imports `DriveThresholds`, stores per-agent authored thresholds, and measures sustained-critical runs against `thresholds.<need>.critical()` instead of a file-local `pm(750)` constant.
3. The scattered and contested goldens still use duplicate file-local threshold truth:
   - `crates/worldwake-ai/tests/golden_survival_scattered.rs:18-67`
   - `crates/worldwake-ai/tests/golden_survival_contested.rs:21-75`
   Both files still hardcode `NEED_CRITICAL_THRESHOLD: u16 = 750` and compare all five tracked needs against `pm(750)`.
4. The authored scenarios already carry more specific per-agent threshold truth in `drive_thresholds`, for example:
   - `scenarios/survival-scattered.ron` Agent A hunger critical `850`
   - `scenarios/survival-contested.ron` Agent A hunger critical `850`, thirst critical `820`, dirtiness critical `900`
   Therefore the current scattered/contested failures can be caused by proof-surface drift even when runtime behavior is consistent with authored scenario profiles.
5. Shared data contract under audit: authored survival-health truth. Today the same fact has multiple transport paths:
   - canonical current path for critical-need cutoffs: `ScenarioDef.agent_defs[*].drive_thresholds`
   - duplicate path: file-local golden constants like `NEED_CRITICAL_THRESHOLD`
   This ticket makes scenario-authored survival-health expectations canonical for the golden family and removes the duplicate file-local survival-envelope path from the existing survival goldens.
6. This is a mixed-layer ticket but not a runtime-behavior ticket. The live behavior under audit is still proven through golden E2E runs; the change is to the authored input and shared test-read contract in `worldwake-cli` plus the consuming goldens in `worldwake-ai`.
7. Intended invariant before trusting the current failures: "survival scenarios should be judged healthy against one authored contract." The current failures in scattered/contested do not yet prove an S116 default-calibration regression because the goldens are still comparing against a stale global `750` threshold and stale per-agent action-family assumptions.
8. Live wash contract after archived `S116DRIESCSUS-010` is local basin-plus-source access, not possessed-water washing. Any required self-care-family expectation in a survival-health contract must be scenario-authored and truthful to that live contract rather than duplicated as a stale file-local golden assumption.
9. `specs/S119-authored-survival-health-contracts.md` already defines the intended substrate:
   - optional `ScenarioDef.survival_health_contract`
   - shared golden helper consumption
   - retrofit of baseline/scattered/contested
   - documentation update in `docs/golden-e2e-testing.md`
   This ticket should implement that spec instead of adding another ad hoc golden-local correction.
10. Adjacent contradictions exposed during S116 reassessment are classified as follows:
   - stale survival-health proof carriage in baseline/scattered/contested: required consequence of this ticket
   - any remaining red survival goldens after the contract becomes truthful: separate runtime/calibration bugs, not to be silently absorbed here
11. Scenario isolation choice: this ticket does not retune live survival behavior. It only moves scenario-owned health expectations into one canonical authored path. Lawful competing runtime branches such as sparse wash usage under contention remain part of the authored scenario contract decision, not something this ticket is allowed to "fix" by weakening runtime or broadening scenario infrastructure.
12. If reassessment after implementation still finds a real survival-behavior contradiction, that contradiction must be tracked separately. This ticket owns truth carriage, not hidden default retuning.
13. Post-retrofit verification on 2026-04-18 split the remaining failures by cause:
   - `golden_survival_baseline -- --ignored` is now green under the authored contract.
   - `golden_survival_scattered all_agents_survive_1440_ticks -- --ignored --exact` fails with `Agent B hunger exceeded authored critical pm(820) for 506 consecutive ticks (max allowed: 400)`. That is now tracked separately in [tickets/S119AUTHSURVHC-002.md](/home/joeloverbeck/projects/worldwake/tickets/S119AUTHSURVHC-002.md) as a classification ticket: wrong authored bound or real runtime contradiction.
   - the earlier contested contract-model gap is now resolved locally by [archive/tickets/S121PERNEEDSHC-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S121PERNEEDSHC-001.md): the contract now supports per-need overrides and the contested golden is green under the authored dirtiness bound.
14. Final 2026-04-18 reassessment: the S119 substrate is implemented locally, the contested contract-model gap is resolved by [archive/tickets/S121PERNEEDSHC-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S121PERNEEDSHC-001.md), and the scattered hunger overrun is classified as a too-strict authored bound by [archive/tickets/S119AUTHSURVHC-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S119AUTHSURVHC-002.md). The full survival-golden family is green under the authored-contract model.

## Architecture Check

1. Implementing S119 is cleaner than patching `S116DRIESCSUS-006` directly because it replaces duplicated file-local survival-envelope truth with one canonical authored contract that every survival golden can read.
2. This approach aligns with `docs/FOUNDATIONS.md`:
   - no magic numbers in goldens for scenario-owned survival bounds
   - no second truth source beside authored profiles/contracts
   - explicit falsification of one authored survival envelope
3. No backwards-compatibility shim is needed. The existing survival goldens should stop carrying their own survival-health constants once the shared scenario contract exists.

## Verification Layers

1. Scenario-authored survival-health contract loads through CLI schema and spawn path -> focused `worldwake-cli` loader/spawn coverage.
2. Shared sustained-critical tracking reads authored `DriveThresholds` and authored health-contract limits -> focused helper/unit coverage at the harness/helper layer.
3. Survival scenario envelope remains truthful after retrofit -> `golden_survival_baseline`, `golden_survival_scattered`, and `golden_survival_contested` ignored golden E2E runs; any remaining red result must be split into explicit authored-contract or runtime follow-up ownership rather than normalized here.
4. Missing contract on a survival scenario is rejected by the intended golden inventory/guard surface -> focused `worldwake-ai` or shared harness coverage.
5. If post-retrofit goldens still fail, the strongest proof remains the golden itself; this ticket does not use a later scenario report or ad hoc trace as a substitute for the authored contract surface.

## What to Change

### 1. Add authored survival-health contract to scenario schema

Extend `crates/worldwake-cli/src/scenario/types.rs` with the authored `survival_health_contract` section described by the spec, including:

- `max_authored_critical_run_ticks`
- `max_idle_window_ticks_with_elevated_need`
- `elevated_need_floor`
- `required_self_care_families`

Ensure the new section loads through the normal CLI scenario path without adding a second config file or CI-only override path.

### 2. Add shared survival-health helper support

In the shared survival-golden support under `crates/worldwake-ai/tests/golden_harness/` (or the current helper module actually used by the survival goldens):

- read the authored survival-health contract from `ScenarioDef`
- derive per-agent critical thresholds from authored `DriveThresholds`
- expose shared helpers for:
  - authored-critical run tracking
  - idle-window envelope checks
  - required self-care-family coverage

### 3. Retrofit authored survival scenarios

Add explicit `survival_health_contract` sections to:

- `scenarios/survival-baseline.ron`
- `scenarios/survival-scattered.ron`
- `scenarios/survival-contested.ron`

These values are scenario-specific. Do not normalize them into one repo-wide default.

### 4. Retrofit existing survival goldens

Update:

- `crates/worldwake-ai/tests/golden_survival_baseline.rs`
- `crates/worldwake-ai/tests/golden_survival_scattered.rs`
- `crates/worldwake-ai/tests/golden_survival_contested.rs`

so the scenario-owned survival envelope comes from the authored contract and authored thresholds rather than file-local constants. File-local constants may remain only for test mechanics that are not part of the scenario-owned health contract.

### 5. Add contract-presence guard and documentation

- Add focused guard coverage so a survival golden cannot quietly run against a scenario missing `survival_health_contract`.
- Update `docs/golden-e2e-testing.md` so long-run survival envelope checks are documented as scenario-authored contract reads, not file-local constant restatements.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify if loader/spawn carriage is needed)
- `crates/worldwake-ai/tests/golden_harness/` shared helper module(s) (modify)
- `crates/worldwake-ai/tests/golden_survival_baseline.rs` (modify)
- `crates/worldwake-ai/tests/golden_survival_scattered.rs` (modify)
- `crates/worldwake-ai/tests/golden_survival_contested.rs` (modify)
- `scenarios/survival-baseline.ron` (modify)
- `scenarios/survival-scattered.ron` (modify)
- `scenarios/survival-contested.ron` (modify)
- `docs/golden-e2e-testing.md` (modify)

## Out of Scope

- Retuning live survival behavior or `DriveEscalationProfile::default()`
- Changing the authoritative wash rule again
- Long-run forensic reporting beyond the authored contract surface (tracked separately by `specs/S120-survival-critical-window-forensics.md`)
- Tightening `golden_survival_contested::MAX_CRITICAL_RUN_TICKS` below the authored scenario contract for S116 ticket 007

## Acceptance Criteria

### Tests That Must Pass

1. The survival-golden family reads authored scenario health contracts instead of file-local survival-envelope constants.
2. `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
3. `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
4. `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
5. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Sustained-critical checks use each agent's authored `DriveThresholds`; there is no second hardcoded critical permille cutoff in the survival goldens.
2. Scenario-owned survival envelope bounds and required self-care families are carried through one canonical authored contract path.
3. If a retrofitted survival golden still fails after this ticket lands, that failure reflects a real remaining runtime/calibration contradiction or an explicitly wrong authored scenario contract, not duplicate file-local drift.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_baseline.rs` — consume shared authored-contract helpers instead of local survival-envelope constants.
2. `crates/worldwake-ai/tests/golden_survival_scattered.rs` — same retrofit for authored threshold and envelope truth.
3. `crates/worldwake-ai/tests/golden_survival_contested.rs` — same retrofit, including required self-care-family expectations.
4. Shared helper / loader-focused coverage in `worldwake-cli` or shared golden-harness support — prove contract loading and missing-contract rejection deterministically.

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
2. `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
3. `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
4. `cargo test -p worldwake-cli scenario`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-18.

- Added `ScenarioDef.survival_health_contract` and supporting schema types in [types.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/types.rs).
- Added shared authored-contract survival helpers in [golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) so survival goldens consume authored thresholds and scenario-owned envelope values instead of file-local constants.
- Retrofitted [golden_survival_baseline.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_baseline.rs), [golden_survival_scattered.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_scattered.rs), and [golden_survival_contested.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_survival_contested.rs) to read the shared authored-contract path.
- Added explicit `survival_health_contract` sections to [survival-baseline.ron](/home/joeloverbeck/projects/worldwake/scenarios/survival-baseline.ron), [survival-scattered.ron](/home/joeloverbeck/projects/worldwake/scenarios/survival-scattered.ron), and [survival-contested.ron](/home/joeloverbeck/projects/worldwake/scenarios/survival-contested.ron).
- Updated [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) so long-run survival-envelope checks are documented as scenario-authored contract reads.
- Closed the 2 post-retrofit blocker paths honestly:
  - contested required richer per-need contract expressiveness, delivered by [archive/tickets/S121PERNEEDSHC-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S121PERNEEDSHC-001.md)
  - scattered required authored-bound classification, delivered by [archive/tickets/S119AUTHSURVHC-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S119AUTHSURVHC-002.md)

## Deviations

- The ticket did not complete in a single straight pass. Truthful verification split the remaining red goldens into 2 explicit follow-up owners before the family could close.

## Verification Result

- Passed `cargo test -p worldwake-cli scenario`
- Passed `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
