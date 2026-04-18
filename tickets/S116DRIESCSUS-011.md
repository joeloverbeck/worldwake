# S116DRIESCSUS-011: Repair stale exact-opportunity self-consume planning

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` goal/search/failure-handling path and `worldwake-sim` current-place belief view
**Deps**: archive/tickets/S116DRIESCSUS-008.md, archive/tickets/S116DRIESCSUS-009.md, archive/tickets/S116DRIESCSUS-010.md

## Problem

`S116DRIESCSUS-006` golden calibration exposed a live planner/runtime contradiction in the self-consume commodity path. Under the authored survival baseline, agents repeatedly select exact `AcquireCommodity(SelfConsume)` opportunities that compile to a root `MoveCargo` / `pick_up` step against a non-colocated target. Runtime then rejects that reproduced request with `PreconditionFailed("TargetAtActorPlace(0)")` for many consecutive ticks instead of converging on a lawful travel/harvest/drink-eat chain or invalidating the stale opportunity. This is now a production bug, not a scenario-only wash-basin issue.

Any fix for this ticket must align with [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): one canonical information path, no scenario-specific workaround logic, belief-only planning preserved, and no shims that keep both the broken and fixed opportunity semantics alive.

## Assumption Reassessment (2026-04-18)

1. Motivating proof surface was the ignored golden baseline rerun, not a hypothetical scenario narrative. The original owned failures were:
   - `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_perform_survival_actions -- --ignored --exact --nocapture`
   - `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact --nocapture`
   - `cargo test -p worldwake-ai --test golden_survival_baseline no_stuck_idle_windows_with_elevated_needs -- --ignored --exact --nocapture`
2. The live goal family under audit is commodity self-care, not Wash. The affected `GoalKind`s are `AcquireCommodity { purpose: SelfConsume, .. }` and `ConsumeOwnedCommodity { .. }` in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), with root operator surfacing through `PlannerOpKind::MoveCargo`, `PlannerOpKind::Harvest`, and `PlannerOpKind::Travel`.
3. Exact shared boundary under audit: planner-visible grounded commodity opportunities vs runtime-reproducible affordances, plus the current-place visibility carrier used to decide whether a local item lot still lawfully exists. The critical symbols are `GroundedGoal::synthesized_root_candidate_targets(...)` in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), `search_candidates_from_affordance(...)` and `candidate_action_place(...)` in [search/candidates.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs), `handle_plan_failure(...)` / `related_place(...)` in [failure_handling.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs), and current-place entity/item visibility in [per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs).
4. The shipped contradiction was not “agents cannot discover the one wash basin.” A traced baseline pass showed repeated decision outcomes selecting `AcquireCommodity(SelfConsume)` or `ConsumeOwnedCommodity` with a one-step `MoveCargo` root while action tracing logged `pick_up` start failures with `PreconditionFailed("TargetAtActorPlace(0)")`. The root cause was that current-place belief reads still allowed stale believed local item lots to remain planner-visible as local entities after they were no longer authoritatively present.
5. This regression appears after the recent S116 planner/contract work but is not obviously caused by the wash-only contract itself. The symptom survives even with temporary local wash basins authored into `survival-baseline.ron`, and it manifests on `drink` and hunger survival, so the honest ticket is broader than dirtiness escalation.
6. Intended verification layer is mixed-layer: decision trace for stale exact-root selection, action trace for authoritative `pick_up` rejection, and golden reruns for end-to-end survival recovery. A local needs-only harness is not sufficient.
7. FOUNDATIONS alignment requirement is substantive here:
   - preserve belief-only planning and locality from [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md)
   - end with one canonical self-consume acquisition path
   - do not add a scenario-only exemption or duplicate fallback path that leaves both stale-lot and lawful travel/harvest semantics active
8. Adjacent contradiction classification:
   - the stale exact current-place local opportunity bug is the owned scope of this ticket
   - the remaining post-fix survival-baseline hunger-critical run is a separate production contradiction and must be tracked independently in `tickets/S116DRIESCSUS-012.md`
9. Reassessment result after implementation on 2026-04-18:
   - focused regressions proving the stale local-opportunity repair pass
   - `golden_survival_baseline::all_agents_perform_survival_actions` passes
   - `golden_survival_baseline::no_stuck_idle_windows_with_elevated_needs` passes
   - `golden_survival_baseline::all_agents_survive_1440_ticks` still fails, but no longer with repeated `TargetAtActorPlace(0)` starts; that residual issue is now explicitly deferred to ticket `012`

## Architecture Check

1. Fixing the canonical grounded-opportunity / root-candidate path is cleaner than patching survival scenarios or weakening golden assertions. The problem is cross-scenario and appears before authoritative action execution.
2. A lawful solution must make stale exact self-consume opportunities impossible to select or must invalidate them at the first shared planner/runtime boundary. That is cleaner than adding per-scenario infrastructure or retry heuristics around repeated `pick_up` failures.
3. No backwards-compatibility aliasing/shims: the end state should have one live interpretation of exact commodity opportunities for self-consume, not parallel stale-lot and fallback path semantics.

## Verification Layers

1. Stale exact self-consume opportunity is no longer selected from stale believed current-place local lots -> focused current-place belief-view and planner/runtime coverage in `worldwake-sim` and `worldwake-ai`
2. Runtime no longer repeatedly reproduces impossible `pick_up` requests for the same stale local opportunity -> focused failure-handling/search coverage in `worldwake-ai`
3. The stale exact-target loop is removed without reopening baseline no-action/idle regressions -> `golden_survival_baseline::all_agents_perform_survival_actions` and `golden_survival_baseline::no_stuck_idle_windows_with_elevated_needs`
4. Residual long hunger-critical exposure after the stale-target fix is tracked explicitly as a separate contradiction -> follow-up ticket `S116DRIESCSUS-012`

## What to Change

### 1. Reassess exact commodity opportunity binding

Audit how grounded self-consume opportunities preserve `evidence_entities`, authoritative targets, and place guidance after remote harvest/resource facts age, move, or are no longer colocated with the actor. Tighten the live contract so exact `MoveCargo` roots only surface when the target is still lawfully local and reproducible.

### 2. Repair stale-root invalidation or fallback

At the canonical planner/runtime boundary, ensure stale exact self-consume opportunities either:
- stop surfacing as direct `MoveCargo` roots, or
- invalidate cleanly so the planner can re-derive a lawful travel/harvest path instead of retrying the same impossible `pick_up`.

The shipped solution must align with `FOUNDATIONS`: concrete state, locality, no workaround architecture, no parallel duplicate path.

### 3. Recover the exact-opportunity surface honestly

Use the existing survival baseline goldens only to confirm that the stale exact-target loop is gone. Do not “solve” this ticket by scattering additional wash basins or other scenario-only resource patches through `survival-baseline.ron`. Any remaining long-run survival-envelope failure after the stale-target loop is removed belongs to the explicit follow-up ticket.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify if needed)
- `crates/worldwake-ai/src/search/candidates.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/tests/` focused planner/action-trace coverage (modify/add)
- `crates/worldwake-ai/tests/golden_survival_baseline.rs` (modify only if stronger assertion or temporary trace helper is required; do not weaken the owned invariant)

## Out of Scope

- Changing the wash contract from `S116DRIESCSUS-010`
- Adding extra wash basins or other scenario-only resource patches to make the baseline pass
- Contested-scenario bound tightening from `S116DRIESCSUS-007`
- Broad drive-escalation retuning or residual survival-envelope tuning now tracked separately in `tickets/S116DRIESCSUS-012.md`

## Acceptance Criteria

### Tests That Must Pass

1. New focused coverage proves stale believed current-place local lots do not remain planner-visible once the actor is co-located and authoritative local state disagrees
2. New focused coverage proves stale exact self-consume opportunities are blocked or invalidated without repeated `pick_up` start failures
3. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_perform_survival_actions -- --ignored --exact`
4. `cargo test -p worldwake-ai --test golden_survival_baseline no_stuck_idle_windows_with_elevated_needs -- --ignored --exact`

### Invariants

1. Self-consume planning remains belief-backed and locality-preserving; the only new authoritative read admitted here is the actor's lawful current-place local visibility
2. Only one canonical current-place local commodity opportunity contract remains live after the change: authoritative local, belief-backed remote

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` tests — focused regression for authoritative current-place local entity visibility over stale beliefs
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — focused regression proving stale believed current-place lots do not emit `ConsumeOwnedCommodity`
3. Existing `worldwake-ai` focused failure-handling/search regressions for stale `MoveCargo` blocker scoping
4. `crates/worldwake-ai/tests/golden_survival_baseline.rs` — existing ignored assertions `all_agents_perform_survival_actions` and `no_stuck_idle_windows_with_elevated_needs` remain the downstream confirmation surface for this ticket

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view::tests::current_place_entities_use_authoritative_local_set_over_stale_beliefs -- --exact`
2. `cargo test -p worldwake-ai agent_tick::tests::stale_current_place_lot_belief_does_not_emit_consume_owned_goal -- --exact`
3. `cargo test -p worldwake-ai failure_handling::tests::handle_plan_failure_scopes_remote_move_cargo_blocker_to_target_place -- --exact`
4. `cargo test -p worldwake-ai candidate_generation::tests::blocked_exact_acquire_target_suppresses_only_stale_move_cargo_opportunity -- --exact`
5. `cargo test -p worldwake-ai search::tests::search_blocks_remote_stale_move_cargo_by_target_place -- --exact`
6. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_perform_survival_actions -- --ignored --exact`
7. `cargo test -p worldwake-ai --test golden_survival_baseline no_stuck_idle_windows_with_elevated_needs -- --ignored --exact`
8. `cargo test -p worldwake-ai --lib`
9. `cargo clippy --workspace --all-targets -- -D warnings`
