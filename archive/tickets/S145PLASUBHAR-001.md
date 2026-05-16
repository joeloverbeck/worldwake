# S145PLASUBHAR-001: Stage-aware strategic budget on ExecutionBudget

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — strategic search budget formula in `worldwake-ai` and a new method on `ExecutionBudget` in `worldwake-core`
**Deps**: None

## Problem

Before this ticket, the strategic search budget at `crates/worldwake-ai/src/search/strategic.rs` was computed by a private helper `strategic_search_budget` as `usize::max(1, usize::from(execution_budget.max_prerequisite_locations()) * 2)`. With the default `max_prerequisite_locations = 3`, every strategic search received 6 expansions regardless of how many stages the goal decomposed into. A 5-stage production chain received the same budget as a 1-stage hunger goal, so longer chains could exhaust the strategic budget at the first wave of permutations and never reach deeper stage permutations. Per S145 D1, the landed formula scales with `stages.len()` so multi-prerequisite acquisition chains receive proportional expansion budget.

## Assumption Reassessment (2026-05-16)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Before implementation, the private helper `strategic_search_budget` existed at `crates/worldwake-ai/src/search/strategic.rs` and was called only by strategic search. The old unit test `strategic_search_budget_tracks_execution_budget_stage_cap` asserted `max(1, ml * 2)` semantics and was removed with the helper. Replacement coverage landed in `crates/worldwake-core/src/execution_budget.rs` for `2 * stages.max(1) * max_prerequisite_locations`.
2. `crates/worldwake-core/src/execution_budget.rs` exposes `ExecutionBudget` as a `Component` with `try_new` rejecting `max_prerequisite_locations == 0` (line 24). The new `strategic_budget_for_stages(stage_count: usize) -> usize` method may rely on this invariant — no defensive `usize::max(1, ...)` floor on `max_prerequisite_locations` is required because zero is unreachable. S145 D1 (Improvement M2 from reassessment) acknowledges this.
3. Shared abstraction boundary: `ExecutionBudget` is per-agent state in `worldwake-core` (used by AI planner via `CognitiveProfile`-driven defaults). Adding a planner-search-specific budget formula as a method on this core type is consistent with the existing `max_prerequisite_locations` accessor — the formula consumes its own field. No new boundary introduced.

## Architecture Check

1. Per FND-28 single-truth, the migration is one ticket: the helper at `strategic.rs:167-175`, its call site at `strategic.rs:113`, and the unit test at `strategic.rs:1004-1017` all change in one diff. No transient state where two formulas coexist.
2. Locating the formula on `ExecutionBudget` rather than keeping a higher-level helper means downstream callers (e.g., S146's per-goal budget composition referenced in `specs/S146-goal-schema-and-per-goal-budgets.md:237`) can consume the same formula directly without reaching into AI-crate internals. This is the substrate hardening S146 depends on.

## Verified Layers

1. Formula correctness across stage counts → focused unit test in `crates/worldwake-core/src/execution_budget.rs` (`strategic_budget_for_stages(0) == 6`, `(1) == 6`, `(3) == 18`, `(5) == 30`, `(8) == 48` with default `max_prerequisite_locations = 3`).
2. Call-site behavioral equivalence on single-stage goals → existing `golden_two_phase_planning.rs` and `golden_planner_pathology.rs` regress unchanged (single-stage budget is unchanged at 6).
3. Single-layer ticket (computation-only): the formula change has no AI-decision-trace or action-trace surface — Verification Layer 6 ("If single-layer ticket, state why additional layer mapping is not applicable") applies. The behavior is a pure function of `(ExecutionBudget, usize)`; the action/event/decision layers do not observe the formula directly.

## Landed Changes

### 1. Added `strategic_budget_for_stages` method to `ExecutionBudget`

In `crates/worldwake-core/src/execution_budget.rs`, this ticket added a `const fn` method to the existing `impl ExecutionBudget` block:

```rust
pub const fn strategic_budget_for_stages(&self, stage_count: usize) -> usize {
    // Treat zero stages as one to preserve the single-stage default expansion
    // budget (with default `max_prerequisite_locations = 3`, a single-stage
    // search receives 2 * 1 * 3 = 6 expansions, identical to the pre-S145
    // formula). No `usize::max(1, ...)` defensive floor is needed because
    // `ExecutionBudget::try_new` rejects `max_prerequisite_locations == 0`.
    let stages = if stage_count == 0 { 1 } else { stage_count };
    2 * stages * self.max_prerequisite_locations() as usize
}
```

### 2. Replaced the call site in strategic search

In `crates/worldwake-ai/src/search/strategic.rs`, strategic search now calls the `ExecutionBudget` method:

```rust
let search_budget = strategic_search_budget(*execution_budget);
```

with:

```rust
let search_budget = execution_budget.strategic_budget_for_stages(stages.len());
```

### 3. Deleted the now-unreachable private helper and its test

Deleted:

- The function `fn strategic_search_budget` at `crates/worldwake-ai/src/search/strategic.rs:167-175`.
- The test `fn strategic_search_budget_tracks_execution_budget_stage_cap` at `crates/worldwake-ai/src/search/strategic.rs:1004-1017`.

### 4. Migrated the unit test to `worldwake-core`

Added `strategic_budget_for_stages_scales_with_stage_count` in the existing `#[cfg(test)] mod tests` block of `crates/worldwake-core/src/execution_budget.rs`. It exercises the spec's D1 table (1→6, 3→18, 5→30, 8→48), the zero-floor case (0→6), and a widened `max_prerequisite_locations = 5` case.

## Landed Files

- `crates/worldwake-core/src/execution_budget.rs` (modify — add method + test)
- `crates/worldwake-ai/src/search/strategic.rs` (modify — replace call site, delete helper + test)

## Out of Scope

- No new `BudgetExhausted` typed terminal for strategic search — per S145 Non-Goals, strategic search continues to return `Option<StrategicPlan>`. That is S149's typed-plan-terminal scope.
- No change to `beam_width`, `max_node_expansions`, or `max_plan_depth` defaults — S146 owns those.
- No `StrategicBudgetTrace` instrumentation here — that is S145PLASUBHAR-002.
- No golden coverage for the 5-stage scaling behavior — that is S145PLASUBHAR-005.

## Acceptance Result

### Passed Tests

1. New unit test in `crates/worldwake-core/src/execution_budget.rs` asserting `strategic_budget_for_stages` across the spec's stage-count table including the zero floor.
2. Existing `cargo test -p worldwake-ai --test golden_two_phase_planning` and `cargo test -p worldwake-ai --test golden_planner_pathology` pass unchanged (single-stage budget invariant preserved).
3. Existing suite: `cargo test --workspace`.

### Invariants

1. With default `ExecutionBudget`, single-stage strategic search receives exactly 6 expansions — identical to pre-S145 behavior.
2. The formula `2 * stages.max(1) * max_prerequisite_locations` is the single authoritative source of the strategic budget; no aliased helper remains in `worldwake-ai`.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/execution_budget.rs` (modify, new test) — covers `strategic_budget_for_stages(0..=8)` with default and widened `max_prerequisite_locations`; replaces the deleted `worldwake-ai` test.
2. `crates/worldwake-ai/src/search/strategic.rs` (modify, delete test) — `strategic_search_budget_tracks_execution_budget_stage_cap` removed in the same change that deletes the helper.

### Verification Commands

1. `cargo test -p worldwake-core execution_budget`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`

## Outcome

Completed on 2026-05-16.

- Added `ExecutionBudget::strategic_budget_for_stages(stage_count)` in `worldwake-core`, using the single authoritative formula `2 * stages.max(1) * max_prerequisite_locations`.
- Updated strategic search to call that method with `stages.len()`.
- Removed the old private `worldwake-ai` helper and its old-formula unit test.
- Added focused `worldwake-core` unit coverage for the default stage-count table plus a widened prerequisite-location case.

## Deviations

- `scripts/verify.sh` was run as the final broad proof instead of separately recording `cargo test --workspace`; the live wrapper includes `cargo test --workspace` plus fmt, repo hygiene scripts, both clippy gates, and scenario coverage.

## Verification Result

- Passed `cargo test -p worldwake-core --lib execution_budget`.
- Passed `cargo test -p worldwake-ai`.
- Passed `scripts/verify.sh`.
