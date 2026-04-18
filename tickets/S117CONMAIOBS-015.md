# S117CONMAIOBS-015: Baseline observer-vs-oracle contract reconciliation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Mixed — disposition first; may resolve to observer contract updates, golden contract updates, or a narrower implementation ticket after reassessment
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
4. The immediate invariant is no longer "planner must remove split-support oscillation." It is: the repo must not simultaneously treat `survival-baseline.ron` as healthy in its strongest authored oracle and as mechanically unhealthy in its baseline observer contract without an explicit documented reason.
5. This ticket is intentionally mixed-layer and may close in one of three honest ways:
   - the observer baseline contract is too strong for the authored healthy baseline and needs narrowing
   - the survival golden is too weak and should prove stronger stress-envelope invariants
   - a third follow-up implementation ticket is needed after the two proving contracts are reconciled
6. `docs/FOUNDATIONS.md` constrains the disposition:
   - do not tune planner behavior merely to satisfy a noisier derived report
   - do not weaken observer output just to preserve a narrative if the stronger authored oracle is actually incomplete
   - keep the resulting contract explainable and concrete

## Architecture Check

1. A reconciliation/disposition ticket is cleaner than forcing more planner work from stale ownership assumptions or weakening detectors ad hoc.
2. This preserves the repo's proof hierarchy: reconcile the competing oracles first, then land any implementation against the clarified invariant.

## Verification Layers

1. Authored healthy-baseline oracle -> `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
2. Observer baseline smell contract -> `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. Detector-specific baseline regression expectations already recorded in `S117CONMAIOBS-007` and archived `S117CONMAIOBS-011.md` -> active ticket/spec audit
4. If a proving surface is changed, its exact contract must be restated and reverified in the owning layer's tests/commands

## What to Change

### 1. Reconcile the two baseline proof surfaces

Audit the exact assertions in `golden_survival_baseline.rs` against the actual observer smell outputs on the same scenario and decide whether the two contracts are genuinely contradictory or merely proving different things.

### 2. Produce one honest owning disposition

At ticket closeout, either:

- narrow the observer baseline expectation,
- strengthen the survival golden expectation, or
- create a new narrower implementation ticket once the discrepancy is understood.

### 3. Retarget active blocker wording

Update `S117CONMAIOBS-007` and any adjacent active roadmap text so the remaining blocker is described by the reconciled contract rather than the stale planner-owned narrative.

## Files to Touch

- `tickets/S117CONMAIOBS-015.md` (new)
- `tickets/S117CONMAIOBS-007.md` (modify)
- `specs/S117-convergence-maintenance-observer-smells.md` (modify if contract wording is factually wrong)
- `crates/worldwake-ai/tests/golden_survival_baseline.rs` (modify only if reassessment proves the baseline oracle is too weak)
- `crates/worldwake-cli/tests/golden_observer_anomalies.rs` (modify only if the baseline observer contract is too strong)

## Out of Scope

- Direct planner retuning before the two proving contracts are reconciled
- Silent detector suppression
- Scenario retuning to hide the disagreement

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The ticket ends with one explicit, current owning contract for baseline health rather than two competing unstated ones.
2. No implementation follow-up is created until the stronger-vs-derived proof relationship is made explicit.

## Test Plan

### New/Modified Tests

1. `None — reconciliation/disposition ticket; verification is command-based until reassessment proves one proving surface must change.`

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. `cargo test -p worldwake-ai`
