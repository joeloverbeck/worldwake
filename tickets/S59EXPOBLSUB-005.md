# S59EXPOBLSUB-005: GoalKind variants + GoalKey + PlannerOpKind integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — GoalKind enum, GoalKey, PlannerOpKind enum, classify_action_def, build_semantics_table
**Deps**: S59EXPOBLSUB-001

## Problem

The AI planner needs GoalKind variants to represent search/report/escort goals, GoalKey entries for deduplication, and PlannerOpKind variants with semantics for GOAP planning. Without these, the planner cannot reason about or select search/report actions.

## Assumption Reassessment (2026-04-06)

1. `GoalKind` at `crates/worldwake-core/src/goal.rs:17` has 28 variants. New variants: `SearchForMissing`, `ReportMissing`, `EscortToSafety`.
2. `GoalKey::from()` at `goal.rs:144-199` maps each GoalKind to a deduplication key. Each new variant needs an entry.
3. `PlannerOpKind` at `crates/worldwake-ai/src/planner_ops.rs:13` has 48 variants. New variants: `SearchPlace`, `AskAboutPerson`, `ReportMissing`, `EscortToSafety`, `ReportFound`.
4. `classify_action_def()` at `planner_ops.rs:83-136` maps `(ActionDomain, action_name)` → `PlannerOpKind`.
5. `build_semantics_table()` at `planner_ops.rs:72-81` creates precondition/effect modeling for GOAP.
6. No existing GoalKind variants for search or escort — confirmed no overlap.

## Architecture Check

1. Adding enum variants and classification entries is additive — existing code unaffected. Follows the established pattern for each integration point.
2. No backward compatibility shims.

## Verification Layers

1. GoalKind variants exist and map to GoalKey → focused unit test
2. PlannerOpKind classification matches action names → focused unit test
3. Semantics table entries produce valid precondition/effect pairs → focused unit test
4. Single-crate changes (core + ai) — no cross-system verification needed.

## What to Change

### 1. Add GoalKind variants

In `crates/worldwake-core/src/goal.rs`, add:

```rust
SearchForMissing {
    subject: EntityId,
    last_seen: Option<EntityId>,
},
ReportMissing {
    subject: EntityId,
    to_office: Option<EntityId>,
},
EscortToSafety {
    subject: EntityId,
    destination: EntityId,
},
```

### 2. Add GoalKey entries

In the `GoalKey::from()` impl, add match arms for the three new variants. `SearchForMissing` keys on subject, `ReportMissing` keys on subject, `EscortToSafety` keys on subject + destination.

### 3. Add PlannerOpKind variants

In `crates/worldwake-ai/src/planner_ops.rs`, add:

```rust
SearchPlace,
AskAboutPerson,
ReportMissing,
EscortToSafety,
ReportFound,
```

### 4. Add classification entries

In `classify_action_def()`, add:

- `(ActionDomain::Epistemic, "search_place")` → `SearchPlace`
- `(ActionDomain::Epistemic, "ask_about_person")` → `AskAboutPerson`
- `(ActionDomain::Social, "report_missing")` → `ReportMissing`
- `(ActionDomain::Care, "escort_to_safety")` → `EscortToSafety`
- `(ActionDomain::Social, "report_found")` → `ReportFound`

### 5. Add semantics table entries

In `build_semantics_table()`, add entries for each new PlannerOpKind variant defining the precondition/effect modeling the GOAP planner needs.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — add GoalKind variants + GoalKey entries)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — add PlannerOpKind variants + classify + semantics)

## Out of Scope

- Candidate generation (emit_search_candidates) — ticket 011
- Action definitions and handlers — tickets 007-010
- GoalBeliefView methods — ticket 004

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::SearchForMissing` maps to correct GoalKey
2. `GoalKind::ReportMissing` maps to correct GoalKey
3. `GoalKind::EscortToSafety` maps to correct GoalKey
4. `classify_action_def` maps "search_place" to `PlannerOpKind::SearchPlace`
5. Semantics table has entries for all 5 new PlannerOpKind variants
6. Existing suite: `cargo test -p worldwake-core && cargo test -p worldwake-ai`

### Invariants

1. GoalKey deduplication prevents duplicate SearchForMissing goals for the same subject
2. PlannerOpKind classification is exhaustive (no unclassified action names)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — GoalKey mapping tests for new variants
2. `crates/worldwake-ai/src/planner_ops.rs` — classification and semantics tests for new variants

### Commands

1. `cargo test -p worldwake-core goal && cargo test -p worldwake-ai planner_ops`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
