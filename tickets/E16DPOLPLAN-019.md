# E16DPOLPLAN-019: BlockedIntent for failed threats (`ThreatenResisted`)

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — blocked_intent.rs variant, failure_handling.rs logic
**Deps**: E16DPOLPLAN-004

## Problem

When `commit_threaten` results in resistance, the agent has no memory of failure. The planner repeatedly selects futile threaten plans against the same resistant target, wasting action cycles (positive feedback loop identified in Section H2).

## Assumption Reassessment (2026-03-18)

1. `BlockingFact` enum in `crates/worldwake-core/src/blocked_intent.rs` ~line 60 — confirmed
2. `BlockedIntentMemory` has `related_entity: Option<EntityId>` for target-specific blocking — confirmed
3. `handle_plan_failure()` in `crates/worldwake-ai/src/failure_handling.rs` — confirmed
4. `is_blocked()` check in candidate generation prevents re-targeting during cooldown — confirmed

## Architecture Check

1. Adds one new `BlockingFact` variant — minimal surface change
2. Recording logic goes in `failure_handling.rs` — closest to the failure detection point
3. Cooldown (20 ticks) is a physical time dampener per Principle 8

## What to Change

### 1. Add `BlockingFact::ThreatenResisted` variant

In `blocked_intent.rs` `BlockingFact` enum:
```rust
ThreatenResisted,
```

### 2. Record blocked intent on threaten resistance

In `handle_plan_failure()`: when a threaten action completes but target resisted (no loyalty increase, hostility added instead), record:
```rust
BlockedIntent {
    goal_key: /* current goal key */,
    blocking_fact: BlockingFact::ThreatenResisted,
    related_entity: Some(target),
    related_place: None,
    related_action: Some(threaten_action_def_id),
    observed_tick: current_tick,
    expires_tick: Tick(current_tick.0 + 20),
}
```

## Files to Touch

- `crates/worldwake-core/src/blocked_intent.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)

## Out of Scope

- BlockedIntent for failed bribes (not identified as a positive feedback loop)
- Changes to `is_blocked()` logic (existing API handles target-specific blocking)
- Golden tests for this feature specifically
- Changes to `commit_threaten` handler

## Acceptance Criteria

### Tests That Must Pass

1. `threaten_resistance_records_blocked_intent` — after failed threaten, `BlockedIntentMemory` contains entry with `ThreatenResisted` and `related_entity == target`
2. `blocked_threaten_prevents_re_targeting` — during cooldown period, candidate generation does not produce goals requiring threatening the blocked target
3. `blocked_threaten_expires_after_cooldown` — after 20 ticks, blocked intent expires and re-targeting is possible
4. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-ai`

### Invariants

1. Only threaten resistance triggers this blocking — not bribe failures
2. Cooldown is finite (20 ticks) — agent can eventually retry
3. Target-specific: blocking one target doesn't block threatening other targets
4. Section H3 dampener: breaks threaten retry loop identified in H2

## Test Plan

### New/Modified Tests

1. Unit test in `failure_handling.rs` — verify blocked intent recording
2. Unit test in `blocked_intent.rs` — verify `ThreatenResisted` variant behavior

### Commands

1. `cargo test -p worldwake-core blocked_intent`
2. `cargo test -p worldwake-ai failure_handling`
3. `cargo test --workspace`
