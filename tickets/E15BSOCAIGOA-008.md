# E15BSOCAIGOA-008: Golden social tests T5–T7 (bystander, stale belief, entity missing)

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — test file only
**Deps**: E15BSOCAIGOA-006, E15BSOCAIGOA-007

## Problem

Tests T5–T7 cover information locality (bystanders don't receive told beliefs), stale belief travel-and-discover scenarios, and entity-missing discovery. These are Tier 1 tests requiring no spec changes but exercising deeper E15 mechanics.

## Assumption Reassessment (2026-03-15)

1. Bystander observation (WitnessedTelling in SocialObservationKind) exists in core. Confirmed.
2. EntityMissing discovery event exists from E15. Confirmed.
3. Passive observation on arrival at a place fires discovery events for mismatched beliefs. Confirmed in E15 spec.
4. `golden_social.rs` will exist after E15BSOCAIGOA-007.

## Architecture Check

1. Adds 3 more tests to existing golden_social.rs file.
2. Each test is self-contained and independent.
3. T6 is the most complex (multi-tick: travel → arrive → observe → replan).

## What to Change

### 1. Add T5–T7 to golden_social.rs

**T5: `golden_bystander_sees_telling_but_gets_no_belief`**
- Setup: 3 agents (A, B, C) co-located. A tells B about remote resource.
- Inject Tell(A→B).
- Assert: C records WitnessedTelling social observation. C has NO belief about the resource. C does NOT generate travel goals to resource location.
- Checks: Determinism, information locality (Principle 7).

**T6: `golden_stale_belief_travel_reobserve_replan`**
- Setup: Agent at Village Square with stale belief (observed_tick far in past) about apples at Orchard Farm. Orchard depleted (Quantity(0)). Agent hungry.
- Step simulation — agent should plan Travel based on stale belief.
- Assert: Agent travels to Orchard Farm → passive observation → InventoryDiscrepancy Discovery → replans (seeks alternative food or switches goal).
- Checks: Conservation, determinism. Validates belief-only planning (agent acts on stale belief, not world state).

**T7: `golden_entity_missing_discovery_updates_belief`**
- Setup: Agent A believes Agent B at Village Square. B has since traveled to Orchard Farm. A is elsewhere, then travels to Village Square.
- Assert: A arrives → passive observation does NOT see B → EntityMissing Discovery fires → A's belief about B updated (no longer at Village Square).
- Checks: Determinism.

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (modify — append tests)

## Out of Scope

- Tests T1–T4 (E15BSOCAIGOA-007)
- Tests T8–T13 (E15BSOCAIGOA-009, E15BSOCAIGOA-010)
- Production code changes
- Harness modifications (E15BSOCAIGOA-006)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_bystander_sees_telling_but_gets_no_belief` — bystander gets social observation but NOT the told belief content
2. `golden_stale_belief_travel_reobserve_replan` — stale belief drives travel, observation corrects, agent replans
3. `golden_entity_missing_discovery_updates_belief` — EntityMissing discovery updates belief about absent agent
4. All 3 tests verify determinism
5. T6 verifies conservation
6. Existing suite: `cargo test -p worldwake-ai --test golden_social` — all T1–T7 pass

### Invariants

1. Bystanders receive social observations but NOT belief content (Principle 7 — information locality)
2. Agents plan from beliefs, never from world state (even when beliefs are stale)
3. Discovery events update beliefs based on observation, not telepathy
4. Conservation verified where items exist

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — 3 new golden E2E tests (T5–T7)

### Commands

1. `cargo test -p worldwake-ai --test golden_social`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
