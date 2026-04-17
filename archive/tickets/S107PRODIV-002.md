# S107PRODIV-002: ExploreLocation field type migration — HomeostaticNeedId to ExplorationMotivation

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — GoalKind variant field type change across all crates
**Deps**: archive/tickets/S107PRODIV-001.md

## Problem

`ExploreLocation.motivating_need` is typed `HomeostaticNeedId`, which cannot represent proactive (curiosity-driven) exploration. Changing it to `ExplorationMotivation` enables the proactive pathway while preserving all existing need-driven exploration behavior via `ExplorationMotivation::NeedDriven(need_id)`.

## Assumption Reassessment (2026-04-17)

1. `ExplorationMotivation` already landed in `crates/worldwake-core/src/goal.rs` via `archive/tickets/S107PRODIV-001.md`, but `GoalKind::ExploreLocation { target_place, motivating_need }` still types `motivating_need` as `HomeostaticNeedId`. GoalKey derivation still keys on `{ target_place, .. }`, so GoalKey remains unaffected.
2. The live migration surface is smaller than the drafted grep inventory in some AI modules and broader in literal/test fallout. Real remaining typed/literal consumers are `goal.rs`, `candidate_generation.rs`, `ranking.rs`, `goal_dispatch_decl.rs`, `travel_actions.rs`, `worldwake-cli/src/display.rs`, `goal_model.rs` tests, `agent_tick/tests.rs`, `search/tests.rs`, and golden tests in `crates/worldwake-ai/tests/`.
3. Most runtime match sites already use `..` and need no semantic change. The real mechanical work is: switch the field type to `ExplorationMotivation`, wrap existing need-driven emissions/literals as `ExplorationMotivation::NeedDriven(need_id)`, and update the ranking path to destructure `NeedDriven(need_id)` while keeping `Proactive` intentionally inert until ticket 006.

## Architecture Check

1. Clean type-level separation of exploration motivation. No runtime behavior changes — all existing ExploreLocation logic continues to work identically, just wrapped in `ExplorationMotivation::NeedDriven(...)`.
2. No backward-compatibility shims. The old `HomeostaticNeedId` field is replaced, not aliased.

## Verification Layers

1. Existing golden tests pass with `NeedDriven` wrappers → golden E2E regression proof
2. GoalKey derivation remains keyed only on `target_place` → focused `goal.rs` unit coverage
3. Candidate emission wraps existing reactive need ids correctly → focused `candidate_generation.rs` tests
4. Ranking treats `ExplorationMotivation::Proactive` as intentionally zero/inert until ticket 006 → focused `ranking.rs` coverage
5. Formatter and literal fallout compile cleanly across CLI, systems, and planner-related tests

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

### 3. Update the real downstream consumers

Mechanical updates in the files that still construct or read the field:
- `goal_dispatch_decl.rs` — dispatch sample value uses `ExplorationMotivation::NeedDriven(...)`
- `ranking.rs` — `motive_score` and `exploration_motive` handle `ExplorationMotivation`
- `candidate_generation.rs` — reactive emission and focused tests wrap existing need ids
- `travel_actions.rs`, `goal_model.rs` tests, `agent_tick/tests.rs`, `search/tests.rs`, and golden tests — update `ExploreLocation` literals
- `worldwake-cli/src/display.rs` — keep formatter output correct for the new enum payload

### 4. Update golden and focused tests

In `crates/worldwake-ai/tests/`:
- `golden_exploration.rs` — wrap `HomeostaticNeedId::*` in `ExplorationMotivation::NeedDriven(...)`
- `golden_survival_baseline.rs` / `golden_survival_scattered.rs` — keep wildcard assertions compiling against the new field type
- `goal_model.rs`, `agent_tick/tests.rs`, and `search/tests.rs` — update remaining `ExploreLocation` test literals
- `ranking.rs` — add focused proof that `ExplorationMotivation::Proactive` stays inert for now

### 5. Update GoalKind unit tests

In `crates/worldwake-core/src/goal.rs` tests, update `ExploreLocation` construction to use `ExplorationMotivation`.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify) — field type change + unit tests
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify) — dispatch sample literal
- `crates/worldwake-ai/src/ranking.rs` (modify) — motivation handling + focused tests
- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — reactive emission + focused tests
- `crates/worldwake-ai/src/goal_model.rs` (modify) — focused test literals
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — focused test literal
- `crates/worldwake-ai/src/search/tests.rs` (modify) — focused test literal
- `crates/worldwake-systems/src/travel_actions.rs` (modify) — test literals
- `crates/worldwake-ai/tests/golden_exploration.rs` (modify)

## Out of Scope

- Adding proactive exploration emission logic (ticket 006)
- Making `ExplorationMotivation::Proactive` produce nonzero exploration motive (ticket 006)
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
2. No behavioral regression — existing need-driven exploration stays behaviorally identical, just type-wrapped
3. `ExplorationMotivation::Proactive` remains defined but inert on this branch until ticket 006

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — update existing GoalKey / bincode tests to use `ExplorationMotivation`
2. `crates/worldwake-ai/src/candidate_generation.rs` — existing exploration candidate tests assert `NeedDriven(...)`
3. `crates/worldwake-ai/src/ranking.rs` — focused proof that `NeedDriven(...)` still uses need pressure and `Proactive` stays zero
4. `crates/worldwake-ai/tests/golden_exploration.rs` — mechanical `NeedDriven(...)` wrapping

### Commands

1. `cargo test -p worldwake-core goal_key_extracts_target_place_for_explore_location`
2. `cargo test -p worldwake-ai explore_location`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-17.

- Changed `GoalKind::ExploreLocation.motivating_need` from `HomeostaticNeedId` to `ExplorationMotivation` in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs).
- Updated reactive exploration emission and all remaining explicit `ExploreLocation` literals to wrap existing needs as `ExplorationMotivation::NeedDriven(...)` across the AI, systems, and golden-test fallout that still constructed the old field shape.
- Updated [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) so need-driven exploration keeps using need pressure while `ExplorationMotivation::Proactive` remains intentionally zero/inert until ticket 006.
- Reassessment narrowed the actual edit set: `worldwake-cli/src/display.rs`, `golden_survival_baseline.rs`, and `golden_survival_scattered.rs` compiled unchanged and did not require code edits.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-core --lib goal::tests::goal_key_extracts_target_place_for_explore_location -- --exact`
- Passed `cargo test -p worldwake-ai --lib ranking::tests::explore_location_motive_uses_need_pressure_times_curiosity -- --exact`
- Passed `cargo test -p worldwake-ai --lib ranking::tests::explore_location_proactive_motive_stays_zero_until_proactive_ranking_lands -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_skips_exploration_when_consecutive_limit_reached -- --exact`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
