# E13DECARC-001: Fix worldwake-ai Cargo.toml and scaffold module structure

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E09, E10, E11, E12 (all completed)

## Problem

`worldwake-ai/Cargo.toml` is missing the `worldwake-sim` dependency. The spec explicitly calls this out. E13 depends on `BeliefView`, `Affordance`, `ActionDefRegistry`, `ActionDefId`, `ActionPayload`, `InputEvent`, `ReplanNeeded`, and `OmniscientBeliefView` — all of which live in `worldwake-sim`. The crate's `lib.rs` is also empty (just a doc comment), so we need the initial module declarations.

## Assumption Reassessment (2026-03-11)

1. `worldwake-ai/Cargo.toml` currently depends on `worldwake-core` and `worldwake-systems` only — confirmed.
2. `worldwake-sim` exports `BeliefView`, `Affordance`, `ActionDefRegistry`, `ActionDefId`, `ActionPayload`, `InputEvent`, `InputKind`, `ReplanNeeded`, `OmniscientBeliefView` — confirmed.
3. `worldwake-ai/src/lib.rs` contains only a doc comment, no module declarations — confirmed.

## Architecture Check

1. Adding `worldwake-sim` follows the documented crate dependency graph: `worldwake-ai` depends on `worldwake-core`, `worldwake-sim`, `worldwake-systems`.
2. No shims or aliasing needed — straightforward dep addition.

## What to Change

### 1. Add `worldwake-sim` dependency to `worldwake-ai/Cargo.toml`

Add `worldwake-sim = { path = "../worldwake-sim" }` to `[dependencies]`.

### 2. Scaffold module declarations in `worldwake-ai/src/lib.rs`

Declare the module structure that subsequent tickets will fill in:

```rust
pub mod goal_model;
pub mod utility_profile;
pub mod blocked_intent;
pub mod pressure;
pub mod candidate_generation;
pub mod planner_ops;
pub mod planning_snapshot;
pub mod planning_state;
pub mod search;
pub mod plan_selection;
pub mod plan_revalidation;
pub mod failure_handling;
pub mod interrupts;
pub mod agent_tick;
pub mod decision_runtime;
pub mod budget;
```

Each module file should be created as an empty file (or with a single doc comment) so the crate compiles. The actual implementations come in subsequent tickets.

## Files to Touch

- `crates/worldwake-ai/Cargo.toml` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (new — empty stub)
- `crates/worldwake-ai/src/utility_profile.rs` (new — empty stub)
- `crates/worldwake-ai/src/blocked_intent.rs` (new — empty stub)
- `crates/worldwake-ai/src/pressure.rs` (new — empty stub)
- `crates/worldwake-ai/src/candidate_generation.rs` (new — empty stub)
- `crates/worldwake-ai/src/planner_ops.rs` (new — empty stub)
- `crates/worldwake-ai/src/planning_snapshot.rs` (new — empty stub)
- `crates/worldwake-ai/src/planning_state.rs` (new — empty stub)
- `crates/worldwake-ai/src/search.rs` (new — empty stub)
- `crates/worldwake-ai/src/plan_selection.rs` (new — empty stub)
- `crates/worldwake-ai/src/plan_revalidation.rs` (new — empty stub)
- `crates/worldwake-ai/src/failure_handling.rs` (new — empty stub)
- `crates/worldwake-ai/src/interrupts.rs` (new — empty stub)
- `crates/worldwake-ai/src/agent_tick.rs` (new — empty stub)
- `crates/worldwake-ai/src/decision_runtime.rs` (new — empty stub)
- `crates/worldwake-ai/src/budget.rs` (new — empty stub)

## Out of Scope

- Any actual type definitions or implementations — those belong in subsequent tickets
- Changes to any other crate's Cargo.toml
- Changes to `worldwake-core` or `worldwake-sim`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build --workspace` compiles without errors
2. `cargo test -p worldwake-ai` passes (no tests yet, but must not fail)
3. `cargo clippy --workspace` passes
4. Existing suite: `cargo test --workspace`

### Invariants

1. `worldwake-ai` depends on exactly `worldwake-core`, `worldwake-sim`, `worldwake-systems`
2. No circular dependencies introduced
3. All existing tests in other crates remain green

## Test Plan

### New/Modified Tests

1. No new tests — this is a scaffold ticket.

### Commands

1. `cargo build -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
