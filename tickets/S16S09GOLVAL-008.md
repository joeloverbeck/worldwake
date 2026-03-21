# S16S09GOLVAL-008: Golden — Refactor Spatial Multi-Hop Golden Helper into Smaller Assertion Boundaries

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `archive/tickets/completed/S16S09GOLVAL-004.md`, `docs/golden-e2e-testing.md`

## Problem

The current spatial golden scenario is behaviorally correct, but its main helper is too large and now relies on a targeted `#[allow(clippy::too_many_lines)]` to satisfy the repo verification baseline.

That is not a production-architecture problem, but it is still a test-architecture smell. The helper currently mixes three separate responsibilities in one long function:

1. scenario setup and observation loop
2. decision-trace assertions about initial plan selection
3. action-trace / authoritative-world assertions about downstream travel, harvest, and hunger relief

Keeping those boundaries fused makes the test harder to extend, harder to debug when one assertion family fails, and more likely to accumulate more suppression-only exceptions over time.

## Assumption Reassessment (2026-03-21)

1. The relevant symbol is `run_spatial_multi_hop_plan_scenario` in `crates/worldwake-ai/tests/golden_ai_decisions.rs`. Reassessment confirmed that this is the only symbol implicated in the clippy failure that surfaced during `scripts/verify.sh`.
2. The current test behavior is already correct and already covered. `golden_spatial_multi_hop_plan` and `golden_spatial_multi_hop_plan_replays_deterministically` exist and pass. This ticket is about test structure, not missing behavior.
3. The current helper lawfully proves multiple layers at once, consistent with `docs/golden-e2e-testing.md`:
   - decision trace for initial selected path
   - action trace for remote harvest lifecycle
   - authoritative world state for travel progression and hunger relief
   The problem is not assertion choice; it is that all of them are embedded into one oversized helper.
4. This remains a tests-only ticket. Reassessment found no production contradiction in planner/runtime behavior, no authoritative-world bug, and no missing lower-layer substrate. The issue is maintainability of the golden test harness code.
5. The clean refactor boundary is to split the existing helper into smaller functions around already-distinct causal layers rather than weakening the test or changing its scenario semantics.
6. Ordering contract: unchanged from the current spatial golden. The ticket does not introduce a new timing contract. It should preserve the current semantic proof shape:
   - tick-0 selected plan proves the initial travel-led route toward `SouthGate`
   - downstream world/action assertions prove the remote OrchardFarm acquisition chain completes
7. Scenario isolation remains unchanged. The current VillageSquare-only-food-at-OrchardFarm setup is intentional and should be preserved exactly; this ticket must not broaden the scenario with new lawful competing affordances.
8. No similarly named helper in another file currently owns this exact scenario. `observe_multi_hop_travel_step` and `MultiHopTravelObservation` already exist in the same file and are the natural lower-level building blocks for a refactor.
9. Corrected scope: remove the local clippy suppression by restructuring the helper into smaller scenario/assertion helpers, not by weakening lint settings, suppressing clippy more broadly, or moving the test into another file without need.

## Architecture Check

1. The cleaner architecture is to separate "run the scenario", "assert initial selected path", and "assert downstream remote acquisition chain" into small helpers with explicit data handoff. That keeps the golden aligned with the assertion hierarchy in `docs/golden-e2e-testing.md`.
2. This is better than retaining the current `#[allow(clippy::too_many_lines)]` because the refactor improves readability and failure localization without altering the test's semantic contract.
3. This is better than deleting assertions to satisfy clippy. The current multi-layer proof shape is architecturally useful and should remain intact.
4. No backwards-compatibility shims, aliases, or duplicate helper paths should be added. Replace the monolithic helper directly.

## Verification Layers

1. Tick-0 plan selection remains travel-led toward `SouthGate` -> decision trace assertion helper
2. Downstream OrchardFarm travel/harvest/hunger-relief chain remains intact -> authoritative world state + action trace assertion helper
3. Deterministic replay contract remains unchanged -> existing deterministic replay golden
4. Additional runtime/action/world verification layering is already part of the current golden and should be preserved, not expanded
5. This ticket changes only test structure, so no production-layer verification beyond the owning golden binary and repo baseline is applicable

## What to Change

### 1. Split the spatial scenario helper into smaller units

Refactor `run_spatial_multi_hop_plan_scenario` into smaller helpers such as:

- one helper to build/run the VillageSquare scenario and return observation state plus hashes
- one helper to assert the tick-0 decision-trace selection boundary
- one helper to assert the downstream travel/harvest/hunger-relief outcomes

The exact helper names can follow file-local style, but the split should make each helper single-purpose and small enough to satisfy clippy without local suppression.

### 2. Preserve the current semantic contract exactly

Do not weaken or remove the existing assertions. The refactor should preserve:

- `SelectedPlanSource::SearchSelection`
- initial `Travel` step toward `SouthGate`
- root travel-pruning provenance checks
- remote harvest lifecycle visibility
- authoritative arrival and hunger-relief checks
- deterministic replay

### 3. Remove the local clippy suppression

Once the helper is split cleanly, remove the targeted `#[allow(clippy::too_many_lines)]`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)

## Out of Scope

- Any planner, runtime, or authoritative-world behavior changes
- Any golden scenario setup changes
- Moving the spatial goldens into a new test file
- Adding new scenario coverage
- Broad clippy policy changes or more suppression attributes

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan`
2. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan_replays_deterministically`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh`

### Invariants

1. The current spatial golden proves the same decision-trace, action-trace, and authoritative-world contracts after refactor
2. No local clippy suppression remains on the spatial scenario helper path
3. Scenario isolation and deterministic replay remain unchanged

## Test Plan

### New/Modified Tests

1. `golden_spatial_multi_hop_plan` in `crates/worldwake-ai/tests/golden_ai_decisions.rs` — unchanged scenario, but revalidated after helper refactor to ensure the decision-trace and downstream world/action assertions still hold
2. `golden_spatial_multi_hop_plan_replays_deterministically` in `crates/worldwake-ai/tests/golden_ai_decisions.rs` — unchanged deterministic replay contract after helper refactor

### Commands

1. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan`
2. `cargo test -p worldwake-ai golden_spatial_multi_hop_plan_replays_deterministically`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh`
