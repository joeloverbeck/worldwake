# S59EXPOBLSUB-005: GoalKind variants + GoalKey + PlannerOpKind integration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — GoalKind enum, GoalKey, planner dispatch tables, PlannerOpKind enum, classify_action_def, build_semantics_table, inert exhaustive planner fallout
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
7. Ticket says only `goal.rs` and `planner_ops.rs` need edits. Live code has exhaustive planner-facing consumers in `crates/worldwake-ai/src/goal_dispatch_key.rs`, `crates/worldwake-ai/src/goal_dispatch_decl.rs`, and `crates/worldwake-ai/src/goal_model.rs` that must be updated for new goal/operator variants. Correction applied: expand the owned surface to include those files and their focused tests. Why safe: this is bounded compile-safe fallout within the same planner contract, not a scope change into candidate generation or runtime actions.
8. Ticket says new variants are purely additive planner surface. Live code routes `GoalKindPlannerExt::relevant_op_kinds()` through dispatch declarations, so each new `GoalDispatchKey` also needs a declaration even if the variants are not emitted yet. Correction applied: treat dispatch declarations as part of this ticket's planner integration surface. Why safe: this preserves existing planner invariants without making the S59 goals behaviorally live ahead of ticket 011.
9. Ticket says semantics work ends at `build_semantics_table()`. Live code has exhaustive `GoalKind` helper matches and coverage lists in `goal_model.rs` that will fail or become dishonest if the new variants are not given inert branches. Correction applied: include inert `goal_model` branches and exhaustive tests for the reserved goal variants. Why safe: the parent spec already reserves these variants here, while ticket 011 still owns candidate emission and first live use.
10. Ticket says the fallout is core + ai only. Live code also has compile-enforced exhaustive consumers in `crates/worldwake-ai/src/agent_tick/observation.rs`, `crates/worldwake-ai/src/failure_handling.rs`, `crates/worldwake-ai/src/goal_policy.rs`, `crates/worldwake-ai/src/ranking.rs`, and `crates/worldwake-cli/src/display.rs`. Correction applied: absorb those bounded enum-match updates as required compile-safe fallout. Why safe: these are non-architectural exhaustiveness updates driven directly by the new shared goal/op variants.

## Architecture Check

1. This ticket lands shared planner-visible types and inert planner integration only. Candidate generation and runtime action behavior stay out of scope until later S59 tickets make these variants live.
2. No backward compatibility shims.

## Verification Layers

1. GoalKind variants exist and map to GoalKey → focused unit test
2. PlannerOpKind classification matches action names → focused unit test
3. GoalDispatchKey and dispatch declarations cover the new variants without breaking exhaustive planner tables → focused unit test
4. Semantics table entries produce valid precondition/effect pairs → focused unit test
5. Reserved goal variants remain compile-safe and planner-inert until later tickets emit or execute them → focused unit test
6. Single-crate changes (core + ai) — no cross-system verification needed.

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

### 3. Add planner dispatch entries

In `crates/worldwake-ai/src/goal_dispatch_key.rs`, add dispatch keys for the three new `GoalKind` variants and extend the exhaustive coverage tests.

In `crates/worldwake-ai/src/goal_dispatch_decl.rs`, add declarations for those new dispatch keys with relevant-op sets and inert invalidation/feasibility strategies that match the reserved planner surface for this ticket.

### 4. Add PlannerOpKind variants

In `crates/worldwake-ai/src/planner_ops.rs`, add:

```rust
SearchPlace,
AskAboutPerson,
ReportMissing,
EscortToSafety,
ReportFound,
```

### 5. Add classification entries

In `classify_action_def()`, add:

- `(ActionDomain::Epistemic, "search_place")` → `SearchPlace`
- `(ActionDomain::Epistemic, "ask_about_person")` → `AskAboutPerson`
- `(ActionDomain::Social, "report_missing")` → `ReportMissing`
- `(ActionDomain::Care, "escort_to_safety")` → `EscortToSafety`
- `(ActionDomain::Social, "report_found")` → `ReportFound`

### 6. Add semantics table entries

In `build_semantics_table()`, add entries for each new PlannerOpKind variant defining the precondition/effect modeling the GOAP planner needs.

### 7. Add inert goal-model branches

In `crates/worldwake-ai/src/goal_model.rs`, extend the exhaustive `GoalKind` / `PlannerOpKind` matches and coverage lists so the new goal and planner-op variants remain compile-safe and planner-inert until later tickets make them behaviorally live.

### 8. Sweep compile-safe enum fallout

Update downstream exhaustive `GoalKind` / `PlannerOpKind` consumers so the new reserved variants render, rank, and fail gracefully without becoming behaviorally live.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — add GoalKind variants + GoalKey entries)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify — add dispatch keys + exhaustive coverage)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — add dispatch declarations + exhaustive coverage)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — add PlannerOpKind variants + classify + semantics)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add inert exhaustive branches/tests for reserved variants)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — exhaustive `PlannerOpKind` fallout)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — exhaustive `PlannerOpKind` fallout)
- `crates/worldwake-ai/src/goal_policy.rs` (modify — exhaustive `GoalKind` fallout)
- `crates/worldwake-ai/src/ranking.rs` (modify — exhaustive `GoalKind` fallout)
- `crates/worldwake-cli/src/display.rs` (modify — render new reserved `GoalKind` variants)

## Out of Scope

- Candidate generation (emit_search_candidates) — ticket 011
- Action definitions and handlers — tickets 007-010
- GoalBeliefView methods — ticket 004

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::SearchForMissing` maps to correct GoalKey
2. `GoalKind::ReportMissing` maps to correct GoalKey
3. `GoalKind::EscortToSafety` maps to correct GoalKey
4. `GoalDispatchKey` / dispatch declarations cover the 3 new goal variants
5. `classify_action_def` maps "search_place" to `PlannerOpKind::SearchPlace`
6. Semantics table has entries for all 5 new PlannerOpKind variants
7. `goal_model` exhaustive helpers accept the reserved variants without making them live before downstream tickets
8. Existing suite: `cargo test -p worldwake-core && cargo test -p worldwake-ai`

### Invariants

1. GoalKey deduplication prevents duplicate SearchForMissing goals for the same subject
2. PlannerOpKind classification is exhaustive (no unclassified action names)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — GoalKey mapping tests for new variants
2. `crates/worldwake-ai/src/goal_dispatch_key.rs` — dispatch-key coverage tests for new variants
3. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — declaration completeness/relevant-op tests for new variants
4. `crates/worldwake-ai/src/planner_ops.rs` — classification and semantics tests for new variants
5. `crates/worldwake-ai/src/goal_model.rs` — inert reserved-variant coverage for exhaustive planner helpers

### Commands

1. `cargo test -p worldwake-core goal`
2. `cargo test -p worldwake-ai goal_dispatch_key`
3. `cargo test -p worldwake-ai goal_dispatch_decl`
4. `cargo test -p worldwake-ai planner_ops`
5. `cargo test -p worldwake-ai goal_model`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo test --workspace`

## Outcome

Completed on 2026-04-06.

Landed the reserved S59 planner substrate without making the behavior live yet:
- added `GoalKind::{SearchForMissing, ReportMissing, EscortToSafety}` and canonical `GoalKey` extraction in `worldwake-core`
- added `GoalDispatchKey` and dispatch declarations for the new goal families with reserved relevant-op sets
- added `PlannerOpKind::{SearchPlace, AskAboutPerson, ReportMissing, EscortToSafety, ReportFound}` plus classification and inert fallback semantics
- extended `goal_model` with compile-safe inert branches and focused reserved-variant tests
- absorbed the bounded exhaustive fallout in AI failure/ranking/policy/observation code and CLI goal formatting so the new shared variants compile cleanly across the workspace

Deviation note:
- The live fallout surface was broader than the original ticket text. No behavioral candidate generation or runtime action wiring was pulled in; the extra edits were compile-safe enum-consumer updates only.

## Verification Result

Passed:
1. `cargo test -p worldwake-core goal`
2. `cargo test -p worldwake-ai goal_dispatch_key`
3. `cargo test -p worldwake-ai goal_dispatch_decl`
4. `cargo test -p worldwake-ai planner_ops`
5. `cargo test -p worldwake-ai goal_model`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`
