# S147HTNMETDEC-005: GoalSchema.methods field

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — extends `GoalSchema` with a per-goal-kind method list. Touches 41 static `DECL_*` initializers in `goal_schema.rs`.
**Deps**: 001 (MethodSchemaId)

## Problem

S147 D11 adds `GoalSchema.methods: &'static [MethodSchemaId]` (or equivalent storage — see Architecture Check) so that each goal kind declares which methods may decompose it. Without this field, the `MethodSelector` (ticket 007) must consult a side-table to find candidate methods per goal kind, which scatters method/goal mapping across the codebase. The field on `GoalSchema` keeps the declarative anchor on the canonical goal-kind registry. Field population is at static-decl time using the trivial empty default; the actual method list per goal is populated by the registry (ticket 006), but the field's existence on `GoalSchema` is the type-level anchor.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalSchema` struct lives at `crates/worldwake-ai/src/goal_schema.rs:63` with 10 existing fields (`trace_label`, `provenance_family`, `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `frontier_exhaustion_strategy`, `family_policy`, `progress_barrier_ops`, `candidate_extractors`, `planning_budget`). 41 `static DECL_*: GoalSchema = GoalSchema { ... };` initializers exist in the same file (verified via `rg -c "^static DECL_.*: GoalSchema = GoalSchema \{" crates/worldwake-ai/src/goal_schema.rs`). Each is a const-evaluable static. All fields are currently const-expressible: scalar enums, `&'static [PlannerOpKind]` slices, etc.
2. `GoalSchema` field additions to a const-evaluable static require either (a) the new field type is also const-evaluable (`&'static [MethodSchemaId]` works; `Vec<MethodSchemaId>` does NOT in `static` decls), or (b) the schemas migrate from `static` to a `LazyLock<GoalSchema>`-style construction. Spec D11 says `Vec<MethodSchemaId>`; the storage type must be reconciled with the const-static requirement. The cleanest reconciliation: use `&'static [MethodSchemaId]` for the field and let each static decl write `methods: &[]` (or a const slice literal for goals with pre-known methods).
3. The runtime registry (ticket 006) populates a separate `BTreeMap<GoalKindDiscriminant, Vec<MethodSchemaId>>` lookup, regardless of whether the field on `GoalSchema` is empty or pre-populated. The field is the type-level anchor; the registry is the runtime source of truth. Note this design choice in What to Change so the implementer doesn't try to mutate `static` decls at runtime.
4. Existing focused tests on `GoalSchema` in `crates/worldwake-ai/src/goal_schema.rs` and `crates/worldwake-ai/src/goal_schema_registry/` directory (`mod.rs`, `registry.rs`, `extractors.rs`) — verify exact test names via `grep -n "#\[test\]" crates/worldwake-ai/src/goal_schema.rs crates/worldwake-ai/src/goal_schema_registry/*.rs` during implementation. The 41-site update is mechanical; existing tests should continue to pass after the field default is added.
5. Spread-syntax check (per Step 2 sub-check (d) at >15 sites threshold): `static` initializers cannot use spread syntax. All 41 sites enumerate fields explicitly. The 15-site rule applies → effort Medium (mechanical but high site count); the field's trivial default (`&[]`) makes each per-site edit a one-line addition.

## Architecture Check

1. Field type `methods: &'static [MethodSchemaId]` is preferable to `Vec<MethodSchemaId>` because it preserves the `static` initializer pattern and avoids forcing 41 schemas through `LazyLock` migration. The semantic intent (each goal has zero or more methods that may decompose it) is preserved; the runtime `MethodRegistry` (ticket 006) is the authoritative lookup. The field on `GoalSchema` is a declarative anchor that registries and tests can iterate without re-deriving the goal-kind list.
2. Alternative considered and rejected: side-table only (no field on GoalSchema). The spec explicitly calls out a field on GoalSchema as the contract — the registry then *consumes* the field rather than maintaining a parallel mapping. Side-table-only would split the goal-method relationship across two truths (FND-28 concern).
3. No backwards-compatibility shims. The field is purely additive; existing 41 static initializers gain one line each.

## Verification Layers

1. Field exists with empty default on all 41 statics → `cargo build -p worldwake-ai` succeeds.
2. Existing `GoalSchema` consumer behavior unchanged → existing tests in `goal_schema.rs` and `goal_schema_registry/` pass without modification.
3. Field can be populated with a non-empty `&[MethodSchemaId]` slice → new test in `tests/goal_schema_methods.rs` constructs a fixture `GoalSchema` with a populated `methods` slice and asserts iteration works.
4. Single-layer ticket — the field is consumed by the registry (ticket 006) and selector (ticket 007), which verify their own surfaces.

## What to Change

### 1. Add `methods` field to `GoalSchema`

Modify `crates/worldwake-ai/src/goal_schema.rs`:

```rust
pub struct GoalSchema {
    // ... existing 10 fields unchanged ...
    pub methods: &'static [MethodSchemaId],   // NEW
}
```

### 2. Update all 41 static `DECL_*` initializers

Each `static DECL_*: GoalSchema = GoalSchema { ... };` gains `methods: &[],` at the end (before the closing brace). Per-static method lists are populated by the registry (ticket 006) at runtime — the static decl carries the empty slice as the type-level anchor.

For goals known at this ticket's writing time to be decomposable by first-ship methods (per S147 D2), the corresponding `static` may pre-declare its methods with a const slice literal. This is optional — the registry's runtime population is the authoritative source. Prefer empty `&[]` for all sites in this ticket to keep the change purely mechanical; let ticket 006 own the actual method-id assignments.

### 3. New focused test for field iteration

New file `crates/worldwake-ai/tests/goal_schema_methods.rs`:
- Constructs a fixture `GoalSchema` with `methods: &[MethodSchemaId(1), MethodSchemaId(2)]` and asserts iteration order is preserved.
- Asserts that every existing `static DECL_*` has `methods: &[]` (or a non-empty slice with at least one method) — sanity check that the field was added consistently.

## Files to Touch

- `crates/worldwake-ai/src/goal_schema.rs` (modify — add field + update 41 static initializers)
- `crates/worldwake-ai/tests/goal_schema_methods.rs` (new)

## Out of Scope

- The runtime `MethodRegistry` and `methods_for(goal_kind)` lookup (ticket 006).
- Per-goal method-id assignments — those are set by `build_method_registry()` in ticket 006, not by static decls in this ticket. This ticket only adds the empty default.
- Migration to `LazyLock<GoalSchema>` — explicitly rejected per the const-static reconciliation in Architecture Check #1.

## Acceptance Criteria

### Tests That Must Pass

1. `tests::goal_schema_methods::iteration_order_preserved` — populated `methods` slice iterates in declaration order.
2. `tests::goal_schema_methods::all_statics_have_methods_field` — every existing `static DECL_*` has the field (compile-time satisfied by the struct definition).
3. Existing suite: `cargo test -p worldwake-ai --lib goal_schema` passes.
4. Existing suite: `cargo test -p worldwake-ai --lib goal_schema_registry` passes.
5. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` clean.

### Invariants

1. All 41 `static DECL_*` initializers carry the `methods` field (compile-time enforced by struct definition).
2. The field type is `&'static [MethodSchemaId]` — const-evaluable, no `Vec` allocation in static decls.
3. The runtime registry (ticket 006) is the authoritative source for which methods belong to which goal; the field on `GoalSchema` is a declarative anchor.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/goal_schema_methods.rs` — new — covers field iteration and verifies all statics carry the field.

### Commands

1. `cargo test -p worldwake-ai --test goal_schema_methods`
2. `cargo test -p worldwake-ai --lib goal_schema`
3. `cargo build -p worldwake-ai`
4. `./scripts/verify.sh`
