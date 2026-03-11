# HARPREE14-002: Action handler registry completeness check

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes -- new validation function in action_handler_registry
**Deps**: None (Wave 1, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-B02

## Problem

`ActionHandlerRegistry` and `ActionDefRegistry` are populated independently. If a new `ActionDef` is registered but its `handler: ActionHandlerId` points to an unregistered handler, the mismatch is only discovered when a runtime path resolves that def/handler pair. The current code does return structured `ActionError::UnknownActionHandler(...)` errors in those paths, but there is no explicit structural validation pass that can fail fast after registry composition. That leaves registry drift detectable only late, at use sites, instead of at composition time.

## Assumption Reassessment (2026-03-11)

1. `ActionHandlerRegistry` exists in `action_handler_registry.rs` -- confirmed
2. `ActionDefRegistry` exists in `action_def_registry.rs` -- confirmed
3. `ActionDef` has a `handler` field pointing to a handler ID -- confirmed
4. No registry-level completeness verifier currently exists -- confirmed
5. Missing handlers are already surfaced as `ActionError::UnknownActionHandler(...)` in `start_action()`, `tick_action()`, and `interrupt_action()` -- confirmed
6. `SimulationState` is not an appropriate integration point because it does not own either registry -- confirmed

## Architecture Check

1. A standalone verification function is cleaner than embedding checks in `register()` because the two registries are intentionally populated independently across modules, then consumed together.
2. The new validation complements existing runtime safety instead of replacing it: fail early at registry composition, still fail safely if an unchecked pair reaches runtime.
3. Automatic invocation should happen only at a future canonical registry-composition boundary. Doing that in `SimulationState` would couple unrelated concerns and would not even cover the current ownership model.
4. No backwards-compatibility shims. Pure additive change.

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

Keep the function in `action_handler_registry.rs` for now. Do not re-export it at the crate root in this ticket; module-qualified access keeps the API precise and avoids naming ambiguity with other completeness verifiers in the workspace.

### 2. Add unit tests

- All-valid case: register matching defs and handlers, verify returns `Ok(())`
- Missing-handler case: register a def whose handler ID has no registered handler, verify returns `Err` with the correct IDs
- Multiple-missing case: verify returns every orphaned `ActionDefId` in deterministic registration order

### 3. Preserve existing runtime behavior tests

Do not replace or rewrite the existing `start_action()` runtime error coverage. The new tests should cover the earlier structural invariant, not duplicate the already-tested runtime failure path.

## Files to Touch

- `crates/worldwake-sim/src/action_handler_registry.rs` (modify -- add function + tests)
- `crates/worldwake-sim/src/lib.rs` only if needed for internal visibility ergonomics; avoid a broad new root-level export unless implementation clearly requires it

## Out of Scope

- Auto-calling this from `SimulationState` initialization
- Creating a new canonical registry builder/composer API
- Reworking existing runtime error handling for missing defs/handlers
- Changing `ActionDefRegistry` or `ActionDef` structure
- Modifying handler registration logic
- Any changes to `action_def_registry.rs` beyond reading its API

## Acceptance Criteria

### Tests That Must Pass

1. New test: `test_verify_completeness_all_valid` -- returns `Ok(())` when all handlers present
2. New test: `test_verify_completeness_missing_handler` -- returns `Err` with correct orphan IDs
3. New test: `test_verify_completeness_reports_all_orphans_in_order` -- returns every missing-handler def ID in deterministic order
4. Existing `start_action()` missing-handler test still passes unchanged
5. `cargo test -p worldwake-sim` -- all existing tests pass unchanged
6. `cargo clippy --workspace` -- no new warnings

### Invariants

1. Existing handler registration behavior unchanged
2. Existing runtime `UnknownActionHandler` behavior unchanged
3. No public API breakage on existing types
4. Golden e2e hashes identical (no behavioral change)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_handler_registry.rs` (test module) -- new structural validation tests for valid, missing, and multi-missing handler coverage
2. `crates/worldwake-sim/src/start_gate.rs` -- existing missing-handler runtime test remains the regression check for late-bound failure behavior

### Commands

1. `cargo test -p worldwake-sim action_handler_registry` (targeted)
2. `cargo test -p worldwake-sim start_action_returns_structured_error_for_missing_handler` (runtime regression spot-check)
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Outcome amended: 2026-03-11

- Completed: 2026-03-11
- What changed:
  - Added `action_handler_registry::verify_completeness(defs, handlers) -> Result<(), Vec<ActionDefId>>` in `crates/worldwake-sim/src/action_handler_registry.rs`
  - Added structural tests for all-valid, single-missing, and multi-missing handler coverage
  - Left existing runtime `UnknownActionHandler` behavior and tests unchanged
  - Added `worldwake_systems::build_full_action_registries()` as the canonical full action-registry composition boundary and migrated duplicated AI/golden test builders to use it
  - Strengthened planner-op tests to assert semantic classification by action name instead of brittle registration-order IDs
- Deviations from original plan:
  - Did not integrate validation into `SimulationState`; reassessment showed `SimulationState` does not own either registry, so that hook would have been the wrong architectural boundary
  - Kept the helper as a module-level public API instead of adding a broad crate-root re-export
  - Follow-up automatic enforcement happened at the registry-composition boundary in `worldwake-systems`, not inside `worldwake-sim`
- Verification results:
  - `cargo test -p worldwake-sim action_handler_registry` passed
  - `cargo test -p worldwake-sim start_action_errors_when_definition_or_handler_is_missing` passed
  - `cargo test -p worldwake-systems build_full_action_registries_returns_complete_phase_two_catalog` passed
  - `cargo test -p worldwake-ai planner_ops` passed
  - `cargo test -p worldwake-ai search` passed
  - `cargo test -p worldwake-ai --test golden_e2e` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
