# S101ACTBASBEL-004: Golden tests for activation decay

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S101ACTBASBEL-003.md

## Problem

The activation-based belief decay system still needs dedicated E2E golden coverage for the parts not already owned by existing suites: entities fading from memory when not re-observed, need-gated item salience extending retention during crises, and stale report claims being pruned by confidence threshold. The broader "no hard capacity wall" and "Forager Lina" regression guards are already covered elsewhere and should be cited rather than duplicated.

## Assumption Reassessment (2026-04-13)

1. After tickets 001-003, `prune_decayed_beliefs` is the active pruning function, `PerceptionProfile` uses activation-based fields, and `BelievedEntityState` retains presentation-tick history. All needed golden harness infrastructure exists in `crates/worldwake-ai/tests/golden_harness/`.
2. Existing golden suites already own two adjacent regression surfaces:
   - `crates/worldwake-ai/tests/golden_perception_exposure.rs` already proves repeated observation plus no hard entity-count clamp for resource-source beliefs.
   - `crates/worldwake-ai/tests/golden_planner_pathology.rs` already proves the late-run "Forager Lina" FreeCarryCapacity regression stays fixed.
3. `PerceptionProfile` defaults still encode the activation-decay contract used by S101: `entity_activation_threshold = 100`, `claim_confidence_threshold = 50`, `need_salience_boost = 500`, `need_salience_urgency_threshold = 500`.
4. From the spec/reference values and the landed helper tests: a single observation yields activation `100` at age `100` ticks and drops below the default threshold at age `101` ticks.
5. Golden test scenarios need explicit `PerceptionProfile` setup whenever the activation/claim arithmetic itself is the contract under test.

## Architecture Check

1. Golden tests verify emergent behavior (FND-1) — they don't script outcomes, they set up initial conditions and verify the system produces correct belief decay patterns.
2. No backward-compatibility shims — tests exercise only the new activation-based system.

## Verification Layers

1. Entity decay timing → golden E2E: a single unrefreshed belief survives at age `100` and disappears on the first prune pass after age crosses `101`.
2. Salience boost → golden E2E: under urgent need, item beliefs survive longer than otherwise-identical non-item or baseline beliefs.
3. Claim confidence threshold → golden E2E: stale report claims are pruned while fresher report claims remain.
4. Adjacent regression ownership remains in existing suites, not this ticket:
   - repeated-observation/no-capacity behavior in `golden_perception_exposure.rs`
   - Lina late-run pathology in `golden_planner_pathology.rs`

## What to Change

### 1. `golden_activation_decay_prunes_stale_entities`

**Setup**: Seed an agent with a direct-observation belief about a remote item lot, then hold the agent inert so no re-observation refresh can occur.
**Run**: Step through the default threshold boundary.
**Assert**: The belief remains present at age `100` ticks and is pruned on the first prune pass after age crosses `101` ticks.

### 2. `golden_need_salience_retains_hungry_item_belief`

**Setup**: Seed an urgently hungry agent with two stale remote beliefs from the same tick: one `ItemLot` and one non-item facility/source.
**Run**: Advance well past the default entity-threshold window.
**Assert**: The item belief survives because need salience applies, while the non-item belief does not receive the salience extension and is pruned.

### 3. `golden_claim_confidence_threshold_prunes_stale_reports`

**Setup**: Seed one listener with two report-backed beliefs using the live claim-recording path: one old enough to decay below `claim_confidence_threshold`, one still fresh enough to remain.
**Run**: Advance past the stale report's confidence window without re-observation.
**Assert**: The stale report claim and derived summary disappear, while the fresher report claim still exists in both `entity_claims` and `known_entities`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_activation_decay.rs` (new) — focused S101 activation-decay golden coverage
- `tickets/S101ACTBASBEL-004.md` (update) — reassessment and closeout
- `docs/generated/golden-*.md` (regenerated) — if the new golden file changes the inventory/docs surface

## Out of Scope

- Unit tests for activation computation (ticket 001)
- Repeating the already-owned no-capacity/resource-source proof from `golden_perception_exposure.rs`
- Repeating the already-owned Forager Lina late-run regression guard from `golden_planner_pathology.rs`
- Unit tests for pruning logic (ticket 003)
- Commodity-specific salience (spec non-goal)
- Variable decay exponent (spec non-goal)
- Forgetting curve visualization (spec non-goal)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_activation_decay_prunes_stale_entities` proves the live age-100 boundary and the first post-age-101 prune pass at the golden layer.
2. `golden_need_salience_retains_hungry_item_belief` proves urgent-need salience extends only item retention.
3. `golden_claim_confidence_threshold_prunes_stale_reports` proves stale report claims are pruned while fresher report claims remain.
4. Existing adjacent regression ownership remains green:
   - `golden_perception_forms_resource_source_beliefs`
   - `degenerate_zero_step_loop_blocks_actionable_goals`
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. A single unrefreshed observation is retained at the exact threshold and pruned on the first prune pass after crossing it.
2. Need-gated salience applies only to `ItemLot` beliefs, not facilities/places/agents.
3. Claim pruning is confidence-based and age-based, not count-based.
4. The broader no-capacity and Lina regression guards stay owned by their existing suites instead of being duplicated here.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_activation_decay.rs` — focused golden E2E coverage for stale entity decay, need salience, and stale report pruning

### Commands

1. `cargo test -p worldwake-ai --test golden_activation_decay`
2. `cargo test -p worldwake-ai -- golden_perception_forms_resource_source_beliefs`
3. `cargo test -p worldwake-ai -- degenerate_zero_step_loop_blocks_actionable_goals`
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`

## Outcome

Completed on 2026-04-14.

- Added `crates/worldwake-ai/tests/golden_activation_decay.rs` with three focused S101 golden scenarios plus deterministic replay companions:
  - `golden_activation_decay_prunes_stale_entities`
  - `golden_need_salience_retains_hungry_item_belief`
  - `golden_claim_confidence_threshold_prunes_stale_reports`
- Kept the broader no-capacity and Forager Lina regression guards on their existing owning surfaces instead of duplicating them:
  - `crates/worldwake-ai/tests/golden_perception_exposure.rs`
  - `crates/worldwake-ai/tests/golden_planner_pathology.rs`
- Regenerated the owned golden inventory/docs with `python3 scripts/golden_inventory.py --write --check-docs`, which added the new generated scenario detail page under `docs/generated/golden-scenario-details/activation-decay.md` and refreshed the inventory/index/matrix pages.

## Deviations

- The original ticket over-claimed a brand-new five-test suite. Honest reassessment showed two of those regression surfaces were already owned elsewhere, so this slice was narrowed to the three activation-decay contracts that were still uncovered.
- The golden stale-entity boundary needed to be phrased as "first prune pass after age crosses 101" rather than a same-sample age-101 assertion, because the live tick-step ordering applies pruning during the subsequent simulation pass.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_activation_decay`
- Passed `cargo test -p worldwake-ai -- golden_perception_forms_resource_source_beliefs`
- Passed `cargo test -p worldwake-ai -- degenerate_zero_step_loop_blocks_actionable_goals`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
