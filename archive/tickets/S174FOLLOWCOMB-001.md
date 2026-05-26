# S174FOLLOWCOMB-001: Restore survival_combat critical-need contract after S174 sleep rework

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — tick-step runtime hostile-proximity interruption predicate
**Deps**: archive/specs/S174-shelter-sleep-surfaces-safe-rest.md (shipped)

## Problem

The `golden-survival/combat` workflow regressed on the S174 PR. The
`survival_combat_proves_combat_and_bandit_camp_abandonment` golden fails
at `assert_authored_critical_runs`:

```
Sentinel Rowan hunger exceeded authored critical pm(850) for 916 consecutive
ticks (max allowed: 190)
```

Reproduction:

```
cargo test --release -p worldwake-ai --test golden_ai -- \
  --ignored --test-threads=1 \
  scenarios::survival_combat::survival_combat_proves_combat_and_bandit_camp_abandonment
```

Before this ticket, instrumenting the test (added/removed during diagnosis on
branch `implement-S174-shelter-sleep-surfaces-safe-rest`) showed:

- `committed_actions = {"attack", "drink", "eat", "loot",
  "queue_for_corpse_use", "toilet"}` — **`sleep` is absent across the
  entire 1440-tick run**.
- Combat resolves at tick 3 (`attack_committed_tick`, `raider_dead_tick`,
  `camp_empty_tick` all `Tick(3)`); camp is abandoned at `Tick(6)`. So the
  combat half of the contract still holds.
- Post-combat, every non-bladder need detonates and stays critical:
  `hunger_max=916`, `thirst_max=971`, `fatigue_max=1051`,
  `dirtiness_max=853`, `bladder_max=39`.

The `survival-combat.ron` scenario has no `RestCapacity` authored on
either Place (Watch Post, Raider Camp). Under S174 that left only the
**rough-sleep** branch reachable. Rowan committing zero sleep actions
across 1437 post-combat ticks initially indicated the rough-sleep path was
either not being emitted or not being selected/executed for this scenario;
this ticket corrected that diagnosis to runtime interruption by a dead
hostile target.

All four of S174's own goldens
(`scenarios::survival_safe_rest::*`,
`scenarios::survival_sleep_contention::*`,
`scenarios::survival_rest_interrupted_by_danger::*`,
`scenarios::survival_failed_rest_cascade::*` and their replay variants)
pass locally. The defect is in how S174's new sleep architecture
interacts with rest-site-free scenarios that previously relied on the
old `FeasibilityStrategy::AlwaysLikely` sleep path.

## Assumption Reassessment (2026-05-26)

1. **`survival_combat_proves_combat_and_bandit_camp_abandonment` is the
   only currently-failing scenario in the golden-survival matrix.** Verified
   from PR #137 run `26442609541`: 1 of 18 matrix jobs failed. The 17 sibling
   scenarios stayed green. The other two failing workflows
   (`golden-observer-anomalies`, `golden-scenario-diagnostics`) were
   resolved on this branch as downstream calibration drift (FND-1
   truth-adjustment row) and do not share root cause.
2. **`scenarios/survival-combat.ron` has not been modified on this branch.**
   Verified via `git log main..HEAD -- scenarios/survival-combat.ron`
   (empty). The behavioral change is upstream in production code.
3. **The combat half of the scenario contract still holds.** Rowan
   commits an attack at tick 3, the raider dies that tick, the camp
   empties immediately and is abandoned by tick 6. The failure is
   entirely in the post-combat self-care half.
4. **Rough-sleep is the only reachable sleep branch in this scenario.**
   `available_rest_site_candidate_places` in
   `crates/worldwake-ai/src/candidate_generation.rs` filters by
   `rest_site_capacity(...).is_some()`. Neither Watch Post nor Raider
   Camp carries `RestCapacity`, so the for-loop in
   `sleep_rest_opportunities` (line ~4498) produces no targeted
   candidates. The only sleep candidate path is the targetless
   `OpportunityAnchor::None` rough-sleep branch (line ~4515).
5. **Voss being dead should *not* block the rough-sleep candidate or interrupt
   a started sleep action.** Reassessment corrected the drafted runtime
   premise: `CombatBeliefView::visible_hostiles_for` already excludes believed
   dead targets, but `local_hostile_present` in
   `crates/worldwake-sim/src/tick_step.rs` had been using `World::is_alive`,
   which means the entity allocation is live, not that the agent lacks
   `DeadAt`. That made Voss's corpse keep interrupting rough sleep as
   `HostileProximity`.
6. **The committed-action set ruled out a planner-budget collapse.** Rowan
   continued to commit `eat`, `drink`, `toilet`, and combat-aftermath
   loot/queue actions throughout the pre-fix run. Planning produced viable
   plans; subsequent trace evidence narrowed the sleep-specific failure to
   runtime interruption rather than candidate emission, scoring, or start-time
   rejection.

## Architecture Check

This is a **defect investigation ticket**, not a feature spec, but the
fix must respect:

1. **No backward-compat shim around the old `FeasibilityStrategy::AlwaysLikely`
   sleep path** (FND-28; explicit Non-Goal of S174). The fix must work
   within the two-branch (`KnownRestSite` + `RoughSleep`) schema.
2. **No new heuristic that bypasses FND-14 belief-only planning** to
   force sleep. The rough-sleep candidate must continue to derive from
   the agent's own fatigue need and same-tick local observation.
3. **No relaxation of the survival-combat critical-need contract.** The
   authored thresholds in `scenarios/survival-combat.ron`
   (`max_authored_critical_run_ticks: 190`,
   `required_self_care_families: [Eat, Drink, Sleep, Relieve, Wash]`)
   are part of the row-15 survival-combat contract. Loosening them to
   make the test green is **not acceptable** — the fix is production code
   that lets Rowan rough-sleep after combat ends.

## Outcome

Completed on 2026-05-26.

Restored the survival-combat critical-need contract by fixing the runtime
hostile-proximity predicate that interrupted every rough-sleep attempt after
combat. The final seam is `tick_step.rs::local_hostile_present`, not candidate
generation, ranking, or action start.

## Reassessment Result

The live failure was not candidate emission, goal ranking, plan adoption, or
sleep start rejection. Existing focused coverage already proved targetless
rough-sleep candidate emission for a place without `RestCapacity`, and a
diagnostic action trace during this ticket showed Rowan repeatedly starting
targetless `sleep` at tick 90 and later.

The first failing layer was runtime interruption: every started rough-sleep
action immediately aborted with `InterruptReason::DangerNearby` /
`SleepFailureCause::HostileProximity`. The culprit was
`crates/worldwake-sim/src/tick_step.rs::local_hostile_present`, which treated
a hostile target whose entity allocation was still live as an interrupting
hostile even after that target had `DeadAt`.

This preserved the S174 rough-sleep architecture and the authored
`survival-combat.ron` contract. No `RestCapacity` was added to the scenario,
and no critical-need thresholds were relaxed.

## Landed Changes

- Updated `local_hostile_present` to require `DeadAt` absence before a hostile
  target can interrupt sleep for nearby danger.
- Added `tick_step::tests::local_hostile_present_ignores_dead_hostile_targets`
  to prove living co-located hostile targets still interrupt sleep while dead
  co-located hostile targets do not.

## Landed Files

- `crates/worldwake-sim/src/tick_step.rs`

## Acceptance Result

- The exact failing survival-combat golden now passes with the original
  authored critical-run limits.
- The survival-combat deterministic replay passes.
- The full ignored `scenarios::survival_` golden family passed locally:
  52 tests, including the repaired combat scenario and replay.
- The four S174 own golden modules and replay variants passed as non-ignored
  tests.
- `./scripts/verify.sh` passed. The live wrapper ran fmt-check, workspace
  tests, active-goal/artifact/debug-view hygiene scripts, workspace clippy,
  all-target clippy with warnings denied, and scenario coverage check.

## Deviations

- The original ticket suspected missing sleep candidate emission or ranking.
  Live trace evidence corrected that: sleep was emitted, planned, and started;
  runtime interruption by a dead hostile target was the actual failing seam.
- The drafted command that listed all four S174 own modules in one Cargo
  invocation was replaced with four valid module-filtered Cargo invocations,
  one per module.
- The existing `survival_combat_replay_is_deterministic` replay test covered
  the determinism requirement; no new golden test file or scenario metadata was
  needed.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::sleep_candidate_emission_without_known_rest_site_is_targetless_rough_sleep -- --exact`
- Passed `cargo test -p worldwake-sim --lib tick_step::tests::local_hostile_present_ignores_dead_hostile_targets -- --exact`
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_combat::survival_combat_proves_combat_and_bandit_camp_abandonment`
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_combat::survival_combat_replay_is_deterministic`
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_`
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --test-threads=1 scenarios::survival_safe_rest`
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --test-threads=1 scenarios::survival_sleep_contention`
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --test-threads=1 scenarios::survival_rest_interrupted_by_danger`
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --test-threads=1 scenarios::survival_failed_rest_cascade`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo fmt --all`
- Passed `./scripts/verify.sh`
