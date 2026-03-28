# S36DECGOAL-001: Declarative ranking provenance families

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai`: consolidate ranking provenance construction and summary dispatch under declarative goal registration
**Deps**: [specs/S36-declarative-goal-registration.md](/home/joeloverbeck/projects/worldwake/specs/S36-declarative-goal-registration.md), [archive/tickets/completed/S34GENEPIACT-007.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-007.md)

## Problem

`RankedGoalProvenance` is now a real architectural surface, not an incidental detail. After `VerifyBelief` landed explicit provenance, adding a new ranking family requires parallel edits across provenance data shape, ranking dispatch, summary formatting, exports, and exhaustive tests. That is the same scattered-goal-dispatch smell S36 is already trying to remove. If left alone, every new non-drive/non-danger goal family will repeat the same manual enum-growth pattern and weaken the “single source of truth” target for AI dispatch and debugability.

## Assumption Reassessment (2026-03-28)

1. The live ranking provenance contract spans [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) (`RankedGoalProvenance`, `RankedDriveGoalProvenance`, `RankedVerificationGoalProvenance`), [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) (`goal_ranking_provenance`, `ranked_priority_class`, `ranked_motive_score`), and [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) (`format_ranked_goal_provenance_summary`).
2. The current code has three provenance families: `Danger`, `Drive`, and `Verification`. The real duplication is not “every family requires many unrelated files”; it is narrower:
   - family selection is still hard-coded in `ranking.rs` by matching concrete `GoalKind`
   - family rendering is correctly centralized on `RankedGoalProvenance` in `decision_trace.rs`
   - exhaustive consumer matches in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) and [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs) are expected and are not themselves an architectural smell
3. The shared abstraction boundary under audit is the AI-layer goal-registration/ranking/traceability contract, not authoritative world logic. This is a structural `worldwake-ai` ticket. No `worldwake-sim` or `worldwake-systems` behavior change is required.
4. Coverage gap classification after reassessment:
   - focused ranking/unit coverage exists in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
   - focused decision-trace summary coverage exists in [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
   - golden coverage indirectly consumes provenance in [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs)
   - no current focused ticket owns the structural provenance-dispatch cleanup
5. [specs/S36-declarative-goal-registration.md](/home/joeloverbeck/projects/worldwake/specs/S36-declarative-goal-registration.md) already identifies ranking family and trace label as declaration-owned properties. Reassessment shows the missing piece is declaration-owned provenance-family selection, not declaration-owned provenance formatting. Formatting remains a responsibility of the provenance type itself.
6. This is not a golden-driven behavior ticket. The intended verification layer is focused unit coverage plus full `worldwake-ai` regression coverage. Golden tests should remain consumers of the stronger structural contract, not the primary proof surface.
7. No ordering-sensitive gameplay contract is under audit here. The problem is compile-time and structural completeness of ranking provenance dispatch.
8. This ticket does not remove a heuristic or filter. It removes scattered dispatch duplication standing in for a missing declarative substrate.
9. Adjacent contradiction classification: this is not a required consequence for S34 functionality, but it is a follow-up cleanup that now has a concrete live trigger and should be made explicit rather than left as informal “future refactor” commentary.
10. `GoalKindTag` is not, by itself, expressive enough to own every provenance-family distinction today. In particular, `GoalKind::AcquireCommodity` splits by `CommodityPurpose` in live ranking code, so any canonical provenance-family surface must remain payload-aware unless S36 later introduces a finer-grained registration key.
11. Mismatch + correction: the earlier ticket draft overreached by trying to move provenance-summary dispatch into goal registration. The cleaner boundary is:
   - goal registration owns which provenance family a `GoalKindTag` uses
   - payload-sensitive goal variants may refine that answer at the `GoalKind` level until S36 grows a finer-grained declaration key
   - family-specific provenance data construction stays in ranking code
   - family-specific summary formatting stays on `RankedGoalProvenance`
   This still removes scattered family selection without coupling declarations to formatter functions or string fragments.

## Architecture Check

1. The clean fix is to make provenance family a canonical goal-model declaration concept, but keep provenance behavior on the provenance type. The registration surface should answer “which provenance family does this concrete goal use?”, with payload-aware refinement where `GoalKindTag` is too coarse. Ranking code remains responsible for constructing truthful family-specific data and `RankedGoalProvenance` remains responsible for summary rendering.
2. Moving trace-summary dispatch into declaration metadata would be a worse architecture than the current enum-based formatter: it would couple goal registration to display logic and duplicate a responsibility that already naturally belongs to the provenance sum type.
3. No backwards-compatibility aliasing or fallback string paths. Do not keep both declaration-driven family selection and the old scattered family-choice match alive. Replace the old family-choice path once the declaration lookup lands.

## Verification Layers

1. Every migrated goal resolves its provenance family through one canonical goal-model path -> focused unit tests on declaration lookup plus ranking tests that exercise real goals
2. Ranked-goal summary formatting stays exhaustive and family-correct after migration -> focused decision-trace summary tests
3. Existing ranking behavior is preserved across migrated families -> focused ranking behavior tests plus `cargo test -p worldwake-ai`
4. Golden tests remain secondary consumers of the contract; they are useful regression proof but not the primary architecture proof surface here
5. Strongest proof surface is focused `worldwake-ai` unit coverage plus full `worldwake-ai` regression coverage; no lower authoritative layer is relevant to this refactor
6. Single-layer `worldwake-ai` ticket; no event-log or action-trace mapping applies

## What to Change

### 1. Extend S36 registration to own provenance-family lookup

Add declaration-owned ranking provenance metadata alongside the existing goal-model dispatch surface. The canonical surface should be able to answer, for a concrete goal:

- whether the family emits no structured provenance, danger provenance, drive provenance, verification provenance, or another future explicit family

This declaration-owned answer is the canonical family-selection path. Do not make `ranking.rs` infer family choice by maintaining a second parallel `match GoalKind` table.

### 2. Move provenance-family selection out of scattered goal matches

Refactor the current scattered logic so:

- [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) no longer chooses provenance family by an ad hoc `match GoalKind`
- [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) continues to render the provenance enum directly rather than becoming a goal-registration dispatch site

The resulting design should keep per-family data truthful and explicit, but centralize family selection so adding a new goal family does not require another round of parallel family-choice edits.

### 3. Keep provenance data explicit and type-safe

This ticket is not permission to collapse everything into untyped strings or a single catch-all map. Preserve explicit structs for family-specific data where they materially differ. The cleanup target is centralized dispatch ownership, not weaker typing.

### 4. Add structural regression coverage

 Add focused tests that fail when:

- a canonical goal-model provenance family is missing for a migrated goal family
- ranking for migrated families falls back to a stale scattered family-choice path
- summary formatting for an existing provenance family regresses after the refactor

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — provenance family declaration lookup alongside existing goal-model dispatch surfaces)
- `crates/worldwake-ai/src/ranking.rs` (modify — replace scattered provenance-family selection with declaration-driven dispatch)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — only if needed to keep provenance rendering tests aligned with the refactor; not as a new registration table)
- `crates/worldwake-ai/src/lib.rs` (modify — export any new declaration/provenance family types if needed)
- `specs/S36-declarative-goal-registration.md` (modify only if implementation requires tightening the spec text to name provenance-family lookup as a declaration-owned deliverable)

## Out of Scope

- New gameplay behavior for any goal family
- Candidate generation, planner search, or authoritative action semantics
- Replacing explicit provenance structs with stringly typed summaries
- Broad refactoring of unrelated AI dispatch sites outside the provenance/summary boundary unless they are required to complete the declarative registration path cleanly

## Acceptance Criteria

### Tests That Must Pass

1. Migrated provenance families resolve through one canonical goal-model family lookup rather than an independent hand-maintained goal-family match in ranking
2. Existing `Danger`, `Drive`, and `Verification` provenance summaries remain correct after migration
3. Adding a new migrated provenance family without wiring its declaration fails compilation or a focused structural test
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

### Invariants

1. Provenance remains explicit and truthful per family; no provenance aliasing across unrelated families
2. Goal-family expansion has one canonical provenance-family registration path instead of scattered parallel family-choice matches
3. Debug summaries remain derived from canonical provenance data, not from goal-registration string fragments or formatter aliases

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — add declaration lookup coverage for concrete goal -> provenance family mapping, including a payload-sensitive `AcquireCommodity` case
2. `crates/worldwake-ai/src/ranking.rs` — add focused structural coverage proving migrated provenance families resolve through declaration-owned family lookup
3. `crates/worldwake-ai/src/decision_trace.rs` — keep or extend focused summary coverage proving provenance families remain exhaustive and correctly rendered

### Commands

1. `cargo test -p worldwake-ai ranking::tests::verify_belief_uses_profile_driven_motive_and_explicit_provenance`
2. `cargo test -p worldwake-ai decision_trace::tests::summary_planning_includes_selected_verification_provenance`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-28
- What actually changed:
  - added a canonical payload-aware provenance-family lookup on `GoalKindPlannerExt`
  - introduced `RankedGoalProvenanceFamily` and routed `ranking.rs` family selection through that single goal-model surface
  - kept provenance summary formatting on `RankedGoalProvenance` rather than moving formatter dispatch into registration metadata
  - added focused goal-model coverage for payload-aware provenance-family selection, including the `AcquireCommodity` `CommodityPurpose` split
- Deviations from original plan:
  - did not move provenance-summary dispatch into declaration metadata because that would couple goal registration to display logic and duplicate responsibility already owned by the provenance enum
  - used a payload-aware goal-model declaration surface instead of `GoalKindTag` alone because `AcquireCommodity` needs `CommodityPurpose` refinement
- Verification results:
  - `cargo test -p worldwake-ai goal_model::tests::ranked_goal_provenance_family_is_payload_aware`
  - `cargo test -p worldwake-ai ranking::tests::verify_belief_uses_profile_driven_motive_and_explicit_provenance`
  - `cargo test -p worldwake-ai decision_trace::tests::summary_planning_includes_selected_verification_provenance`
  - `cargo test -p worldwake-ai`
  - `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
