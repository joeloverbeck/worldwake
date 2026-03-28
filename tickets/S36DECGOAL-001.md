# S36DECGOAL-001: Declarative ranking provenance families

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai`: consolidate ranking provenance construction and summary dispatch under declarative goal registration
**Deps**: [specs/S36-declarative-goal-registration.md](/home/joeloverbeck/projects/worldwake/specs/S36-declarative-goal-registration.md), [archive/tickets/completed/S34GENEPIACT-007.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-007.md)

## Problem

`RankedGoalProvenance` is now a real architectural surface, not an incidental detail. After `VerifyBelief` landed explicit provenance, adding a new ranking family requires parallel edits across provenance data shape, ranking dispatch, summary formatting, exports, and exhaustive tests. That is the same scattered-goal-dispatch smell S36 is already trying to remove. If left alone, every new non-drive/non-danger goal family will repeat the same manual enum-growth pattern and weaken the “single source of truth” target for AI dispatch and debugability.

## Assumption Reassessment (2026-03-28)

1. The live ranking provenance contract now spans [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) (`RankedGoalProvenance`, `RankedDriveGoalProvenance`, `RankedVerificationGoalProvenance`), [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) (`goal_ranking_provenance`, `ranked_priority_class`, `ranked_motive_score`), and [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) (`format_ranked_goal_provenance_summary`).
2. The current code already has three provenance families: `Danger`, `Drive`, and `Verification`. Each required manual edits in more than one file, and exhaustive test matches in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) and [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs) had to be updated after the enum grew.
3. The shared abstraction boundary under audit is the AI-layer goal-registration/ranking/traceability contract, not authoritative world logic. This is a structural `worldwake-ai` ticket. No `worldwake-sim` or `worldwake-systems` behavior change is required.
4. Coverage gap classification after reassessment:
   - focused ranking/unit coverage exists in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
   - focused decision-trace summary coverage exists in [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
   - golden coverage indirectly consumes provenance in [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs)
   - no current focused ticket owns the structural provenance-dispatch cleanup
5. [specs/S36-declarative-goal-registration.md](/home/joeloverbeck/projects/worldwake/specs/S36-declarative-goal-registration.md) already identifies ranking family and trace label as declaration-owned properties, but it does not yet include ranking provenance construction or provenance summary dispatch as first-class declaration surfaces. That is the exact architectural gap exposed by `S34GENEPIACT-007`.
6. This is not a golden-driven behavior ticket. The intended verification layer is focused unit coverage plus full `worldwake-ai` regression coverage. Golden tests should remain consumers of the stronger structural contract, not the primary proof surface.
7. No ordering-sensitive gameplay contract is under audit here. The problem is compile-time and structural completeness of ranking provenance dispatch.
8. This ticket does not remove a heuristic or filter. It removes scattered dispatch duplication standing in for a missing declarative substrate.
9. Adjacent contradiction classification: this is not a required consequence for S34 functionality, but it is a follow-up cleanup that now has a concrete live trigger and should be made explicit rather than left as informal “future refactor” commentary.
10. Mismatch + correction: S36 currently frames ranking cleanup mostly as “ranking family” and “trace label” registration. Reassessment shows that provenance shape and provenance-summary dispatch belong in that same registration story or S36 will still leave goal-family expansion partially scattered.

## Architecture Check

1. The clean fix is to make provenance family a declaration-owned concept, not another ad hoc `match GoalKind` and `match RankedGoalProvenance` pair. Goal registration should own not just “which broad ranking family” a goal uses, but also which provenance family it emits and how that provenance is rendered for traces.
2. No backwards-compatibility aliasing or fallback string paths. Do not keep both declaration-driven provenance dispatch and the old scattered enum/match tables alive. Replace the old path once the declarative path lands.

## Verification Layers

1. Every `GoalKindTag` with explicit provenance family resolves through one declaration-owned path -> focused unit tests on the declaration lookup and ranking dispatch
2. Ranked-goal summary formatting stays exhaustive and family-correct after migration -> focused decision-trace summary tests
3. Existing ranking behavior is preserved across migrated families -> focused ranking behavior tests plus `cargo test -p worldwake-ai`
4. Golden tests are not the primary proof surface because the contract under audit is structural dispatch completeness, not scenario behavior
5. Strongest proof surface is focused AI-unit coverage with exhaustive declaration lookup; no lower authoritative layer is relevant to this refactor
6. Single-layer `worldwake-ai` ticket; no additional event-log or action-trace mapping applies

## What to Change

### 1. Extend S36 registration to own provenance family

Add declaration-owned ranking provenance metadata alongside the existing ranking family declaration. The declaration should be able to answer, for a `GoalKindTag`:

- whether the family emits no structured provenance, danger provenance, drive provenance, verification provenance, or another future explicit family
- which summary formatter or summary fragment implementation applies to that family

Do not make `decision_trace.rs` the place where new provenance families are “registered” by hand.

### 2. Move provenance-family dispatch out of scattered matches

Refactor the current scattered logic so:

- [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) does not need independent family-choice matches for provenance, ranked priority, and ranked motive
- [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) no longer acts as a second manual dispatch table for provenance families

The resulting design should keep per-family data truthful and explicit, but centralize family selection so adding a new goal family does not require another round of parallel edits.

### 3. Keep provenance data explicit and type-safe

This ticket is not permission to collapse everything into untyped strings or a single catch-all map. Preserve explicit structs for family-specific data where they materially differ. The cleanup target is centralized dispatch ownership, not weaker typing.

### 4. Add structural regression coverage

Add focused tests that fail when:

- a declaration-owned provenance family is missing for a migrated goal family
- summary formatting for a migrated provenance family is not wired
- ranking for migrated families falls back to a stale scattered path

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — provenance family declarations and explicit provenance data ownership)
- `crates/worldwake-ai/src/ranking.rs` (modify — replace scattered provenance-family selection with declaration-driven dispatch)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — remove manual provenance-family registration by hand)
- `crates/worldwake-ai/src/lib.rs` (modify — export any new declaration/provenance family types if needed)
- `specs/S36-declarative-goal-registration.md` (modify only if implementation requires tightening the spec text to name provenance family as a declaration-owned deliverable)

## Out of Scope

- New gameplay behavior for any goal family
- Candidate generation, planner search, or authoritative action semantics
- Replacing explicit provenance structs with stringly typed summaries
- Broad refactoring of unrelated AI dispatch sites outside the provenance/summary boundary unless they are required to complete the declarative registration path cleanly

## Acceptance Criteria

### Tests That Must Pass

1. Migrated provenance families resolve through declaration-owned dispatch rather than independent hand-maintained matches in both ranking and decision-trace summary formatting
2. Existing `Danger`, `Drive`, and `Verification` provenance summaries remain correct after migration
3. Adding a new migrated provenance family without wiring its declaration fails compilation or a focused structural test
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

### Invariants

1. Provenance remains explicit and truthful per family; no provenance aliasing across unrelated families
2. Goal-family expansion has one canonical provenance-dispatch registration path instead of scattered parallel tables
3. Debug summaries remain derived from canonical provenance data, not a second ad hoc source of truth

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — add focused structural coverage proving migrated provenance families resolve through declaration-owned dispatch
2. `crates/worldwake-ai/src/decision_trace.rs` — add focused summary coverage proving migrated provenance families remain exhaustive and correctly rendered
3. `crates/worldwake-ai/src/goal_model.rs` — add declaration lookup coverage or compile-time completeness checks for migrated provenance families

### Commands

1. `cargo test -p worldwake-ai ranking::tests`
2. `cargo test -p worldwake-ai decision_trace::tests`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
