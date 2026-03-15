# E15BSOCAIGOA-005: Replace placeholder ShareBelief motive scoring with subject-specific derived ranking

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — ranking logic in ai crate
**Deps**: E15BSOCAIGOA-001, E15BSOCAIGOA-003

## Problem

The social-goal stack is further along than this ticket originally assumed: `GoalKind::ShareBelief`, `GoalKindTag::ShareBelief`, `PlannerOpKind::Tell`, `social_weight`, and autonomous social candidate generation are already implemented. The remaining gap is narrower and more architectural: `motive_score()` still assigns the placeholder constant `1` to every `ShareBelief` goal, so socially motivated agents do not differ from low-social agents and all tellable subjects rank identically.

## Assumption Reassessment (2026-03-15)

1. `is_suppressed()` in `crates/worldwake-ai/src/ranking.rs` already suppresses `GoalKind::ShareBelief { .. }` when `danger_high_or_above() || self_care_high_or_above()`.
2. `priority_class()` already assigns `GoalPriorityClass::Low` to `GoalKind::ShareBelief { .. }`.
3. `motive_score()` still uses a placeholder constant (`1`) for `GoalKind::ShareBelief { .. }`; this is the real unfinished ranking work.
4. `UtilityProfile` already has `social_weight: Permille`; this is no longer a dependency to implement.
5. `GoalBeliefView` already exposes `known_entity_beliefs(agent)`, which is sufficient to derive social motive from the speaker's current belief state without adding new stored state.
6. Candidate generation already emits `ShareBelief { listener, subject }` goals and already has focused tests in `crates/worldwake-ai/src/candidate_generation.rs`; this ticket should not re-solve candidate generation.
7. The best architectural fit is per-subject scoring, not aggregate whole-store pressure: ranking operates on one concrete `ShareBelief { listener, subject }` candidate at a time, so its motive should derive from the exact believed subject being shared.
8. `rank_candidates()` currently does not receive `Tick`, so freshness-aware scoring requires small local tick plumbing from existing callers (`agent_tick` and `goal_explanation`) into ranking.

## Architecture Check

1. Existing suppression and low-priority treatment are architecturally sound; this ticket should preserve them and add regression coverage rather than reworking ranking bands.
2. The placeholder constant should be replaced by the same ranking shape used elsewhere: `weight * derived_pressure`.
3. The derived pressure should be subject-specific. Aggregating all fresh beliefs into one store-wide pressure would flatten every `ShareBelief` candidate to the same motive score and make per-subject ranking less explainable.
4. Reuse existing belief provenance/staleness semantics where possible. `belief_confidence(...)` already encodes source quality plus age decay; duplicating a second freshness model in ranking would create policy drift.
5. No new stored state or compatibility layer. Ranking should derive its result directly from existing `BelievedEntityState` data (Principles 3 and 26).

## Note

This ticket now covers the ranking half of the remaining autonomous-social-behavior gap: `E15BSOCAIGOA-004` already emits `ShareBelief` candidates, and this ticket should replace the placeholder motive score so socially motivated agents actually differ from low-social agents in robust, explainable ways.

## What to Change

### 1. Preserve existing suppression logic

Keep `ShareBelief` suppressed when `danger_high_or_above() || self_care_high_or_above()`.
This behavior already exists and should be retained and covered by tests rather than re-added blindly.

### 2. Preserve baseline ShareBelief priority class

Keep `GoalKind::ShareBelief { .. }` at `GoalPriorityClass::Low`.
The current architecture should continue to differentiate social behavior via motive score within the low-priority band rather than promoting social goals into survival/enterprise bands.

### 3. Add subject-specific social pressure derivation

New helper function (private):
```rust
fn social_pressure_for_subject(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    subject: EntityId,
    current_tick: Tick,
) -> Permille {
    // Read the BelievedEntityState for `subject` from known_entity_beliefs(agent)
    // Derive pressure from the existing provenance + staleness model
    // Return zero when the subject belief is absent or fully stale
    // This is DERIVED, never stored (Principle 3)
}
```

Prefer reusing `belief_confidence(...)` over introducing a parallel "retention window freshness" formula inside ranking.
Passing `current_tick` into ranking is in scope if needed to keep staleness derivation honest.

### 4. Replace placeholder ShareBelief motive score

In `motive_score()`, replace the current placeholder arm:
```rust
GoalKind::ShareBelief { subject, .. } => {
    score_product(
        utility.social_weight,
        social_pressure_for_subject(view, agent, subject, current_tick),
    )
}
```

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (pass `Tick` through existing read-phase context)
- `crates/worldwake-ai/src/goal_explanation.rs` (pass `current_tick` through existing API)

## Out of Scope

- GoalKind::ShareBelief definition (already implemented)
- social_weight addition to UtilityProfile (already implemented)
- Candidate generation (already implemented in E15BSOCAIGOA-004 scope)
- PlannerOpKind::Tell (already implemented)
- Promotion to Medium priority (current architecture keeps social behavior in the Low band; this ticket should not reopen that policy)
- GoalKind::InvestigateMismatch ranking (future spec)
- Shared relay-subject extraction refactor (E15BSOCAIGOA-011)

## Acceptance Criteria

### Tests That Must Pass

1. Existing suppression behavior for ShareBelief remains covered and passing.
2. Existing `GoalPriorityClass::Low` behavior for ShareBelief remains covered and passing.
3. ShareBelief motive score increases with higher social_weight
4. ShareBelief motive score is higher for fresher / higher-confidence subject beliefs than for stale / low-confidence subject beliefs.
5. Agent with `social_weight = 0` produces motive score `0` for ShareBelief.
6. Missing or fully stale subject belief yields motive score `0` rather than a fallback constant.
7. ShareBelief ranked below `ConsumeOwnedCommodity` at Critical/High priority (survival first).
8. ShareBelief ranked below enterprise goals at equal motive score (`Medium > Low`).
9. Existing suite: `cargo test -p worldwake-ai` — no regressions

### Invariants

1. ShareBelief can never outrank Critical or High priority goals (survival, combat, healing)
2. social pressure is a pure derived computation — never stored as component state (Principle 3)
3. Ranking determinism preserved — no HashMap, no floats, no nondeterministic iteration
4. ShareBelief ranking derives from the concrete belief being shared, not from an abstract store-wide gossip score (Principle 3)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (module tests) — suppression, priority class, motive score for ShareBelief

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Replaced the placeholder `ShareBelief` motive score with subject-specific derived scoring in `crates/worldwake-ai/src/ranking.rs`.
  - Reused existing belief provenance/staleness semantics through `belief_confidence(...)` instead of inventing a parallel social freshness formula.
  - Threaded `Tick` into `rank_candidates()` from `agent_tick` and `goal_explanation` so freshness is derived from real current time.
  - Added regression tests covering ShareBelief suppression, Low priority preservation, social-weight sensitivity, subject-confidence sensitivity, zero-score cases, and ordering beneath enterprise/self-care goals.
- Deviations from original plan:
  - The ticket was corrected before implementation because `ShareBelief`, `PlannerOpKind::Tell`, `social_weight`, and social candidate generation were already present.
  - The implemented scoring is per-subject rather than aggregate whole-store social pressure, because ranking already operates on concrete `ShareBelief { listener, subject }` candidates.
  - The implementation reuses the canonical default belief-confidence policy; it does not yet thread per-agent perception confidence policy into ranking.
- Verification results:
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
