# S107PRODIV-002: ExploreLocation field type migration — HomeostaticNeedId to ExplorationMotivation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — GoalKind variant field type change across all crates
**Deps**: S107PRODIV-001

## Problem

`ExploreLocation.motivating_need` is typed `HomeostaticNeedId`, which cannot represent proactive (curiosity-driven) exploration. Changing it to `ExplorationMotivation` enables the proactive pathway while preserving all existing need-driven exploration behavior via `ExplorationMotivation::NeedDriven(need_id)`.

## Assumption Reassessment (2026-04-17)

1. `ExploreLocation { target_place: EntityId, motivating_need: HomeostaticNeedId }` confirmed at `goal.rs:115-118`. GoalKey derivation at line 229 uses `{ target_place, .. }` — ignores motivating_need, so GoalKey is unaffected.
2. Key match sites confirmed via grep (306 occurrences across 51 files, but most are in docs/specs/archive). Runtime match sites in AI crate: `goal_model.rs` (9), `goal_dispatch_key.rs` (3), `goal_dispatch_decl.rs` (6), `ranking.rs` (5), `candidate_generation.rs` (8), `feasibility.rs` (1). Systems crate: `travel_actions.rs` (4). Golden tests: `golden_exploration.rs` (19), `golden_survival_baseline.rs` (3), `golden_survival_scattered.rs` (3).
3. All match sites use either `ExploreLocation { target_place, motivating_need }` destructuring or `ExploreLocation { target_place, .. }` wildcard. The mechanical update: wrap `need_id` in `NeedDriven(need_id)` at emission, destructure `NeedDriven(need_id)` at consumption sites.

## Architecture Check

1. Clean type-level separation of exploration motivation. No runtime behavior changes — all existing ExploreLocation logic continues to work identically, just wrapped in `ExplorationMotivation::NeedDriven(...)`.
2. No backward-compatibility shims. The old `HomeostaticNeedId` field is replaced, not aliased.

## Verification Layers

1. Existing golden tests pass with NeedDriven wrapper → golden E2E (confirms no behavioral regression)
2. GoalKey derivation unchanged → focused unit test in goal.rs (existing test at line 760)
3. Candidate emission wraps need_id correctly → existing `emit_exploration_candidates` tests
4. Ranking/dispatch/feasibility updated → compilation + existing golden tests

## What to Change

### 1. Change ExploreLocation field type

In `crates/worldwake-core/src/goal.rs`, change:
```rust
ExploreLocation {
    target_place: EntityId,
    motivating_need: HomeostaticNeedId,  // → ExplorationMotivation
}
```

### 2. Update emit_exploration_candidates

In `crates/worldwake-ai/src/candidate_generation.rs`, wrap `need_id` at emission site (~line 2409):
```rust
motivating_need: ExplorationMotivation::NeedDriven(need_id),
```

### 3. Update all match sites in AI crate

Mechanical updates in:
- `goal_model.rs` — GoalKindPlannerExt methods: destructure `ExplorationMotivation::NeedDriven(need_id)` where `need_id` is used, use `..` where it's ignored
- `goal_dispatch_key.rs` — `from_goal_kind` match arms
- `goal_dispatch_decl.rs` — dispatch declaration matches
- `ranking.rs` — motive_score and priority class: destructure to get inner `HomeostaticNeedId` for need-pressure lookups
- `feasibility.rs` — feasibility check match
- `candidate_generation.rs` — remaining match sites beyond the emission change

### 4. Update travel_actions.rs

In `crates/worldwake-systems/src/travel_actions.rs`, update ExploreLocation match arms (4 sites).

### 5. Update golden tests

In `crates/worldwake-ai/tests/`:
- `golden_exploration.rs` — wrap all `HomeostaticNeedId::*` in `ExplorationMotivation::NeedDriven(...)` (19 sites)
- `golden_survival_baseline.rs` — same wrapping (3 sites)
- `golden_survival_scattered.rs` — same wrapping (3 sites)

### 6. Update GoalKind unit tests

In `crates/worldwake-core/src/goal.rs` tests (lines 760-781), update ExploreLocation construction to use ExplorationMotivation.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify) — field type change + unit tests
- `crates/worldwake-ai/src/goal_model.rs` (modify) — 9 match sites
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify) — 3 match sites
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify) — 6 match sites
- `crates/worldwake-ai/src/ranking.rs` (modify) — 5 match sites
- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — 8 match sites
- `crates/worldwake-ai/src/feasibility.rs` (modify) — 1 match site
- `crates/worldwake-systems/src/travel_actions.rs` (modify) — 4 match sites
- `crates/worldwake-ai/tests/golden_exploration.rs` (modify) — 19 sites
- `crates/worldwake-ai/tests/golden_survival_baseline.rs` (modify) — 3 sites
- `crates/worldwake-ai/tests/golden_survival_scattered.rs` (modify) — 3 sites

## Out of Scope

- Adding proactive exploration emission logic (ticket 006)
- Handling `ExplorationMotivation::Proactive` in ranking (ticket 006)
- Any behavioral changes — this is a pure type migration

## Acceptance Criteria

### Tests That Must Pass

1. All existing golden exploration tests pass with NeedDriven wrapper
2. GoalKey from ExploreLocation still keys on target_place only
3. Existing suite: `cargo test -p worldwake-ai`
4. Existing suite: `cargo test -p worldwake-core`
5. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. GoalKey derivation is unchanged — ExploreLocation keys on target_place, not motivation
2. No behavioral regression — all existing exploration behavior is identical, just type-wrapped
3. ExplorationMotivation::Proactive is defined but not yet emitted by any code path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — update existing GoalKey tests to use ExplorationMotivation
2. `crates/worldwake-ai/tests/golden_exploration.rs` — mechanical NeedDriven wrapping
3. `crates/worldwake-ai/tests/golden_survival_baseline.rs` — mechanical NeedDriven wrapping
4. `crates/worldwake-ai/tests/golden_survival_scattered.rs` — mechanical NeedDriven wrapping

### Commands

1. `cargo test -p worldwake-core -- goal`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
