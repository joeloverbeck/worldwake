# E16CINSBELRECCON-011: GoalKindTag::ConsultRecord + PlannerOpKind::ConsultRecord + S12 Integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new goal kind, planner op, prerequisite integration
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-005, E16CINSBELRECCON-010

## Problem

For agents to autonomously seek out institutional knowledge, the AI must have a `ConsultRecord` goal kind and planner operation. The GOAP search must be able to plan multi-step sequences like "Travel to record place → ConsultRecord → Travel back → PoliticalAction". S12's prerequisite-aware planning must integrate with this so `prerequisite_places()` returns the record's home place when institutional beliefs are Unknown.

## Assumption Reassessment (2026-03-21)

1. `GoalKind` in `goal.rs` currently has variants through `SupportCandidateForOffice`. No `ConsultRecord` variant exists.
2. `GoalKindTag` in `goal_model.rs` currently has tags through `SupportCandidateForOffice` (line 22+). No `ConsultRecord` tag.
3. `PlannerOpKind` in `planner_ops.rs` currently has ops through `DeclareSupport` (line 13+). No `ConsultRecord` op.
4. `classify_action_def()` in `planner_ops.rs` maps `(ActionDomain, name, payload)` triples to `PlannerOpKind`. Must add `(ActionDomain::Social, "consult_record", _) => Some(PlannerOpKind::ConsultRecord)`.
5. `semantics_for()` must define `ConsultRecord` semantics: `may_appear_mid_plan: true` (it serves as a prerequisite step), relevant goal kinds: `[GoalKindTag::ConsultRecord]`.
6. S12 `prerequisite_places()` integration: when a political goal needs institutional belief and the belief is `Unknown`, return the record's home place.
7. N/A — no heuristic removal.
8. N/A.
9. N/A.
10. N/A.
11. No mismatch.
12. N/A.

## Architecture Check

1. Follows the existing pattern: GoalKind variant → GoalKindTag → GoalKindPlannerExt → PlannerOpKind → semantics. No novel patterns.
2. S12 prerequisite integration follows the existing `prerequisite_places()` pattern.
3. No backward-compatibility shims.

## Verification Layers

1. `GoalKind::ConsultRecord` → `GoalKindTag::ConsultRecord` mapping → unit test
2. `classify_action_def` recognizes consult_record → unit test
3. `ConsultRecord` semantics has `may_appear_mid_plan: true` → semantics test
4. `prerequisite_places()` returns record place for Unknown institutional belief → unit test
5. Plan search produces Travel → ConsultRecord → Travel → PoliticalAction → integration test

## What to Change

### 1. Add `GoalKind::ConsultRecord` in `goal.rs`

```rust
ConsultRecord { record: EntityId },
```

Update `GoalKey::from` implementation. Add to any exhaustive match blocks.

### 2. Add `GoalKindTag::ConsultRecord` in `goal_model.rs`

Add the tag variant. Implement `GoalKindPlannerExt` for the new goal kind:
- `goal_kind_tag()` returns `GoalKindTag::ConsultRecord`
- `goal_priority_class()` — institutional consultation should be medium priority (below survival, above idle)
- Implement grounding and terminal check methods

### 3. Add `PlannerOpKind::ConsultRecord` in `planner_ops.rs`

Add variant to `PlannerOpKind`. Add classification rule in `classify_action_def()`:
```rust
(ActionDomain::Social, "consult_record", _) => Some(PlannerOpKind::ConsultRecord),
```

Define semantics in `semantics_for()`:
- `may_appear_mid_plan: true`
- `relevant_goal_kinds: &[GoalKindTag::ConsultRecord]`
- barriers: record not at current place (Travel prerequisite)

### 4. S12 prerequisite integration

Extend `prerequisite_places()` to return the record's home place when:
- The current goal requires institutional belief knowledge
- The actor's belief for the relevant key is `Unknown`
- A record of the appropriate kind is known to exist at a remote place

### 5. Search integration

Ensure `search_plan()` can produce multi-step plans: Travel(to record place) → ConsultRecord → Travel(back) → original goal action.

Hypothetical ConsultRecord transition in search must call `PlanningState::override_institutional_belief()` to change `Unknown` → `Certain` so the subsequent goal step becomes viable.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — add `ConsultRecord` variant to `GoalKind`, update `GoalKey`)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add `GoalKindTag::ConsultRecord`, implement `GoalKindPlannerExt`)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — add `PlannerOpKind::ConsultRecord`, classification, semantics)
- `crates/worldwake-ai/src/search.rs` (modify — hypothetical ConsultRecord transition, S12 prerequisite integration)

## Out of Scope

- Candidate generation emitting ConsultRecord goals (ticket -012)
- Ranking changes for ConsultRecord (ticket -013)
- Failure handling for stale/conflicted beliefs (ticket -013)
- Live helper seam removal (ticket -014)
- ConsultRecord action def/handler (ticket -005 — must already exist)

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::ConsultRecord` maps to `GoalKindTag::ConsultRecord`
2. `classify_action_def` recognizes `consult_record` action as `PlannerOpKind::ConsultRecord`
3. `ConsultRecord` semantics has `may_appear_mid_plan: true`
4. Plan search can produce Travel → ConsultRecord → PoliticalAction sequence
5. Hypothetical ConsultRecord in search transitions Unknown → Certain for the relevant belief
6. `prerequisite_places()` returns record home place when institutional belief is Unknown
7. `GoalKind::ConsultRecord` roundtrips through bincode
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. ConsultRecord is a mid-plan prerequisite operation, not a terminal goal
2. Plan search does not produce ConsultRecord when belief is already Certain
3. S12 prerequisite chains are finite (no infinite Travel → Consult loops)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — GoalKind::ConsultRecord roundtrip and GoalKey
2. `crates/worldwake-ai/src/goal_model.rs` — tag mapping, priority class
3. `crates/worldwake-ai/src/planner_ops.rs` — classification, semantics
4. `crates/worldwake-ai/src/search.rs` — multi-step plan with ConsultRecord prerequisite

### Commands

1. `cargo test -p worldwake-ai goal_model`
2. `cargo test -p worldwake-ai planner_ops`
3. `cargo test -p worldwake-ai search`
4. `cargo clippy --workspace && cargo test --workspace`
