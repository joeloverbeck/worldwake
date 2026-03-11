# HARPREE14-002: Action handler registry completeness check

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes -- new validation function in action_handler_registry
**Deps**: None (Wave 1, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-B02

## Problem

`ActionHandlerRegistry` and `ActionDefRegistry` are populated independently. If a new `ActionDef` is registered but its `handler: ActionHandlerId` points to an unregistered handler, the mismatch is only discovered at runtime when the action fires. This is a silent correctness hazard.

## Assumption Reassessment (2026-03-11)

1. `ActionHandlerRegistry` exists in `action_handler_registry.rs` -- confirmed
2. `ActionDefRegistry` exists in `action_def_registry.rs` -- confirmed
3. No `verify_completeness` function currently exists -- confirmed
4. `ActionDef` has a `handler` field pointing to a handler ID -- confirmed

## Architecture Check

1. A standalone verification function is cleaner than embedding checks in `register()` because it validates cross-registry consistency after both are fully populated.
2. No backwards-compatibility shims. Pure additive change.

## What to Change

### 1. Add `verify_completeness()` function

Add a public function:
```rust
pub fn verify_completeness(
    defs: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> Result<(), Vec<ActionDefId>>
```
That iterates all registered `ActionDef` entries, checks that each one's `handler` field resolves to a registered handler, and returns `Err` with the list of orphaned `ActionDefId`s if any are missing.

### 2. Add unit tests

- All-valid case: register matching defs and handlers, verify returns `Ok(())`
- Missing-handler case: register a def whose handler ID has no registered handler, verify returns `Err` with the correct IDs

## Files to Touch

- `crates/worldwake-sim/src/action_handler_registry.rs` (modify -- add function + tests)

## Out of Scope

- Auto-calling this in `SimulationState` initialization (can be done in a follow-up)
- Changing `ActionDefRegistry` or `ActionDef` structure
- Modifying handler registration logic
- Any changes to `action_def_registry.rs` beyond reading its API

## Acceptance Criteria

### Tests That Must Pass

1. New test: `test_verify_completeness_all_valid` -- returns `Ok(())` when all handlers present
2. New test: `test_verify_completeness_missing_handler` -- returns `Err` with correct orphan IDs
3. `cargo test -p worldwake-sim` -- all existing tests pass unchanged
4. `cargo clippy --workspace` -- no new warnings

### Invariants

1. Existing handler registration behavior unchanged
2. No public API breakage on existing types
3. Golden e2e hashes identical (no behavioral change)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_handler_registry.rs` (test module) -- two new tests for valid and missing-handler cases

### Commands

1. `cargo test -p worldwake-sim action_handler` (targeted)
2. `cargo test --workspace`
3. `cargo clippy --workspace`
