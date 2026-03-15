# E15BSOCAIGOA-005: Add ShareBelief ranking: suppression, priority class, and motive score

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — ranking logic in ai crate
**Deps**: E15BSOCAIGOA-001, E15BSOCAIGOA-003

## Problem

Even with ShareBelief candidates generated, the ranking system has no logic for scoring them. Without suppression rules, priority class assignment, and motive score calculation, ShareBelief goals would either crash (unmatched arm) or receive nonsensical priority.

## Assumption Reassessment (2026-03-15)

1. `is_suppressed()` in `crates/worldwake-ai/src/ranking.rs` currently suppresses LootCorpse and BuryCorpse when `danger_high_or_above() || self_care_high_or_above()`. ShareBelief follows same pattern.
2. `priority_class()` assigns GoalPriorityClass to each GoalKindTag. LootCorpse/BuryCorpse get `Low`. ShareBelief gets `Low` baseline.
3. `motive_score()` calculates per-goal motivation. For ShareBelief: `social_weight * social_pressure()`.
4. `UtilityProfile` will have `social_weight: Permille` after E15BSOCAIGOA-003.
5. Social pressure must be a derived computation from belief store state (source type, observed_tick, count). Never stored.

## Architecture Check

1. Follows exact pattern of LootCorpse/BuryCorpse for suppression and priority class.
2. Social pressure derivation is new but follows motive score patterns (weight * pressure product).
3. No new stored state — pressure is computed at ranking time from existing belief store data (Principle 3).

## What to Change

### 1. Add ShareBelief to suppression logic

In `is_suppressed()`, add `GoalKindTag::ShareBelief` to the same branch as LootCorpse/BuryCorpse:
- Suppressed when `danger_high_or_above() || self_care_high_or_above()`
- Rationale: starving agents do not gossip

### 2. Add ShareBelief priority class

In `priority_class()`, add:
- `GoalKindTag::ShareBelief` → `GoalPriorityClass::Low`
- Note: spec says "promotable to Medium via social_weight scaling" but existing ranking compares priority class then motive score — a higher motive score within Low is sufficient differentiation without promoting to Medium

### 3. Add social_pressure() derived computation

New helper function (private):
```rust
fn social_pressure(view: &dyn GoalBeliefView, agent: EntityId, current_tick: Tick) -> Permille {
    // Count beliefs with fresh PerceptionSource (DirectObservation or Report with low chain_len)
    // Apply recency factor: (retention_window - age) / retention_window as rough freshness
    // Return aggregate pressure as Permille
    // This is DERIVED, never stored (Principle 3)
}
```

### 4. Add ShareBelief motive score

In `motive_score()`, add arm:
```rust
GoalKindTag::ShareBelief => score_product(utility.social_weight, social_pressure(view, agent, current_tick))
```

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)

## Out of Scope

- GoalKind::ShareBelief definition (E15BSOCAIGOA-001)
- social_weight addition to UtilityProfile (E15BSOCAIGOA-003)
- Candidate generation (E15BSOCAIGOA-004)
- PlannerOpKind::Tell (E15BSOCAIGOA-002)
- Promotion to Medium priority (spec mentions possibility but Low with high motive score is sufficient)
- GoalKind::InvestigateMismatch ranking (future spec)

## Acceptance Criteria

### Tests That Must Pass

1. ShareBelief suppressed when danger >= High threshold
2. ShareBelief suppressed when any self-care need >= High threshold
3. ShareBelief NOT suppressed when all needs and danger are below High
4. ShareBelief gets `GoalPriorityClass::Low` (never Critical or High)
5. ShareBelief motive score increases with higher social_weight
6. ShareBelief motive score increases with more fresh beliefs (higher social_pressure)
7. Agent with social_weight=0 produces motive score of 0 for ShareBelief
8. ShareBelief ranked below ConsumeOwnedCommodity at Critical/High priority (survival first)
9. ShareBelief ranked below enterprise goals at equal motive score (Medium > Low)
10. Existing suite: `cargo test -p worldwake-ai` — no regressions

### Invariants

1. ShareBelief can never outrank Critical or High priority goals (survival, combat, healing)
2. social_pressure is a pure derived computation — never stored as component state (Principle 3)
3. Ranking determinism preserved — no HashMap, no floats, no nondeterministic iteration

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (module tests) — suppression, priority class, motive score for ShareBelief

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
