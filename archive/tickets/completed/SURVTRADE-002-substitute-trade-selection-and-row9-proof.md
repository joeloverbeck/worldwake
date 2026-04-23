# SURVTRADE-002: Prefer the Substitute Trade Branch at Selection and Row-9 Proof Surfaces

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI ranking/selection; roadmap-row proof narrowed to follow-up golden isolation
**Deps**: `archive/tickets/completed/SURVTRADE-001-substitute-trade-planner-integration.md`; `docs/scenario-roadmap.md` row 9 `survival-trade`

## Problem

`SURVTRADE-001` now lands the explicit AI substitute-goal emission bridge, but row 9 still is not truthfully landed. The live branch can still surface rival same-category acquisition candidates after the substitute goal appears, and there is still no focused golden/selection proof showing the substitute branch is the selected causal reason rather than an incidental available alternative.

## Assumption Reassessment (2026-04-23)

1. `crates/worldwake-ai/src/candidate_generation.rs` now emits a preferred substitute-backed explicit `AcquireCommodity(SelfConsume)` goal with seller-backed evidence when the preferred local trade path is missing.
2. `crates/worldwake-systems/src/trade_actions.rs` still owns deterministic stored-order and valuation-approved substitute selection at the lower trade seam.
3. The remaining live boundary is no longer "can AI emit a substitute goal?" but "does ranking/selection choose the substitute branch at the truthful proof surface, and can row 9 prove that branch rather than a rival lawful food path?"
4. The exact mixed-layer boundary under audit is candidate generation / ranking / plan selection for `GoalKind::AcquireCommodity { purpose: SelfConsume }` plus the backing `golden_survival_trade.rs` proof surface.
5. `docs/scenario-roadmap.md` still says row 9 is blocked on explicit planner/runtime integration for substitute preferences. After `SURVTRADE-001`, that wording is stale for the candidate-generation seam.
6. Focused live evidence now shows the remaining contradiction splits in two: the AI ranking/selection seam was still live and fixable, but the authored row-9 golden still proves the bread-market branch because `scenarios/survival-trade.ron` stages Bread and still lists `SubstitutePreferences` as `{ Food: [Bread, Apple, Grain] }`.
7. The exact mixed-layer boundary that landed here is `ranking::compare_ranked_goals` for same-category `GoalKind::AcquireCommodity { purpose: SelfConsume }`, plus the existing explicit-trade payload proof in `goal_model`.
8. Mismatch + correction: the original row-proof scope was too broad for the live authored scenario. This ticket now owns substitute-aware AI selection preference; scenario/golden isolation for row 9 moves to follow-up `SURVTRADE-003`.

## Architecture Check

1. This follow-up keeps the separation clean: explicit substitute goal emission stays in candidate generation, while branch preference/row-proof ownership moves to the ranking/selection and golden layers that actually decide and prove the chosen path.
2. No backwards-compatibility aliasing or hidden payload rewrites are needed; the remaining work is about truthful branch preference and proof, not about adding a second runtime mutation path.

## Verification Layers

1. Preferred substitute wins over rival same-category acquisition branches -> focused ranking proof
2. Selected branch remains an explicit substitute commodity trade path -> focused goal-model trade-payload proof
3. Existing roadmap golden still proves only the bread-market branch -> authored `golden_survival_trade.rs` run plus scenario-file reassessment

## What to Change

### 1. Selection preference

Make the live AI branch prefer the substitute-backed explicit self-consume trade goal at the truthful ranking/selection seam when that is the intended substitute path.

### 2. Truthful row-9 closeout

Update the roadmap wording to reflect the landed focused AI seam and move substitute-isolation golden ownership to a follow-up ticket.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` and/or adjacent selection surfaces (modify)
- `crates/worldwake-ai/tests/` (modify/add focused proof)
- `crates/worldwake-ai/tests/golden_survival_trade.rs` (no-change cited file; rerun as the disproving roadmap-owned proof surface)
- `docs/scenario-roadmap.md` (modify when the row wording can be truthfully updated)
- `crates/worldwake-ai/src/goal_model.rs` (no-change cited file; existing explicit trade payload proof reused)
- `crates/worldwake-sim/src/save_load.rs` (modify for persisted ranking provenance shape)

## Out of Scope

- Reopening the lower substitute helper in `trade_actions.rs` unless reassessment proves the lower valuation/order seam regressed
- Commit-time payload rewriting of already-started trade actions
- Recipe-input or merchant-restock substitute planning

## Acceptance Criteria

### Tests That Must Pass

1. A focused AI selection test proves the preferred substitute-backed explicit trade goal wins over rival same-category acquisition branches at the truthful live seam.
2. A focused trade-payload proof shows the selected branch stays an explicit substitute commodity trade path.
3. The existing roadmap golden still passes as the truthful bread-market proof surface while row 9 remains `In Progress`.

### Invariants

1. Substitute pursuit remains an explicit commodity goal and explicit trade path; no hidden payload mutation is introduced.
2. The roadmap row is only marked landed when the substitute branch is behaviorally proved, not merely structurally active.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — focused ranking proof for the substitute branch
2. `crates/worldwake-ai/src/goal_model.rs` — reuse the explicit trade-payload proof at the selected-branch seam
3. `crates/worldwake-ai/tests/golden_survival_trade.rs` — rerun the existing bread-market golden as the disproving row-owned proof surface

### Commands

1. `cargo test -p worldwake-ai ranking::tests::substitute_preference_order_outranks_same_category_self_consume_rival -- --exact`
2. `cargo test -p worldwake-ai goal_model::tests::acquire_goal_builds_trade_payload_override_from_goal_semantics -- --exact`
3. `cargo test -p worldwake-ai --test golden_survival_trade survival_trade_proves_live_market_branch -- --ignored --exact`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-23.

- Added planner-visible substitute preference rank carriage on self-consume drive provenance and taught `ranking::compare_ranked_goals` to prefer earlier stored substitute order over rival same-category `AcquireCommodity(SelfConsume)` branches.
- Added focused ranking coverage proving the preferred substitute outranks a same-category rival even when raw commodity ordering would previously have picked the wrong branch.
- Reused the existing `goal_model` trade-payload proof to keep the selected branch on an explicit substitute commodity trade path.
- Updated `docs/scenario-roadmap.md` to reflect the landed AI selection seam while keeping row 9 `In Progress`.
- Created follow-up `SURVTRADE-003` for the still-unproved roadmap-owned substitute scenario/golden isolation seam.

## Deviations

- The stronger row-9 roadmap landing remains false on the live branch. `scenarios/survival-trade.ron` still stages Bread and still authors `SubstitutePreferences` as `{ Food: [Bread, Apple, Grain] }`, so `golden_survival_trade.rs` truthfully remains a bread-market proof rather than a substitute-isolation proof.
- Because `RankedDriveGoalProvenance` is serialized through agenda state, this ticket also bumps `SAVE_FORMAT_VERSION` to keep the current-format save boundary honest.

## Verification Result

- Passed `cargo test -p worldwake-ai ranking::tests::substitute_preference_order_outranks_same_category_self_consume_rival -- --exact`
- Passed `cargo test -p worldwake-ai goal_model::tests::acquire_goal_builds_trade_payload_override_from_goal_semantics -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_trade survival_trade_proves_live_market_branch -- --ignored --exact`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
