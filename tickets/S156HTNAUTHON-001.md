# S156HTNAUTHON-001: Remove `GoalSchema.methods` fossil; single registry authority

**Status**: PENDING
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

## Verification Layers

1. Single method-assignment authority (no second declared surface) -> focused integration test in
   `goal_schema_methods.rs` asserting every goal kind's methods come from `MethodRegistry::methods_for`.
2. Compilation cleanliness (no dead field, no unused import) -> `cargo clippy --workspace
   --all-targets -- -D warnings`.
3. Single-layer ticket: this is a static-declaration removal in the AI crate with no runtime
   ordering, action lifecycle, or authoritative-state effect — additional layer mapping is not
   applicable.

## What to Change

### 1. Remove the `methods` field from `GoalSchema`

Delete `pub methods: &'static [MethodSchemaId]` from the `GoalSchema` struct in `goal_schema.rs`,
and remove the `methods: &[],` line from all 41 `GoalDispatchKey` declarations in the same file.
Remove the now-unused `MethodSchemaId` import if it is no longer referenced in `goal_schema.rs`.

### 2. Rewrite the integration test

In `crates/worldwake-ai/tests/integration/goal_schema_methods.rs`:
- Delete `iteration_order_preserved` — it exists only to exercise the removed field.
- Rewrite `all_dispatch_declarations_expose_empty_method_anchors` into an assertion that
  `MethodRegistry` is the sole method-assignment authority: e.g. build the registry
  (`build_method_registry()`), and assert that for each `GoalKindDiscriminant`, the methods
  available come from `MethodRegistry::methods_for`, and that no `GoalSchema`/`GoalDispatchKey`
  surface declares method assignment (the `methods` field no longer exists).

## Files to Touch

- `crates/worldwake-ai/src/goal_schema.rs` (modify)
- `crates/worldwake-ai/tests/integration/goal_schema_methods.rs` (modify)

## Out of Scope

- Any change to `MethodRegistry`, `MethodSchema`, `MethodPrecondition`, or method definitions
  (covered by S156HTNAUTHON-002/003/004).
- Trace or fallback changes (S156HTNAUTHON-005).

## Acceptance Criteria

### Tests That Must Pass

1. New/rewritten `goal_schema_methods.rs` test asserts `MethodRegistry` is the sole
   method-assignment authority and compiles without referencing `GoalSchema.methods`.
2. The deleted `iteration_order_preserved` test is gone; no test references `.methods` on a
   `GoalSchema`.
3. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. `GoalSchema` carries no method-assignment field; `MethodRegistry` is the only surface that maps
   a goal kind to its methods (FND-28: one authority per concept).
2. No production code path reads method assignment from `GoalSchema`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/integration/goal_schema_methods.rs` — rewrite to assert single
   registry authority; delete the field-exercising iteration test.

### Commands

1. `cargo test -p worldwake-ai --test goal_schema_methods`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `./scripts/verify.sh` (before PR)
