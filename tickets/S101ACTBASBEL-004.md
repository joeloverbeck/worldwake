# S101ACTBASBEL-004: Golden tests for activation decay

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S101ACTBASBEL-003

## Problem

The activation-based belief decay system needs E2E golden tests that verify the emergent behavior: entities fade from memory when not re-observed, frequently observed entities persist, need-gated salience keeps survival-critical items alive during crises, no hard capacity wall prevents agents from knowing items, and stale claims are pruned by confidence threshold. The "Forager Lina" regression (starving because items were evicted by hard cap) must never recur.

## Assumption Reassessment (2026-04-13)

1. After tickets 001-003, `prune_decayed_beliefs` is the active pruning function, `PerceptionProfile` has activation-based fields, `BelievedEntityState` has the ring buffer. All golden test harness infrastructure exists in `crates/worldwake-ai/tests/golden_harness/`.
2. Golden tests follow the pattern in existing `golden_*.rs` files: scenario setup with agents, places, items → run simulation for N ticks → assert belief store state or action outcomes.
3. `PerceptionProfile` defaults: `entity_activation_threshold: Permille(100)`, `observation_buffer_capacity: 5`, `need_salience_boost: Permille(500)`, `need_salience_urgency_threshold: Permille(500)`.
4. From the spec's reference table: a single observation has activation=100 at age 100 ticks (exactly at default threshold). At age 101+, a single observation drops below threshold and should be pruned.
12. Golden test scenarios need `PerceptionProfile` on agents that need to observe, and may need custom profiles to exercise diversity parameters.

## Architecture Check

1. Golden tests verify emergent behavior (FND-1) — they don't script outcomes, they set up initial conditions and verify the system produces correct belief decay patterns.
2. No backward-compatibility shims — tests exercise only the new activation-based system.

## Verification Layers

1. Entity decay timing → golden E2E: agent observes items, travels away, verify items pruned after ~100 ticks
2. Persistence under re-observation → golden E2E: agent stays at location, verify items remain indefinitely
3. Salience boost → golden E2E: hungry agent retains item knowledge longer than baseline
4. No capacity wall → golden E2E: Lina-reproduction scenario, verify items + infrastructure both retained
5. Claim confidence threshold → golden E2E: stale tell-claims pruned, fresh claims persist
6. All verification is at golden E2E layer — these tests exercise the full simulation stack.

## What to Change

### 1. `golden_activation_decay_prunes_stale_entities`

**Setup**: 2 places (A, B) connected. 1 agent at A. 5 item lots at A. Agent observes items at A, then travels to B and stays.
**Run**: 200 ticks.
**Assert**: After ~100 ticks at B without re-observation, items from A are pruned from agent's belief store. Agent's known_entities count decreases over time.

### 2. `golden_frequently_observed_entities_persist`

**Setup**: 1 place. 1 agent. 3 item lots at the same place. Agent stays.
**Run**: 500 ticks.
**Assert**: Items remain in agent's beliefs for the entire duration due to continuous re-observation. known_entities always includes the 3 items.

### 3. `golden_need_salience_prevents_item_decay`

**Setup**: 2 places (A, B). 1 agent at A with hunger=750 (above default urgency threshold 500). Items at A. Agent travels to B and stays.
**Run**: 200 ticks.
**Assert**: Items from A persist longer than they would without salience boost. Compare against the reference: with salience boost of ~375 (750*500/1000), items survive until activation + 375 < 100, i.e., much longer than the baseline ~100 ticks.

### 4. `golden_no_capacity_wall_with_many_places`

**Setup**: Reproduces the Forager Lina scenario: 3+ places, 5+ facilities, ground items. 1 agent with default perception profile (no custom capacity).
**Run**: 300 ticks.
**Assert**: Agent retains knowledge of BOTH items AND infrastructure. `pick_up` affordances are generated when agent is at a location with items. This is the direct regression guard — the failure mode that motivated the spec must never recur.

### 5. `golden_claim_confidence_threshold_prunes_stale_claims`

**Setup**: 2 agents (teller, listener) at same place. Teller tells listener about several entities. Time passes without re-observation.
**Run**: 300 ticks.
**Assert**: Low-confidence stale claims are pruned from listener's belief store. Fresh/direct-observation claims persist. No hard count limit — if all claims are fresh, all persist.

## Files to Touch

- `crates/worldwake-ai/tests/golden_activation_decay.rs` (new) — all 5 golden tests
- `crates/worldwake-ai/Cargo.toml` (modify) — if new test file needs to be registered (typically auto-discovered)

## Out of Scope

- Unit tests for activation computation (ticket 001)
- Unit tests for pruning logic (ticket 003)
- Commodity-specific salience (spec non-goal)
- Variable decay exponent (spec non-goal)
- Forgetting curve visualization (spec non-goal)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_activation_decay_prunes_stale_entities` — items fade from memory after ~100 ticks
2. `golden_frequently_observed_entities_persist` — continuous observation prevents decay
3. `golden_need_salience_prevents_item_decay` — hungry agents retain item knowledge longer
4. `golden_no_capacity_wall_with_many_places` — no capacity wall, items + infrastructure coexist
5. `golden_claim_confidence_threshold_prunes_stale_claims` — stale claims pruned, no hard count
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No agent ever loses item knowledge solely due to a capacity number
2. Frequently observed entities always survive pruning
3. Need-gated salience applies only to ItemLot entities (not agents, places, facilities)
4. Claim pruning is confidence-based, not count-based

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_activation_decay.rs` — 5 golden E2E tests covering all spec scenarios

### Commands

1. `cargo test -p worldwake-ai -- golden_activation_decay`
2. `cargo test -p worldwake-ai -- golden_frequently_observed`
3. `cargo test -p worldwake-ai -- golden_need_salience`
4. `cargo test -p worldwake-ai -- golden_no_capacity_wall`
5. `cargo test -p worldwake-ai -- golden_claim_confidence_threshold`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo test --workspace`
