# E13DECARC-014: Plan revalidation by affordance identity

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — AI-layer logic
**Deps**: E13DECARC-005, E13DECARC-009

## Problem

Before executing the next plan step, the agent must verify the step is still valid by checking that an identical affordance still exists in the current beliefs. This is the canonical revalidation rule — E13 must NOT duplicate precondition logic from the action framework.

## Assumption Reassessment (2026-03-11)

1. `get_affordances(view, actor, registry)` returns `Vec<Affordance>` — confirmed.
2. `Affordance` has `def_id`, `actor`, `bound_targets`, `payload_override` — confirmed.
3. `PlannedStep` has `def_id`, `targets`, `payload_override` from E13DECARC-009 — dependency.
4. `ActionPayload` implements `PartialEq` — need to verify or implement.

## Architecture Check

1. Revalidation is by affordance identity comparison, not by re-implementing preconditions.
2. The identity key is: `(def_id, targets, payload_override)`.
3. If no matching affordance exists, the step is invalid and triggers failure handling (E13DECARC-013).
4. This keeps E13 decoupled from the action framework's precondition logic.

## What to Change

### 1. Implement revalidation in `worldwake-ai/src/plan_revalidation.rs`

```rust
pub fn revalidate_next_step(
    view: &dyn BeliefView,
    actor: EntityId,
    step: &PlannedStep,
    registry: &ActionDefRegistry,
) -> bool
```

Steps:
1. Call `get_affordances(view, actor, registry)`
2. Search for an affordance matching:
   - Same `def_id`
   - Same ordered `targets` (exact match)
   - Same `payload_override` (exact match)
3. Return `true` if found, `false` otherwise

### 2. Ensure `ActionPayload` comparison works

If `ActionPayload` doesn't implement `PartialEq`, we need to add it in `worldwake-sim`. Check and add if missing.

## Files to Touch

- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — was empty stub)
- `crates/worldwake-sim/src/action_def.rs` (modify — add `PartialEq` to `ActionPayload` if missing)

## Out of Scope

- Precondition logic duplication — explicitly forbidden
- Failure handling after invalid revalidation — E13DECARC-013
- Agent tick integration — E13DECARC-016
- Reactive interrupts — E13DECARC-015

## Acceptance Criteria

### Tests That Must Pass

1. Step with matching affordance in current beliefs returns `true`
2. Step with same `def_id` but different targets returns `false`
3. Step with same `def_id` and targets but different payload returns `false`
4. Step with no matching `def_id` returns `false`
5. Revalidation uses `get_affordances()` — no custom precondition code (verified by code review / grep)
6. `ActionPayload` supports equality comparison
7. Existing suite: `cargo test --workspace`

### Invariants

1. No precondition logic duplicated from action framework
2. Identity key is exactly `(def_id, targets, payload_override)`
3. Revalidation is a pure read operation
4. Affordance matching is exact (ordered targets)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_revalidation.rs` — revalidation tests with mock BeliefView and registry

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
