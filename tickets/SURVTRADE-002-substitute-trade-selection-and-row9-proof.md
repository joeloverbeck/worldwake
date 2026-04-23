# SURVTRADE-002: Prefer the Substitute Trade Branch at Selection and Row-9 Proof Surfaces

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI ranking/selection and roadmap-row substitute proof
**Deps**: `archive/tickets/completed/SURVTRADE-001-substitute-trade-planner-integration.md`; `docs/scenario-roadmap.md` row 9 `survival-trade`

## Problem

`SURVTRADE-001` now lands the explicit AI substitute-goal emission bridge, but row 9 still is not truthfully landed. The live branch can still surface rival same-category acquisition candidates after the substitute goal appears, and there is still no focused golden/selection proof showing the substitute branch is the selected causal reason rather than an incidental available alternative.

## Assumption Reassessment (2026-04-23)

1. `crates/worldwake-ai/src/candidate_generation.rs` now emits a preferred substitute-backed explicit `AcquireCommodity(SelfConsume)` goal with seller-backed evidence when the preferred local trade path is missing.
2. `crates/worldwake-systems/src/trade_actions.rs` still owns deterministic stored-order and valuation-approved substitute selection at the lower trade seam.
3. The remaining live boundary is no longer "can AI emit a substitute goal?" but "does ranking/selection choose the substitute branch at the truthful proof surface, and can row 9 prove that branch rather than a rival lawful food path?"
4. The exact mixed-layer boundary under audit is candidate generation / ranking / plan selection for `GoalKind::AcquireCommodity { purpose: SelfConsume }` plus the backing `golden_survival_trade.rs` proof surface.
5. `docs/scenario-roadmap.md` still says row 9 is blocked on explicit planner/runtime integration for substitute preferences. After `SURVTRADE-001`, that wording is stale for the candidate-generation seam but still truthful for the stronger selected-branch/golden claim.
6. Required consequence of this follow-up: if the live branch still keeps rival same-category candidates lawful, the golden must isolate and prove the substitute branch honestly instead of inferring row landing from survival alone.

## Architecture Check

1. This follow-up keeps the separation clean: explicit substitute goal emission stays in candidate generation, while branch preference/row-proof ownership moves to the ranking/selection and golden layers that actually decide and prove the chosen path.
2. No backwards-compatibility aliasing or hidden payload rewrites are needed; the remaining work is about truthful branch preference and proof, not about adding a second runtime mutation path.

## Verification Layers

1. Preferred substitute wins over rival same-category acquisition branches -> focused ranking/selection or plan-selection proof
2. Selected branch remains an explicit substitute commodity trade path -> focused AI/runtime plan proof or action-trace proof at the selected-plan seam
3. Row-9 roadmap claim is truthful -> `golden_survival_trade.rs` or a focused replacement golden that excludes rival lawful food branches

## What to Change

### 1. Selection preference

Make the live AI branch prefer the substitute-backed explicit self-consume trade goal at the truthful ranking/selection seam when that is the intended substitute path.

### 2. Truthful row-9 proof

Update the roadmap-owned proof surface so the substitute branch is proven for the authored causal reason instead of remaining structurally active only.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` and/or adjacent selection surfaces (modify)
- `crates/worldwake-ai/tests/` (modify/add focused proof)
- `crates/worldwake-ai/tests/golden_survival_trade.rs` (modify if it becomes the truthful row-9 proof surface)
- `docs/scenario-roadmap.md` (modify when the row wording can be truthfully updated)

## Out of Scope

- Reopening the lower substitute helper in `trade_actions.rs` unless reassessment proves the lower valuation/order seam regressed
- Commit-time payload rewriting of already-started trade actions
- Recipe-input or merchant-restock substitute planning

## Acceptance Criteria

### Tests That Must Pass

1. A focused AI selection test proves the preferred substitute-backed explicit trade goal wins over rival same-category acquisition branches at the truthful live seam.
2. A focused or golden proof shows the selected branch stays an explicit substitute commodity trade path.
3. If row 9 wording changes, the backing golden named in the roadmap passes and proves the substitute branch for the authored reason.

### Invariants

1. Substitute pursuit remains an explicit commodity goal and explicit trade path; no hidden payload mutation is introduced.
2. The roadmap row is only marked landed when the substitute branch is behaviorally proved, not merely structurally active.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/...` — focused ranking/selection proof for the substitute branch
2. `crates/worldwake-ai/tests/golden_survival_trade.rs` — truthful row-9 proof only if the scenario can isolate the substitute branch honestly

### Commands

1. `cargo test -p worldwake-ai <focused substitute selection test>`
2. `cargo test -p worldwake-ai <truthful substitute golden or focused integration proof>`
3. `cargo clippy --workspace --all-targets -- -D warnings`
