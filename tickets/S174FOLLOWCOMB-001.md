# S174FOLLOWCOMB-001: Restore survival_combat critical-need contract after S174 sleep rework

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate generation, sleep handler / start contract, possibly goal scoring
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

Instrumenting the test (added/removed during diagnosis on branch
`implement-S174-shelter-sleep-surfaces-safe-rest`) shows:

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
either Place (Watch Post, Raider Camp). Under S174 that should leave only
the **rough-sleep** branch reachable. Rowan committing zero sleep actions
across 1437 post-combat ticks indicates the rough-sleep path is either
not being emitted or not being selected/executed for this scenario.

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
5. **Voss being dead should *not* block the rough-sleep candidate.**
   `local_hostile_present` in `crates/worldwake-sim/src/tick_step.rs` and
   the corresponding planner check in
   `CombatBeliefView::visible_hostiles_for` both filter `is_alive` /
   `!is_dead`. Voss dies at tick 3, so from tick 4 onward neither runtime
   interruption nor planner suppression should fire on his account. The
   live presence of his **corpse / camp / loot artifacts** as a competing
   affordance is the more likely investigation surface.
6. **The committed-action set rules out a planner-budget collapse.** Rowan
   continues to commit `eat`, `drink`, `toilet`, and combat-aftermath
   loot/queue actions throughout the run. Planning produces *some* viable
   plans; sleep is the specific kind missing. So the suspect surface is
   sleep-candidate emission, scoring, or start-time rejection — not a
   wholesale planner failure.

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

## Verification Layers

1. **Per-tick decision trace** for Sentinel Rowan around the first tick
   his fatigue crosses `low` threshold (≈ tick 200 depending on rate /
   start value). Confirm whether a `Sleep` `GoalOffered` event is emitted
   under `OpportunityAnchor::None`. Trace tooling: `docs/debugging-traces.md`.
   If no `GoalOffered` appears → defect is in
   `sleep_rest_opportunities` / `available_rest_site_candidate_places` /
   the upstream branch gate.
2. **Plan-adoption trace** for the same window. If `GoalOffered`
   appears but no `PlanAdopted` does → defect is in goal ranking /
   plan-search reachability (suspect: rough-sleep's lower scored value vs.
   competing combat-aftermath goals like `LootCorpse` /
   `queue_for_corpse_use`).
3. **Action-start trace** for the same window. If `PlanAdopted` appears
   but `start_sleep_episode` rejects → defect is in `sleep_place_and_mode`
   (`crates/worldwake-systems/src/needs_actions.rs` line ~487). Inspect
   whether `instance.targets.first()` is `None` (rough sleep) or whether
   the targeted-but-no-RestCapacity case falls through unexpectedly.
4. **Re-run the failing golden**:
   `cargo test --release -p worldwake-ai --test golden_ai -- --ignored
   --test-threads=1
   scenarios::survival_combat::survival_combat_proves_combat_and_bandit_camp_abandonment`
   passes after the fix.
5. **Re-run all 4 S174 own goldens and their replay variants** to confirm
   the fix does not unmask new regressions:
   `cargo test --release -p worldwake-ai --test golden_ai -- --test-threads=1
   scenarios::survival_safe_rest scenarios::survival_sleep_contention
   scenarios::survival_rest_interrupted_by_danger
   scenarios::survival_failed_rest_cascade`.
6. **Full gated golden-survival matrix re-run** locally to confirm no
   sibling scenario was masking a similar defect:
   `cargo test --release -p worldwake-ai --test golden_ai -- --ignored
   --test-threads=1 scenarios::survival_`.
7. **Determinism**: `survival_combat_replays_deterministically` (if it
   exists post-S174; otherwise add a replay variant if the fix changes
   sleep state). Confirm replay still hashes identically.

## What to Change

This is an **investigation-first** ticket. The first PR for this work
must contain *no production code change* and instead deliver the trace
evidence narrowing the surface (Verification Layers 1-3). Only after the
defect is localized to one of (candidate emission / goal scoring /
action start) should production code change, and that change must ship
with the verification layers above all green.

### 1. Narrow the defect surface

Run the three trace layers above against
`survival_combat_proves_combat_and_bandit_camp_abandonment` and record
which layer first goes silent for `Sleep`. Attach the trace excerpt to
this ticket before opening the fix PR.

### 2. Fix the localized defect

Production code edit confined to whichever surface Step 1 implicates.
Must not introduce a backward-compat shim, must not relax the
`survival-combat.ron` contract.

### 3. Re-enable matrix coverage of S174's own goldens

Out of scope for this ticket but flagged: the four `survival_safe_rest`
/ `survival_sleep_contention` / `survival_rest_interrupted_by_danger` /
`survival_failed_rest_cascade` scenarios are not currently in
`.github/workflows/golden-survival.yml`'s matrix. They run as part of
`cargo test --workspace` (no `#[ignore]`) so are exercised by
`verify.sh`, but adding them to the matrix would make their results
visible in PR check summaries alongside the rest of the survival
family. File as a separate small ticket if desired.

## Files to Touch

Investigation phase: none (trace runs only).
Fix phase: one of
- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — if the
  rough-sleep candidate is not being emitted post-combat.
- `crates/worldwake-ai/src/ranking.rs` or related scoring (modify) — if
  the candidate is emitted but never selected.
- `crates/worldwake-systems/src/needs_actions.rs` (modify) — if the
  candidate is selected but `start_sleep_episode` rejects.

## Out of Scope

- Changes to S174's intended sleep architecture (rest-site identity,
  occupancy, KnownRestSite / RoughSleep split, structured wake causes).
- Adding `RestCapacity` to `scenarios/survival-combat.ron` to make
  KnownRestSite candidates available. The contract is that rough-sleep
  alone suffices for scenarios authored without rest sites.
- Adjusting `survival-combat.ron`'s critical-need thresholds.

## Acceptance Criteria

### Tests That Must Pass

1. `scenarios::survival_combat::survival_combat_proves_combat_and_bandit_camp_abandonment`
   (gated, golden-survival workflow).
2. All four S174 own goldens (`survival_safe_rest`,
   `survival_sleep_contention`, `survival_rest_interrupted_by_danger`,
   `survival_failed_rest_cascade`) and their replay variants.
3. Full `golden-survival` matrix (18 scenarios) under `--ignored
   --test-threads=1`.
4. `scripts/verify.sh`.

### Invariants

1. Rough-sleep candidate emission for an agent with fatigue ≥ low
   threshold, no local hostile, at a Place without `RestCapacity`, is a
   permanent contract of the rough-sleep branch and must hold for every
   such (place, agent) pair.
2. `survival-combat.ron`'s authored `max_authored_critical_run_ticks: 190`
   continues to apply unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (tests module) — if
   the defect is in emission, add a focused unit test that asserts a
   rough-sleep candidate is emitted for an agent at a Place without
   `RestCapacity` with fatigue ≥ low, no local hostile.
2. `crates/worldwake-ai/tests/scenarios/survival_combat.rs` —
   contingent on the fix; may need a tighter assertion that Rowan's
   committed_actions contains "sleep" at least once if the existing
   `assert_authored_critical_runs` is not specific enough as a
   regression trap.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored
   --test-threads=1
   scenarios::survival_combat::survival_combat_proves_combat_and_bandit_camp_abandonment`
2. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored
   --test-threads=1 scenarios::survival_`
3. `scripts/verify.sh`
