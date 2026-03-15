# E15BSOCAIGOA-005: Add ShareBelief ranking: suppression, priority class, and motive score

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — ranking logic in ai crate
**Deps**: E15BSOCAIGOA-001, E15BSOCAIGOA-003

## Problem

Even with ShareBelief candidates generated, the ranking system still uses placeholder scoring for them. The codebase already has suppression and baseline priority handling for `ShareBelief`, so the real remaining problem is that motive scoring does not yet reflect per-agent social motivation or belief freshness. Without that, autonomous social behavior will be mechanically possible but poorly differentiated.

## Assumption Reassessment (2026-03-15)

1. `is_suppressed()` in `crates/worldwake-ai/src/ranking.rs` already suppresses `GoalKind::ShareBelief { .. }` when `danger_high_or_above() || self_care_high_or_above()`.
2. `priority_class()` already assigns `GoalPriorityClass::Low` to `GoalKind::ShareBelief { .. }`.
3. `motive_score()` still uses a placeholder constant (`1`) for `GoalKind::ShareBelief { .. }`; this is the real unfinished ranking work.
4. `UtilityProfile` now has `social_weight: Permille` from E15BSOCAIGOA-003.
5. Social pressure should be a derived computation from belief-store state (source type, observed_tick, count). Never stored.

## Architecture Check

1. Existing suppression and low-priority treatment are architecturally sound; this ticket should not duplicate or rewrite them unless tests prove they are wrong.
2. Social pressure derivation is the new work and should follow existing motive-score patterns (`weight * pressure`) rather than introducing a parallel ranking system.
3. No new stored state — pressure is computed at ranking time from existing belief store data (Principle 3).

## Note

This ticket now covers the second half of the remaining autonomous-social-behavior gap: once `E15BSOCAIGOA-004` starts emitting `ShareBelief` candidates, this ticket should replace the placeholder motive score so socially motivated agents actually differ from low-social agents in robust, explainable ways.

## What to Change

### 1. Preserve existing suppression logic

Keep `ShareBelief` suppressed when `danger_high_or_above() || self_care_high_or_above()`.
This behavior already exists and should be retained and covered by tests rather than re-added blindly.

### 2. Preserve baseline ShareBelief priority class

Keep `GoalKind::ShareBelief { .. }` at `GoalPriorityClass::Low`.
The current architecture should continue to differentiate social behavior via motive score within the low-priority band rather than promoting social goals into survival/enterprise bands.

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

### 4. Replace placeholder ShareBelief motive score

In `motive_score()`, replace the current placeholder arm:
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

1. Existing suppression behavior for ShareBelief remains covered and passing
2. Existing `GoalPriorityClass::Low` behavior for ShareBelief remains covered and passing
3. ShareBelief motive score increases with higher social_weight
4. ShareBelief motive score increases with more fresh beliefs (higher social_pressure)
5. Agent with social_weight=0 produces motive score of 0 for ShareBelief
6. ShareBelief ranked below ConsumeOwnedCommodity at Critical/High priority (survival first)
7. ShareBelief ranked below enterprise goals at equal motive score (Medium > Low)
8. Existing suite: `cargo test -p worldwake-ai` — no regressions

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
