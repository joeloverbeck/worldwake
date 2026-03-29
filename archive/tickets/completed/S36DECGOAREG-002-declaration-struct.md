# S36DECGOAREG-002: Create GoalDispatchDeclaration struct and static declarations

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` static dispatch declaration substrate
**Deps**: S36DECGOAREG-001

## Problem

With the dispatch key in place (001), the next step is defining the declaration struct that holds all static dispatch properties per key, and populating one declaration per dispatch-distinguishing key. This is the data foundation that later tickets will consume.

## Assumption Reassessment (2026-03-29)

1. `GoalDispatchKey` already exists in [`crates/worldwake-ai/src/goal_dispatch_key.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs) with exhaustive `GoalKind -> GoalDispatchKey` mapping and focused coverage for acquire-purpose splits, punish splits, recipe-input collapse-by-shape, and whole-enum representative coverage.
2. The shared abstraction boundary under audit is AI static goal-dispatch metadata:
   - authoritative identity remains `worldwake_core::GoalKind`
   - derived dispatch identity is `worldwake_ai::GoalDispatchKey`
   - the declaration substrate should own the static dispatch facts now scattered across `goal_model.rs`, `planner_ops.rs`, and `decision_trace.rs`
3. Static dispatch surfaces confirmed in live code:
   - `trace_label`: [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) still renders selected goals through raw `Debug` formatting (`format!("{:?}", g.kind)`) in planning summaries. No stable label surface exists yet.
   - `provenance_family`: [`GoalKindPlannerExt::ranked_goal_provenance_family()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) returns `Option<RankedGoalProvenanceFamily>` and is consumed by [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs).
   - `relevant_ops`: [`GoalKindPlannerExt::relevant_op_kinds()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) returns `&'static [PlannerOpKind]` and is consumed by the search pipeline.
4. `RankedGoalProvenanceFamily` is defined in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) with live variants `Danger` and `Drive`.
5. `PlannerOpKind` is defined in [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs) and currently has 27 variants, not 28.
6. Existing focused coverage already locks in some of the behavior this declaration layer must mirror:
   - `goal_model::tests::ranked_goal_provenance_family_is_payload_aware`
   - `goal_model::tests::restock_goal_relevant_ops_include_trade_production_and_cargo`
   - `goal_model::tests::steal_goal_uses_move_cargo_ops_while_punishment_uses_live_verdict_actions`
7. Mismatch + scope correction: adding declaration constants without any equivalence proof would introduce a temporary second manual dispatch matrix. That is not a desirable end-state architecture. This ticket remains valid only as a bounded phase-2 prerequisite because sibling tickets 003, 004, and 005 already exist to migrate provenance/ops, reverse-membership, and trace labels onto the declaration substrate. The ticket therefore needs stronger equivalence tests to make that temporary duplication safe until those migrations land.
8. Strategy selectors (`InvalidationStrategy`, `FeasibilityStrategy`) remain deferred to tickets 006 and 007 per [`specs/S36-declarative-goal-registration.md`](/home/joeloverbeck/projects/worldwake/specs/S36-declarative-goal-registration.md).

## Architecture Check

1. The long-term architecture is better than the current scattered matches: one declaration per dispatch-distinguishing key is easier to audit than separate provenance, relevant-op, reverse-membership, and trace-label tables. That aligns with P25/P27 from [`docs/FOUNDATIONS.md`](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): the declaration is a derived read-model over authoritative `GoalKind`, and it gives one inspection point for static AI dispatch behavior.
2. A trait-based registry or dynamic builder would add indirection without benefit because all fields in scope for this ticket are compile-time static. `GoalDispatchKey::declaration() -> &'static GoalDispatchDeclaration` keeps the contract explicit and deterministic.
3. Important correction: this ticket alone does not produce the ideal architecture, because the old dispatch sites remain temporarily authoritative until 003–005 migrate them. That temporary duplication is acceptable only because this ticket is deliberately constrained, equivalence-tested, and part of an active migration sequence already captured in sibling tickets. If the migration sequence stalled here, the architecture would be worse than today.
4. No backwards-compatibility aliasing or shims: the declaration layer is additive, and later tickets should replace old tables rather than wrapping them.

## Verification Layers

1. Declaration completeness -> compile-time: exhaustive match in `GoalDispatchKey::declaration()` with no wildcard ensures every key has a declaration.
2. Declaration data correctness -> focused unit tests: for every live dispatch key, declaration provenance and relevant-ops data match the current authoritative implementations in `GoalKindPlannerExt`.
3. Trace-label coverage -> focused unit tests: every declaration has a stable non-empty label, and payload-sensitive splits that matter to humans (`Acquire*`, `Punish*`) get distinct labels.
4. Single-layer ticket: static AI dispatch metadata only. No runtime, authoritative, or event-log behavior changes land here.

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

Add focused declaration tests in the new module. The tests should verify whole-key coverage plus equivalence against the current live provenance and relevant-op behavior, not just spot-check a few hand-picked branches.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add module declaration)

## Out of Scope

- Migrating existing dispatch consumers to declaration lookups (`goal_model.rs`, `planner_ops.rs`, `decision_trace.rs`) — tickets 003–005 own that follow-on work
- `InvalidationStrategy` and `FeasibilityStrategy` fields (tickets 006–007)
- Modifying `GoalKindTag`, `GoalKind`, or any `worldwake-core` type
- Candidate generation, intention domain progress-op ownership
- Removing or modifying the existing `*_OPS` const arrays or `ranked_goal_provenance_family()` implementation in this ticket

## Acceptance Criteria

### Tests That Must Pass

1. `test_declaration_completeness`: Every `GoalDispatchKey` variant has a declaration (enforced by exhaustive match — this is a compile-time guarantee, but a runtime test iterating all keys confirms no panic).
2. `test_declaration_provenance_matches_live_goal_model`: for one representative `GoalKind` per live dispatch key, `GoalDispatchKey::from_goal_kind(goal).declaration().provenance_family` equals `goal.ranked_goal_provenance_family()`.
3. `test_declaration_relevant_ops_match_live_goal_model`: for one representative `GoalKind` per live dispatch key, `GoalDispatchKey::from_goal_kind(goal).declaration().relevant_ops` equals `goal.relevant_op_kinds()`.
4. `test_punish_fine_vs_exile_ops`: `PunishFine` and `PunishExile` declarations have different `relevant_ops` slices.
5. `test_trace_labels_nonempty_and_distinct_for_payload_splits`: every declaration has a non-empty `trace_label`, and at minimum the `Acquire*` and `Punish*` payload-sensitive splits do not collapse to the same label.
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Adding a `GoalDispatchKey` variant without a corresponding declaration fails compilation.
2. Declaration values must exactly match the current live static dispatch behavior for all represented goal shapes (zero behavioral change).
3. Declarations are `&'static` — zero runtime allocation.
4. This ticket must not widen the static drift surface beyond the declaration module itself; equivalence tests are required because old consumers still remain in place until follow-on tickets land.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — `test_declaration_completeness`
Rationale: proves the new declaration table covers every dispatch key and that the lookup stays exhaustive.
2. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — `test_declaration_provenance_matches_live_goal_model`
Rationale: prevents the new declaration substrate from drifting from the current authoritative provenance-family dispatch before ticket 003 migrates consumers.
3. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — `test_declaration_relevant_ops_match_live_goal_model`
Rationale: prevents declaration `relevant_ops` from diverging from the current search-facing operator surface before ticket 003 migrates consumers.
4. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — `test_punish_fine_vs_exile_ops`
Rationale: locks in the live payload-sensitive `PunishAccused` operator split at the declaration level.
5. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — `test_trace_labels_nonempty_and_distinct_for_payload_splits`
Rationale: ensures the declaration creates a real stable label contract rather than a placeholder field that collapses meaningful human-facing distinctions.

### Commands

1. `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_declaration_completeness`
2. `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_declaration_provenance_matches_live_goal_model`
3. `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_declaration_relevant_ops_match_live_goal_model`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed: added [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs) with `GoalDispatchDeclaration`, one static declaration per `GoalDispatchKey`, and an exhaustive `GoalDispatchKey::declaration()` lookup; re-exported the declaration type from [`crates/worldwake-ai/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/lib.rs); added focused declaration completeness, provenance equivalence, relevant-op equivalence, punishment-op split, and trace-label tests.
- Deviations from original plan: the ticket was corrected before implementation to acknowledge that this phase temporarily introduces a second manual static-dispatch matrix and therefore needs explicit equivalence coverage. During implementation, that coverage caught a wrong declaration assumption: `EngageHostile` must carry `Some(RankedGoalProvenanceFamily::Danger)` to match the live goal model.
- Verification results: `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_declaration_completeness`, `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_declaration_provenance_matches_live_goal_model`, `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_declaration_relevant_ops_match_live_goal_model`, `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_punish_fine_vs_exile_ops`, `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_trace_labels_nonempty_and_distinct_for_payload_splits`, `cargo test -p worldwake-ai`, `cargo test --workspace`, and `cargo clippy --workspace` all passed on 2026-03-29.
