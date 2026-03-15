# E15BSOCAIGOA-001: Add GoalKind::ShareBelief and GoalKindTag::ShareBelief

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — goal identity types in core and ai
**Deps**: E15 (completed)

## Problem

The AI planner has no goal type for social information sharing. Without `GoalKind::ShareBelief`, agents cannot autonomously initiate Tell actions. This is the foundational type that all other E15b tickets depend on.

## Assumption Reassessment (2026-03-15)

1. `GoalKind` in `crates/worldwake-core/src/goal.rs` has 14 variants (ConsumeOwnedCommodity through BuryCorpse). Confirmed no ShareBelief exists.
2. `GoalKindTag` in `crates/worldwake-ai/src/goal_model.rs` has 13 matching tag variants. Confirmed no ShareBelief tag exists.
3. `GoalKey` extraction logic exists — must add ShareBelief arm following BuryCorpse precedent (entity: Some(listener), place: Some(subject)).
4. `GoalKindPlannerExt` trait impl in `goal_model.rs` has match arms for all 14 GoalKind variants — must add ShareBelief arm.

## Architecture Check

1. Follows existing GoalKind/GoalKindTag mirroring pattern exactly. ShareBelief { listener, subject } parallels BuryCorpse { corpse, burial_site } in using two EntityId fields.
2. No shims — pure addition of a new variant to existing enums.

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
- `relevant_op_kinds()` → `&[PlannerOpKind::Tell]` (added in E15BSOCAIGOA-002)
- `relevant_observed_commodities()` → `None` (no commodity involved)
- `build_payload_override()` → `Ok(None)` (Tell payload built by action handler)

Note: This arm references `PlannerOpKind::Tell` from E15BSOCAIGOA-002. If implementing in parallel, use a temporary `todo!()` for `relevant_op_kinds` and complete when E15BSOCAIGOA-002 lands.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)

## Out of Scope

- PlannerOpKind::Tell (E15BSOCAIGOA-002)
- social_weight in UtilityProfile (E15BSOCAIGOA-003)
- Candidate generation (E15BSOCAIGOA-004)
- Ranking logic (E15BSOCAIGOA-005)
- Golden test harness extensions (E15BSOCAIGOA-006)
- All golden tests (E15BSOCAIGOA-007 through E15BSOCAIGOA-010)
- GoalKind::InvestigateMismatch (future spec, not E15b)

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::ShareBelief` constructs with two EntityIds and produces correct GoalKey (entity=listener, place=subject)
2. `GoalKindTag::ShareBelief` round-trips through serde
3. Two ShareBelief goals with different listeners produce different GoalKeys (deduplication correctness)
4. Two ShareBelief goals with same listener but different subjects produce different GoalKeys
5. Existing suite: `cargo test -p worldwake-core` — no regressions
6. Existing suite: `cargo test -p worldwake-ai` — no regressions (exhaustive match arms compile)

### Invariants

1. All existing GoalKind match arms remain exhaustive — adding a variant must update every match site or the build fails
2. GoalKey uniqueness: no two semantically different goals may produce the same GoalKey
3. GoalKindTag and GoalKind remain in 1:1 correspondence

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` (inline tests) — ShareBelief GoalKey extraction correctness
2. `crates/worldwake-ai/src/goal_model.rs` (inline tests) — GoalKindTag::ShareBelief maps correctly

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
