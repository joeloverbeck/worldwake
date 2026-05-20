# S156HTNAUTHON-004: Remove unenforced `MethodSchema` fields

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` HTN method schema
**Deps**: archive/tickets/S156HTNAUTHON-003.md

## Problem

`MethodSchema` declares three fields that no runtime code reads: `expected_artifacts`,
`required_claims`, and `failure_modes`. `expected_artifacts` and `required_claims` have zero
consumers anywhere; `failure_modes` has a single validation-test consumer
(`every_method_declares_at_least_one_failure_mode`) but is never read by runtime failure
attribution. A schema that *looks* like it encodes artifact/claim/failure semantics it does not
enforce creates false confidence (FND-20, FND-28). This ticket removes the three fields from the
struct, the `method_schema!` macro, all method bodies, and the constructors/tests, deferring any
re-introduction to a future method-required goal that actually enforces them.

## Assumption Reassessment (2026-05-20)

1. `MethodSchema` (`crates/worldwake-ai/src/htn/method_schema.rs:8-19`) has
   `expected_artifacts: Vec<ArtifactTemplate>`, `required_claims: Vec<ClaimRequirement>`,
   `failure_modes: Vec<MethodFailureMode>`. They are populated positionally through the
   `method_schema!` macro / `MethodParts` builder (`htn/methods.rs:~40-78`) and supplied by every
   method definition.
2. Consumer audit (workspace grep): `expected_artifacts` and `required_claims` have zero read
   sites. `failure_modes` has exactly one read site — the integration test
   `every_method_declares_at_least_one_failure_mode`
   (`crates/worldwake-ai/tests/integration/htn_registry_validation.rs:49-59`), which asserts the
   field is non-empty. No runtime code reads any of the three. `search/strategic.rs` never reads them.
3. Shared boundary under audit: the `MethodSchema` struct and the `method_schema!` macro
   signature — the single construction surface for all methods. After
   `archive/tickets/S156HTNAUTHON-003.md` deleted the two dead methods, 11 method definitions
   remain to update.
4. The trace field `MethodPlanAttemptTrace.failure_mode: Option<MethodFailureMode>`
   (`decision_trace.rs:1222-1227`) and the `MethodFailureMode` *type* are NOT removed — only the
   `failure_modes: Vec<MethodFailureMode>` field on `MethodSchema`. `MethodFailureMode` survives as
   the trace field's type and is referenced by S156HTNAUTHON-005.
5. Existing tests on the changed surface: `method_schema_constructs_and_clones`
   (`method_schema.rs:293`) sets all three fields; `every_method_declares_at_least_one_failure_mode`
   (`htn_registry_validation.rs:49`) reads `failure_modes`; test helpers in `htn/selector.rs:513-515`
   and `search/strategic.rs:1992-1994` set the three fields to `Vec::new()`. All must be updated or
   (for the failure-mode validation test) deleted.
6. Adjacent contradiction classification: the `every_method_declares_at_least_one_failure_mode`
   test is a falsification check (FND-31) over a field this ticket removes; deleting it is a
   required consequence, not a separate bug. No new method-quality invariant is owed because the
   field it guarded no longer exists.

## Architecture Check

1. Removing fields with no enforcement deletes declared-but-unenforced semantics rather than
   leaving them as a trap a future contributor could rely on. This is the FND-20/FND-28 clean
   path: the fields return only when a method-required goal actually enforces them (see D6 / future
   work).
2. No shim: the macro signature shrinks and all call sites drop the three positional arguments. No
   default-empty field is retained "just in case."

## Verification Layers

1. No method-schema field survives without a live consumer -> `cargo clippy --workspace
   --all-targets -- -D warnings` (the removed fields must leave no unused-field/import warnings) +
   workspace grep proving zero references to the three field names.
2. Method selection and stage building unaffected -> existing `htn_methods` goldens and selector
   unit tests pass unchanged (the removed fields were never read at runtime).
3. Single-layer ticket: this is a struct/macro shape change in the AI crate with no authoritative
   state, action lifecycle, or ordering effect — additional layer mapping is not applicable.

## What to Change

### 1. Remove the three fields from `MethodSchema`

Delete `expected_artifacts`, `required_claims`, and `failure_modes` from the `MethodSchema` struct
in `method_schema.rs`.

### 2. Shrink the `method_schema!` macro / `MethodParts` builder

Remove the three corresponding positional parameters from the `method_schema!` macro and the
`MethodParts` construction in `htn/methods.rs`, then drop the three argument expressions
(`vec![…artifacts]`, `vec![…claims]`, `vec![…failure_modes]`) from every remaining method
definition (11 methods after `archive/tickets/S156HTNAUTHON-003.md`).

### 3. Update constructors and test helpers

- `method_schema_constructs_and_clones` (`method_schema.rs:293`): remove the three field
  assignments from the fixture.
- Test helpers at `htn/selector.rs:513-515` and `search/strategic.rs:1992-1994`: remove the
  `Vec::new()` assignments for the three fields.

### 4. Delete the failure-mode validation test

Delete `every_method_declares_at_least_one_failure_mode` from
`crates/worldwake-ai/tests/integration/htn_registry_validation.rs`; it asserts a property of a
field that no longer exists.

## Files to Touch

- `crates/worldwake-ai/src/htn/method_schema.rs` (modify)
- `crates/worldwake-ai/src/htn/methods.rs` (modify)
- `crates/worldwake-ai/src/htn/selector.rs` (modify)
- `crates/worldwake-ai/src/search/strategic.rs` (modify)
- `crates/worldwake-ai/tests/integration/htn_registry_validation.rs` (modify)

## Out of Scope

- The `MethodFailureMode` type itself and `MethodPlanAttemptTrace.failure_mode` (used by
  S156HTNAUTHON-005).
- Any criteria/precondition/method removal (S156HTNAUTHON-002/003).
- Reintroducing the fields with enforcement (future method-required goal; documented in
  S156HTNAUTHON-006).

## Acceptance Criteria

### Tests That Must Pass

1. Workspace grep returns zero references to `expected_artifacts`, `required_claims`, and
   `failure_modes` (as `MethodSchema` fields).
2. `method_schema_constructs_and_clones` compiles and passes without the three fields.
3. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. Every surviving `MethodSchema` field is consumed by runtime code (FND-28: no unenforced
   declarations).
2. `MethodFailureMode` remains available for the trace layer; only the schema `failure_modes`
   field is removed.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/htn/method_schema.rs` (test module) — update construct-and-clone
   fixture.
2. `crates/worldwake-ai/tests/integration/htn_registry_validation.rs` — delete the failure-mode
   validation test.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `./scripts/verify.sh` (before PR)
