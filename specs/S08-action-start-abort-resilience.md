**Status**: PENDING

# Action Start Abort Resilience

## Summary

Fix two related defects in the action framework's error handling for actions that are legally planned but whose preconditions change between planning and execution:

1. **`AbortRequested` during action start crashes the simulation** — `is_best_effort_start_failure()` only recognizes `ReservationUnavailable | PreconditionFailed | InvalidTarget`. When an `on_start` handler returns `AbortRequested` (e.g., `TargetHasNoWounds` because the wound naturally recovered between planning and start), `step_tick` propagates a hard `Err` instead of recording a graceful start failure.

2. **`start_heal` consumes medicine before treatment is confirmed** — The heal action's `on_start` handler consumes one unit of Medicine *immediately*, then the `on_tick` handler re-validates wound presence. If the wound recovers naturally between start and tick, the action aborts — but the medicine is already destroyed. This is a conservation violation: a resource consumed with no effect.

## Discovered Via

Golden E2E emergent tests (S07 care interaction coverage). When an agent had wounds that naturally recovered while the heal action was in-flight, the simulation crashed with `Action(AbortRequested(TargetHasNoWounds))`. The root cause is the intersection of (a) natural wound recovery and (b) action start error handling.

## Foundation Alignment

- **Principle 8** (Every Action Has Preconditions, Duration, Cost, and Occupancy): Resource consumption (medicine) should only occur when the action's commit conditions are met — not speculatively at start time.
- **Principle 9** (Outcomes Are Granular and Leave Aftermath): An aborted heal should leave medicine intact. "Failure is new state" — but the new state should be *no treatment attempted*, not *medicine wasted silently*.
- **Principle 19** (Intentions Are Revisable Commitments): The agent's commitment to heal should be revisable if the patient's wounds resolve before treatment applies. The framework must handle this revision gracefully, not crash.

## Phase

Phase 3: Information & Politics (bug fix, no phase dependency)

## Crates

- `worldwake-sim` (action framework error handling)
- `worldwake-systems` (heal action handler resource consumption timing)

## Dependencies

None. All prerequisite infrastructure exists. This is a bug fix to existing code.

## Deliverables

### 1. Extend `is_best_effort_start_failure` to include `AbortRequested`

**File**: `crates/worldwake-sim/src/tick_step.rs`

`AbortRequested` during action start is a normal world-state evolution — the plan was valid when constructed, but the world changed by the time the action started. This is semantically identical to `PreconditionFailed` and should be handled the same way in BestEffort mode.

```rust
fn is_best_effort_start_failure(error: &ActionError) -> bool {
    matches!(
        error,
        ActionError::ReservationUnavailable(_)
            | ActionError::PreconditionFailed(_)
            | ActionError::InvalidTarget(_)
            | ActionError::AbortRequested(_) // NEW
    )
}
```

**Effect**: BestEffort action starts that encounter `AbortRequested` are recorded as `ActionStartFailure` (already traced and fed into `handle_plan_failure`), not propagated as fatal errors. The agent replans on the next tick.

### 2. Move medicine consumption from `start_heal` to `commit_heal`

**File**: `crates/worldwake-systems/src/combat.rs`

Currently `start_heal` (line ~1013) calls `consume_one_unit_of_commodity(Medicine)` at action start. Move this consumption into the commit path so medicine is only consumed when treatment is actually applied.

**Before** (current):
```
start_heal:  validate → consume_medicine → Ok
tick_heal:   validate → apply_treatment → Commit/Continue
```

**After** (proposed):
```
start_heal:  validate → Ok  (no resource consumption)
tick_heal:   validate → apply_treatment → consume_medicine → Commit/Continue
```

The medicine must be consumed in the same transaction that applies the wound reduction, ensuring atomicity: either both happen or neither does.

**Reservation alternative**: If the design prefers to *reserve* the medicine at start time (preventing another action from consuming it during the heal), use the existing reservation system to hold the lot, then consume-and-release at commit time. This is more robust but adds complexity; the simpler approach (consume at commit) is sufficient if heal actions are short-duration.

### 3. Tests

**Unit test** (worldwake-sim): BestEffort action start with `AbortRequested` error is recorded as start failure, not propagated.

**Unit test** (worldwake-systems): Heal action that starts while target has wounds, but target's wounds are cleared before commit, does not consume medicine.

**Golden test** (worldwake-ai): Remove `no_recovery_combat_profile` workaround from `golden_emergent.rs` tests — if the fix is correct, care tests should work with default combat profiles because the race is handled gracefully.

## Risks

- Moving medicine consumption to commit time means another agent could consume the same medicine lot between start and commit. The reservation system mitigates this, but the current heal action does not reserve the medicine lot. If multi-agent medicine contention is rare (it is in current scenarios), the simpler approach is acceptable. If contention becomes common, add lot reservation to `start_heal`.

## Information-Path Analysis (FND-01 Section H)

Not applicable — this is a bug fix to existing action framework mechanics, not a new information pathway.

## Stored State vs. Derived

No new stored state. The fix changes the timing of an existing mutation (medicine consumption) within the same transaction boundary.
