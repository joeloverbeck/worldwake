# S43COMTYPDIF-003: Class-aware suppression and ranking boost in worldwake-ai

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — goal policy suppression differentiated, ranking motive multiplied
**Deps**: S43COMTYPDIF-002

## Problem

All ShareBelief goals are suppressed uniformly at `GoalPriorityClass::High` and ranked with uniform `social_weight × social_pressure`. A panicked alarm about a witnessed death is suppressed under the same stress threshold as idle gossip about co-presence, and has the same motive weight. Agents under survival stress cannot relay urgent safety information, and alarms compete equally with gossip for attention.

## Assumption Reassessment (2026-04-03)

1. `GoalFamilyPolicy` for ShareBelief at `goal_policy.rs:172-180` — currently grouped with BuryCorpse, RegroupWithFaction, etc. under `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High)`. Confirmed.
2. ShareBelief ranking at `ranking.rs:608-610`: `score_product(context.utility.social_weight, social_pressure_for_topic(context, topic))`. Confirmed.
3. `score_product` at `ranking.rs` — multiplies two Permille values: `(a.value() as u32 * b.value() as u32) / 1000`. Returns `u32`.
4. `SuppressionRule::Never` variant exists at `goal_policy.rs:37`. Confirmed.
5. `GoalPriorityClass::Critical` variant exists at `goal_model.rs:1623`. Confirmed.
6. After ticket 002, `GoalKind::ShareBelief` will carry `communication_class: CommunicationClass`, readable in both policy and ranking match arms.
7. This ticket does not touch the authoritative Tell handler or TellProfile — those are ticket 004. The separation is clean: this ticket changes AI-layer behavior (what the agent decides to do), ticket 004 changes authoritative-layer behavior (what happens when the Tell commits).

## Architecture Check

1. The suppression change splits one match arm into three class-specific arms within the same function. No new functions, no new abstractions — just a finer-grained match. This is cleaner than introducing a per-class policy lookup table.
2. The ranking boost is a single multiplier applied to the existing `social_pressure_for_topic` result before `score_product`. Uses saturating arithmetic to avoid overflow. No new ranking infrastructure.
3. No backwards-compatibility shims. The old uniform suppression/ranking is replaced, not wrapped.

## Verification Layers

1. Alarm-class suppression = Never -> decision trace: ShareBelief with Alarm class survives stress suppression
2. Testimony-class suppression = Critical -> decision trace: ShareBelief with Testimony class suppressed at Critical, not at High
3. Gossip-class suppression = High -> decision trace: preserves current behavior (existing tests)
4. Alarm ranking boost -> focused unit test: alarm-class motive > gossip-class motive given equal social_weight and social_pressure
5. Single-crate (worldwake-ai) changes — no cross-system verification needed

## What to Change

### 1. Split ShareBelief suppression in goal_policy.rs

In `goal_family_policy()`, break the ShareBelief arm out of the grouped match:

```rust
GoalKind::ShareBelief { communication_class, .. } => match communication_class {
    CommunicationClass::Alarm => GoalFamilyPolicy {
        suppression: SuppressionRule::Never,
        penalty_interrupt: PenaltyInterruptEligibility::Never,
        free_interrupt: FreeInterruptRole::Normal,
    },
    CommunicationClass::Testimony => GoalFamilyPolicy {
        suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Critical),
        penalty_interrupt: PenaltyInterruptEligibility::Never,
        free_interrupt: FreeInterruptRole::Normal,
    },
    CommunicationClass::Gossip => GoalFamilyPolicy {
        suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High),
        penalty_interrupt: PenaltyInterruptEligibility::Never,
        free_interrupt: FreeInterruptRole::Normal,
    },
},
```

Update the `GoalKind::BuryCorpse | ... | GoalKind::ShareBelief { .. }` grouped arm to remove `ShareBelief`.

### 2. Add alarm ranking multiplier in ranking.rs

In `compute_raw_motive()`, the ShareBelief arm currently is:

```rust
GoalKind::ShareBelief { topic, .. } => score_product(
    context.utility.social_weight,
    social_pressure_for_topic(context, topic),
),
```

Change to:

```rust
GoalKind::ShareBelief { topic, communication_class, .. } => {
    let pressure = social_pressure_for_topic(context, topic);
    let boosted = match communication_class {
        CommunicationClass::Alarm => pressure.saturating_add(pressure).saturating_add(pressure), // ×3 via saturating
        _ => pressure,
    };
    score_product(context.utility.social_weight, boosted)
}
```

The ×3 boost uses `Permille::saturating_add` which caps at 1000, preventing overflow.

### 3. Update goal_policy test

The existing `test_all_goal_kinds_have_policy` test in `goal_policy.rs` constructs dummy ShareBelief goals. After ticket 002, these already have the `communication_class` field. Add test assertions for the three class-specific policies.

## Files to Touch

- `crates/worldwake-ai/src/goal_policy.rs` (modify) — split ShareBelief suppression
- `crates/worldwake-ai/src/ranking.rs` (modify) — add alarm ranking boost

## Out of Scope

- Tell handler acceptance changes (ticket 004)
- TellProfile.acceptance_fidelity removal (ticket 004)
- Golden test scenarios (ticket 005)
- Changes to candidate generation (already done in ticket 002)

## Acceptance Criteria

### Tests That Must Pass

1. `test_all_goal_kinds_have_policy` still passes (updated for three-class suppression)
2. New test: Alarm-class ShareBelief policy returns `SuppressionRule::Never`
3. New test: Gossip-class ShareBelief policy returns `WhenStressedAtOrAbove(High)` (regression — current behavior preserved)
4. New test: Alarm-class motive score > Gossip-class motive score given equal social_weight and social_pressure inputs
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Gossip-class ShareBelief suppression is identical to pre-S43 behavior (`High`)
2. Alarm ranking boost uses saturating arithmetic — no `u32` overflow possible
3. `penalty_interrupt` and `free_interrupt` unchanged for all ShareBelief classes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_policy.rs` — extend existing policy test with class-specific assertions
2. `crates/worldwake-ai/src/ranking.rs` — new unit test for alarm motive multiplier

### Commands

1. `cargo test -p worldwake-ai -- goal_policy`
2. `cargo test -p worldwake-ai -- ranking`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
