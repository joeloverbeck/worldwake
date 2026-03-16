# S02GOADECPOLUNI-001: Create goal_policy module with types, DecisionContext, and policy lookup

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-ai
**Deps**: None (foundational ticket for S02)

## Problem

Goal-family decision policy is currently split across `ranking.rs` (suppression) and `interrupts.rs` (penalty/free interrupt rules). This ticket creates the single authoritative policy surface that all subsequent tickets consume.

## Assumption Reassessment (2026-03-16)

1. `GoalKind` enum has exactly 17 variants (ConsumeOwnedCommodity, AcquireCommodity, Sleep, Relieve, Wash, EngageHostile, ReduceDanger, Heal, ProduceCommodity, SellCommodity, RestockCommodity, MoveCargo, LootCorpse, BuryCorpse, ShareBelief, ClaimOffice, SupportCandidateForOffice) — confirmed in `crates/worldwake-core/src/goal.rs:15-63`.
2. `GoalPriorityClass` is defined in `crates/worldwake-ai/src/goal_model.rs` and has variants Background, Low, Medium, High, Critical with Ord derived — confirmed.
3. `InterruptTrigger` enum lives in `crates/worldwake-ai/src/interrupts.rs:17-24` with CriticalSurvival, CriticalDanger, HigherPriorityGoal, SuperiorSameClassPlan, PlanInvalid, OpportunisticLoot — confirmed.
4. `RankingContext` currently computes `max_self_care_class()` and `danger_class()` as private methods — confirmed in `ranking.rs:66-99`.
5. `CommodityPurpose::SelfConsume` is the discriminator for self-care acquisition goals — confirmed in `ranking.rs:119-123` and `interrupts.rs:236-243`.

## Architecture Check

1. Creating a standalone `goal_policy.rs` module gives both ranking and interrupts a single import point. Policy types are co-located with the lookup function, making it impossible to add a goal family without declaring its policy.
2. No backwards-compatibility shims. This ticket creates new types only; old code is not touched until tickets 002–005.

## What to Change

### 1. Create `crates/worldwake-ai/src/goal_policy.rs`

Define:
- `DecisionContext` struct with `max_self_care_class: GoalPriorityClass` and `danger_class: GoalPriorityClass`, plus `is_stressed_at_or_above()` method.
- `SuppressionRule` enum: `Never`, `WhenStressedAtOrAbove(GoalPriorityClass)`.
- `PenaltyInterruptEligibility` enum: `WhenCritical { trigger: InterruptTrigger }`, `Never`.
- `FreeInterruptRole` enum: `Reactive`, `Opportunistic`, `Normal`.
- `GoalFamilyPolicy` struct holding suppression, penalty_interrupt, free_interrupt.
- `GoalPolicyOutcome` enum: `Available`, `Suppressed { threshold, max_self_care, danger }`.
- `goal_family_policy(kind: &GoalKind) -> GoalFamilyPolicy` — exhaustive match over all 17 GoalKind variants + AcquireCommodity purpose discrimination.
- `evaluate_suppression(kind: &GoalKind, context: &DecisionContext) -> GoalPolicyOutcome`.

### 2. Register module in `crates/worldwake-ai/src/lib.rs`

Add `pub mod goal_policy;` and re-export key types: `DecisionContext`, `GoalFamilyPolicy`, `GoalPolicyOutcome`, `goal_family_policy`, `evaluate_suppression`, `FreeInterruptRole`.

## Files to Touch

- `crates/worldwake-ai/src/goal_policy.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Modifying `ranking.rs` (ticket 002)
- Modifying `interrupts.rs` (tickets 003, 004)
- Modifying `agent_tick.rs` (ticket 005)
- Removing old functions (`is_suppressed`, `is_critical_survival_goal`, `is_reactive_goal`, `no_medium_or_above_self_care_or_danger`)
- Adding new GoalKind variants
- Changing `GoalPriorityClass`, `InterruptTrigger`, or any authoritative core types
- Any changes to `worldwake-core` or `worldwake-sim`

## Acceptance Criteria

### Tests That Must Pass

1. `goal_family_policy()` returns `SuppressionRule::WhenStressedAtOrAbove(High)` for LootCorpse, BuryCorpse, ShareBelief, ClaimOffice, SupportCandidateForOffice
2. `goal_family_policy()` returns `SuppressionRule::Never` for all self-care, danger, healing, enterprise, and combat goals
3. `goal_family_policy()` returns `PenaltyInterruptEligibility::WhenCritical { CriticalSurvival }` for ConsumeOwnedCommodity, AcquireCommodity(SelfConsume), Sleep, Relieve, Wash
4. `goal_family_policy()` returns `PenaltyInterruptEligibility::WhenCritical { CriticalDanger }` for ReduceDanger
5. `goal_family_policy()` returns `PenaltyInterruptEligibility::Never` for Heal, EngageHostile, enterprise, corpse, social, political goals
6. `goal_family_policy()` returns `FreeInterruptRole::Reactive` for self-care + ReduceDanger + Heal
7. `goal_family_policy()` returns `FreeInterruptRole::Opportunistic` for LootCorpse
8. `goal_family_policy()` returns `FreeInterruptRole::Normal` for BuryCorpse, ShareBelief, ClaimOffice, SupportCandidateForOffice, EngageHostile, enterprise goals
9. `evaluate_suppression()` returns `Suppressed` for LootCorpse when `max_self_care_class >= High`
10. `evaluate_suppression()` returns `Suppressed` for LootCorpse when `danger_class >= High`
11. `evaluate_suppression()` returns `Available` for LootCorpse when both classes are below High
12. `evaluate_suppression()` returns `Available` for self-care goals regardless of stress
13. `DecisionContext::is_stressed_at_or_above(Medium)` returns true when danger is Medium, false when both are Low
14. Existing suite: `cargo test -p worldwake-ai`
15. `cargo clippy --workspace`

### Invariants

1. Policy is keyed on `&GoalKind` (not `GoalKindTag`) to discriminate `AcquireCommodity` by purpose
2. The match in `goal_family_policy()` is exhaustive — adding a GoalKind variant forces a compile error
3. `DecisionContext` contains only shared pressure state, not interrupt-specific parameters
4. All types are deterministic (derive Eq, PartialEq, Debug, Copy/Clone as appropriate)
5. No new authoritative components or world state introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_policy.rs` — unit tests for all 17 goal families' policy declarations, suppression evaluation at various stress levels, DecisionContext threshold checks

### Commands

1. `cargo test -p worldwake-ai goal_policy`
2. `cargo test --workspace && cargo clippy --workspace`
