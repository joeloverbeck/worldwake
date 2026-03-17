# S02GOADECPOLUNI-002: Migrate ranking to embed DecisionContext and consume shared policy

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — ranking.rs refactor
**Deps**: S02GOADECPOLUNI-001

## Problem

`ranking.rs` currently owns goal-family suppression logic via `is_suppressed()` and separately computes `max_self_care_class()` / `danger_class()` inside `RankingContext`. This duplicates pressure classification that interrupts also need. Migrating ranking to embed `DecisionContext` and call `evaluate_suppression()` unifies the suppression surface.

## Assumption Reassessment (2026-03-16)

1. `is_suppressed()` at `ranking.rs:102-111` checks LootCorpse, BuryCorpse, ShareBelief, ClaimOffice, SupportCandidateForOffice against `danger_high_or_above() || self_care_high_or_above()` — confirmed.
2. `RankingContext` computes `max_self_care_class()` (lines 81-99) and `danger_class()` (lines 74-79) as private methods — confirmed.
3. `rank_candidates()` signature is `(candidates, view, agent, current_tick, utility, recipes) -> Vec<RankedGoal>` — confirmed at line 14.
4. `rank_candidates()` is called from `agent_tick.rs:506` with these exact parameters — confirmed.
5. The spec says `RankingContext` should embed a `DecisionContext` field, and `is_suppressed()` should be replaced by `evaluate_suppression()` — confirmed in spec Deliverable 5.

## Architecture Check

1. Embedding `DecisionContext` in `RankingContext` means the two class derivations (`max_self_care_class`, `danger_class`) happen once and are reusable. The `RankingContext` helper methods `self_care_high_or_above()` and `danger_high_or_above()` become dead code once `is_suppressed()` is replaced and can be removed.
2. No backwards-compatibility shims. `is_suppressed()` is deleted outright and replaced with `evaluate_suppression()`.

## What to Change

### 1. Add `decision_context` field to `RankingContext`

In `RankingContext::new()`, build a `DecisionContext` from the existing `max_self_care_class()` and `danger_class()` computations. Store it as a field.

### 2. Replace `is_suppressed()` with `evaluate_suppression()`

Change the filter in `rank_candidates()` from:
```rust
.filter(|candidate| !is_suppressed(candidate, &context))
```
to:
```rust
.filter(|candidate| {
    matches!(
        evaluate_suppression(&candidate.key.kind, &context.decision_context),
        GoalPolicyOutcome::Available
    )
})
```

### 3. Remove dead code

- Delete `fn is_suppressed()`
- Delete `fn self_care_high_or_above()` and `fn danger_high_or_above()` from `RankingContext` impl (only used by `is_suppressed`)
- Keep `max_self_care_class()` and `danger_class()` as they are still used by `RankingContext::new()` to populate `DecisionContext`, but make them free functions or keep as private helpers in the constructor

### 4. Expose DecisionContext from rank_candidates

The spec requires `DecisionContext` to be built once per agent tick and threaded to both ranking and interrupts. There are two valid approaches:
- (a) `rank_candidates()` returns `(Vec<RankedGoal>, DecisionContext)` so the caller can pass it to interrupts
- (b) `DecisionContext` is built outside `rank_candidates()` and passed in

Option (b) is cleaner (ticket 005 builds it in agent_tick), but requires a signature change. Option (a) avoids changing the signature now but creates a temporary coupling. This ticket implements option (b): add `decision_context: &DecisionContext` parameter to `rank_candidates()`.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — update `rank_candidates()` call site to pass `DecisionContext`)

## Out of Scope

- Modifying `interrupts.rs` (tickets 003, 004)
- Changing `priority_class()` or `motive_score()` derivation logic
- Changing `compare_ranked_goals()` sort logic
- Adding new goal families
- Changes to `worldwake-core` or `worldwake-sim`
- Building `DecisionContext` properly in agent_tick (ticket 005 does the full wiring; this ticket uses a temporary construction inline at the call site)

## Acceptance Criteria

### Tests That Must Pass

1. `rank_candidates()` drops LootCorpse when `max_self_care_class >= High` (same behavior as before)
2. `rank_candidates()` drops BuryCorpse when `danger_class >= High` (same behavior as before)
3. `rank_candidates()` keeps LootCorpse/BuryCorpse when stress is below High (same behavior as before)
4. `rank_candidates()` never drops self-care, danger, healing, or enterprise goals via suppression
5. `is_suppressed()` function no longer exists in `ranking.rs`
6. `self_care_high_or_above()` and `danger_high_or_above()` no longer exist as RankingContext methods
7. All existing ranking unit tests continue to pass
8. All existing golden tests pass: `cargo test -p worldwake-ai`
9. `cargo clippy --workspace`

### Invariants

1. Suppression behavior is identical to pre-migration for all 17 goal families
2. `ranking.rs` does not contain goal-family-specific suppression branches — it delegates to `evaluate_suppression()`
3. `DecisionContext` is the sole source of pressure classification for suppression
4. Deterministic ranking output is unchanged (same sort, same filtering)

## Test Plan

### New/Modified Tests

1. Existing `ranking.rs` tests — verify they pass unchanged (behavioral equivalence)
2. If any ranking test directly tested `is_suppressed()`, update to test through `rank_candidates()` or `evaluate_suppression()`

### Commands

1. `cargo test -p worldwake-ai ranking`
2. `cargo test -p worldwake-ai` (includes golden tests)
3. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-17
- **What changed**:
  - Added `build_decision_context()` pub helper in `ranking.rs` (exported via `lib.rs`) to extract pressure classification from a belief view
  - Added `decision_context: &DecisionContext` parameter to `rank_candidates()`
  - Stored `DecisionContext` in `RankingContext`; replaced `is_suppressed()` with `evaluate_suppression()` from `goal_policy.rs`
  - Replaced `context.danger_class()` calls in `priority_class()` with `context.decision_context.danger_class`
  - Deleted dead code: `is_suppressed()`, `self_care_high_or_above()`, `danger_high_or_above()`, `danger_class()`, `max_self_care_class()`
  - Updated call sites in `agent_tick.rs` and `goal_explanation.rs`
  - Added test helper `rank()` in ranking tests to avoid updating 20+ test call signatures individually
- **Deviations**: Kept `danger_pressure` field on `RankingContext` (used by `heal_motive_score`, not just `danger_class`). Added `build_decision_context()` helper not mentioned in ticket — needed for tests and call sites to construct `DecisionContext` from belief views.
- **Verification**: `cargo test --workspace` all pass, `cargo clippy --workspace` clean
