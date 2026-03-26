# E17CRITHEJUS-005: Planner support for new goal kinds

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — GoalKindTag, PlannerOpKind, goal policy, ranking, feasibility, binding
**Deps**: E17CRITHEJUS-004 (needs GoalKind variants)

## Problem

The AI planner cannot process `StealItem`, `Accuse`, or `PunishAccused` goals. This ticket adds the full planner integration: goal tags, planner operators, goal policy, ranking, binding, and feasibility hints.

## Assumption Reassessment (2026-03-25)

1. `GoalKindTag` enum in `crates/worldwake-ai/src/goal_model.rs` maps 1:1 from `GoalKind` variants. `GoalKindPlannerExt` provides `terminal_op()`, `matches_binding()`, and `tag()` per variant.
2. `PlannerOpKind` enum in `crates/worldwake-ai/src/planner_ops.rs` has per-action-type semantics. `PlannerOpSemantics` provides barriers, mid-plan viability, and goal relevance.
3. `GoalFamilyPolicy` in `crates/worldwake-ai/src/goal_policy.rs` defines suppression, priority class, and interrupt behavior per goal family.
4. `ranking.rs` dispatches `GoalPriorityClass` per goal kind.
5. `feasibility.rs` dispatches `FeasibilityHint` per goal kind.
6. `matches_binding()` in `goal_model.rs` uses S03's exact-bound pattern for goals like `LootCorpse`, `EngageHostile`.
7. This ticket adds AI-layer planner infrastructure only. No candidate generation (E17CRITHEJUS-010/011), no action handlers (E17CRITHEJUS-006/008/009).
8. N/A.
9. N/A.
10. N/A.
11. No mismatches found.
12. N/A.

## Architecture Check

1. Each addition follows the established pattern exactly: one new `GoalKindTag` per `GoalKind`, one or more `PlannerOpKind` per action, one `GoalFamilyPolicy` entry per goal family. The Steal/Accuse/Fine/Exile operators mirror the structure of existing operators like `Loot`, `DeclareSupport`, `PressForce`.
2. No backwards-compatibility aliasing introduced.

## Verification Layers

1. `GoalKindTag::StealItemTag` <-> `GoalKind::StealItem` mapping -> focused unit test
2. `PlannerOpKind::Steal` terminal for `StealItem` goal -> focused unit test
3. `PlannerOpKind::Accuse` terminal for `Accuse` goal -> focused unit test
4. `PlannerOpKind::Fine` terminal for `PunishAccused { Fine }` goal -> focused unit test
5. `PlannerOpKind::Exile` terminal for `PunishAccused { Exile }` goal -> focused unit test
6. `matches_binding()` rejects wrong-target affordances for each exact-bound goal -> focused unit test
7. Goal policy: all 3 goals suppressed at `Medium` stress -> focused unit test
8. Ranking: all 3 goals get `GoalPriorityClass::Low` -> focused unit test
9. Feasibility dispatch doesn't panic for new goal kinds -> focused unit test

## What to Change

### 1. `GoalKindTag` in `goal_model.rs`

Add `StealItemTag`, `AccuseTag`, `PunishAccusedTag`. Implement `tag()` mapping from `GoalKind`.

### 2. `GoalKindPlannerExt` in `goal_model.rs`

For each new GoalKind:
- `StealItem`: terminal = `PlannerOpKind::Steal`, exact-bound on `target_item`
- `Accuse`: terminal = `PlannerOpKind::Accuse`, exact-bound on `accused`
- `PunishAccused`: terminal = `PlannerOpKind::Fine` or `PlannerOpKind::Exile` depending on `punishment` variant, exact-bound on `accused`

### 3. `PlannerOpKind` and `PlannerOpSemantics` in `planner_ops.rs`

Add 4 variants: `Steal`, `Accuse`, `Fine`, `Exile`.

Semantics:
- `Steal`: barriers = [item not at place, item possessed by other, actor wrong place]. Domain = Transport.
- `Accuse`: barriers = [not at CrimeRegister, accusation already filed]. Domain = Social.
- `Fine`: barriers = [no unresolved accusation, accused not present]. Domain = Social.
- `Exile`: barriers = [no unresolved accusation, accused not faction member]. Domain = Social.

### 4. Goal policy in `goal_policy.rs`

- `StealItem`: family `Crime`, suppression `WhenStressedAtOrAbove(Medium)`, not critical survival, not reactive, free-interrupt = false.
- `Accuse`: family `Justice`, same suppression, not critical, not reactive, free-interrupt = false.
- `PunishAccused`: family `Justice`, same suppression, not critical, not reactive, free-interrupt = false.

### 5. Ranking in `ranking.rs`

All 3 goals: `GoalPriorityClass::Low`.

### 6. Feasibility in `feasibility.rs`

- `StealItem`: `Likely` if co-located with target, `Uncertain` if remote, `Unlikely` if no known location.
- `Accuse`: `Likely` if has `SuspectedTheft` with suspect + knows CrimeRegister, `Uncertain` otherwise.
- `PunishAccused`: `Likely` if has authority + knows unresolved accusations, `Uncertain` otherwise.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/goal_policy.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/feasibility.rs` (modify)
- `crates/worldwake-ai/src/search/transition.rs` (modify — add transition rules for new ops)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — terminal recognition for new ops)

## Out of Scope

- Candidate generation functions (`emit_theft_candidates`, `emit_justice_candidates`) — E17CRITHEJUS-010/011
- Action definitions and handlers — E17CRITHEJUS-006/008/009
- Planner conformance tests for new ops (could be a follow-up S-series ticket)
- Golden tests — E17CRITHEJUS-012/013
- Changes to `agent_tick/` observation or active_action modules

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKindTag` round-trip: `StealItem -> StealItemTag -> StealItem`
2. `matches_binding()` accepts correct target, rejects wrong target for `StealItem`
3. `matches_binding()` accepts correct accused, rejects wrong accused for `Accuse`
4. `matches_binding()` accepts correct accused, rejects wrong accused for `PunishAccused`
5. Goal policy: `StealItem` suppressed when stress >= Medium
6. Goal policy: `Accuse` suppressed when stress >= Medium
7. Ranking: all 3 goals rank as `GoalPriorityClass::Low`
8. Feasibility: dispatch for new goals doesn't panic (basic smoke)
9. Existing suite: `cargo test -p worldwake-ai`
10. Existing suite: `cargo build --workspace`

### Invariants

1. `GoalKindTag` remains total (no unmatched GoalKind variants)
2. `PlannerOpSemantics` implemented for all new ops (no default/panic arms)
3. `GoalFamilyPolicy` covers all goal families including new `Crime` and `Justice`
4. Existing goal kinds completely unaffected — no behavioral change to prior tests
5. No `HashMap`/`HashSet` introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — binding acceptance/rejection for 3 new goal kinds
2. `crates/worldwake-ai/src/goal_policy.rs` — suppression evaluation for Crime and Justice families
3. `crates/worldwake-ai/src/ranking.rs` — priority class for new goals
4. `crates/worldwake-ai/src/planner_ops.rs` — semantics smoke test for 4 new ops

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo build --workspace`
3. `cargo clippy --workspace`
