# AGEFOOREP-004: Repair golden regressions introduced by the AGEFOOREP-002 trade overhaul

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — AI goal arbitration (candidate generation / ranking / search / planning state) and merchant justice flow in `worldwake-ai` + `worldwake-systems`
**Deps**: Introduced by `6d627d68 Implemented AGEFOOREP-002.` (the ~1500-line merchant/trade overhaul). Surfaced on PR #142, which bundles AGEFOOREP-001/002/003 + AILIBBASE-001 against `origin/main`. AGEFOOREP-002 never had its own CI run; these gated golden families (`golden-*.yml`) are `#[ignore]`d and skipped by `verify.sh` / `cargo test -p worldwake-ai`, so the regressions only appeared on the PR.

## Problem

AGEFOOREP-002 broke three gated golden families plus one derived fixture. All four are **green on `origin/main`** (last Golden Survival green at PR #141, commit `4029a60b`) and **red on PR #142's first run**. Bisect confirms the patrol family passes at AGEFOOREP-001 (`04187015`) and fails at AGEFOOREP-002 (`6d627d68`).

The clippy lint at `crates/worldwake-systems/src/trade_actions.rs:3642` (`match_wildcard_for_single_variants`, also introduced by AGEFOOREP-002 test code) was fixed in the same push that created this ticket. This ticket covers only the behavior regressions below.

## Failing goldens (all reproduced locally)

Run each with `--release ... -- --ignored --test-threads=1`. Anchor filters as `scenarios::<scenario>::` to bind to the module.

1. **`golden-survival / patrol`** — `scenarios::survival_patrol::survival_patrol_proves_patrol_and_remote_pursuit_execution` and `..::survival_patrol_replay_is_deterministic`.
   - Symptom: `first_market_road_patrol_tick` is `None` (panic at `crates/worldwake-ai/tests/scenarios/survival_patrol.rs:259`). The guard patrols Watch Post (tick 32), selects `Patrol{Market Road}` at tick 33, but on arrival at Market Road (tick ~35) **switches to the `EngageHostile` pursuit goal** and travels straight through to Old Mill to attack the fugitive, never committing the Market Road patrol. Expected: patrol both authored waypoints *before* committing to the long pursuit.
   - Suspected surface: `EngageHostile` carries **Danger provenance** (`RankedGoalProvenance::Danger`), so its score is `score_product(danger_weight, danger_pressure)` — **not** the Drive motive path. The regression is in how the in-range pursuit candidate is emitted/scored on arrival vs. the scheduled patrol goal: look at AGEFOOREP-002 changes to `candidate_generation.rs`, `planning_state.rs`, `search/*`, and any danger-pressure / pursuit route-cost scoring, not the Drive motive aggregation.

2. **`golden-survival / justice`** — `scenarios::survival_justice::survival_justice_proves_fine_punishment_for_same_theft_case` (panic at `survival_justice.rs:702`, `fine_tick` is `None`) and `..::survival_justice_proves_institutional_bounty_posted` (panic at `survival_justice.rs:809`, depends on the fine branch).
   - Symptom: the merchant commits `accuse` but never commits the `fine` for the same accusation case, so the downstream bounty branch is also unreachable.
   - Suspected surface: AGEFOOREP-002 touched `trade.rs`, `trade_actions.rs`, `stock_actions.rs`, and the merchant/justice candidate path. Localize where the fine candidate is generated/selected for an accusation case.

3. **`golden-planner-pathology / degenerate-zero-step-loop`** — `scenarios::planner_pathology_degenerate::degenerate_zero_step_loop_blocks_actionable_goals` (panic at `crates/worldwake-ai/tests/planner_pathology_harness/mod.rs:957`, `late_eat_commit` is false). Backed by `scenarios/cli-evaluation.ron` (the late-run "Lina" window).
   - Symptom: the late-run `FreeCarryCapacity` loop never clears into an executable `DropItem` / late `eat` commit.
   - Suspected surface: consumption-goal arbitration. AGEFOOREP-002 added `self_consume_drive_score` (sum of self-care drive factors), a hard `return 1000` in `self_consume_freshness_factor` for `ConsumeOwnedCommodity`, and the `ranked_motive_score_with_memory` rescale (see ruled-out note below). The `FreeCarryCapacity`/`DropItem`/carry-capacity interaction with the new consumption scoring is the place to localize.

4. **`golden-scenario-diagnostics / fixture`** — `scenarios::scenario_diagnostics_fixture::golden_scenario_diagnostics_survival_baseline_fixture_is_stable` (drift assertion at `scenario_diagnostics_harness/mod.rs:138`).
   - This is a **derived artifact downstream of the above behavior changes**. Regenerate it (`WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test --release -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture:: -- --ignored --test-threads=1`) **only after** the three behavior roots above are fixed and the resulting baseline trajectory is confirmed intentional — never to mask the regressions. If, after fixing roots 1–3, the survival-baseline trajectory is genuinely and intentionally different, the regeneration is a legitimate FND-1 truth-adjustment; cite the motivating change in the commit.

## Ruled out (do not re-investigate)

- **`ranked_motive_score_with_memory` `.max()` → `.fold(0, u32::saturating_add)` rescale** (`crates/worldwake-ai/src/ranking.rs:367`, AGEFOOREP-002). This change is real and broad (it sums the motive inputs of *every* Drive-provenance goal rather than taking the max), and is the reason AILIBBASE-001 had to bump the theft motive expectation `567_000 → 623_000`. However, an architecturally-correct split fix (sum only for `self_consume_commodity` consumables, `max` for all other Drive goals) left **both** patrol and planner-pathology failing with **byte-identical decision traces** — i.e. it is inert against these failures. `EngageHostile` is Danger-provenance and never went through this path. Reverting/splitting this line is not the fix for any of the four failures above (though it may still be worth a separate correctness review, since summing arbitrary multi-input non-consumption goals is suspect — out of scope here).

## Assumption Reassessment (2026-06-03)

1. Patrol regression bisected: passes at `04187015` (AGEFOOREP-001), fails at `6d627d68` (AGEFOOREP-002). Confirmed via the gated golden repro above in a clean worktree.
2. Justice and planner-pathology are 002-era (same overhaul) but not independently bisected; confirm each is introduced by 002 before deep code-reading.
3. Shared abstraction boundary under audit: the goal-arbitration pipeline (candidate emission → ranking/provenance → search/selection) plus the merchant justice (accuse → fine → bounty) flow, both rewritten by AGEFOOREP-002.
4. Intended invariants restated: (patrol) the guard commits patrol at *both* authored waypoints before committing the remote pursuit; (justice) a funded merchant commits the fine for the same accusation case before any bounty; (planner-pathology) the late-run `FreeCarryCapacity` loop clears into an executable plan and a late eat commit.
5. Live `GoalKind`s under test: `Patrol`, `EngageHostile` (Danger provenance), `ConsumeOwnedCommodity` / `FreeCarryCapacity`, and the merchant fine/accuse goals. Verify the current operator/affordance surface for each before editing.
6. AI regression layer: candidate generation + runtime arbitration + golden E2E (full action registries required — these are scenario-backed survival/cli goldens, not needs-only harnesses).
7. Ordering layer: patrol vs. pursuit divergence is a selection/priority-class ordering on the *arrival* tick at Market Road; determine whether it turns on Danger priority class, danger pressure, pursuit route-cost scoring, or candidate suppression — not the Drive motive aggregate (ruled out above).
13. Adjacent contradiction: the `ranked_motive_score_with_memory` sum-vs-max question is a separate correctness concern; keep it out of this ticket's scope unless a root fix proves it relevant.

## Architecture Check

1. Fix each regression at its owning seam (candidate generation / danger-pursuit scoring / merchant justice flow), not by relaxing the golden contracts. Goldens are authoritative (CLAUDE.md Authoritative-to-AI Impact Rule).
2. No backward-compat shims. If AGEFOOREP-002's approach is structurally wrong for a given failure, correct the seam rather than special-casing the scenario.
3. Respect FND-14 (belief-only planning) and FND-26 (system decoupling) — the merchant justice fix in `worldwake-systems` must not import a sibling system crate.

## Verification Layers

1. Patrol waypoint ordering → action trace (`patrol` committed at Market Road before the first `EngageHostile` selection) + the existing `survival_patrol` golden.
2. Merchant fine → action trace (`fine` committed after `accuse` for the same case) + the existing `survival_justice` goldens (both the fine and bounty proofs).
3. Carry-loop clearance → planner trace (no repeated 0-step `FreeCarryCapacity`; late `eat` commit) + the `degenerate_zero_step_loop` golden.
4. Determinism → `survival_patrol_replay_is_deterministic` and any sibling `*_replays_deterministically` for justice.
5. Derived fixture → regenerate `expected-scenario-diagnostics.json` only after roots 1–3 land; cite the motivating change.

## What to Change

To be determined during implementation — root causes are not yet localized to specific symbols. Start from the suspected surfaces named per-failure above.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (likely — patrol/consumption candidate emission)
- `crates/worldwake-ai/src/planning_state.rs`, `crates/worldwake-ai/src/search/*` (likely — pursuit vs. patrol selection ordering)
- `crates/worldwake-systems/src/trade.rs`, `trade_actions.rs`, `stock_actions.rs` (likely — merchant fine flow)
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (regenerate last, only if intentional)

## Out of Scope

- The `ranked_motive_score_with_memory` sum-vs-max correctness review (ruled out as the cause of these four failures; track separately if pursued).
- The clippy lint at `trade_actions.rs:3642` (already fixed in the push that created this ticket).
- Any new trade/merchant feature work beyond restoring the four golden contracts.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_patrol:: -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-ai --test golden_ai scenarios::survival_justice::survival_justice_proves_fine_punishment_for_same_theft_case -- --ignored --test-threads=1` and the bounty test (run separately).
3. `cargo test --release -p worldwake-ai --test golden_ai scenarios::planner_pathology_degenerate:: -- --ignored --test-threads=1`
4. `cargo test --release -p worldwake-ai --test golden_ai scenarios::scenario_diagnostics_fixture:: -- --ignored --test-threads=1`
5. Whole gated golden families for any touched engine seam (CLAUDE.md: ALL goldens must pass when framework code changes), plus `./scripts/verify.sh`.

### Invariants

1. Guard patrols both authored waypoints before remote pursuit; pursuit still selects and executes from last-seen memory afterward.
2. A funded merchant commits the fine for an accusation case before any institutional bounty.
3. The late-run carry loop clears into an executable plan and a late eat commit; replay stays deterministic.

## Out-of-cycle note

Surfaced during a `/fix-ci-failures` cycle on branch `implemented-ailibbase-001`. That cycle shipped only the clippy fix; these behavior regressions were deferred here because they are 3+ independent, unlocalized roots in a large feature that never had its own CI, and the one obvious localization candidate (the ranking rescale) was validated inert. The originating `golden-survival`, `golden-planner-pathology`, and `golden-scenario-diagnostics` workflows on PR #142 stay red by design until this ticket lands.
