# S14SURPAT-001: Select and execute remote pursuit from patrol survival state

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes - AI candidate ranking/selection or planner admission for remote hostile pursuit
**Deps**: `scenarios/survival-patrol.ron`, `crates/worldwake-ai/tests/golden_survival_patrol.rs`

## Problem

`survival-patrol.ron` can now author patrol, pursuit profile, directed hostility, and last-seen memory in one survival scenario. The retained golden proves `Guard Mira` survives, completes the authored patrol route, and generates an in-range remote `EngageHostile` candidate for `Fugitive Vale` from authored hostility plus last-seen memory.

The row is still not landed because the generated remote pursuit branch is not selected or executed under the survival envelope. The guard repeatedly keeps patrol/self-care branches and never commits the terminal `attack`, so the roadmap cannot truthfully claim interrupt-driven remote pursuit.

## Assumption Reassessment (2026-04-24)

1. `crates/worldwake-ai/tests/golden_survival_patrol.rs` proves the current retained seam: `survival_patrol_proves_patrol_and_remote_pursuit_candidate_generation` passes and asserts the remote pursuit candidate has `route_cost == 3`, `omission == None`, `remote_pursuit_selected == false`, and `attack_committed == false`.
2. `scenarios/survival-patrol.ron` structurally activates `Patrol`, `Pursuit`, and supporting `Combat` under `crates/worldwake-cli/src/bin/scenario_coverage.rs`; combat remains supporting substrate only, not the row-15 combat/bandit-camp landing.
3. Shared abstraction boundary: `GoalKind::EngageHostile { target }` admission across `candidate_generation.rs` remote pursuit diagnostics, `planning_snapshot.rs` target/evidence capture, `search_plan` remote travel-plus-attack planning, and `plan_selection.rs` ranking/selection.
4. Intended invariant: after scheduled patrol reaches a place within `PursuitProfile::max_pursuit_travel_ticks` of the remembered hostile target, the AI should be able to select and execute the remote pursuit branch when survival needs do not impose a higher-priority interruption.
5. Live goal under test: `GoalKind::EngageHostile { target: Fugitive Vale }`; current candidate evidence exposes `PursuitDiagnostic { omission: None, route_cost: Some(3) }`, but no selected pursuit plan or `attack` commit follows.
6. Ordering layer: the desired proof should use decision trace for candidate/ranking/selection, action trace for `travel`/`attack` lifecycle, and authoritative world state for the final wound/position consequence if attack commits.
7. Adjacent contradiction classification: `CombatProfile` is structurally active because attack is the current pursuit terminal action. That is supporting substrate for this ticket, not permission to mark `survival-combat` landed.

## Architecture Check

1. The fix must make the existing belief-backed pursuit path selectable/executable through normal AI planning. Do not add a scenario-specific request, forced action, or test-only payload override.
2. No backwards-compatibility aliases or shim goals should be introduced.

## Verification Layers

1. Remote pursuit candidate remains generated from authored hostility plus last-seen memory -> decision trace `CandidateEvidenceTrace::pursuit`.
2. Candidate survives ranking/selection when in range and survival needs are not dominant -> decision trace `PlanningPipelineTrace.selection`.
3. Remote pursuit plan includes travel toward the believed target place before attack -> decision trace `PlanSearchOutcome::Found` planned steps.
4. Execution starts and commits the pursuit terminal action -> action trace `travel` and `attack`.
5. Supporting combat consequence remains bounded and does not become the row-15 combat landing -> authoritative wound state or action trace, with roadmap prose still classifying combat as structurally active only.

## What to Change

### 1. AI pursuit selection

Diagnose why `EngageHostile` with a successful `PursuitDiagnostic` remains unselected in `survival-patrol`. Adjust the narrowest correct live boundary: ranking, plan admission, plan search, or selection.

### 2. Roadmap golden

Strengthen `golden_survival_patrol.rs` from candidate-generation proof to selected/executed remote pursuit once the live branch is supported.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify if candidate/ranking evidence is wrong)
- `crates/worldwake-ai/src/plan_selection.rs` (modify if ranking/selection is wrong)
- `crates/worldwake-ai/src/search/*` (modify if remote pursuit planning is wrong)
- `crates/worldwake-ai/tests/golden_survival_patrol.rs` (modify)
- `docs/scenario-roadmap.md` (modify when landing)

## Out of Scope

- Landing `survival-combat` or bandit camps.
- Adding helper-only external requests for the pursuit branch.
- Weakening the `survival-patrol` survival-health contract.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_survival_patrol -- --ignored --test-threads=1`
2. Focused AI tests for the exact boundary changed.

### Invariants

1. Remote pursuit remains belief/local-memory backed; the AI does not query omniscient target location.
2. Combat stays supporting substrate for this row until `survival-combat` lands its own behavior.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_patrol.rs` - strengthen the existing partial seam to selected and executed remote pursuit.
2. Focused unit/runtime test at the changed AI boundary.

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_patrol -- --ignored --test-threads=1`
2. `cargo test -p worldwake-ai <focused_test_name>`
3. `cargo test -p worldwake-ai`
