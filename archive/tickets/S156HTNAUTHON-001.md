# S156HTNAUTHON-001: Remove `GoalSchema.methods` fossil; single registry authority

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` goal-schema declarations
**Deps**: specs/S156-htn-authority-honesty.md (D1)

## Problem

`GoalSchema` declares a `methods: &'static [MethodSchemaId]` field that every one of the 41
`GoalDispatchKey` declarations sets to the empty slice `&[]`, and the only code that touches it
is a test asserting it is empty "because method assignment belongs to the method registry." The
real, sole method-assignment authority is `MethodRegistry` (`by_goal_kind` keyed by
`GoalKindDiscriminant`, queried via `methods_for`). Two live-looking authorities for one concept
is exactly the fossil FND-28 prohibits. This ticket removes the shadow field so `MethodRegistry`
is unambiguously the only method-assignment surface.

## Assumption Reassessment (2026-05-20)

1. `GoalSchema` (`crates/worldwake-ai/src/goal_schema.rs:63-79`) has `pub methods: &'static
   [MethodSchemaId]` (line 78). All 41 `GoalDispatchKey` static declarations (lines ~290-831) set
   `methods: &[]`; no declaration sets it non-empty. Confirmed by grep — the field is set only in
   `goal_schema.rs` and the test fixture.
2. The only runtime reads of `.methods` are in the integration test
   `crates/worldwake-ai/tests/integration/goal_schema_methods.rs` (lines 28, 35) — there are zero
   production consumers. `MethodRegistry::methods_for` (`crates/worldwake-ai/src/htn/registry.rs:27-30`)
   is the live method-lookup path used by the selector (`htn/selector.rs:36-37`).
3. Shared boundary under audit: the `GoalSchema` struct definition and the `GoalDispatchKey`
   declaration table, both in `goal_schema.rs`. No cross-crate boundary — `GoalSchema` is
   re-exported from `worldwake-ai` (`lib.rs:114-117`) but the cross-crate grep for `GoalSchema {`
   construction outside worldwake-ai is empty.
4. Existing tests on the changed surface: `goal_schema_methods.rs` holds two tests —
   `iteration_order_preserved` (line 9, constructs a fixture with `methods: &[MethodSchemaId(1),
   MethodSchemaId(2)]` solely to exercise the field) and
   `all_dispatch_declarations_expose_empty_method_anchors` (line 32, asserts every declaration's
   `methods` is empty). The former is deleted (its subject field is gone); the latter is rewritten
   to assert single registry authority.
5. Adjacent contradiction classification: none. Removing the field is self-contained; no separate
   bug or deferred cleanup is exposed.

## Architecture Check

1. Removing the empty shadow field collapses two apparent authorities into one — `MethodRegistry`
   becomes the unambiguous method-assignment surface, matching how the selector already resolves
   methods. This is the clean FND-28 path: delete the dead declaration rather than wrap or alias it.
2. No backward-compatibility shim is introduced. The field is removed outright; no deprecated
   accessor or fallback remains.

## Verified Layers

1. Single method-assignment authority (no second declared surface) -> focused integration test in
   `goal_schema_methods.rs` asserting every goal kind's methods come from `MethodRegistry::methods_for`.
2. Compilation cleanliness (no dead field, no unused import) -> `cargo clippy --workspace
   --all-targets -- -D warnings`.
3. Single-layer ticket: this is a static-declaration removal in the AI crate with no runtime
   ordering, action lifecycle, or authoritative-state effect — additional layer mapping is not
   applicable.

## Landed Changes

### 1. Removed the `methods` field from `GoalSchema`

Deleted `pub methods: &'static [MethodSchemaId]` from the `GoalSchema` struct in
`goal_schema.rs`, removed the `methods: &[],` line from all 41 `GoalDispatchKey` declarations in
the same file, and removed the now-unused `MethodSchemaId` import.

### 2. Rewrote the integration test

In `crates/worldwake-ai/tests/integration/goal_schema_methods.rs`:
- Deleted `iteration_order_preserved` because it existed only to exercise the removed field.
- Replaced `all_dispatch_declarations_expose_empty_method_anchors` with focused assertions that
  dispatch declarations still expose schema metadata and that all registered method ids returned
  by `MethodRegistry::methods_for` resolve back to a `MethodSchema` with the same
  `GoalKindDiscriminant`.

## Landed Files

- `crates/worldwake-ai/src/goal_schema.rs`
- `crates/worldwake-ai/tests/integration/goal_schema_methods.rs`

## Out of Scope

- Any change to `MethodRegistry`, `MethodSchema`, `MethodPrecondition`, or method definitions
  (covered by S156HTNAUTHON-002/003/004).
- Trace or fallback changes (S156HTNAUTHON-005).

## Acceptance Result

### Tests That Passed

1. Rewritten `goal_schema_methods.rs` test asserts `MethodRegistry` is the sole
   method-assignment authority and compiles without referencing `GoalSchema.methods`.
2. The deleted `iteration_order_preserved` test is gone; no test references `.methods` on a
   `GoalSchema`.
3. Existing suite passed: `cargo test -p worldwake-ai`.

### Invariants

1. `GoalSchema` carries no method-assignment field; `MethodRegistry` is the only surface that maps
   a goal kind to its methods (FND-28: one authority per concept).
2. No production code path reads method assignment from `GoalSchema`.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-ai/tests/integration/goal_schema_methods.rs` — rewritten to assert single
   registry authority; delete the field-exercising iteration test.

### Command Results

1. Passed `cargo test -p worldwake-ai --test integration_ai goal_schema_methods -- --list`
2. Passed `cargo test -p worldwake-ai --test integration_ai goal_schema_methods`
3. Passed `cargo test -p worldwake-ai`
4. Passed `cargo clippy --workspace --all-targets -- -D warnings`
5. Waived `./scripts/verify.sh` for this ticket iteration because the harness runs it only before
   the final PR push; the ticket-owned source/test surface was covered by the focused selector,
   full `worldwake-ai` suite, and CI-matching clippy gate.

## Outcome

Completed on 2026-05-20.

- `GoalSchema` no longer carries a method-assignment field.
- All 41 `GoalDispatchKey` declarations now expose only schema metadata; method assignment remains
  centralized in `MethodRegistry`.
- The focused integration test now proves the surviving dispatch schema surface and registry-owned
  method assignment contract.

## Deviations

- The drafted command `cargo test -p worldwake-ai --test goal_schema_methods` was stale because the
  live package exposes the integration tests through `integration_ai`. The verified focused command
  is `cargo test -p worldwake-ai --test integration_ai goal_schema_methods`.

## Verification Result

- Passed `cargo test -p worldwake-ai --test integration_ai goal_schema_methods -- --list`
- Passed `cargo test -p worldwake-ai --test integration_ai goal_schema_methods`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
