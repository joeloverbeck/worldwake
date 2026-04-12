# S95RELPLAHEU-003: RPG types and `compute_ff_heuristic` algorithm

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new types and function in worldwake-ai search module
**Deps**: S95 spec

## Problem

The current tactical search uses landmark count as its heuristic, which provides coarse distance estimates. An FF-style relaxed-plan heuristic computes a more informative plan-distance estimate by building and extracting a relaxed plan, and identifies "helpful actions" — a more selective preferred-operator set. This ticket implements the core RPG algorithm without integrating it into the search loop.

## Assumption Reassessment (2026-04-12)

1. `PlanningFact` (enum, `landmarks.rs:12-19`), `PlanningOperator` (struct with `preconditions`, `add_effects`, `del_effects` as `BTreeSet<PlanningFact>`, `landmarks.rs:22-26`), and `LandmarkSet` (`landmarks.rs:29-32`) exist. The RPG algorithm reuses these types directly.
2. `planning_facts_from_state` exists at `landmarks.rs:40` with signature `fn(&PlanningState<'_>) -> BTreeSet<PlanningFact>`. The RPG function takes `&BTreeSet<PlanningFact>` as input, so it operates on already-extracted facts.
3. `PlanningFact` derives `Clone, Debug, Eq, Ord, PartialEq, PartialOrd` — sufficient for the `BTreeSet`/`BTreeMap`-based deterministic implementation this ticket owns. No `Hash` derivation is present or needed.
4. `crates/worldwake-ai/src/search/landmarks.rs` already owns focused `#[cfg(test)]` coverage for planning-fact helpers and preferred-operator behavior, so the new RPG proof belongs in that existing test module rather than a new test file.

## Architecture Check

1. The RPG algorithm is a pure function: `(facts, goals, operators) -> Option<result>`. No side effects, no state mutation, no world interaction. This makes it trivially testable, deterministic, and composable with the existing search infrastructure.
2. Placing the types and function in `landmarks.rs` alongside `PlanningFact` and `PlanningOperator` keeps the planning-fact vocabulary in one module. The `pub(super)` visibility matches existing types.
3. No backward-compatibility shims. New code only.

## Verification Layers

1. RPG correctness (forward phase reaches goals) → unit tests with known operator graphs
2. Relaxed plan extraction (backward phase selects correct operators) → unit tests verifying h_ff values
3. Helpful action selectivity (only layer-0 operators in set) → unit test verifying index membership
4. Dead-end detection (None return) → unit test with unreachable goals
5. Determinism → unit test verifying identical output across runs
6. Single-layer ticket — algorithm only, no search integration.

## What to Change

### 1. Add RelaxedPlanResult type

In `crates/worldwake-ai/src/search/landmarks.rs`:

```rust
/// Result of building an RPG and extracting a relaxed plan.
pub(super) struct RelaxedPlanResult {
    /// Number of operators in the extracted relaxed plan (delete-relaxed
    /// plan length). This is the h_ff heuristic value.
    pub(super) h_ff: u32,
    /// Indices into the operators slice for layer-0 operators whose
    /// add_effects were used by the relaxed plan. These are "helpful
    /// actions" — candidates that make immediate progress toward the goal
    /// under delete-relaxation.
    pub(super) helpful_action_indices: BTreeSet<usize>,
}
```

### 2. Implement `compute_ff_heuristic`

```rust
pub(super) fn compute_ff_heuristic(
    initial_facts: &BTreeSet<PlanningFact>,
    goal_facts: &BTreeSet<PlanningFact>,
    operators: &[PlanningOperator],
) -> Option<RelaxedPlanResult>
```

**Forward phase**: Build accumulated fact sets layer by layer. Each layer applies all operators whose preconditions are subset of accumulated facts (delete-relaxation — ignore `del_effects`). Record `first_achiever: BTreeMap<PlanningFact, (u8, usize)>` for each newly achieved fact. Stop when all goal facts reached or no new facts added (return `None`). Max depth bounded by `operators.len()`.

**Backward phase**: Starting from goal facts, trace `first_achiever` backward to layer 0. Count distinct selected operators as `h_ff`. Collect layer-0 operators whose `add_effects` were used as `helpful_action_indices`.

**Edge cases**:
- Goal facts are subset of initial facts → `h_ff = 0`, empty helpful actions
- No operators and goal not in initial → `None`
- Empty goal facts → `h_ff = 0`, empty helpful actions

### 3. Unit tests

Add `#[cfg(test)]` module in `landmarks.rs` with tests 1-6 from the spec:

1. **RPG fixpoint — no operators**: No operators + unmet goals → `None`
2. **Goal already satisfied**: Goals subset of initial → `h_ff = 0`, empty helpful
3. **Linear chain**: A→B→C with two operators → `h_ff = 2`, helpful = {layer-0 operator}
4. **Delete-relaxation correctness**: Operator with del_effects on needed fact still allows parallel achievement → verify h_ff
5. **Helpful action selectivity**: Only layer-0 used operators in helpful set
6. **Determinism**: Same inputs produce same output across multiple calls

## Files to Touch

- `crates/worldwake-ai/src/search/landmarks.rs` (modify)

## Out of Scope

- Search loop integration (ticket 004)
- CognitiveProfile field (ticket 001)
- Decision trace fields (ticket 002)
- Per-successor RPG computation (spec Non-Goal)
- Cached/precomputed RPGs (spec Non-Goal)

## Acceptance Criteria

### Tests That Must Pass

1. `compute_ff_heuristic` returns `None` for unreachable goals
2. `compute_ff_heuristic` returns `h_ff = 0` for already-satisfied goals
3. `compute_ff_heuristic` returns correct `h_ff` for linear operator chains
4. Delete-relaxation ignores del_effects correctly
5. Helpful actions contain only layer-0 operators from the relaxed plan
6. Determinism: identical results across repeated calls
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. All iteration uses `BTreeSet`/`BTreeMap` — deterministic ordering
2. No floats — integer arithmetic only
3. `pub(super)` visibility — not exposed outside search module

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/landmarks.rs` (new `#[cfg(test)]` tests) — 6 unit tests covering RPG correctness, edge cases, and determinism

### Commands

1. `cargo test -p worldwake-ai -- landmarks`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-12.

- Added `RelaxedPlanResult` and `compute_ff_heuristic` to `crates/worldwake-ai/src/search/landmarks.rs` as deterministic, planner-internal RPG scaffolding for ticket 004.
- Implemented forward relaxed-graph expansion with accumulated fact layers, first-achiever tracking, dead-end detection, and backward relaxed-plan extraction that returns `h_ff` plus layer-0 helpful action indices.
- Extended the existing `landmarks.rs` unit-test module with focused coverage for unreachable goals, already-satisfied goals, linear relaxed plans, delete-relaxation behavior, helpful-action selectivity, and determinism.
- Marked the new helper surface `#[allow(dead_code)]` on landing because ticket 004 still owns wiring the algorithm into the live search loop and CI-matching clippy forbids leaving staged shared helpers unintentionally unused.

## Deviations

- Reassessment corrected the ticket's type-shape claim: `PlanningFact` is ordered for `BTreeSet`/`BTreeMap` use and does not derive `Hash`.

## Verification Result

- Passed `cargo test -p worldwake-ai -- landmarks`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
