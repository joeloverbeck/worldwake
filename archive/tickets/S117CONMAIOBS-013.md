# S117CONMAIOBS-013: Baseline split-support planner ownership reassessment after oracle restoration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — reassessment, command-based verification, and ownership retarget only
**Deps**: `archive/tickets/S117CONMAIOBS-011.md`, `archive/specs/S104-survival-baseline-recovery.md`, `docs/planner-contracts.md`, `archive/tickets/S117CONMAIOBS-014.md`

## Problem

`archive/tickets/S117CONMAIOBS-011.md` routed the remaining `survival-baseline.ron` contradiction to planner behavior because the baseline observer dump showed acute/maintenance windows and the ignored survival golden was blocked by a proof-surface bug. After `archive/tickets/S117CONMAIOBS-014.md` restored the survival golden oracle, this ticket had to re-check whether planner-side split-support preparation was still the honest owning boundary on the live branch before any further `worldwake-ai` change landed.

## Assumption Reassessment (2026-04-18)

1. The proof-surface blocker from `archive/tickets/S117CONMAIOBS-014.md` is resolved on the live branch. The exact selector `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact` now passes again, so the authored survival baseline is once more a lawful behavior-level oracle.
2. Shared abstraction boundary under audit changed during reassessment. The live contradiction is no longer "planner behavior fails the authored survival contract"; it is now the relationship between two read-side proof surfaces:
   - the authored survival golden in `crates/worldwake-ai/tests/golden_survival_baseline.rs`
   - the observer baseline report from `crates/worldwake-cli/src/bin/observer.rs`
3. On the current branch, the strongest authored survival oracle passes while the observer baseline rerun still reports 12 anomalies: `ACTION_LOOP` on Agents A/B/C, `SUSTAINED_CRITICAL_NEED` on Agent B, `MAINTENANCE_STARVATION` on Agents A/B, and `ACUTE_NEED_SPIKE` on Agent B. That means the branch no longer proves a planner bug severe enough to violate the authored baseline contract.
4. The earlier planner-owned narrative in `archive/tickets/S117CONMAIOBS-011.md` was based on a branch state where the ignored survival golden could not reach its health assertion. Once the oracle was restored, that ownership claim became contingent rather than settled fact.
5. I tested one narrow planner-side candidate-generation intervention during live reassessment: defer some remote self-consume travel branches when stronger complementary local relief exists. The experiment made the exact survival golden fail (`Agent B thirst exceeded authored critical pm(820) for 118 consecutive ticks`) before it was rolled back. That result is evidence against forcing another planner change without first reconciling the observer-vs-oracle contract.
6. `docs/planner-contracts.md` still does not expose an existing multi-step "prepare complementary support, then travel" substrate, but the current branch also does not prove that such a planner addition is the right next step. The stronger live fact is that survival remains within the authored envelope even while the observer reports stress smells.
7. This ticket therefore no longer owns production behavior. The honest remaining problem is mixed-layer: determine whether the observer baseline smell contract is too strong for the authored healthy baseline, whether the survival golden is too weak for the observer's intended semantics, or whether a different proving contract should mediate the two.
8. Per `docs/FOUNDATIONS.md`, it would be wrong to mutate planner behavior purely to satisfy a noisier derived read-model while the strongest authored survival oracle is green. The next owner must start by reconciling those proof surfaces, not by assuming the planner is still guilty.

## Architecture Check

1. Converting this ticket into a reassessment/disposition closeout is cleaner than forcing more `worldwake-ai` work against stale ownership assumptions. The branch evidence changed materially once `014` restored the ignored oracle.
2. This keeps the architecture honest: planner behavior should not be retuned just to make a derived observer report quieter when the authored survival contract already passes.

## Verification Layers

1. Authored baseline survival envelope still holds -> `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
2. Observer baseline still reports split-support stress signals -> `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. Candidate-generation planner tweak is not presently justified by the strongest oracle -> live reassessment experiment, then rollback after the exact survival golden failed
4. This is a ticket/doc-only reassessment closeout; no further implementation-layer proof is applicable in the current ticket

## What to Change

### 1. Close the stale planner implementation assumption

Record that the restored authored survival oracle now passes on the live branch, so this ticket no longer has a proven planner-owned implementation target.

### 2. Create the correct owning follow-up

Create a new mixed-layer follow-up ticket that reconciles the remaining observer baseline smell contract against the passing authored survival oracle, then retarget active deps/blocker wording to that new ticket.

## Files to Touch

- `tickets/S117CONMAIOBS-013.md` (modify)
- `tickets/S117CONMAIOBS-015.md` (new)
- `tickets/S117CONMAIOBS-007.md` (modify)

## Out of Scope

- Any new planner-side production change in `worldwake-ai`
- Weakening observer detectors without reconciling them against the authored baseline contract
- Scenario retuning to make the observer report quiet

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The ticket closes with ownership aligned to the live branch rather than the pre-`014` branch narrative.
2. No planner or observer production change is justified in this ticket unless it improves the strongest authored oracle instead of merely quieting derived smell output.

## Test Plan

### New/Modified Tests

1. `None — reassessment/disposition ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
2. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
3. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-04-18.

- Reassessed the live branch after `archive/tickets/S117CONMAIOBS-014.md` restored the ignored survival-baseline oracle.
- Confirmed that the strongest authored survival selector now passes again, while the observer baseline rerun still reports split-support stress anomalies.
- Proved that a first planner-side intervention was not yet safely justified: the candidate-generation experiment caused the exact survival golden to fail and was rolled back.
- Created `tickets/S117CONMAIOBS-015.md` as the new owning follow-up for the remaining observer-vs-oracle contract reconciliation, and retargeted `tickets/S117CONMAIOBS-007.md` to that ticket.

## Deviations

- The drafted ticket assumed planner ownership remained settled once the survival-golden harness blocker was fixed. Live reassessment showed the branch evidence had changed more fundamentally: the authored survival contract passes, so the remaining contradiction is no longer a proven planner implementation bug.
- No planner code was kept from this ticket's reassessment experiment. The experiment was rolled back after it made the strongest authored oracle fail.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output /tmp/baseline-dump.md`
- Observer baseline still reports 12 anomalies on the current branch; that residual contradiction is now owned by `S117CONMAIOBS-015`
