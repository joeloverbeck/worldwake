# S155BELVIEBOU-002: Belief-gate `ControlBeliefView::can_control` in place

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` belief view (`ControlBeliefView::can_control` impl)
**Deps**: None (parallel with S155BELVIEBOU-001; shares `per_agent_belief_view.rs`, different method)

## Problem

`ControlBeliefView::can_control` (`crates/worldwake-sim/src/per_agent_belief_view.rs:433`) has **no belief gate**: after the FND-14A co-location unowned-item shortcut it falls straight through to authoritative `world.can_exercise_control(actor, entity)`. Its sibling `believed_rights()` (`:420`) gates on belief accessibility (self / `believed_entity` / possessed / owned) and returns early when inaccessible, because control rights are a social/jurisdictional fact (FND-24). `can_control` is consumed only from belief-facing planning/affordance paths, so the missing gate lets an agent's planner treat as controllable an entity it has no belief path to — an FND-14/FND-14A violation that shapes affordances and emitted candidates.

## Assumption Reassessment (2026-05-20)

<!-- Spec S155 reassessed this session (/reassess-spec); abbreviated spot-check confirmed targets. -->

1. **Current code**: `can_control` is the `ControlBeliefView` impl at `per_agent_belief_view.rs:433` (trait declared `belief_view.rs:839`; also surfaced via the `GoalControlBeliefView` blanket forward `belief_view.rs:1670`/`:2051`). It has the FND-14A unowned-item co-location shortcut, then `world.can_exercise_control(actor, entity).is_ok()` with no belief gate. `believed_rights` (`:420`) is the gate pattern to mirror. Confirmed.
2. **Current specs/docs**: `specs/S155-belief-view-boundary-correctness.md` D2 (post-reassessment, in-place) + its Authoritative-to-AI Impact Analysis section. `docs/FOUNDATIONS.md` FND-14A (social/relational facts always need a belief entry even when co-located), FND-24 (ownership/rights/permission distinct), FND-28 (no parallel authority).
3. **Shared boundary under audit (mixed-layer)**: belief-facing `ControlBeliefView::can_control` ("do I *believe* I may control this?") vs. authoritative `World::can_exercise_control` ("will the world allow it at commit?"). The fix tightens the belief-facing answer only; the authoritative method is unchanged.
4. **Intended invariant (restated)**: `can_control` returns `true` for a non-self entity only when it is FND-14A co-located-unowned, or belief-accessible (in `believed_entity`, possessed, or owned) AND authoritatively controllable. No belief path → `false`.
5. **Live planner surface**: `can_control` gates candidate emission and affordance/plan reachability, not a single `GoalKind`. The ~18 callers (item 13) consume it across affordance query, candidate generation, snapshot/state entity filtering, replan, and goal explanation.
6. **AI regression layer**: focused unit test on `PerAgentBeliefView` plus runtime ripple via the full AI golden suite — gating changes which candidates emit and which targets are affordable (decision-trace-visible). Full action registries are exercised by the golden suite, not a needs-only harness.
7. **Ordering**: not an ordering-sensitive change; the gate is a per-call predicate, no tick/lifecycle ordering dependency.
8. **Heuristic-removal discipline**: this does not remove a heuristic — it *adds* the missing belief substrate (`believed_rights`-style accessibility gate) that the un-gated authoritative read was illegitimately standing in for. The substrate (belief accessibility) already exists; this ticket applies it.
9. **Existing focused coverage**: `believed_rights_returns_rights_for_known_entity:5272`, `believed_rights_returns_empty_for_unknown_entity:5300`, `believed_rights_surfaces_jurisdiction_without_control:5323` (the gate pattern being mirrored), plus a `can_control` assertion around `:5362`. Confirm none asserts the un-gated authoritative fallthrough as desired behavior; if one does, it encodes the bug and must be corrected (never adapt tests to bugs).
13. **Adjacent contradictions / blast radius**: `can_control` has ~18 belief-facing callers — `affordance_query.rs:286,378,933`, `per_agent_belief_view.rs:336`, the `belief_view.rs` blanket forward, `enterprise.rs:164,170`, `exhaustion.rs:505`, `plan_revalidation.rs:198`, `goal_explanation.rs:293`, `planning_snapshot.rs:1103`, `planning_state.rs:2956`, `effect_sink_hypothetical.rs:607`, `goal_model.rs:4028`, `candidate_generation.rs:1641,5352,7157,7177,8312`. **None is a dispatch caller**; dispatch uses `World::can_exercise_control` directly (unchanged). All inherit the gate automatically — this is a *required consequence* of the fix, not separate work; no caller file is edited.

## Architecture Check

1. Gating `can_control` **in place** (vs. introducing a parallel `believed_can_control` and migrating ~18 sites) corrects every belief-facing consumer at once, leaves `World::can_exercise_control` as the untouched dispatch authority, and introduces no fossil second method — directly satisfying FND-28 (no two live authorities for one concept). A new method would also trigger the New-Component-Read-by-AI-Crate wiring (trait method + `RuntimeBeliefView` impl + `impl_goal_belief_view!` forward) for no benefit.
2. The gate mirrors the established `believed_rights()` accessibility check in the same impl block — consistent local idiom, no new abstraction.

## Verification Layers

1. Belief-inaccessible non-self entity → `can_control` returns `false` (was `true` via authoritative fallthrough) → focused unit test on `PerAgentBeliefView`.
2. Belief-accessible + authoritatively controllable entity, and the FND-14A co-located-unowned case → `can_control` returns `true` (no regression) → focused unit test (positive controls).
3. Candidate-emission ripple (gating changes which candidates emit) → decision-trace / full AI golden suite; the candidate-gen and replan call sites are the load-bearing ripple per the spec's Authoritative-to-AI Impact Analysis.
4. Authoritative dispatch legality unchanged → `World::can_exercise_control` is not modified; no action-trace/event-log change expected from the dispatch path.

## What to Change

### 1. Add the belief-accessibility gate to `ControlBeliefView::can_control`

In `crates/worldwake-sim/src/per_agent_belief_view.rs:433`, keep the existing FND-14A co-location unowned-item shortcut (returns `true`). Before consulting `world.can_exercise_control`, require belief accessibility mirroring `believed_rights()` (`:420`): `entity == self.agent` || `self.believed_entity(entity).is_some()` || `self.world.possessor_of(entity) == Some(self.agent)` || `self.world.owner_of(entity) == Some(self.agent)`. If not accessible, return `false`; otherwise return `world.can_exercise_control(actor, entity).is_ok()`. Do not add a new method; do not touch `World::can_exercise_control` or any caller file.

### 2. Add focused unit test(s) (TDD — write first, confirm they fail against current code)

Belief-inaccessible non-self entity (not in `believed_entity`, not possessed/owned, not co-located-unowned) → `can_control` returns `false`. Positive controls: belief-accessible + authoritatively controllable → `true`; FND-14A co-located unowned item → `true`.

### 3. Trace the Authoritative-to-AI ripple and re-baseline goldens

Run the full AI suite; the gate narrows candidate emission (`candidate_generation.rs` sites) and replan legality (`plan_revalidation.rs:198`). Re-baseline expected golden trace shifts (fewer candidates for belief-inaccessible targets); confirm no world-outcome regressions.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — `can_control` impl body at `:433`; new `#[cfg(test)]` unit test(s))

## Out of Scope

- `effective_place` location fix — S155BELVIEBOU-001.
- Golden E2E (unknown-ownership-beside-chest, control-source-swap symmetry) + the `planner-contracts.md` doc line — S155BELVIEBOU-003.
- Any modification to `World::can_exercise_control`, the `ControlBeliefView` trait declaration, or any of the ~18 caller files (they inherit the gate automatically).
- Introducing a `believed_can_control` method (explicitly rejected by S155 reassessment, FND-28).

## Acceptance Criteria

### Tests That Must Pass

1. New: belief-inaccessible non-self entity → `can_control` returns `false`.
2. New: belief-accessible + authoritatively controllable, and FND-14A co-located-unowned → `true` (no regression).
3. Existing suite: `cargo test -p worldwake-sim per_agent_belief_view` and `cargo test -p worldwake-ai` (re-baseline expected candidate/affordance trace shifts; no world-outcome regressions).

### Invariants

1. `can_control` consults authoritative `World::can_exercise_control` only after the entity passes the belief-accessibility gate (or the FND-14A co-located-unowned shortcut). No belief path → `false` (FND-14A, FND-24).
2. Exactly one belief-facing control answer exists (`can_control`); no parallel `believed_can_control` method (FND-28). `World::can_exercise_control` remains the unchanged dispatch authority.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (`#[cfg(test)]`) — belief-inaccessible negative case + accessible/co-located positive controls for `can_control`.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-ai` (Authoritative-to-AI Impact Rule golden check; re-baseline expected trace shifts)
3. `./scripts/verify.sh`
