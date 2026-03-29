# S36DECGOAREG-002: Create GoalDispatchDeclaration struct and static declarations

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S36DECGOAREG-001

## Problem

With the dispatch key in place (001), the next step is defining the declaration struct that holds all static dispatch properties per key, and populating one declaration per dispatch-distinguishing key. This is the data foundation that later tickets will consume.

## Assumption Reassessment (2026-03-29)

1. `GoalDispatchKey` enum exists (delivered by 001) with exhaustive `GoalKind → GoalDispatchKey` mapping.
2. Static dispatch surfaces to consolidate into declarations (confirmed against live code):
   - `trace_label`: Currently `Debug` formatting via `format!("{:?}", ...)` in `decision_trace.rs`. No stable label surface exists yet.
   - `provenance_family`: `ranked_goal_provenance_family()` in `goal_model.rs:518-550`. Returns `Option<RankedGoalProvenanceFamily>`.
   - `relevant_ops`: `relevant_op_kinds()` in `goal_model.rs:552-578`. Returns `&'static [PlannerOpKind]`. Uses 21 `*_OPS` const arrays defined at `goal_model.rs:109-183`.
3. `RankedGoalProvenanceFamily` enum is defined at `goal_model.rs:53-57` with variants `Danger`, `Drive`.
4. `PlannerOpKind` enum is defined at `planner_ops.rs:10-57` with 28 variants.
5. Strategy selectors (`InvalidationStrategy`, `FeasibilityStrategy`) are deferred to tickets 006/007 per the spec's incremental migration plan.

## Architecture Check

1. A static `const` declaration struct is simpler and more debuggable than a trait-based approach. All data is compile-time known. The `declaration()` method on `GoalDispatchKey` returns `&'static GoalDispatchDeclaration`, making lookups zero-cost. P27 (debuggability): one place to inspect all dispatch properties for any goal.
2. No backwards-compatibility shims. Declarations are new additive types. Existing dispatch surfaces continue to work unchanged until migrated in tickets 003–005.

## Verification Layers

1. Declaration completeness → compile-time: exhaustive match in `GoalDispatchKey::declaration()` with no wildcard ensures every key has a declaration.
2. Declaration data correctness → focused unit tests: spot-check that declaration values match the current live dispatch for representative goal shapes.
3. Single-layer ticket: type definitions and static data only.

## What to Change

### 1. New file: `crates/worldwake-ai/src/goal_dispatch_decl.rs`

Define:

```rust
pub struct GoalDispatchDeclaration {
    pub trace_label: &'static str,
    pub provenance_family: Option<RankedGoalProvenanceFamily>,
    pub relevant_ops: &'static [PlannerOpKind],
}
```

Define one `static` declaration constant per `GoalDispatchKey` variant. Populate `trace_label` with human-readable strings (e.g., `"AcquireCommodity(Restock)"`), `provenance_family` from current `ranked_goal_provenance_family()` logic, and `relevant_ops` from current `relevant_op_kinds()` logic.

### 2. Exhaustive lookup on `GoalDispatchKey`

Implement `GoalDispatchKey::declaration(&self) -> &'static GoalDispatchDeclaration` with exhaustive match, no wildcard.

### 3. Register in `lib.rs`

Add `mod goal_dispatch_decl;` and public re-exports to `crates/worldwake-ai/src/lib.rs`.

### 4. Unit tests

Spot-check declarations against current dispatch behavior for payload-sensitive and payload-insensitive cases.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add module declaration)

## Out of Scope

- Migrating any existing dispatch site to use declarations (tickets 003–005)
- `InvalidationStrategy` and `FeasibilityStrategy` fields (tickets 006–007)
- Modifying `GoalKindTag`, `GoalKind`, or any `worldwake-core` type
- Candidate generation, intention domain progress-op ownership
- Removing or modifying the existing `*_OPS` const arrays or `ranked_goal_provenance_family()` method

## Acceptance Criteria

### Tests That Must Pass

1. `test_declaration_completeness`: Every `GoalDispatchKey` variant has a declaration (enforced by exhaustive match — this is a compile-time guarantee, but a runtime test iterating all keys confirms no panic).
2. `test_acquire_restock_declaration`: `AcquireRestock` declaration has `provenance_family: None` and includes `PlannerOpKind::Trade` in `relevant_ops`.
3. `test_acquire_need_driven_declaration`: `AcquireNeedDriven` declaration has `provenance_family: Some(Drive)`.
4. `test_punish_fine_vs_exile_ops`: `PunishFine` and `PunishExile` declarations have different `relevant_ops` slices.
5. `test_trace_labels_nonempty`: Every declaration has a non-empty `trace_label`.
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Adding a `GoalDispatchKey` variant without a corresponding declaration fails compilation.
2. Declaration values must exactly match the current live dispatch behavior for all goal shapes (zero behavioral change).
3. Declarations are `&'static` — zero runtime allocation.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_decl.rs` (test module) — declaration completeness, spot-check provenance/ops/labels against known values.

### Commands

1. `cargo test -p worldwake-ai goal_dispatch_decl`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
