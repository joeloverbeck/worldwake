# GOLDEN-003: Memory Retention Backlog Reassessment (Scenario 11)

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None expected; this ticket should correct stale test assumptions first, then add focused coverage only if a real planner-boundary gap remains
**Deps**: None

## Problem

`reports/golden-e2e-coverage-analysis.md` still treats Scenario 11 as a missing golden end-to-end test and assumes the clean solution is a new `crates/worldwake-ai/tests/golden_perception.rs` scenario where an agent forgets a distant food source and then discovers a colocated local one.

Those assumptions need to be rechecked against the current tree before adding another golden layer. If the current architecture already has the right lower-level boundaries and the proposed golden setup misstates how perception and retention actually work, the ticket should be corrected before any test implementation.

## Report Reference

Backlog item **Scenario 11** in `reports/golden-e2e-coverage-analysis.md`.

## Assumption Reassessment (2026-03-14)

1. `AgentBeliefStore::enforce_capacity()` exists in `crates/worldwake-core/src/belief.rs` and already has direct unit coverage for stale-entity eviction.
2. `enforce_capacity()` is only invoked from the perception pipeline in `crates/worldwake-systems/src/perception.rs`, specifically when passive/entity observation or witnessed-event processing refreshes a belief store. The ticket's old wording implied unconditional time-based decay after enough ticks pass; that is not how the current code works.
3. A colocated food source present from tick 0 would be observed immediately by passive same-place perception with a reliable `PerceptionProfile`. The old scenario claim that Dana could remain unaware of apples sitting at `VILLAGE_SQUARE` until after forgetting `ORCHARD_FARM` is therefore stale.
4. `crates/worldwake-ai/tests/golden_perception.rs` still does not exist, but that absence alone does not justify creating a new golden binary if the better proof belongs at the runtime/planner boundary.
5. `crates/worldwake-ai/src/agent_tick.rs` already contains focused runtime tests for adjacent E14 contracts:
   - `same_place_perception_seeds_seller_belief_for_runtime_candidates`
   - `unseen_seller_relocation_preserves_stale_acquisition_belief`
   - `unseen_death_does_not_create_corpse_reaction_without_reobservation`
6. The prototype topology assumption is directionally correct: `VILLAGE_SQUARE -> SOUTH_GATE -> EAST_FIELD_TRAIL -> ORCHARD_FARM` is a real multi-hop path. But the stale-local-discovery setup still overclaims what same-place perception would allow.

## Architecture Check

1. A new golden end-to-end file is not currently the cleanest proof for this invariant. The missing contract is narrower: expired remote beliefs should stop driving acquisition candidates once perception refresh triggers retention enforcement.
2. The most direct and durable place to prove that is the existing runtime boundary in `crates/worldwake-ai/src/agent_tick.rs`, where the repo already tests how `PerAgentBeliefView` affects candidate generation.
3. A golden scenario that depends on thirst timing, action durations, and a supposedly "undiscovered" colocated food lot would be more brittle and less honest than a focused runtime test that encodes the actual architecture.
4. No backwards-compatibility shims, alias files, or duplicate harness layers should be introduced just to preserve the original `golden_perception.rs` plan.

## What to Change

### 1. Correct the ticket scope before any implementation work

Rewrite this ticket around planner-boundary coverage and backlog cleanup, not around creating `golden_perception.rs`.

### 2. Add focused runtime coverage only for the real uncovered gap

Add or update tests in `crates/worldwake-ai/src/agent_tick.rs` to prove:

- a stale remote acquisition belief still influences candidate generation until a later perception refresh enforces retention
- once that refresh occurs after the retention window, the stale belief is evicted and the acquisition goal disappears

This should validate the real architecture without inventing a fragile same-place-discovery golden setup.

### 3. Update `reports/golden-e2e-coverage-analysis.md`

- Remove Scenario 11 from the golden backlog if the focused runtime tests prove the contract more directly than a new golden binary would.
- Add a short note explaining why the original golden setup was stale:
  - colocated local food would be perceived immediately
  - retention enforcement is perception-refresh-driven, not a standalone forgetting system
- Update the pending backlog summary and recommended implementation order accordingly.

### 4. Keep production-code changes out unless reassessment proves a real engine flaw

Do not change `PerceptionProfile`, `enforce_capacity()`, or the perception pipeline just to satisfy the stale golden scenario. If the reassessment reveals a genuine architectural defect worth fixing, that should become a separate engine ticket rather than being smuggled in here.

## Files to Touch

- `tickets/GOLDEN-003.md` (this reassessment)
- `crates/worldwake-ai/src/agent_tick.rs` (focused runtime tests if needed)
- `reports/golden-e2e-coverage-analysis.md` (remove/update stale backlog item)

## Out of Scope

- New `crates/worldwake-ai/tests/golden_perception.rs`
- New production-code changes in `worldwake-core`, `worldwake-sim`, `worldwake-systems`, or `worldwake-ai/src/` unless the reassessment finds a real engine bug
- Reframing retention as a full standalone forgetting system inside this ticket
- Multi-agent belief divergence or gossip/report propagation scenarios

## Acceptance Criteria

### Tests That Must Pass

1. Focused runtime tests covering the retention-driven candidate change pass.
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

### Invariants

1. The ticket and golden report no longer claim the stale same-place-local-discovery golden scenario is the right proof if the repository's current architecture proves the contract more cleanly at runtime level.
2. Expired remote beliefs are shown to stop influencing candidate generation once perception refresh applies retention enforcement.
3. No duplicate golden harness layer is added just to preserve the original plan.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` runtime tests for retention-driven candidate removal after perception refresh.

### Commands

1. `cargo test -p worldwake-ai memory_retention`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - corrected the ticket scope from "add a new golden perception scenario" to "reassess the stale golden backlog item against the current perception/runtime architecture"
  - added focused runtime coverage in `crates/worldwake-ai/src/agent_tick.rs` for the real retention contract:
    - `expired_remote_acquisition_belief_remains_until_perception_refresh`
    - `perception_refresh_evicts_expired_remote_acquisition_belief_and_removes_goal`
  - updated `reports/golden-e2e-coverage-analysis.md` to remove Scenario 11 from the golden backlog and document why a new `golden_perception.rs` file would have been the wrong layer
- Deviations from original plan:
  - did not add `crates/worldwake-ai/tests/golden_perception.rs`
  - did not add a thirst-driven end-to-end scenario with colocated "undiscovered" food, because that setup contradicted the current passive observation rules
  - did not change production code; the reassessment showed the missing coverage was at the runtime-test layer, not in engine behavior
- Verification results:
  - `cargo test -p worldwake-ai expired_remote_acquisition_belief_remains_until_perception_refresh -- --nocapture`
  - `cargo test -p worldwake-ai perception_refresh_evicts_expired_remote_acquisition_belief_and_removes_goal -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
