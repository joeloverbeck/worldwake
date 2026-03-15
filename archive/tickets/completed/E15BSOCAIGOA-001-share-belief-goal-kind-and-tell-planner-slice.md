# E15BSOCAIGOA-001: Add GoalKind::ShareBelief and wire the minimal Tell planner slice

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — goal identity types and the minimal planner op bridge in core and ai
**Deps**: E15 (completed)

## Problem

The AI planner has no goal type for social information sharing. Without `GoalKind::ShareBelief`, agents cannot autonomously initiate Tell actions. However, in the current codebase a new goal family is not architecturally complete unless the planner can also classify the existing `tell` action into a matching `PlannerOpKind`. Landing only the enum variant would create an orphaned goal family and a temporary incomplete path.

## Assumption Reassessment (2026-03-15)

1. `GoalKind` in `crates/worldwake-core/src/goal.rs` currently has 14 variants. Confirmed no `ShareBelief` exists.
2. `GoalKindTag` in `crates/worldwake-ai/src/goal_model.rs` currently has 14 matching tag variants. Confirmed no `ShareBelief` tag exists.
3. `GoalKey` extraction logic exists and already uses the `(entity, place)` pair as the canonical discriminator for two-entity goals such as `BuryCorpse`. `ShareBelief { listener, subject }` can follow that same shape without adding a new key field.
4. `GoalKindPlannerExt` in `goal_model.rs` has exhaustive match sites beyond `goal_kind_tag()`, including `relevant_op_kinds()`, `relevant_observed_commodities()`, `build_payload_override()`, `apply_planner_step()`, `is_progress_barrier()`, and `is_satisfied()`. Adding a goal variant will require every match site to stay exhaustive even when behavior is intentionally minimal.
5. `PlannerOpKind` in `crates/worldwake-ai/src/planner_ops.rs` currently has 15 variants, and the test suite explicitly asserts that the registered `tell` action is not yet classified into planner semantics. This ticket must update that existing deferred-state test if it absorbs the minimal Tell bridge.
6. Current targeted test coverage already exists in both touched areas:
   - `crates/worldwake-core/src/goal.rs` has inline `GoalKey` tests.
   - `crates/worldwake-ai/src/goal_model.rs` has inline `GoalKindTag` and `relevant_op_kinds()` tests.
   - `crates/worldwake-ai/src/planner_ops.rs` has inline semantics-table tests that currently expect `tell` to remain unclassified.

## Architecture Check

1. `ShareBelief { listener, subject }` fits the existing goal-identity model cleanly. It is a concrete, belief-mediated intention anchored to a specific listener and specific subject, which matches Principles 7, 18, 19, and 23.
2. The clean architecture is a minimal vertical slice: shared goal identity plus the matching planner-op classification for the already-existing `tell` action. That keeps the 1:1 correspondence between goal families and planner affordance families intact.
3. Do not introduce a temporary `todo!()`, placeholder empty op list, or other compatibility shim. If `ShareBelief` exists in the planner model, `tell` must be classifiable in the same change.
4. This ticket should remain minimal. It should not pull in social candidate generation, ranking, or utility weighting. Those remain separate concerns.

## What to Change

### 1. Add GoalKind::ShareBelief variant

In `crates/worldwake-core/src/goal.rs`, add:
```rust
ShareBelief {
    listener: EntityId,
    subject: EntityId,
}
```

Add GoalKey extraction arm:
- `entity: Some(listener)`
- `place: Some(subject)` (reuses place slot as second discriminator, same pattern as BuryCorpse)

### 2. Add GoalKindTag::ShareBelief

In `crates/worldwake-ai/src/goal_model.rs`, add `ShareBelief` to the `GoalKindTag` enum.

### 3. Wire GoalKindPlannerExt for ShareBelief

In `crates/worldwake-ai/src/goal_model.rs`, add match arm in `GoalKindPlannerExt` impl:
- `goal_kind_tag()` → `GoalKindTag::ShareBelief`
- `relevant_op_kinds()` → `&[PlannerOpKind::Tell]`
- `relevant_observed_commodities()` → `Some(BTreeSet::new())` (matches existing non-commodity goals)
- `build_payload_override()` → `Ok(None)` (Tell payload built by action handler)
- `apply_planner_step()` → no hypothetical state mutation required
- `is_progress_barrier()` → `false`
- `is_satisfied()` → `false` for now; completion is action-driven, not pre-satisfied from current hypothetical world state

### 4. Absorb the minimal Tell planner-op bridge

In `crates/worldwake-ai/src/planner_ops.rs`:
- add `PlannerOpKind::Tell`
- classify the existing social action definition `("tell", ActionDomain::Social, ActionPayload::None)` as `PlannerOpKind::Tell`
- add `PlannerOpSemantics` for `Tell`:
  - `may_appear_mid_plan: false`
  - `is_materialization_barrier: false`
  - `transition_kind: GoalModelFallback`
  - `relevant_goal_kinds: &[GoalKindTag::ShareBelief]`

This ticket intentionally absorbs only the minimal planner-op plumbing required to avoid an orphaned goal family. It does not absorb social candidate generation or ranking.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)

## Out of Scope

- social_weight in UtilityProfile (E15BSOCAIGOA-003)
- Candidate generation (E15BSOCAIGOA-004)
- Ranking logic (E15BSOCAIGOA-005)
- Golden test harness extensions (E15BSOCAIGOA-006)
- All golden tests (E15BSOCAIGOA-007 through E15BSOCAIGOA-010)
- GoalKind::InvestigateMismatch (future spec, not E15b)

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::ShareBelief` constructs with two EntityIds and produces correct GoalKey (entity=listener, place=subject)
2. `GoalKindTag::ShareBelief` round-trips through serde/bincode like existing goal-model types
3. Two ShareBelief goals with different listeners produce different GoalKeys (deduplication correctness)
4. Two ShareBelief goals with same listener but different subjects produce different GoalKeys
5. `GoalKind::ShareBelief.relevant_op_kinds()` returns exactly `&[PlannerOpKind::Tell]`
6. The planner semantics table classifies the registered `tell` action and maps it to `PlannerOpKind::Tell`
7. `PlannerOpKind::Tell` semantics are standalone-only and target exactly `GoalKindTag::ShareBelief`
8. Existing suite: `cargo test -p worldwake-core` — no regressions
9. Existing suite: `cargo test -p worldwake-ai` — no regressions (including updated deferred-state assertions)

### Invariants

1. All existing GoalKind match arms remain exhaustive — adding a variant must update every match site or the build fails
2. GoalKey uniqueness: no two semantically different goals may produce the same GoalKey
3. GoalKindTag and GoalKind remain in 1:1 correspondence
4. The registered `tell` action must not remain a planner-semantic orphan once `ShareBelief` exists
5. No temporary compatibility path (`todo!()`, empty relevant-op list, or dead enum variant) is allowed

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` (inline tests) — ShareBelief GoalKey extraction and uniqueness correctness
2. `crates/worldwake-ai/src/goal_model.rs` (inline tests) — ShareBelief tag mapping and Tell op-family mapping
3. `crates/worldwake-ai/src/planner_ops.rs` (inline tests) — update the existing deferred `tell` semantics assertions to require Tell classification instead

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed on 2026-03-15.
- Updated the ticket first to reflect the real codebase: corrected enum counts, corrected `relevant_observed_commodities()` semantics, and expanded scope to absorb the minimal `PlannerOpKind::Tell` bridge required for a coherent landing.
- Added `GoalKind::ShareBelief { listener, subject }` in `worldwake-core` and extended `GoalKey` extraction so the listener is the canonical `entity` and the subject is the canonical secondary discriminator in `place`.
- Added `GoalKindTag::ShareBelief` and wired `GoalKindPlannerExt` so ShareBelief maps to Tell-only planner operations and participates in the existing goal-model trait surface without introducing placeholders or compatibility shims.
- Added `PlannerOpKind::Tell`, classified the existing `tell` action definition into planner semantics, and updated planner-op tests that previously asserted Tell was intentionally unclassified.
- Added the narrow goal/planner tests this slice needed:
  - `crates/worldwake-core/src/goal.rs`: ShareBelief key extraction and key uniqueness coverage.
  - `crates/worldwake-ai/src/goal_model.rs`: ShareBelief tag mapping, Tell-op mapping, and empty observed-commodity tracking coverage.
  - `crates/worldwake-ai/src/planner_ops.rs`: Tell classification and semantics-table coverage.
- Deviation from the original plan:
  - To satisfy exhaustive workspace gates cleanly, the implementation also added conservative non-candidate semantics in `agent_tick.rs`, `failure_handling.rs`, and `ranking.rs` so the new goal/op pair behaves coherently before the later social candidate-generation and ranking tickets land.
  - No social candidate generation, utility-profile work, or golden social scenarios were added here; those remain separate work.
- Verification:
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
