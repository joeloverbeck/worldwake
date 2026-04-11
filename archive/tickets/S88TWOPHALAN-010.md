# S88TWOPHALAN-010: Broaden two-phase planner integration beyond remote care

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — expands planner-root two-phase integration in worldwake-ai
**Deps**: S88TWOPHALAN-003, S88TWOPHALAN-004, S88TWOPHALAN-005, S88TWOPHALAN-006, S88TWOPHALAN-007

## Problem

`S88TWOPHALAN-007` landed the safe remote-`TreatWounds` slice of the two-phase planner, but the broader S88 rollout remains incomplete. `ProduceCommodity` and other staged strategic families still fall back to the flat planner path, so the spec’s wider strategic decomposition and landmark-guided rollout is not yet live.

## Assumption Reassessment (2026-04-11)

1. `search_plan()` now has the two-phase substrate live at the planner root: `DualFrontier`, tactical spatial guidance, landmark heuristic combination, and transition-derived landmark operators all exist in `crates/worldwake-ai/src/search/`.
2. `S88TWOPHALAN-007` intentionally narrowed live activation to remote `TreatWounds` after broadened verification showed that enabling additional strategic families immediately regressed conformance and golden behavior. The archived outcome for `S88TWOPHALAN-007` is the authoritative evidence for that narrowing.
3. The remaining scope is not generic “turn on all strategic families.” Each candidate family must be reassessed against its current `GoalKind` contract, tactical destination semantics, barrier/goal-satisfaction behavior, and existing conformance/golden ownership before activation.
4. `ProduceCommodity` is the nearest staged family because `strategic::plan()` already derives recipe-input and goal-location stages, but the current planner contract still expects the full bounded multistep production plan shape proved by `crates/worldwake-ai/tests/conformance_execution_budget.rs`.
5. Commodity-search/social-query families are not automatically lawful follow-ons from the existing `AskWitness` path. Reassessment must name the exact goal family and current operator surface before reusing or extending that epistemic barrier machinery.
6. This is planner-root work. Per `docs/planner-contracts.md`, the audit surface includes `search/mod.rs`, `search/transition.rs`, `goal_model.rs`, current planner traces, and the affected conformance/golden proof surfaces.
7. If a family cannot be integrated without widening trace, golden, or helper ownership beyond this ticket’s intended slice, split the work again instead of silently broadening the ticket.

## Architecture Check

1. Family-by-family integration is cleaner than another “all remaining strategic goals” batch because the current planner still has goal-specific contracts around barriers, candidate synthesis, and bounded plan-shape expectations.
2. No backwards-compatibility aliasing or duplicate planner paths should be introduced. Each newly activated family should use the existing two-phase substrate directly.

## Verification Layers

1. Newly activated goal-family plan shape -> focused `search::tests` or conformance test for that exact `GoalKind`
2. Planner-root regression safety -> `cargo test -p worldwake-ai -- search::tests`
3. Existing bounded-production contract (if `ProduceCommodity` is activated) -> `cargo test -p worldwake-ai --test conformance_execution_budget`
4. Golden behavioral preservation for the activated family -> targeted existing golden file(s) for that family
5. Full planner regression surface -> `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`

## What to Change

### 1. Reassess one remaining strategic goal family at a time

Start with the strongest live candidate (`ProduceCommodity` unless reassessment disproves it). Name the exact `GoalKind`, current terminal/barrier behavior, and current golden/conformance proof that must be preserved.

### 2. Extend the tactical activation gate lawfully

Update `search_plan()` so the chosen family can enter the existing strategic/tactical path without regressing non-target families or breaking the already-landed remote-care slice.

### 3. Preserve existing family-specific planner contracts

If the activated family needs prerequisite-first narrowing, ensure the planner resumes the normal downstream search once the prerequisite is satisfied rather than freezing on an intermediate tactical slice.

### 4. Add focused proof for the newly activated family

Add or update the narrowest tests that prove the family now benefits from the two-phase path while preserving the existing plan-shape contract.

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/transition.rs` (modify if family-specific tactical continuation requires it)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/tests/conformance_execution_budget.rs` (modify if `ProduceCommodity` is activated and conformance expectations need focused proof updates)
- other family-owning golden/conformance surfaces only if reassessment proves they are part of the same live slice

## Out of Scope

- Decision trace enrichment (S88TWOPHALAN-008)
- New golden scenario families beyond the activated goal-family proof needed to keep existing behavior honest
- Broad commodity-query/social-query design unless reassessment proves a specific live goal family already owns it

## Acceptance Criteria

### Tests That Must Pass

1. Focused proof for the newly activated strategic family
2. `cargo test -p worldwake-ai -- search::tests`
3. Any affected existing conformance/golden target for that family
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

### Invariants

1. Non-activated goal families remain on their current lawful planner path
2. Newly activated families still plan from beliefs only (FND-14)
3. The activated family preserves or improves its existing bounded plan-shape contract rather than regressing it
4. No duplicate planner path or compatibility shim is introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — focused proof for the newly activated family
2. Existing conformance/golden file for that family — only if reassessment proves it is the owning proof surface

### Commands

1. `cargo test -p worldwake-ai -- search::tests`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completion date: 2026-04-11

What changed:

- `search_plan()` now activates the existing two-phase planner substrate for `GoalKind::ProduceCommodity` in addition to the previously landed remote-`TreatWounds` slice.
- Added focused planner-root proof in `crates/worldwake-ai/src/search/tests.rs` for remote recipe-input acquisition, including zero-landmark degradation.
- Preserved the existing production-family contract: the planner still returns a full remote production path with travel, pickup, return travel, and craft.

Deviations from original plan:

- Reassessment confirmed that `ProduceCommodity` is the only additional family justified by the current conformance and golden ownership. Broader strategic-family rollout remains deferred rather than being folded into this ticket.
- No `search/transition.rs` or conformance-file changes were needed for the lawful `ProduceCommodity` slice once the planner-root activation gate and focused proof were in place.

Verification results:

- `cargo test -p worldwake-ai -- search::tests::search_produce_commodity_uses_two_phase_pick_up_before_craft` passed
- `cargo test -p worldwake-ai -- search::tests::search_produce_commodity_with_zero_landmarks_preserves_two_phase_plan_shape` passed
- `cargo test -p worldwake-ai -- search::tests` passed
- `cargo test -p worldwake-ai --test conformance_execution_budget` passed
- `cargo test -p worldwake-ai --test golden_production golden_remote_acquire_commodity_recipe_input` passed
- `cargo test -p worldwake-ai --test planner_conformance conformance_craft_noop_coverage_gap` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test --workspace` passed
