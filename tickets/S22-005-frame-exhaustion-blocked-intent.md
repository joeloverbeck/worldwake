# S22-005: Implement frame exhaustion → BlockedIntent integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new blocked intent creation path on frame exhaustion/assumption failure
**Deps**: S22-002 (IntentionFrame active), S22-003 (assumption evaluation), S22-004 (stalled_ticks tracking), S23 (BlockedIntentMemory compound-keyed system)

## Problem

When an IntentionFrame reaches patience exhaustion (`stalled_ticks >= patience_limit`) or a critical assumption fails, the agent must not immediately re-adopt the same goal. Without a BlockedIntent, the agent would exhaust its frame, replan, immediately re-adopt the same unreachable goal, create a new frame, exhaust again — an infinite adopt-stall-exhaust cycle. The BlockedIntent with TTL breaks this cycle.

## Assumption Reassessment (2026-03-24)

1. `BlockedIntentMemory` uses compound `BlockerKey` with fields: `goal_key`, `place`, `target`, `action_def`. Confirmed from `blocked_intent.rs`.
2. `BlockingFact::PatienceExhausted` and `BlockingFact::AssumptionFailed` are added in S22-001. Both have `blocks_goal_generation() == true`.
3. `budget.structural_block_ticks` provides the TTL for structural blocked intents. Already used in `failure_handling.rs` for other blocked intent types.
4. The domain-specific target entity for `BlockerKey::target` varies: `destination` for Travel, `patient` for Care, `ward` for Escort, `destination` for Errand, `None` for Generic. This mapping must be implemented.
5. S22-003 transitions the frame to `Exhausted` state but does NOT create the BlockedIntent. This ticket adds the BlockedIntent creation as a follow-up to the state transition.
6. This is an AI runtime ticket. The intended layer is `agent_tick`. Needs to be wired into the frame lifecycle where exhaustion is detected.

## Architecture Check

1. Creating BlockedIntents on frame exhaustion follows the existing pattern in `failure_handling.rs` where plan failures create blocked intents. The new path is structurally identical but triggered by frame state rather than action failure.
2. No backward-compatibility concerns — new integration point on new types.

## Verification Layers

1. Patience exhaustion creates `BlockedIntent` with `PatienceExhausted` → focused test
2. Critical assumption failure creates `BlockedIntent` with `AssumptionFailed` → focused test
3. Goal completion does NOT create a BlockedIntent → focused test
4. Voluntary goal switch does NOT create a BlockedIntent → focused test
5. After exhaustion, same goal is suppressed in candidate generation for `structural_block_ticks` ticks → integration test
6. Golden tests pass → `cargo test -p worldwake-ai`

## What to Change

### 1. Domain-specific target extraction

Add a helper function (in `agent_tick/frame.rs` or `failure_handling.rs`) that extracts the `BlockerKey::target` from an `IntentionDomain`:

```rust
fn frame_blocker_target(domain: &IntentionDomain) -> Option<EntityId> {
    match domain {
        IntentionDomain::Travel { destination } => Some(*destination),
        IntentionDomain::Care { patient } => Some(*patient),
        IntentionDomain::Escort { ward, .. } => Some(*ward),
        IntentionDomain::Errand { destination } => Some(*destination),
        IntentionDomain::Generic => None,
    }
}
```

### 2. BlockedIntent creation on patience exhaustion

In the frame lifecycle (after `stalled_ticks >= patience_limit` detected in the per-tick pipeline):
- Build `BlockerKey` from frame's `goal`, agent's current place, domain-specific target, `action_def: None`
- Record `BlockedIntent` with `BlockingFact::PatienceExhausted`, TTL = `budget.structural_block_ticks`
- Set `last_frame_clear_reason = PatienceExhausted`

### 3. BlockedIntent creation on critical assumption failure

When S22-003's assumption evaluation transitions frame to `Exhausted` (critical failure):
- Build `BlockerKey` with same structure, target = assumption's entity if applicable
- Record `BlockedIntent` with `BlockingFact::AssumptionFailed`, TTL = `budget.structural_block_ticks`
- Set `last_frame_clear_reason = AssumptionFailed`

### 4. Wire into agent_tick pipeline

Ensure `BlockedIntentMemory` is available where frame exhaustion is detected. Pass it through the existing pipeline context.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — add `frame_blocker_target()`, blocked intent creation on exhaustion)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — wire BlockedIntentMemory access for frame exhaustion path)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — if frame exhaustion blocked intent creation is co-located here)

## Out of Scope

- Decision trace recording of blocked intent creation (S22-006)
- Assumption evaluation logic (S22-003 — already implemented)
- Progress detection logic (S22-004 — already implemented)
- BlockedIntentMemory compound key design (S23 — already implemented)
- Changes to `candidate_generation.rs` — blocked intent suppression already works via `blocks_goal_generation()` on the new `BlockingFact` variants from S22-001

## Acceptance Criteria

### Tests That Must Pass

1. Focused test: frame with `stalled_ticks >= patience_limit` → `BlockedIntent` recorded with `BlockingFact::PatienceExhausted`
2. Focused test: `BlockerKey` has correct `goal_key`, `place`, `target` for Travel domain
3. Focused test: `BlockerKey` has correct `target = Some(patient)` for Care domain
4. Focused test: Generic domain → `BlockerKey::target = None`
5. Focused test: critical assumption failure → `BlockedIntent` with `AssumptionFailed`
6. Focused test: goal completion → NO blocked intent created
7. Focused test: voluntary goal switch → NO blocked intent created
8. Integration test: after patience exhaustion, agent does NOT re-adopt same goal for `structural_block_ticks` ticks
9. `cargo test -p worldwake-ai` — all golden tests pass
10. `cargo clippy --workspace` — no new warnings

### Invariants

1. BlockedIntent TTL = `budget.structural_block_ticks` (not hardcoded)
2. Only exhaustion (patience or assumption) creates blocked intents — completion and voluntary switch do not
3. `BlockerKey::action_def` is always `None` for frame exhaustion (frame exhaustion is goal-level, not action-level)
4. Domain-specific target mapping is exhaustive over all `IntentionDomain` variants

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/frame.rs` (test module) — `frame_blocker_target()` mapping for all 5 domains
2. `crates/worldwake-ai/src/agent_tick/frame.rs` (test module) — blocked intent creation on patience exhaustion
3. `crates/worldwake-ai/src/agent_tick/frame.rs` (test module) — blocked intent creation on assumption failure
4. `crates/worldwake-ai/src/agent_tick/tests.rs` — integration: exhausted goal suppressed in subsequent candidate generation

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
