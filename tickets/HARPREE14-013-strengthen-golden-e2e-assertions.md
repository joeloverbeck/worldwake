# HARPREE14-013: Strengthen weak assertions in golden e2e

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: ALL other HARPREE14 tickets (must be implemented LAST)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-D03

## Problem

Two assertions in the golden e2e are observational rather than required:
1. Line 696: Blocked intent check is wrapped in `if saw_blocker { eprintln!(...) }` -- observational only
2. Line 1023: Loot assertion is `if !b_looted { eprintln!("Note: ...non-fatal") }` -- observational only

These weaken the test's value as a regression gate.

## Assumption Reassessment (2026-03-11)

1. Line 696-698: blocked intent assertion is observational (eprintln, not assert) -- confirmed
2. Line 1023-1024: loot assertion is observational (eprintln, not assert) -- confirmed
3. The golden e2e is the primary regression gate for the AI architecture -- confirmed

## Architecture Check

1. Hard assertions make the golden e2e a stronger regression gate.
2. If the AI architecture is working correctly, looting should happen deterministically within 100 ticks.
3. If converting to hard assertions causes failures, the fix must be in the AI behavior, not weakening the test (TDD principle from CLAUDE.md).

## What to Change

### 1. Convert loot assertion to hard assert

Change the observational loot check (line 1023) to:
```rust
assert!(b_looted, "Agent B should have looted within 100 ticks after killing Agent A");
```

If this fails, investigate and fix the underlying AI behavior rather than keeping it observational.

### 2. Document blocked intent check rationale

For the blocked intent check (line 696): if blocked intents are not reliably generated (planner may skip rather than fail), add a doc comment explaining WHY it remains observational. Example:
```rust
// NOTE: Blocked intent recording is non-deterministic because the planner may
// find an alternative plan before hitting the barrier. This check remains
// observational rather than a hard assert.
```

If investigation shows blocked intents ARE deterministically generated, convert to a hard assert instead.

## Files to Touch

- `crates/worldwake-ai/tests/golden_e2e.rs` (modify)

## Out of Scope

- Changing AI planning logic to force deterministic blocked intents
- Adding new assertions beyond the two identified
- Modifying existing hard assertions
- Changes to any production code

## Acceptance Criteria

### Tests That Must Pass

1. Golden e2e passes with the loot assertion as a hard `assert!`
2. Blocked intent check either converted to hard assert (if deterministic) or documented as observational with rationale
3. `cargo test --workspace` passes
4. `cargo clippy --workspace` -- no new warnings

### Invariants

1. No production code changes
2. All existing hard assertions still pass
3. Golden e2e remains the authoritative regression gate
4. If loot hard assert fails, the AI behavior must be fixed (never weaken the test)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_e2e.rs` -- strengthened assertions

### Commands

1. `cargo test -p worldwake-ai --test golden_e2e` (targeted)
2. `cargo test --workspace`
3. `cargo clippy --workspace`
