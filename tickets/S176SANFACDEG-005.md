# S176SANFACDEG-005: Planner-op integration for cleaning prerequisites

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `PlannerOpKind` (ai, 2 variants), `classify_action_def` (ai), `GoalKind::Wash`/`Relieve` plan search (ai, `goal_model.rs`)
**Deps**: S176SANFACDEG-003 (degradation preconditions), S176SANFACDEG-004 (the cleaning actions)

## Problem

The cleaning actions exist but the planner cannot reach them: when a wash/toilet precondition is blocked, the GOAP search must insert the corresponding cleaning action as a prerequisite (S176 D6, Q1=(a) prerequisite-op model). This ticket wires the new actions into the existing `Wash`/`Relieve` plan search, mirroring `QueueForFacilityUse`. It also carries the belief-backed read discipline (D7, distributed).

## Assumption Reassessment (2026-05-29)

1. `PlannerOpKind` is at `crates/worldwake-ai/src/planner_ops.rs:14`; `classify_action_def` maps `(ActionDomain, name)` → op at `:87-141`. `QueueForFacilityUse` is the precedent: a prerequisite op inserted mid-plan within the `Wash`/`Relieve` goals (handled at `goal_model.rs:1143`). The `Wash`/`Relieve` relevant-op lists are at `goal_model.rs:618, 667, 1602, 1649`; `apply_planner_step` arms at `:1249` (Relieve) / `:1253` (Wash).
2. `PlannerOpKind` is referenced across **9** ai files (`planner_ops.rs`, `goal_schema.rs`, `goal_model.rs`, `failure_handling.rs`, `search/candidates.rs`, `search/transition.rs`, `agent_tick/observation.rs`, `agent_tick/frame.rs`, `agent_tick/active_action.rs`). Confirm which carry exhaustive matches needing a new arm vs. `_ =>` catch-all (`semantics_for`, `may_appear_mid_plan`, `is_materialization_barrier` in `planner_ops.rs` need explicit handling for the new ops).
3. Shared boundary under audit (D7 distributed): the cleaning prerequisite is synthesized only from belief-backed / same-tick-local facility condition. Existing accessors: `facility_wash_basin_state(entity) -> Option<WashBasinState>` (`belief_view.rs:495`, returns `None` when remote/unknown — preferred for the gating read), `wash_basin_state(agent, basin)` (`:561`) and `latrine_fullness(agent, place)` (`:557`) which return defaults; treat a defaulted/absent read as "condition unknown" and do **not** synthesize a cleaning prerequisite for a fully-unknown remote facility (FND-14B). No new accessor is added.
4. Live planner surface: `GoalKind::Wash` (`goal.rs:73`) / `Relieve` (`goal.rs:72`); the cleaning ops are inserted as prerequisites of these goals — **no new `GoalKind`**, and therefore no `GoalDispatchKey` / `GoalDispatchDeclaration` / `GoalKindPlannerExt` surface (confirmed against the spec's Non-Goals).
5. Heuristic-removal discipline (N/A): no heuristic is removed; this adds a new prerequisite op alongside `QueueForFacilityUse`. The op advances the goal by establishing the `TargetWashBasinNotTooDirty` / `PlaceLatrineNotFull` precondition (from 003).

## Architecture Check

1. Plain GOAP prerequisite ops mirroring `QueueForFacilityUse` — the smallest lawful surface that lets recovery emerge from search, not a scripted maintenance loop (FND-20).
2. FND-14B: the synthesized prerequisite depends only on belief-backed/local facility condition; removing the belief removes the candidate. No new `GoalKind` keeps the dispatch surface unchanged.

## Verification Layers

1. Cleaning prerequisite insertion when gate blocked → decision trace (planner inserts `CleanWashBasin`/`EmptyLatrine` step ahead of the terminal self-care op).
2. Belief barrier → decision trace (no cleaning prerequisite synthesized for a fully-unknown remote facility).
3. Replan after rejection → `handle_plan_failure` exercised; proven end-to-end by S176SANFACDEG-008 goldens.

## What to Change

### 1. PlannerOpKind + classification

Add `PlannerOpKind::{CleanWashBasin, EmptyLatrine}`; add `classify_action_def` arms for `(ActionDomain::Needs, "clean_wash_basin")` and `(ActionDomain::Needs, "empty_latrine")`; handle the new ops in `semantics_for` / `may_appear_mid_plan` / `is_materialization_barrier`.

### 2. Wash/Relieve search integration

Add the new ops to the `Wash`/`Relieve` relevant-op lists and `apply_planner_step`; extend the mid-plan-op handling (the `QueueForFacilityUse` path) so the cleaning op is inserted as a prerequisite when the degradation precondition blocks the terminal op.

### 3. Belief-backed read guard (D7)

In the prerequisite-synthesis path, read facility condition via the existing accessors and skip synthesis for fully-unknown remote facilities.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — variants, `classify_action_def`, semantics)
- `crates/worldwake-ai/src/goal_model.rs` (modify — op lists, `apply_planner_step`, prerequisite insertion)
- Likely: `crates/worldwake-ai/src/goal_schema.rs`, `crates/worldwake-ai/src/failure_handling.rs`, `crates/worldwake-ai/src/search/candidates.rs`, `crates/worldwake-ai/src/search/transition.rs`, `crates/worldwake-ai/src/agent_tick/{observation,frame,active_action}.rs` (modify — confirm exhaustive `PlannerOpKind` matches via `rg 'PlannerOpKind::' crates/worldwake-ai/src/` during implementation; add arms only where the match is exhaustive)

## Out of Scope

- The cleaning action definitions themselves — S176SANFACDEG-004.
- The preconditions that trigger the prerequisite — S176SANFACDEG-003.
- New `GoalKind` / dispatch surface — explicitly excluded per spec Non-Goals.

## Acceptance Criteria

### Tests That Must Pass

1. When a co-located basin is too dirty, the `Wash` plan search inserts a `clean_wash_basin` prerequisite step ahead of the wash.
2. When a co-located latrine is full, the `Relieve` plan search inserts an `empty_latrine` prerequisite (or falls back to wilderness relief).
3. No cleaning prerequisite is synthesized for a remote facility with no belief carrier.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No new `GoalKind`; cleaning is a prerequisite op within `Wash`/`Relieve` (FND-20).
2. Prerequisite synthesis reads only belief-backed/local facility condition (FND-14B).
3. The new ops integrate through `classify_action_def`, not a hardcoded action-name check in the search.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` (or `planner_ops.rs`) tests — new: op classification + Wash/Relieve prerequisite insertion on blocked gate.
2. Decision-trace focused test — belief-barrier (no synthesis for unknown remote facility).

### Commands

1. `cargo test -p worldwake-ai goal_model && cargo test -p worldwake-ai planner_ops`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`
