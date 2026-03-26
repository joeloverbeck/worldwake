# E17CRITHEJUS-004: Add crime/justice GoalKind variants and shared goal-model totality

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `GoalKind`/`GoalKey` extension in core plus minimal AI totality updates on the shared goal-model boundary
**Deps**: [archive/tickets/E17CRITHEJUS-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E17CRITHEJUS-001.md), [archive/tickets/completed/E17CRITHEJUS-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/E17CRITHEJUS-002.md)

## Problem

The E17 spec requires three new goals, but the live shared goal contract still has no `GoalKind::StealItem`, `GoalKind::Accuse`, or `GoalKind::PunishAccused`. That blocks not only future crime/justice candidate generation, but also any clean extension of the AI pipeline because the authoritative goal identity and the AI goal-model surface diverge.

## Assumption Reassessment (2026-03-26)

1. The exact shared abstraction boundary under audit is the typed goal contract spanning [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) and the AI-side `GoalKindTag` / `GoalKindPlannerExt` totality in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs). This is not a core-only change.
2. `PunishmentKind` already exists in [crates/worldwake-core/src/crime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/crime.rs). The original live dependency note saying this ticket "needs `PunishmentKind`" is stale; that type is already delivered.
3. `ViolationId` already exists and is already used by `GoalKind::InvestigateViolation` in [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs). The new accusation goal can depend on the existing type directly.
4. `GoalKey` identity is derived from `GoalKind` via exhaustive `From<GoalKind>` / `From<&GoalKind>` impls in [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs). Adding new goal kinds without updating this mapping would break blocked-intent identity and canonical goal deduplication.
5. The live AI architecture does not yet have `PlannerOpKind::{Steal, Accuse, Fine, Exile}` in [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs). Adding fake operators here would overlap E17CRITHEJUS-005 and the later action tickets instead of keeping the layering honest.
6. Because no current candidate-generation path emits these goals yet, the clean architecture for this ticket is to make the new goal kinds explicit and total across the shared goal-model surface while keeping them intentionally unsupported for planning until their real operators exist. That is better than scattering placeholder arms that imply planner support already exists.
7. The relevant E17 spec path is [specs/E17-crime-theft-justice.md](/home/joeloverbeck/projects/worldwake/specs/E17-crime-theft-justice.md). The original ticket cited non-existent spec filenames.
8. `cargo test -p worldwake-ai -- --list` succeeds on the live repo and confirms the current unit/golden targets. Focused verification for this ticket should stay in core `goal.rs` and AI goal-model/policy/feasibility/ranking unit tests, then expand to workspace build/lint.

## Architecture Check

1. The cleanest architecture is to extend the shared goal type and the AI goal-model boundary together, so every live exhaustive surface knows these goals exist even before they become plannable. That preserves one authoritative representation of goal meaning and avoids temporary alias paths.
2. This ticket should not invent dummy planner operators or pretend the goals are already executable. Explicit unsupported semantics with exact binding identity is cleaner than placeholder actions because it keeps future E17 work additive and honest.
3. `GoalKey` should keep using canonical entity identity for these goals: the stolen item for theft, and the accused agent for accusation/punishment. That matches the existing exact-bound-goal pattern and gives blocked-memory deduplication a stable concrete target.

## Verification Layers

1. Core goal identity and serde for `StealItem` / `Accuse` / `PunishAccused` -> focused unit tests in [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs)
2. AI goal-model totality (`GoalKindTag`, relevant-op dispatch, satisfaction/binding handling) -> focused unit tests in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
3. Goal-family policy remains exhaustive for new goals without introducing reactive/critical behavior -> focused unit tests in [crates/worldwake-ai/src/goal_policy.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs)
4. Ranking and feasibility exhaustiveness for new goals -> focused unit tests in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) and [crates/worldwake-ai/src/feasibility.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/feasibility.rs)
5. Cross-crate exhaustiveness remains intact -> `cargo build --workspace`

## What to Change

### 1. Extend `GoalKind` and `GoalKey` in `goal.rs`

Add:

```rust
StealItem { target_item: EntityId },
Accuse { accused: EntityId, violation_id: ViolationId },
PunishAccused { accused: EntityId, punishment: PunishmentKind },
```

Canonical `GoalKey` extraction:

- `StealItem { target_item }` -> `GoalKey::entity = Some(target_item)`
- `Accuse { accused, .. }` -> `GoalKey::entity = Some(accused)`
- `PunishAccused { accused, .. }` -> `GoalKey::entity = Some(accused)`

### 2. Extend the AI goal-model boundary in `goal_model.rs`

Add three `GoalKindTag` variants and make every exhaustive `GoalKindPlannerExt` surface total for them:

- `goal_kind_tag()`
- `relevant_op_kinds()` -> explicit empty slice for now
- `relevant_observed_commodities()` -> empty set
- `is_satisfied()` -> `false`
- `goal_relevant_places()` / `prerequisite_places()` -> conservative deferred behavior
- `matches_binding()` -> exact-bound on stolen item or accused entity

This ticket does **not** add real crime/justice planner operators. The new goals become first-class but intentionally unplanned until the later tickets add the real action/operator path.

### 3. Update exhaustive AI policy/ranking/feasibility matches

Make [crates/worldwake-ai/src/goal_policy.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs), [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), and [crates/worldwake-ai/src/feasibility.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/feasibility.rs) total for the new goal kinds with conservative deferred behavior appropriate for "typed but not yet executable" goals.

## Files to Touch

- [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs)
- [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [crates/worldwake-ai/src/goal_policy.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs)
- [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
- [crates/worldwake-ai/src/feasibility.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/feasibility.rs)

## Out of Scope

- `PlannerOpKind::{Steal, Accuse, Fine, Exile}` and their transition semantics — E17CRITHEJUS-005 plus the action tickets
- Theft/justice candidate generation — E17CRITHEJUS-010 and E17CRITHEJUS-011
- Authoritative action definitions and handlers — E17CRITHEJUS-006, E17CRITHEJUS-008, E17CRITHEJUS-009
- Golden crime scenarios — E17CRITHEJUS-012 and E17CRITHEJUS-013

## Acceptance Criteria

### Tests That Must Pass

1. `GoalKind::StealItem` bincode round-trip passes
2. `GoalKind::Accuse` bincode round-trip passes
3. `GoalKind::PunishAccused` bincode round-trip passes for both `Fine` and `Exile`
4. `GoalKey` extracts the canonical entity for each new goal kind
5. `GoalKindTag` and `GoalKindPlannerExt` are total for the new goal kinds
6. `matches_binding()` accepts the correct target and rejects the wrong target for the new exact-bound goals
7. `cargo test -p worldwake-core`
8. `cargo test -p worldwake-ai goal_model::`
9. `cargo test -p worldwake-ai goal_policy::`
10. `cargo test -p worldwake-ai ranking::`
11. `cargo test -p worldwake-ai feasibility::`
12. `cargo build --workspace`

### Invariants

1. `GoalKind` remains a deterministic serializable value type
2. `GoalKey` extraction stays total and canonical
3. No fake planner operators or backward-compatibility alias paths are introduced
4. Existing non-crime goal behavior stays unchanged

## Tests

### New/Modified Tests

1. [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) — add serde and `GoalKey` coverage for all three new goal kinds.
Rationale: this is the authoritative goal-identity surface the rest of the stack depends on.

2. [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) — add `GoalKindTag`, relevant-op, and exact-binding coverage for the new goals.
Rationale: this proves the AI goal-model boundary is total without pretending the goals are already executable.

3. [crates/worldwake-ai/src/goal_policy.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs), [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), and [crates/worldwake-ai/src/feasibility.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/feasibility.rs) — add smoke coverage for the new exhaustive branches.
Rationale: these files currently participate in the live exhaustive `GoalKind` surface and must compile with explicit behavior for the new goals.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai goal_model::`
3. `cargo test -p worldwake-ai goal_policy::`
4. `cargo test -p worldwake-ai ranking::`
5. `cargo test -p worldwake-ai feasibility::`
6. `cargo build --workspace`
7. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-26
- What actually changed:
  - Added `GoalKind::StealItem`, `GoalKind::Accuse`, and `GoalKind::PunishAccused` plus canonical `GoalKey` extraction in core.
  - Extended the AI shared goal-model surface so the new goals are total across `GoalKindTag`, binding, ranking, policy, feasibility, and goal-model helper dispatch.
  - Added focused tests covering core serde/identity plus AI tag, binding, ranking, policy, and feasibility behavior.
- Deviations from original plan:
  - The original ticket claimed this was effectively core-only with placeholder AI match arms. After reassessment, the clean implementation required explicit AI totality work, but it intentionally did not add fake planner operators or executable crime/justice actions.
  - The stale dependency assumption on `PunishmentKind` was removed because that type already existed.
  - The stale spec references were corrected to `specs/E17-crime-theft-justice.md`.
- Verification results:
  - `cargo test -p worldwake-core` passed.
  - `cargo test -p worldwake-ai goal_model::` passed.
  - `cargo test -p worldwake-ai goal_policy::` passed.
  - `cargo test -p worldwake-ai ranking::` passed.
  - `cargo test -p worldwake-ai feasibility::` passed.
  - `cargo test -p worldwake-ai` passed.
  - `cargo build --workspace` passed.
  - `cargo clippy --workspace` passed.
