# E17CRITHEJUS-010: Implement emit_theft_candidates()

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new candidate generation function in AI crate
**Deps**: E17CRITHEJUS-005 (needs planner support for StealItem), E17CRITHEJUS-001 (needs TheftDispositionProfile)

## Problem

Agents cannot form theft goals. No candidate generation function exists for `GoalKind::StealItem`. Without `emit_theft_candidates()`, the AI pipeline cannot produce theft goals even when an agent has a `TheftDispositionProfile` and co-located stealable items exist.

## Assumption Reassessment (2026-03-25)

1. `candidate_generation.rs` in worldwake-ai contains the `emit_*` family of functions (`emit_needs_candidates`, `emit_production_candidates`, `emit_enterprise_candidates`, `emit_combat_candidates`, `emit_social_candidates`, `emit_political_candidates`, `emit_violation_candidates`). Each follows the same pattern: guard on a profile component, scan beliefs, emit `GroundedGoal`.
2. The belief view provides `believed_owner_of()` (S01) and entity position queries. Co-located entity scanning uses existing `entities_at_place()` or equivalent belief query.
3. `emit_candidate_with_trace()` (S28) is the current standard for emitting candidates with knowledge-path provenance.
4. `KnowledgePath::DirectObservation` is the correct provenance for items the agent directly observes at their location.
5. `TheftDispositionProfile` provides `theft_motive_weight` and `witness_risk_penalty`. Motive = `theft_motive_weight - (witness_risk_penalty * co_located_agent_count)`.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatches found.
12. Motive math: `theft_motive_weight: Permille(300)`, `witness_risk_penalty: Permille(100)`, 2 observers -> 300 - 200 = 100 > 0 (candidate emitted). 3 observers -> 300 - 300 = 0 (candidate NOT emitted).

## Architecture Check

1. A new `emit_theft_candidates()` function follows the exact same pattern as existing `emit_*` functions. Guard on `TheftDispositionProfile`, scan beliefs, filter, calculate motive, emit. No new abstractions introduced.
2. No backwards-compatibility aliasing.

## Verification Layers

1. Candidate generated for stealable co-located item -> focused unit test with mock belief view
2. No candidate when agent lacks `TheftDispositionProfile` -> focused unit test
3. No candidate when motive reduced to zero by witnesses -> focused unit test
4. No candidate for unowned items -> focused unit test
5. No candidate for items agent can exercise control over -> focused unit test
6. No candidate for items possessed by another agent -> focused unit test
7. Knowledge path is `DirectObservation` -> decision trace check in focused test

## What to Change

### 1. New `emit_theft_candidates()` in `candidate_generation.rs`

Guard: return early if agent has no `TheftDispositionProfile` component.

Algorithm:
1. Query co-located `ItemLot` entities at agent's current place (from beliefs)
2. For each item lot: check `believed_owner_of(item) != Some(self)` AND `believed_owner_of(item).is_some()` (item must be owned by someone else) AND `can_exercise_control(self, item) == false` in the belief view
3. Filter: item not currently possessed by another agent, item not reserved, agent has load capacity
4. Calculate motive: `theft_motive_weight - (witness_risk_penalty * co_located_non_self_agent_count)`. If motive <= Permille(0), skip.
5. Emit `GroundedGoal { kind: GoalKind::StealItem { target_item }, motive, priority_class: GoalPriorityClass::Low }` via `emit_candidate_with_trace()` with `KnowledgePath::DirectObservation`.

### 2. Wire into candidate generation dispatch

Add call to `emit_theft_candidates()` in the main candidate generation dispatch function (where other `emit_*` functions are called).

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Justice candidate generation (E17CRITHEJUS-011)
- Steal action handler (E17CRITHEJUS-006)
- Planner ops and goal policy (E17CRITHEJUS-005)
- Golden tests (E17CRITHEJUS-012)
- Affordance generation for steal (handled by existing affordance_query + action_def infrastructure)
- Risk assessment beyond witness count (no stealth model per spec)

## Acceptance Criteria

### Tests That Must Pass

1. Agent with `TheftDispositionProfile` at place with owned item -> `StealItem` candidate emitted
2. Agent WITHOUT `TheftDispositionProfile` -> no theft candidates
3. Item owned by actor -> no candidate (can't steal your own items)
4. Item unowned -> no candidate (taking unowned items is lawful pickup)
5. Item where `can_exercise_control == true` -> no candidate (use pickup instead)
6. Item possessed by another agent -> no candidate (robbery out of scope)
7. Motive reduced to zero by witness count -> no candidate
8. Motive reduced but still positive -> candidate emitted with reduced motive
9. Knowledge path on emitted candidate is `DirectObservation`
10. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Only agents with `TheftDispositionProfile` ever generate theft candidates
2. Motive is always profile-driven (P2 — no magic numbers)
3. Witness risk penalty is per-observer and subtractive (physical deterrence model)
4. `GoalPriorityClass::Low` for all theft candidates (opportunistic, below survival)
5. No `HashMap`/`HashSet` in candidate scanning

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for `emit_theft_candidates()` covering all acceptance criteria

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
