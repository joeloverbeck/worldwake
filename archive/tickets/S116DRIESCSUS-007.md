# S116DRIESCSUS-007: Retrofit contested authored critical-run bound 400 → 300

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (scenario-authored contract value + rationale comment update)
**Deps**: archive/tickets/S116DRIESCSUS-006.md

## Problem

Spec S116 D8 requires the contested survival critical-run bound to tighten from 400 to 300 after the full S116 pipeline lands and ticket 006 confirms empirical improvement on the contested scenario. The current 400-tick bound was explicitly relaxed to accommodate the wash-starvation dynamic; with motive escalation interrupting the priority-override cycle, the bound can safely tighten without approaching the water-possession bottleneck that S116 does not address.

## Assumption Reassessment (2026-04-18)

1. The live contested bound is no longer a file-local test constant. After archived [archive/tickets/S119AUTHSURVHC-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S119AUTHSURVHC-001.md), `golden_survival_contested.rs` reads the authored scenario contract through `observation.contract.max_authored_critical_run_ticks` and optional per-need overrides.
2. The canonical authored bound now lives in [survival-contested.ron](/home/joeloverbeck/projects/worldwake/scenarios/survival-contested.ron): `survival_health_contract.max_authored_critical_run_ticks: 400`.
3. The earlier contested contract-model gap was resolved by archived [archive/tickets/S121PERNEEDSHC-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S121PERNEEDSHC-001.md), so the dirtiness-specific override remains separate at `critical_run_limits.dirtiness: 1300`. This ticket only tightens the scenario-wide default bound used for the other survival needs.
4. Empirical max-consecutive-run figures from `reports/scenario-analysis-report.md` for survival-contested seed 306006: Agent A = 319 ticks, Agent B = 315 ticks, Agent C = 313 ticks, Agent D = 372 ticks. Worst case = 372. Tightening the scenario-wide bound to 300 requires motive escalation to break non-dirtiness runs ≥72 ticks earlier than today's worst case — the prerequisite S116 golden/calibration proof is now complete in [archive/tickets/S116DRIESCSUS-006.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S116DRIESCSUS-006.md).
5. Live `GoalKind` and affordance surface unchanged: `wash_preconditions` (`crates/worldwake-systems/src/needs_actions.rs:196`) still requires `TargetDirectlyPossessedByActor(0)` with `CommodityKind::Water`. This remains the architectural limit on tightening further than 300, which is why dirtiness keeps its per-need override instead of being folded into the new default.
6. Follow-up: water-possession bottleneck resolution is named in the spec D8 text and in the updated rationale comment. Not in scope here.
7. Intended verification layer (precision rule 3): Golden E2E coverage. Harness already full-action-registries per the existing test's setup — no change to the harness itself.
8. Live harness contract check on 2026-04-18: `cargo test -p worldwake-ai --test golden_survival_contested -- --list` shows the owned contested assertions (`all_agents_survive_1440_ticks`, `all_agents_perform_survival_actions`, `both_camp_sides_reach_food`, `both_water_sources_are_used`, `no_budget_exhaustion_on_survival_goals`, `no_stuck_idle_windows_with_elevated_needs`) are all `#[ignore]` long-running tests on this branch. The truthful verification selector for this ticket therefore requires `-- --ignored`; a plain `cargo test -p worldwake-ai --test golden_survival_contested` would only exercise the fast non-owned helper surface.

## Architecture Check

1. Purely a test-constant tightening + rationale update. No production code changes, no new tests.
2. If the tightened bound fails empirically after this ticket is implemented (i.e., ticket 006's calibration held but 300 is still too aggressive on unseeded variations), apply the 1-3-1 rule to the user rather than silently relaxing.
3. `golden_survival_scattered.rs` is explicitly out of scope — the spec's D8 text notes its tightening is conditional on empirical data supporting a bound tighter than 300, which ticket 006's calibration does not specifically investigate.

## Verification Layers

1. Contested regression bound → `golden_survival_contested::all_agents_survive_1440_ticks` passes with `survival_health_contract.max_authored_critical_run_ticks = 300`. Authoritative proof surface: the existing per-agent contract-read assertion in the golden helper path.
2. Single-layer ticket — the golden provides the only verification surface needed. No action-trace or event-log assertion needed at this layer (they are ticket 003's territory).

## What to Change

### 1. Tighten the authored scenario contract

Change [survival-contested.ron](/home/joeloverbeck/projects/worldwake/scenarios/survival-contested.ron) from:

```ron
max_authored_critical_run_ticks: 400,
```

to:

```ron
max_authored_critical_run_ticks: 300,
```

### 2. Replace rationale comment on the canonical contract surface

Replace or extend the nearby contested scenario comment so the canonical authored contract records the S116 rationale:

```text
Tightened from 400 to 300 by S116 (Drive Escalation Under Sustained
Critical Need). Motive-score escalation interrupts the wash-cycle
priority-override that previously allowed critical runs up to ~372
ticks empirically. The 300-tick default gives >=72 ticks of headroom
over the pre-S116 worst case while not depending on resolving the
`wash_preconditions` water-possession bottleneck
(`needs_actions.rs:196` still requires directly possessed Water).
Further tightening toward 200 requires a follow-up spec that changes
acquire-water-for-wash precedence or the wash precondition itself.
```

## Files to Touch

- `scenarios/survival-contested.ron` (modify — authored bound + rationale comment)

## Out of Scope

- `golden_survival_scattered::MAX_CRITICAL_RUN_TICKS` adjustment — ticket 006's calibration pass does not specifically investigate whether scattered's bound can tighten; leave as-is unless a subsequent scenario-analysis run motivates a follow-up.
- Resolving the water-possession bottleneck — separate spec.
- New test additions — existing assertion surface is sufficient.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_contested::all_agents_survive_1440_ticks` passes with `max_authored_critical_run_ticks = 300`.
2. Existing suite: `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`.

### Invariants

1. The scenario comment accurately describes the remaining architectural limit (water-possession bottleneck) so future tightening attempts have explicit context.
2. No production code change beyond the authored scenario contract value and comment.

## Test Plan

### New/Modified Tests

1. None — comment-and-scenario-contract ticket only; the bound change exercises existing contested assertions. Verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_contested all_agents_survive_1440_ticks -- --ignored --exact`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-18.

- Tightened the canonical contested authored survival-health bound in [survival-contested.ron](/home/joeloverbeck/projects/worldwake/scenarios/survival-contested.ron) from `max_authored_critical_run_ticks: 400` to `300`.
- Recorded the S116 rationale directly at the canonical contract surface in the scenario file, including the remaining `wash_preconditions` water-possession bottleneck and why dirtiness still keeps its per-need override.
- Left production code and the shared contested golden helper path untouched. The live golden already reads the authored contract, so the real implementation surface was the scenario contract rather than a file-local test constant.

## Deviations

- Reassessment showed the original ticket narrative was stale after archived [archive/tickets/S119AUTHSURVHC-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S119AUTHSURVHC-001.md) and [archive/tickets/S121PERNEEDSHC-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S121PERNEEDSHC-001.md). The owned bound no longer lived in `golden_survival_contested.rs`; it lived in the scenario-authored `survival_health_contract`, so the ticket was corrected to that live contract before implementation.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_survival_contested all_agents_survive_1440_ticks -- --ignored --exact`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
