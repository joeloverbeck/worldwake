# S147HTNMETDEC-005: GoalSchema.methods field

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — extends `GoalSchema` with a per-goal-kind method list. Touches 41 static `DECL_*` initializers in `goal_schema.rs`.
**Deps**: `archive/tickets/S147HTNMETDEC-001.md` (MethodSchemaId)

## Problem

S147 D11 adds `GoalSchema.methods: &'static [MethodSchemaId]` (or equivalent storage — see Architecture Check) so that each goal kind declares which methods may decompose it. Without this field, the `MethodSelector` (ticket 007) must consult a side-table to find candidate methods per goal kind, which scatters method/goal mapping across the codebase. The field on `GoalSchema` keeps the declarative anchor on the canonical goal-kind registry. Field population is at static-decl time using the trivial empty default; the actual method list per goal is populated by the registry (ticket 006), but the field's existence on `GoalSchema` is the type-level anchor.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalSchema` struct lives at `crates/worldwake-ai/src/goal_schema.rs:63` with 10 existing fields (`trace_label`, `provenance_family`, `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `frontier_exhaustion_strategy`, `family_policy`, `progress_barrier_ops`, `candidate_extractors`, `planning_budget`). 41 `static DECL_*: GoalSchema = GoalSchema { ... };` initializers exist in the same file (verified via `rg -c "^static DECL_.*: GoalSchema = GoalSchema \{" crates/worldwake-ai/src/goal_schema.rs`). Each is a const-evaluable static. All fields are currently const-expressible: scalar enums, `&'static [PlannerOpKind]` slices, etc.
2. `GoalSchema` field additions to a const-evaluable static require either (a) the new field type is also const-evaluable (`&'static [MethodSchemaId]` works; `Vec<MethodSchemaId>` does NOT in `static` decls), or (b) the schemas migrate from `static` to a `LazyLock<GoalSchema>`-style construction. The original S147 D11 sketch used `Vec<MethodSchemaId>`; this ticket reconciled that sketch to `&'static [MethodSchemaId]` and truth-synced the active spec.
3. The runtime registry (ticket 006) populates a separate `BTreeMap<GoalKindDiscriminant, Vec<MethodSchemaId>>` lookup, regardless of whether the field on `GoalSchema` is empty or pre-populated. The field is the type-level anchor; the registry is the runtime source of truth. This ticket recorded that design in Landed Changes so later work does not try to mutate `static` declarations at runtime.
4. Existing focused tests on `GoalSchema` in `crates/worldwake-ai/src/goal_schema.rs` and `crates/worldwake-ai/src/goal_schema_registry/` directory (`mod.rs`, `registry.rs`, `extractors.rs`) remained green after the field default was added.
5. Spread-syntax check (per Step 2 sub-check (d) at >15 sites threshold): `static` initializers cannot use spread syntax. All 41 sites enumerate fields explicitly. The 15-site rule applies → effort Medium (mechanical but high site count); the field's trivial default (`&[]`) makes each per-site edit a one-line addition.

## Architecture Check

1. Field type `methods: &'static [MethodSchemaId]` is preferable to `Vec<MethodSchemaId>` because it preserves the `static` initializer pattern and avoids forcing 41 schemas through `LazyLock` migration. The semantic intent (each goal has zero or more methods that may decompose it) is preserved; the runtime `MethodRegistry` (ticket 006) is the authoritative lookup. The field on `GoalSchema` is a declarative anchor that registries and tests can iterate without re-deriving the goal-kind list.
2. Alternative considered and rejected: side-table only (no field on GoalSchema). The spec explicitly calls out a field on GoalSchema as the contract — the registry then *consumes* the field rather than maintaining a parallel mapping. Side-table-only would split the goal-method relationship across two truths (FND-28 concern).
3. No backwards-compatibility shims. The field is purely additive; existing 41 static initializers gain one line each.

## Verified Layers

1. Field exists with empty default on all 41 statics → `cargo build -p worldwake-ai` succeeds.
2. Existing `GoalSchema` consumer behavior unchanged → existing tests in `goal_schema.rs` and `goal_schema_registry/` pass without modification.
3. Field can be populated with a non-empty `&[MethodSchemaId]` slice → new test in `tests/goal_schema_methods.rs` constructs a fixture `GoalSchema` with a populated `methods` slice and asserts iteration works.
4. Single-layer ticket — the field is consumed by the registry (ticket 006) and selector (ticket 007), which verify their own surfaces.

## Landed Changes

### 1. Add `methods` field to `GoalSchema`

Modified `crates/worldwake-ai/src/goal_schema.rs`:

```rust
pub struct GoalSchema {
    // ... existing 10 fields unchanged ...
    pub methods: &'static [MethodSchemaId],   // NEW
}
```

### 2. Updated all 41 static `DECL_*` initializers

Each `static DECL_*: GoalSchema = GoalSchema { ... };` gained `methods: &[],` at the end. Per-static method lists remain empty until the registry ticket installs real method-id assignments.

The landed scope uses empty `&[]` for all sites to keep this ticket mechanical; ticket 006 owns actual method-id assignments.

### 3. Added focused test for field iteration

Added `crates/worldwake-ai/tests/goal_schema_methods.rs`:
- `iteration_order_preserved` constructs a fixture `GoalSchema` with `methods: &[MethodSchemaId(1), MethodSchemaId(2)]` and asserts iteration order is preserved.
- `all_dispatch_declarations_expose_empty_method_anchors` asserts every existing dispatch declaration has an empty method anchor.

## Landed Files

- `crates/worldwake-ai/src/goal_schema.rs` (modified — added field and updated 41 static initializers)
- `crates/worldwake-ai/tests/goal_schema_methods.rs` (added)
- `archive/specs/S147-htn-method-decomposition.md` (modified — truth-synced `GoalSchema.methods` from `Vec` to const slice)

## Out of Scope

- The runtime `MethodRegistry` and `methods_for(goal_kind)` lookup (ticket 006).
- Per-goal method-id assignments — those are set by `build_method_registry()` in ticket 006, not by static decls in this ticket. This ticket only adds the empty default.
- Migration to `LazyLock<GoalSchema>` — explicitly rejected per the const-static reconciliation in Architecture Check #1.

## Acceptance Result

### Tests Passed

1. `iteration_order_preserved` passed and proves populated `methods` slice iteration order.
2. `all_dispatch_declarations_expose_empty_method_anchors` passed and proves every existing dispatch declaration exposes an empty method anchor.
3. Existing suite `cargo test -p worldwake-ai --lib goal_schema` passed.
4. Existing suite `cargo test -p worldwake-ai --lib goal_schema_registry` passed.
5. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` passed.

### Invariants

1. All 41 `static DECL_*` initializers carry the `methods` field (compile-time enforced by struct definition).
2. The field type is `&'static [MethodSchemaId]` — const-evaluable, no `Vec` allocation in static decls.
3. The runtime registry (ticket 006) is the authoritative source for which methods belong to which goal; the field on `GoalSchema` is a declarative anchor.

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/tests/goal_schema_methods.rs` — covers field iteration and verifies current static declarations expose empty method anchors.

### Commands Run

1. `cargo test -p worldwake-ai --test goal_schema_methods`
2. `cargo test -p worldwake-ai --lib goal_schema`
3. `cargo test -p worldwake-ai --lib goal_schema_registry`
4. `cargo build -p worldwake-ai`
5. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-17.

- Added `GoalSchema.methods: &'static [MethodSchemaId]` as a const-evaluable method anchor.
- Updated all 41 static `GoalSchema` declarations with `methods: &[]`.
- Added focused integration coverage for populated method-slice iteration and current empty declaration anchors.
- Truth-synced S147 so the active spec names the landed const-slice field rather than the earlier `Vec` sketch.

## Deviations

- The active spec's `Vec<MethodSchemaId>` sketch was reconciled to `&'static [MethodSchemaId]` because `GoalSchema` uses `static` declarations and the field must remain const-evaluable.
- Registry-resolution coverage was not added here. Ticket 006 owns installing real method IDs and proving registry resolution.

## Verification Result

- Passed `cargo test -p worldwake-ai --test goal_schema_methods`.
- Passed `cargo test -p worldwake-ai --lib goal_schema`.
- Passed `cargo test -p worldwake-ai --lib goal_schema_registry`.
- Passed `cargo build -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
