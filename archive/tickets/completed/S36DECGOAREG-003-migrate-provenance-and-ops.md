# S36DECGOAREG-003: Migrate provenance family and relevant ops to declaration lookups

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` static dispatch cleanup
**Deps**: S36DECGOAREG-001, S36DECGOAREG-002, `specs/S36-declarative-goal-registration.md`

## Problem

`GoalDispatchKey` and `GoalDispatchDeclaration` already exist, and the declaration module already carries the static `provenance_family` and `relevant_ops` data with focused equivalence tests. But `GoalKindPlannerExt::ranked_goal_provenance_family()` and `GoalKindPlannerExt::relevant_op_kinds()` in `goal_model.rs` still duplicate that same static dispatch logic through separate `match GoalKind` tables and duplicate `*_OPS` arrays. That leaves two competing sources of truth for the same static AI dispatch facts.

This ticket should finish the migration for those two surfaces by making the trait methods declaration-backed and deleting the now-redundant static tables from `goal_model.rs`. It should not widen into planner-op reverse membership or trace-label migration; those remain separate S36 tickets because they touch different consumers and proof surfaces.

## Assumption Reassessment (2026-03-29)

1. The shared abstraction boundary under audit is AI static goal-dispatch metadata for `GoalKindPlannerExt`:
   - authoritative identity remains [`worldwake_core::GoalKind`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs)
   - derived dispatch identity is [`GoalDispatchKey`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs)
   - static declaration substrate is [`GoalDispatchDeclaration`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs)
2. Live code already contains the declaration substrate this ticket originally treated as planned work:
   - [`crates/worldwake-ai/src/goal_dispatch_key.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs) defines a payload-aware exhaustive `GoalKind -> GoalDispatchKey` mapping with focused tests for acquire-purpose splits, punish splits, and full representative coverage.
   - [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs) defines `GoalDispatchDeclaration`, one declaration per key, and focused tests proving declaration `provenance_family` and `relevant_ops` match the current live goal-model behavior.
3. The remaining duplication is still live in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs):
   - `GoalKindPlannerExt::ranked_goal_provenance_family()` is still an exhaustive `match GoalKind`.
   - `GoalKindPlannerExt::relevant_op_kinds()` is still an exhaustive `match GoalKind`.
   - the `CONSUME_OPS` through `EXILE_OPS` slice constants are duplicated there even though equivalent constants already exist in [`goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs).
4. Current consumers confirm this ticket is still single-layer static dispatch work:
   - [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) consumes `ranked_goal_provenance_family()`.
   - [`crates/worldwake-ai/src/search/candidates.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs) and [`crates/worldwake-ai/src/search/transition.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/transition.rs) consume `relevant_op_kinds()`.
   - no authoritative runtime control boundary or cross-crate contract changes are involved.
5. The current declaration tests already prove the intended zero-behavior-change contract:
   - `goal_dispatch_decl::tests::test_declaration_provenance_matches_live_goal_model`
   - `goal_dispatch_decl::tests::test_declaration_relevant_ops_match_live_goal_model`
   Those tests presently compare declarations against the old `goal_model.rs` matches. After this ticket, they remain valid as regression tests for the declaration-backed trait methods.
6. `cargo test -p worldwake-ai -- --list` confirms the existing focused tests named in the ticket are real. The original command examples `cargo test -p worldwake-ai -- ranked_goal_provenance` and `cargo test -p worldwake-ai -- relevant_op` are too fuzzy to satisfy the ticket contract; this ticket must name exact current tests.
7. Mismatch + scope correction: the original ticket text assumed the declaration migration for provenance and relevant ops had not started. That is stale. The real work is not “introduce declaration lookups” in the abstract; it is “remove the duplicate `goal_model.rs` static dispatch path now that the declaration substrate already exists.” The new scope should therefore explicitly include deleting the duplicated `*_OPS` constants from `goal_model.rs` if they become dead after routing through declarations.
8. Adjacent S36 work remains out of scope and should stay separate:
   - reverse-membership derivation in [`tickets/S36DECGOAREG-004-derive-reverse-membership.md`](/home/joeloverbeck/projects/worldwake/tickets/S36DECGOAREG-004-derive-reverse-membership.md)
   - trace-label migration in [`tickets/S36DECGOAREG-005-trace-label-migration.md`](/home/joeloverbeck/projects/worldwake/tickets/S36DECGOAREG-005-trace-label-migration.md)
   - invalidation / feasibility strategy migration in tickets 006 and 007

## Architecture Check

1. Routing `ranked_goal_provenance_family()` and `relevant_op_kinds()` through `GoalDispatchKey::from_goal_kind(self).declaration()` is cleaner than the current architecture because it removes a duplicate static dispatch matrix that can drift from the declaration substrate introduced in 001/002. One declaration table remains the single static source of truth.
2. Deleting the mirrored `*_OPS` constants from `goal_model.rs` is architecturally better than keeping them “for convenience.” Keeping duplicate constant sets across two modules would preserve exactly the long-term maintenance risk S36 is trying to remove.
3. This change is more beneficial than the current architecture because it improves robustness without adding indirection: consumers keep the same trait methods and signatures, but the implementation boundary becomes explicit and centralized.
4. No backwards-compatibility aliasing or shim paths are allowed. The old `goal_model.rs` match bodies should be replaced, not wrapped or left alongside the declaration path.

## Verification Layers

1. Provenance-family dispatch equivalence -> focused unit tests in `goal_dispatch_decl.rs` plus direct `goal_model.rs` focused tests.
2. Relevant-op dispatch equivalence -> focused unit tests in `goal_dispatch_decl.rs` plus direct `goal_model.rs` focused tests.
3. Search/ranking regression safety -> `cargo test -p worldwake-ai`.
4. Workspace integration safety -> `cargo test --workspace` and `cargo clippy --workspace`.
5. Single-layer ticket: no additional action-trace, event-log, or authoritative-world-state mapping is required because this change only redirects static AI metadata lookups.

## What to Change

### 1. Replace `goal_model.rs` static dispatch bodies with declaration lookups

Update [`GoalKindPlannerExt::ranked_goal_provenance_family()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) and [`GoalKindPlannerExt::relevant_op_kinds()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) to delegate through `GoalDispatchKey::from_goal_kind(self).declaration()`.

### 2. Delete dead duplicated `*_OPS` tables from `goal_model.rs`

If `goal_model.rs` no longer uses the local `CONSUME_OPS` through `EXILE_OPS` constants after step 1, remove them there rather than preserving a second copy of declaration-owned static data.

### 3. Keep or strengthen focused behavior tests

Preserve the existing focused tests that lock payload-sensitive provenance and relevant-op behavior in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs). If needed, tighten them to ensure this ticket still has direct local proof in addition to the declaration-module equivalence tests.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — route through declarations; remove dead duplicated ops constants if unused)

## Out of Scope

- Deriving `PlannerOpSemantics.relevant_goal_kinds` from declarations (`planner_ops.rs`) — ticket 004
- Migrating decision-trace labels to declaration-owned labels — ticket 005
- Invalidation and feasibility strategy routing — tickets 006 and 007
- Any change to `GoalKindPlannerExt` method signatures
- Any change to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `goal_dispatch_decl::tests::test_declaration_provenance_matches_live_goal_model`
2. `goal_dispatch_decl::tests::test_declaration_relevant_ops_match_live_goal_model`
3. `goal_model::tests::ranked_goal_provenance_family_is_payload_aware`
4. `goal_model::tests::steal_goal_uses_move_cargo_ops_while_punishment_uses_live_verdict_actions`
5. Existing suite: `cargo test -p worldwake-ai`
6. Full workspace: `cargo test --workspace`
7. Lint: `cargo clippy --workspace`

### Invariants

1. `GoalKindPlannerExt` public method signatures remain unchanged.
2. `ranked_goal_provenance_family()` and `relevant_op_kinds()` read declaration metadata rather than maintaining a second static match table.
3. `goal_model.rs` must not retain dead duplicate `*_OPS` tables after the migration.
4. Zero behavioral change: all existing tests continue to pass.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — keep `ranked_goal_provenance_family_is_payload_aware`
Rationale: preserves a local focused proof that payload-sensitive provenance behavior remains correct at the trait surface used by ranking.
2. `crates/worldwake-ai/src/goal_model.rs` — keep `steal_goal_uses_move_cargo_ops_while_punishment_uses_live_verdict_actions`
Rationale: preserves a local focused proof that relevant-op lookup still distinguishes the live operator families consumed by search.
3. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — keep `test_declaration_provenance_matches_live_goal_model`
Rationale: ensures the declaration table and the trait surface stay identical after the migration instead of drifting again.
4. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — keep `test_declaration_relevant_ops_match_live_goal_model`
Rationale: ensures declaration `relevant_ops` remain exactly what the trait surface exposes to the planner.

### Commands

1. `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_declaration_provenance_matches_live_goal_model`
2. `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_declaration_relevant_ops_match_live_goal_model`
3. `cargo test -p worldwake-ai goal_model::tests::ranked_goal_provenance_family_is_payload_aware`
4. `cargo test -p worldwake-ai goal_model::tests::steal_goal_uses_move_cargo_ops_while_punishment_uses_live_verdict_actions`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed: [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) now routes `GoalKindPlannerExt::ranked_goal_provenance_family()` and `GoalKindPlannerExt::relevant_op_kinds()` through `GoalDispatchKey::from_goal_kind(self).declaration()`. The duplicated `CONSUME_OPS` through `EXILE_OPS` constant tables were removed from `goal_model.rs`.
- Deviations from original plan: the ticket was corrected before implementation because `GoalDispatchKey`, `GoalDispatchDeclaration`, and declaration equivalence tests had already landed in earlier S36 tickets. No new tests were added because the existing focused declaration-equivalence tests plus the existing direct `goal_model.rs` payload-sensitive tests already covered the exact invariant this migration needed to preserve.
- Verification results: `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_declaration_provenance_matches_live_goal_model`, `cargo test -p worldwake-ai goal_dispatch_decl::tests::test_declaration_relevant_ops_match_live_goal_model`, `cargo test -p worldwake-ai goal_model::tests::ranked_goal_provenance_family_is_payload_aware`, `cargo test -p worldwake-ai goal_model::tests::steal_goal_uses_move_cargo_ops_while_punishment_uses_live_verdict_actions`, `cargo test -p worldwake-ai`, `cargo test --workspace`, and `cargo clippy --workspace` all passed on 2026-03-29.
