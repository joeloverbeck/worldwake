# S12PLAPREAWA-004: Update `agent_tick.rs` call site for new `search_plan()` signature

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — call site change in `agent_tick.rs`
**Deps**: S12PLAPREAWA-003 (new `search_plan()` signature)

## Problem

After S12PLAPREAWA-003 changes `search_plan()` to take `recipes: &RecipeRegistry` instead of `goal_relevant_places: &[EntityId]`, the single call site in `agent_tick.rs` must be updated. The pre-search `goal_relevant_places` computation (lines ~1034-1038) becomes unnecessary and should be removed.

## Assumption Reassessment (2026-03-21)

1. `search_plan()` is called exactly once in the codebase, at `crates/worldwake-ai/src/agent_tick.rs` — confirmed via grep.
2. The call site computes `goal_relevant_places` via `ranked.grounded.key.kind.goal_relevant_places(&PlanningState::new(&snapshot), recipe_registry)` and passes it as `&goal_relevant_places` — confirmed.
3. `recipe_registry` is already in scope at the call site (passed to `AgentTickDriver::tick()`) — confirmed.
4. `budget` is already in scope at the call site — confirmed.
5. After S12PLAPREAWA-003, `search_plan()` no longer accepts `goal_relevant_places: &[EntityId]` — it accepts `recipes: &RecipeRegistry` instead.
6. Single-layer call site update — no AI regression, ordering, or heuristic concerns.

## Architecture Check

1. Removing the pre-computed `goal_relevant_places` and passing `recipe_registry` directly is the minimal change. The computation now happens per-node inside `search_plan()` via `combined_relevant_places()`.
2. No backwards-compatibility shims. The old local variable is removed.

## Verification Layers

1. Call site compiles and passes `recipe_registry` → compilation check
2. All existing golden tests pass with unchanged behavior for goals without prerequisites → `cargo test -p worldwake-ai`
3. Single-layer ticket — verification is compilation + existing test suite.

## What to Change

### 1. Remove pre-search `goal_relevant_places` computation

In `crates/worldwake-ai/src/agent_tick.rs`, remove the lines that compute:

```rust
let goal_relevant_places = ranked
    .grounded
    .key
    .kind
    .goal_relevant_places(&crate::PlanningState::new(&snapshot), recipe_registry);
```

### 2. Update `search_plan()` call

Replace `&goal_relevant_places` argument with `recipe_registry`:

```rust
let result = search_plan(
    &snapshot,
    &ranked.grounded,
    semantics_table,
    action_defs,
    action_handlers,
    budget,
    recipe_registry,  // was: &goal_relevant_places
    ...
);
```

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify)

## Out of Scope

- `search_plan()` internals (S12PLAPREAWA-003)
- `goal_model.rs` changes (S12PLAPREAWA-002)
- `budget.rs` changes (S12PLAPREAWA-001)
- Decision trace changes (S12PLAPREAWA-005)
- Golden tests (S12PLAPREAWA-007)
- Any other file

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build -p worldwake-ai` — compilation succeeds
2. Existing suite: `cargo test -p worldwake-ai`
3. Existing suite: `cargo test --workspace`

### Invariants

1. `search_plan()` is called with the correct new signature
2. No stale `goal_relevant_places` variable remains in `agent_tick.rs`
3. `recipe_registry` reference passed to `search_plan()` has the correct lifetime

## Test Plan

### New/Modified Tests

1. None — call site update only; verification is compilation and existing test coverage.

### Commands

1. `cargo build -p worldwake-ai`
2. `cargo test --workspace && cargo clippy --workspace`

## Implementation Note

This ticket MUST be implemented together with S12PLAPREAWA-003 in the same commit — the signature change and call site update are an atomic compilation unit. Implementing one without the other will not compile.
