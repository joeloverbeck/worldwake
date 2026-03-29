# E18BANDYN-007: Planner ops and search integration for raid and regroup goals

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai (planner_ops.rs, search.rs, ranking.rs, goal_switching.rs)
**Deps**: E18BANDYN-002 (GoalKind variants), E18BANDYN-006 (candidate generation)

## Problem

The GOAP planner needs to know how to construct plans for `RaidTarget` and `RegroupWithFaction` goals. This includes planner op semantics (barriers, mid-plan viability, goal relevance), priority class assignment for ranking, suppression rules, and goal-switching thresholds.

## Assumption Reassessment (2026-03-29)

1. `PlannerOpKind` in `crates/worldwake-ai/src/planner_ops.rs` already has `Travel` and `Combat` variants. `RegroupWithFaction` maps to `Travel`. `RaidTarget` maps to `Combat` (co-location attack).
2. `PlannerOpSemantics` in `crates/worldwake-ai/src/planner_ops.rs` defines barrier checks, mid-plan viability, and goal relevance per op kind. New goal kinds need entries in the semantics table.
3. `search_plan()` in `crates/worldwake-ai/src/search.rs` performs GOAP-style best-first search. It uses `PlannerOpSemantics` to determine valid expansions. The search needs to handle:
   - `RaidTarget`: terminal when actor is at target's place and can attack. Single-step if co-located, or Travel + Raid if not.
   - `RegroupWithFaction`: terminal when actor arrives at rally place. Plan is: Travel to rally place.
4. `RankedGoal` in `crates/worldwake-ai/src/ranking.rs` assigns priority classes and motive scores. Per spec:
   - `RegroupWithFaction`: below immediate survival (ReduceDanger, ConsumeCommodity) but above enterprise goals
   - `RaidTarget`: enterprise-level priority (analogous to merchant trade goals)
5. `GoalSwitchKind` and `compare_goal_switch()` in `crates/worldwake-ai/src/goal_switching.rs` determine when running action should be interrupted for a higher-priority goal. Raid goals should not interrupt survival goals. Survival goals should interrupt raid goals.
6. `InterruptDecision` and `evaluate_interrupt()` in `crates/worldwake-ai/src/interrupts.rs` handle in-progress action interruption. Existing interrupt logic should naturally handle new goal kinds if priority classes are correctly assigned.

## Architecture Check

1. Mapping new goal kinds to existing planner op kinds (`Travel`, `Combat`) is cleaner than creating new op kinds because: (a) the planner already knows how to search Travel and Combat plans, (b) the distinction between raid and attack is in candidate generation and priority, not in plan structure, (c) regroup is just "Travel to a specific place" from the planner's perspective.
2. Priority class assignment uses the existing ranking infrastructure — no new priority classes needed, just correct placement within the existing hierarchy.
3. No backwards-compatibility shims. Additive entries in semantics tables and ranking logic.

## Verification Layers

1. Planner finds Travel plan for `RegroupWithFaction` → decision trace: `planning.attempts` shows successful plan with Travel op
2. Planner finds Combat plan for `RaidTarget` when co-located → decision trace: successful single-step plan
3. Planner finds Travel+Combat plan for `RaidTarget` when not co-located → decision trace: multi-step plan
4. Priority class ordering → focused unit test: `RegroupWithFaction` ranks below survival, above enterprise
5. `RaidTarget` ranks at enterprise level → focused unit test: rank comparison with trade goals
6. Goal switching: survival interrupts raid → focused unit test: `compare_goal_switch` returns switch for ReduceDanger over RaidTarget
7. Goal switching: raid does NOT interrupt survival → focused unit test: `compare_goal_switch` returns no-switch

## What to Change

### 1. Add PlannerOpSemantics entries

In `crates/worldwake-ai/src/planner_ops.rs`:
- `RegroupWithFaction` → op kind: `Travel`, barriers: `NoKnownPath`, terminal: at rally place
- `RaidTarget` → op kind: `Combat`, barriers: `CombatTooRisky`, terminal: target defeated or fled

### 2. Update search terminal conditions

In `crates/worldwake-ai/src/search.rs`:
- `RegroupWithFaction`: terminal when agent is `located_in` the believed rally place
- `RaidTarget`: terminal when agent is at target's place and raid action is available

### 3. Assign priority classes in ranking

In `crates/worldwake-ai/src/ranking.rs`:
- `RegroupWithFaction`: priority class between survival and enterprise (e.g., Social or a new "Faction" class if the ranking system supports it — check current priority class hierarchy)
- `RaidTarget`: enterprise priority class (analogous to `SellCommodity`, `RestockCommodity`)

### 4. Add suppression rules

- `RegroupWithFaction`: suppressed when `stress >= Critical` (survival first)
- `RaidTarget`: suppressed when `CombatTooRisky` blocked intent is active for the target location

### 5. Update goal-switching thresholds

In `crates/worldwake-ai/src/goal_switching.rs`:
- Survival goals (ReduceDanger, ConsumeCommodity) can interrupt `RaidTarget`
- `RegroupWithFaction` can be interrupted by survival goals but not by enterprise goals

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — add semantics entries for new goal kinds)
- `crates/worldwake-ai/src/search.rs` (modify — add terminal conditions for new goal kinds)
- `crates/worldwake-ai/src/ranking.rs` (modify — assign priority classes)
- `crates/worldwake-ai/src/goal_switching.rs` (modify — add switching rules)
- `crates/worldwake-ai/src/interrupts.rs` (modify — if interrupt logic has goal-kind-specific branches)
- Any files with exhaustive match on `GoalKind` in the AI crate (modify — add arms)

## Out of Scope

- Candidate generation (E18BANDYN-006 — produces the goals this ticket teaches the planner to plan for)
- Raid action handler mechanics (E18BANDYN-003)
- EstablishCamp action (E18BANDYN-004)
- Route threat estimation (E18BANDYN-008)
- bandit_camp_system (E18BANDYN-005)
- Golden test T22 (E18BANDYN-009)

## Acceptance Criteria

### Tests That Must Pass

1. Planner produces valid Travel plan for `RegroupWithFaction` goal
2. Planner produces valid Combat plan for `RaidTarget` when co-located with target
3. Planner produces Travel+Combat plan for `RaidTarget` when not co-located
4. `RegroupWithFaction` ranks below ReduceDanger and ConsumeCommodity
5. `RegroupWithFaction` ranks above enterprise goals (SellCommodity, RestockCommodity)
6. `RaidTarget` ranks at enterprise level
7. Survival goals interrupt `RaidTarget` via goal switching
8. `RaidTarget` does not interrupt survival goals
9. `RegroupWithFaction` suppressed at Critical stress
10. Existing golden tests pass: `cargo test -p worldwake-ai`
11. Existing suite: `cargo clippy --workspace`

### Invariants

1. FND-18 (Resource-Bounded Reasoning): plans are constructed through the same GOAP search as all other goals
2. FND-19 (Revisable Commitments): raid and regroup plans can be interrupted by higher-priority goals
3. No new planner op kinds — reuse existing Travel and Combat ops
4. No magic numbers — priority classes and thresholds are consistent with existing ranking logic
5. All existing golden tests continue to pass (no regression)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` tests — plan search for `RegroupWithFaction` and `RaidTarget`
2. `crates/worldwake-ai/src/ranking.rs` tests — priority class ordering for new goal kinds
3. `crates/worldwake-ai/src/goal_switching.rs` tests — switching rules for new goal kinds

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo build --workspace`
