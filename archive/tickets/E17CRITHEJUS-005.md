# E17CRITHEJUS-005: Planner contract for deferred crime and justice goal kinds

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — ticket scope correction plus focused AI regression coverage for the deferred planner boundary
**Deps**: E17CRITHEJUS-004 (needs GoalKind variants), E17CRITHEJUS-006, E17CRITHEJUS-008, E17CRITHEJUS-009

## Problem

This ticket was written as if `StealItem`, `Accuse`, and `PunishAccused` still lacked all AI-layer integration. That is no longer true. The live code already wires these `GoalKind`s through goal tags, binding, ranking, suppression, and feasibility, but intentionally leaves them with no live planner operators because the authoritative `steal` / `accuse` / `fine` / `exile` actions do not exist yet. The ticket must be corrected before implementation so we do not add dead planner aliases or duplicate precondition logic ahead of the action layer.

## Assumption Reassessment (2026-03-26)

1. The live `GoalKind` surface already includes `StealItem`, `Accuse`, and `PunishAccused` in [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs). This ticket is not introducing the goal variants.
2. The live AI goal model already covers these variants in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs): `GoalKindTag` mapping exists, `matches_binding()` enforces exact-bound identity on item/accused targets, and focused tests already cover those bindings.
3. The live AI ranking and suppression layers already cover these variants in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), [crates/worldwake-ai/src/goal_policy.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs), and [crates/worldwake-ai/src/feasibility.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/feasibility.rs). Current behavior is `GoalPriorityClass::Low`, suppression at `High` stress, and feasibility defaulting to `Uncertain`.
4. The shared abstraction boundary under audit is: `GoalKind` -> `GoalKindPlannerExt::relevant_op_kinds()` -> `search::candidates::relevant_action_defs()` over the registered authoritative action registry. That is the canonical planner-operator surface in the current architecture.
5. Reassessment shows the live operator surface differs from the original narrative: `GoalKindPlannerExt::relevant_op_kinds()` returns `DEFERRED_CRIME_JUSTICE_OPS = &[]` for all three crime/justice goals in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs). The current planner intentionally exposes no live operator surface for them.
6. That deferral is consistent with the current authoritative action layer. There are no registered `steal`, `accuse`, `fine`, or `exile` action defs anywhere under `crates/worldwake-systems/` or `crates/worldwake-sim/`. In this architecture, live `PlannerOpKind` semantics are derived from registered action defs in [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs), so adding standalone planner ops now would create dead, duplicated behavior.
7. Candidate-generation tickets [tickets/E17CRITHEJUS-010.md](/home/joeloverbeck/projects/worldwake/tickets/E17CRITHEJUS-010.md) and [tickets/E17CRITHEJUS-011.md](/home/joeloverbeck/projects/worldwake/tickets/E17CRITHEJUS-011.md) currently depend on this ticket, but the real hard dependency is the authoritative action surface in [tickets/E17CRITHEJUS-006.md](/home/joeloverbeck/projects/worldwake/tickets/E17CRITHEJUS-006.md), [tickets/E17CRITHEJUS-008.md](/home/joeloverbeck/projects/worldwake/tickets/E17CRITHEJUS-008.md), and [tickets/E17CRITHEJUS-009.md](/home/joeloverbeck/projects/worldwake/tickets/E17CRITHEJUS-009.md).
8. Existing focused coverage already names the deferred contract directly: `goal_model::tests::deferred_crime_and_justice_goals_have_no_live_planner_ops_yet`, `feasibility::tests::deferred_crime_and_justice_goals_default_to_uncertain`, and `ranking::tests::deferred_crime_and_justice_goals_rank_low_with_minimal_motive`.
9. This is an AI-layer contract ticket, not a start-failure or action-handler ticket. The strongest proof surface is focused search/runtime coverage inside `worldwake-ai`, not golden tests.
10. The relevant test binary layout was verified with `cargo test -p worldwake-ai -- --list`.
11. Mismatch: the original ticket claimed "full planner integration" was pending and proposed adding `PlannerOpKind::{Steal,Accuse,Fine,Exile}` immediately. That no longer matches the codebase. Correct scope is to keep the crime/justice goals partially integrated but explicitly deferred at the live planner-op boundary until the authoritative action tickets land.
12. Adjacent contradiction exposed: [tickets/E17CRITHEJUS-010.md](/home/joeloverbeck/projects/worldwake/tickets/E17CRITHEJUS-010.md) and [tickets/E17CRITHEJUS-011.md](/home/joeloverbeck/projects/worldwake/tickets/E17CRITHEJUS-011.md) describe this ticket as if it should create live operators now. That is future cleanup for those tickets, not a reason to force stale scope here.

## Architecture Check

1. The cleaner architecture is to derive live planner operators from real registered action defs, then map them into `PlannerOpKind` semantics in one place. That keeps authoritative preconditions, affordances, planner search, and runtime execution on the same action surface.
2. Adding fake `PlannerOpKind::{Steal,Accuse,Fine,Exile}` before the action layer exists would be worse than the current architecture. It would introduce dead planner aliases, split the source of truth for crime/justice preconditions across AI and systems, and invite future divergence when the real action defs land.
3. The robust change for this ticket is therefore narrow: document the corrected scope and strengthen tests around the current deferred boundary. Activation of live operators belongs in a follow-up once the underlying action defs exist.
4. No backwards-compatibility aliasing or shim paths are introduced.

## Verification Layers

1. Goal-family identity and exact-bound target matching remain wired for all three deferred goals -> focused unit tests in `goal_model.rs`
2. Deferred live-operator boundary (`relevant_op_kinds() == []`) remains intact -> focused unit test in `goal_model.rs`
3. Search sees no relevant action defs and therefore produces no root candidates for crime/justice goals before the action layer exists -> focused search-layer unit test in `search/tests.rs`
4. Ranking, suppression, and feasibility remain intentionally low-priority / stress-suppressible / uncertain for deferred goals -> focused unit tests in `ranking.rs`, `goal_policy.rs`, and `feasibility.rs`
5. Additional golden or action-trace mapping is not applicable yet because no authoritative crime/justice actions exist to execute

## What to Change

### 1. Correct the ticket scope

Rewrite the ticket around the live architecture:
- record which AI surfaces are already implemented
- name the real shared planner boundary (`relevant_op_kinds()` -> `relevant_action_defs()`)
- remove the stale requirement to add new live planner ops before the action layer exists
- correct the dependency story to make the action tickets the real prerequisite for later planner activation

### 2. Strengthen focused regression coverage at the search boundary

Add one focused test proving that, with the full current action registry loaded, deferred crime/justice goals still surface zero relevant action defs and zero search candidates. That is the actual contract preventing half-built crime/justice goals from entering GOAP search prematurely.

## Files to Touch

- `tickets/E17CRITHEJUS-005.md` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- Adding `PlannerOpKind::{Steal, Accuse, Fine, Exile}` before real action defs exist
- Changing ranking, suppression, feasibility, or exact-binding behavior that is already implemented
- Candidate generation functions (`emit_theft_candidates`, `emit_justice_candidates`) — E17CRITHEJUS-010/011
- Action definitions and handlers — E17CRITHEJUS-006/008/009
- Golden tests — E17CRITHEJUS-012/013
- Runtime `agent_tick` changes

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKindTag`, exact-bound `matches_binding()`, ranking, suppression, and feasibility tests for `StealItem`, `Accuse`, and `PunishAccused` still pass unchanged
2. Full current action registry still yields zero relevant action defs for all three deferred crime/justice goals
3. Search produces zero root candidates for all three deferred crime/justice goals while authoritative crime/justice actions are absent
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo build --workspace`
6. Existing suite: `cargo clippy --workspace`

### Invariants

1. Crime/justice goals remain partially integrated at the AI metadata layer but have no live planner-op surface until authoritative action defs exist
2. The canonical planner-op source remains the registered action registry plus `build_semantics_table()`, not ad hoc goal-only aliases
3. Existing non-crime goal behavior remains unchanged
4. No backwards-compatibility shims or alias paths are introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — add a focused regression test proving deferred crime/justice goals expose no relevant defs and no search candidates against the live action registry
2. Existing focused tests in `goal_model.rs`, `goal_policy.rs`, `ranking.rs`, and `feasibility.rs` remain the proof surfaces for already-implemented AI metadata behavior

### Commands

1. `cargo test -p worldwake-ai deferred_crime_and_justice_goals_have_no_search_surface_before_actions_land`
2. `cargo test -p worldwake-ai`
3. `cargo build --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-26
- What actually changed:
  - Corrected the ticket scope to match the live architecture: crime/justice goals are already integrated into AI metadata surfaces, but their live planner-op surface is intentionally deferred until authoritative `steal` / `accuse` / `fine` / `exile` actions exist.
  - Added focused search-layer regression coverage in `crates/worldwake-ai/src/search/tests.rs` proving that the full current action registry still exposes zero relevant defs and zero search candidates for `StealItem`, `Accuse`, and `PunishAccused`.
- Deviation from original plan:
  - Did not add `PlannerOpKind::{Steal, Accuse, Fine, Exile}` or planner transitions. Reassessment showed that would duplicate and preempt the authoritative action layer, which would be a worse long-term architecture than the current deferred boundary.
- Verification results:
  - `cargo test -p worldwake-ai deferred_crime_and_justice_goals_have_no_search_surface_before_actions_land`
  - `cargo test -p worldwake-ai`
  - `cargo build --workspace`
  - `cargo clippy --workspace`
