# S36DECGOAREG-008: Wildcard audit and exhaustive match enforcement

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S36DECGOAREG-005, S36DECGOAREG-006, S36DECGOAREG-007, S36DECGOAREG-009

## Problem

After the remaining S36 dispatch/cleanup surfaces are migrated to declaration-owned routing (005–007, 009), remaining wildcard `_` arms in `GoalKind` matches across the AI crate should be audited. Wildcards that serve as shortcuts (should be reviewed per variant) must be converted to exhaustive matches. Wildcards that provide a meaningful correct-for-all-future-variants default should be documented.

## Assumption Reassessment (2026-03-29)

1. Priority dispatch sites that MUST become exhaustive (per spec):
   - `GoalDispatchKey::from_goal_kind()` — already exhaustive (delivered by 001).
   - `GoalDispatchKey::declaration()` — already exhaustive (delivered by 002).
   - `derive_invalidation_conditions()` strategy routing — already exhaustive after 006.
   - `relevant_op_kinds()` — already routes through declarations after 003.
   - `goal_specific_feasibility()` strategy routing — already exhaustive after 007.
   - Any residual coarse `GoalKindTag` shadow-identity routing should be removed by 009 before this audit runs.
2. Remaining wildcard arms in `goal_model.rs` (confirmed from exploration):
   - `build_payload_override()` (lines 617-783): ~6 wildcard arms. These are in `GoalKindPlannerExt` goal-semantic methods that are **out of scope for S36** per the spec. The wildcards match on `PlannerOpKind`, not `GoalKind`. Leave as-is but document.
   - `apply_planner_step()` (lines 786-924): ~4 wildcard arms. Same — goal-semantic method, out of scope.
   - `is_progress_barrier()` (lines 958-1026): 1 wildcard arm. Goal-semantic method, out of scope.
   - `is_satisfied()` (lines 1028-1106): 2 wildcard arms. Goal-semantic method, out of scope.
   - `goal_relevant_places()` (lines 1108-1250): 1 wildcard arm. Goal-semantic method, out of scope.
3. Wildcard arms in `decision_trace.rs`:
   - `omitted_political_reason_for_goal()` (line 1068): `_ => None` — correct default for non-political goals. Document.
   - `omitted_social_reason_for_goal()` (line 1082): `_ => None` — correct default for non-social goals. Document.
   - `goal_history_entry()` (line 994): `_ => ...` on `DecisionOutcome`, not `GoalKind`. Not in scope.
4. `goal_kind_tag()` currently exists only as a coarse shadow identity and is expected to be removed by 009 rather than audited here as an enduring dispatch surface.
5. `goal_kind_discriminant()` (ranking.rs:1064-1088): Already exhaustive, no wildcard.

## Architecture Check

1. This ticket is a cleanup/audit pass, not a structural change. It verifies that the migration achieved the spec's exhaustive-match goal and documents remaining wildcards that are intentionally correct. P26 (No Backward Compatibility): no shims are introduced.
2. The spec explicitly excludes goal-semantic methods (`is_satisfied`, `build_payload_override`, etc.) from S36 scope. Wildcards in those methods are documented but not converted.

## Verification Layers

1. Exhaustive enforcement → compile-time: verified by attempting to add a dummy `GoalKind` variant in a test build and confirming compilation fails at all priority dispatch sites.
2. Documentation → code review: remaining wildcards have inline comments explaining why the default is correct.
3. Single-layer ticket: audit and documentation only.

## What to Change

### 1. Audit all `match` on `GoalKind` / `GoalKindTag` in `worldwake-ai`

Use grep to find all match sites. For each:
- If it's a declaration/dispatch site migrated in 001–007: confirm it is exhaustive (no wildcard).
- If it's a goal-semantic method excluded from S36: document the wildcard with a comment explaining why it's correct.
- If it's a rendering/classification site with a legitimate default: add `#[deny(unreachable_patterns)]` or a comment.

### 2. Add `#[deny(unreachable_patterns)]` where appropriate

For critical dispatch matches that must remain exhaustive, add the lint attribute as a compile-time guard.

### 3. Document remaining wildcards

Add inline comments to wildcard arms in decision_trace.rs explaining they are correct defaults for non-matching goal families.

### 4. Cleanup dead code

Remove any `*_OPS` const arrays in `goal_model.rs` that are now unused after 003 migrated `relevant_op_kinds()` to declarations and 004 replaced `GOALS_*` arrays. Verify with `cargo clippy` / dead code warnings.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — add documentation comments to excluded wildcards, remove dead `*_OPS` arrays if unused)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — add documentation comments to legitimate wildcards)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify — add `#[deny(unreachable_patterns)]` if needed)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — add `#[deny(unreachable_patterns)]` if needed)

## Out of Scope

- Converting wildcards in goal-semantic methods (`is_satisfied`, `build_payload_override`, `apply_planner_step`, `is_progress_barrier`, `goal_relevant_places`, `matches_binding`) — these are explicitly excluded from S36 per the spec.
- Any behavioral changes
- Any changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all existing tests pass unchanged.
2. `cargo clippy --workspace` — no new warnings (dead code removal may fix existing warnings).
3. Full workspace: `cargo test --workspace`

### Invariants

1. Zero behavioral change.
2. All priority dispatch sites (declaration key lookup, declaration table lookup, invalidation strategy routing, relevant_ops, feasibility strategy routing) have exhaustive matches with no wildcard.
3. Remaining wildcards in excluded methods and legitimate-default sites are documented.
4. No dead `*_OPS` const arrays remain if they are unused after migration.

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
