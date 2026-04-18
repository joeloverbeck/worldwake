# S117CONMAIOBS-015: Baseline observer-vs-oracle contract reconciliation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — reconciliation/disposition, ticket/spec correction, and blocker retarget only
**Deps**: `archive/tickets/S117CONMAIOBS-011.md`, `archive/tickets/S117CONMAIOBS-013.md`, `archive/tickets/S117CONMAIOBS-014.md`, `specs/S117-convergence-maintenance-observer-smells.md`, `docs/FOUNDATIONS.md`, `docs/planner-contracts.md`

## Problem

On the live branch, the strongest authored survival oracle passes:

- `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`

But the observer baseline rerun still reports 12 anomalies on `scenarios/survival-baseline.ron`, including `SUSTAINED_CRITICAL_NEED`, `MAINTENANCE_STARVATION`, and `ACUTE_NEED_SPIKE` for the same baseline the project still treats as healthy. Before making any more planner or observer changes, the project needs one ticket that reconciles those two proof surfaces and decides which contract is wrong, incomplete, or in need of narrowing.

## Assumption Reassessment (2026-04-18)

1. `archive/tickets/S117CONMAIOBS-014.md` restored the ignored survival golden as a lawful oracle by fixing the in-transit `effective_place` proof-surface bug. That selector now passes again on the live branch.
2. `archive/tickets/S117CONMAIOBS-011.md` correctly established that the original baseline stress windows were real on the then-live branch, but `archive/tickets/S117CONMAIOBS-013.md` has now completed a second reassessment showing planner ownership is no longer proven once the restored golden is considered.
3. Shared abstraction boundary under audit: the baseline health contract across two read-side proving surfaces:
   - the authored survival golden in `crates/worldwake-ai/tests/golden_survival_baseline.rs`
   - the observer anomaly/report contract in `crates/worldwake-cli/src/bin/observer.rs`
4. The live proof surfaces are not actually asserting the same invariant. `golden_survival_baseline.rs` proves the authored survival envelope: all agents survive 1440 ticks, authored critical runs stay bounded, required self-care families appear, budget exhaustion stays absent, and elevated-need idle windows stay within the authored limit. The observer report in `crates/worldwake-cli/src/bin/observer.rs` is a read-side smell surface that can still emit `ACTION_LOOP`, `SUSTAINED_CRITICAL_NEED`, `MAINTENANCE_STARVATION`, and `ACUTE_NEED_SPIKE` while that authored envelope remains satisfied.
5. The stale contradiction came from spec/ticket wording, not from a live runtime mismatch. `specs/S117-convergence-maintenance-observer-smells.md` still said the healthy baseline should produce “no false positives,” and `S117CONMAIOBS-007` inherited that broad claim even though the landed automated regression only asserts that `GEOGRAPHIC_CONVERGENCE` stays absent on baseline. Live reruns confirm that narrower contract: convergence stays absent, while other stress smells still appear.
6. `docs/FOUNDATIONS.md` therefore points to a documentation/ticket correction rather than another code change:
   - do not tune planner behavior merely to satisfy a noisier derived report
   - do not weaken observer output just to preserve a narrative if the stronger authored oracle is actually incomplete
   - keep the resulting contract explainable and concrete
7. The honest reconciled contract is: `survival-baseline.ron` is healthy with respect to the authored survival oracle, while the observer anomaly surface remains a heuristic forensic layer that may still show stress smells inside that healthy envelope. Baseline regression for S117 therefore only promises the specific absence claims the repo actually wants to enforce mechanically on that scenario, currently `GEOGRAPHIC_CONVERGENCE`.

## Architecture Check

1. A reconciliation/disposition ticket is cleaner than forcing more planner work from stale ownership assumptions or weakening detectors ad hoc.
2. This preserves the repo's proof hierarchy: authored survival goldens own survival-health assertions; observer anomalies remain derived diagnostics unless a tighter baseline guarantee is explicitly authored.

## Verification Layers

1. Authored healthy-baseline oracle -> `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
2. Observer baseline smell dump -> `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. Landed observer baseline regression ownership in `crates/worldwake-cli/tests/golden_observer_anomalies.rs` and `S117CONMAIOBS-007` -> active ticket/spec audit
4. If a proving surface is changed, its exact contract must be restated and reverified in the owning layer's tests/commands

## What to Change

### 1. Reconcile the two baseline proof surfaces

Audit the exact assertions in `golden_survival_baseline.rs` against the actual observer smell outputs on the same scenario and classify whether they are competing contracts or different proof layers.

### 2. Correct stale contract wording

If the disagreement is only in ticket/spec prose, rewrite the affected observer/spec/ticket text so the baseline contract says exactly what the live proof surfaces really guarantee.

### 3. Retarget active blocker wording

Update `S117CONMAIOBS-007` and any adjacent active roadmap text so there is no remaining blocker on a nonexistent planner-vs-observer contradiction.

## Files to Touch

- `tickets/S117CONMAIOBS-015.md` (modify)
- `tickets/S117CONMAIOBS-007.md` (modify)
- `specs/S117-convergence-maintenance-observer-smells.md` (modify if contract wording is factually wrong)

## Out of Scope

- Direct planner retuning before the proof surfaces are reconciled
- Silent detector suppression
- Scenario retuning to hide the disagreement
- Expanding the survival golden beyond its authored survival-envelope contract without a new motivating ticket

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The ticket ends with one explicit, current owning contract for baseline health rather than two competing unstated ones.
2. No production implementation follow-up is created unless reassessment shows the reconciled contracts are still incompatible.

## Test Plan

### New/Modified Tests

1. `None — reconciliation/disposition ticket; verification is command-based until reassessment proves one proving surface must change.`

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-04-18.

- Reconciled the live baseline proof surfaces and confirmed they are proving different layers, not contradicting each other.
- Kept `golden_survival_baseline.rs` as the authored survival-health oracle for `survival-baseline.ron`.
- Narrowed the stale observer-side baseline claim in `S117CONMAIOBS-007` and `specs/S117-convergence-maintenance-observer-smells.md` so baseline regression no longer falsely promises a smell-free report.
- Removed the stale blocker chain from `S117CONMAIOBS-007`; no new implementation ticket was required.

## Deviations

- The drafted ticket assumed reconciliation might require a new implementation follow-up. Live reassessment showed the remaining disagreement was only stale contract wording: the survival golden and observer report are different proof layers with different semantics.
- No code or test file changed in this ticket. The honest landing surface was ticket/spec correction only.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
- Live baseline dump still reports 12 anomalies (`ACTION_LOOP`, `SUSTAINED_CRITICAL_NEED`, `MAINTENANCE_STARVATION`, `ACUTE_NEED_SPIKE`), which is now treated as lawful observer smell output inside a scenario whose authored survival envelope still passes
