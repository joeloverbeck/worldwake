# S11WOULIFAUD-004: Recovery-aware AI priority boost for need goals

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI ranking logic in worldwake-ai
**Deps**: None (reads wound data already available via `GoalBeliefView::wounds()`)

## Problem

The combat system recovers clotted wounds only when `recovery_conditions_met()` is true (not in combat, hunger/thirst/fatigue all below `high` threshold). The AI ranking system has zero awareness of these conditions — wound recovery is an accidental side-effect of satisfying needs. An agent with clotted wounds and high hunger has no reason to prioritize eating over other High-priority goals, even though eating would unblock wound recovery.

## Assumption Reassessment (2026-03-21)

1. `RankingContext` (ranking.rs line 109) is private, has fields: `view`, `agent`, `current_tick`, `utility`, `needs`, `thresholds`, `danger_pressure`, `decision_context`. No `has_clotted_wounds` field.
2. `drive_priority()` (ranking.rs line 206) takes `context`, `pressure` closure, `band` closure. Returns `GoalPriorityClass`. Used for Sleep, Relieve, Wash. No `recovery_relevant` parameter.
3. `relevant_self_consume_factors()` (ranking.rs line 405) returns `Vec<(Permille, Permille, ThresholdBand)>`. Used for hunger/thirst commodity consumption goals. No 4th boolean element.
4. `GoalBeliefView::wounds()` returns `Vec<Wound>` with `bleed_rate_per_tick` data — sufficient to detect clotted wounds.
5. N/A — no heuristic removal.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. No mismatch. Spec matches codebase.

## Architecture Check

1. Adding `has_clotted_wounds` to `RankingContext` and a `recovery_relevant` parameter to `drive_priority()` is minimal and local. The boost logic (`High` → `Critical` when clotted wounds + need at high) is a small conditional. No new types, no new traits, no new goal kinds.
2. No backwards-compatibility shims. `drive_priority()` is private, so the signature change is internal.

## Verification Layers

1. Clotted wound + hunger at high → Critical priority → focused unit test
2. Bleeding wound → no boost → focused unit test
3. Clotted wound + hunger below high → no boost → focused unit test
4. Clotted wound + sleep at high → Critical → focused unit test
5. Relieve/Wash → never boosted → focused unit test
6. No wounds → no boost → focused unit test
7. Already Critical → stays Critical → focused unit test
8. Single-layer ticket (AI ranking). No authoritative action/event layers affected.

## What to Change

### 1. Add `has_clotted_wounds` helper function

```rust
fn has_clotted_wounds(view: &dyn GoalBeliefView, agent: EntityId) -> bool {
    view.wounds(agent).iter().any(|w| w.bleed_rate_per_tick.value() == 0 && w.severity.value() > 0)
}
```

### 2. Add `has_clotted_wounds: bool` field to `RankingContext`

Compute in `RankingContext::new()` (or wherever the context is constructed) by calling the helper.

### 3. Add `recovery_relevant: bool` parameter to `drive_priority()`

```rust
fn drive_priority(
    context: &RankingContext<'_>,
    pressure: impl Fn(HomeostaticNeeds) -> Permille,
    band: impl Fn(DriveThresholds) -> ThresholdBand,
    recovery_relevant: bool,
) -> GoalPriorityClass {
    let base = match (context.needs, context.thresholds) {
        (Some(needs), Some(thresholds)) => classify_band(pressure(needs), &band(thresholds)),
        _ => GoalPriorityClass::Background,
    };
    if recovery_relevant && context.has_clotted_wounds && base == GoalPriorityClass::High {
        GoalPriorityClass::Critical
    } else {
        base
    }
}
```

### 4. Update `drive_priority()` call sites in `priority_class()`

- `GoalKind::Sleep` → `recovery_relevant: true`
- `GoalKind::Relieve` → `recovery_relevant: false`
- `GoalKind::Wash` → `recovery_relevant: false`

### 5. Update `relevant_self_consume_factors()` return type

Change return type to `Vec<(Permille, Permille, ThresholdBand, bool)>` where the 4th element is `recovery_relevant`:
- Hunger factors (food commodities) → `true`
- Thirst factors (water commodities) → `true`

### 6. Update `self_consume_priority()` to apply boost

When the 4th element is `true`, `context.has_clotted_wounds` is true, and base class is `High`, boost to `Critical`.

### 7. Add cross-reference comment

Add a comment near the boost logic referencing `recovery_conditions_met()` in `crates/worldwake-systems/src/combat.rs` to document the coupling.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)

## Out of Scope

- Changing `recovery_conditions_met()` in combat.rs
- Changing `UtilityProfile` or adding personality parameters
- Changing `GoalBeliefView` trait
- Adding new `GoalKind` variants
- Wound progression or pruning logic
- Golden test hash recapture (done in S11WOULIFAUD-005)
- Any changes to candidate_generation.rs or search.rs

## Acceptance Criteria

### Tests That Must Pass

1. `clotted_wound_boosts_hunger_high_to_critical` — agent with clotted wound, hunger at high → eat goal priority is Critical
2. `bleeding_wound_no_boost` — agent with actively bleeding wound, hunger at high → priority stays High
3. `clotted_wound_no_boost_below_high` — agent with clotted wound, hunger below high → no boost
4. `clotted_wound_boosts_sleep_high_to_critical` — agent with clotted wound, fatigue at high → sleep goal priority is Critical
5. `clotted_wound_no_boost_relieve_or_wash` — agent with clotted wound, bladder/dirtiness at high → priority stays High
6. `no_wounds_no_boost` — agent with no wounds, hunger at high → priority stays High
7. `critical_stays_critical` — agent with clotted wound, hunger at critical → priority stays Critical (no double-boost)
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The boost applies only when: (a) agent has clotted wounds, (b) need is recovery-relevant, (c) base priority is exactly `High`
2. `Critical` is the maximum boost — no further elevation
3. No new fields on `UtilityProfile` or `GoalBeliefView`
4. The coupling with `recovery_conditions_met()` is documented via comment

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs::tests::clotted_wound_boosts_hunger_high_to_critical`
2. `crates/worldwake-ai/src/ranking.rs::tests::bleeding_wound_no_boost`
3. `crates/worldwake-ai/src/ranking.rs::tests::clotted_wound_no_boost_below_high`
4. `crates/worldwake-ai/src/ranking.rs::tests::clotted_wound_boosts_sleep_high_to_critical`
5. `crates/worldwake-ai/src/ranking.rs::tests::clotted_wound_no_boost_relieve_or_wash`
6. `crates/worldwake-ai/src/ranking.rs::tests::no_wounds_no_boost`
7. `crates/worldwake-ai/src/ranking.rs::tests::critical_stays_critical`

### Commands

1. `cargo test -p worldwake-ai -- clotted_wound bleeding_wound no_wounds critical_stays`
2. `cargo clippy -p worldwake-ai`
3. `cargo test -p worldwake-ai`
