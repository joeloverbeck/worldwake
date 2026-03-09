# E07ACTFRA-005: Action Handler Function Table + HandlerRegistry

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — defines handler execution contract
**Deps**: E07ACTFRA-001 (IDs), E07ACTFRA-004 (ActionInstance, ActionState)

## Problem

Action handlers contain the executable logic for each action type. The spec requires that active actions store only IDs and serializable state — never function pointers or references. Handlers must mutate world state only through `WorldTxn`. A deterministic registry maps `ActionHandlerId` to handler logic with stable iteration order.

## Assumption Reassessment (2026-03-09)

1. `WorldTxn` exists in `worldwake-core/src/world_txn.rs` — it takes a mutable `World` reference and emits events.
2. `ActionHandlerId(u32)` will be defined in E07ACTFRA-001.
3. `ActionInstance` and `ActionState` will be defined in E07ACTFRA-004.
4. Handlers are registered statically — the registry does not change at runtime.
5. Concrete handlers are static code, not data, so the registry can use plain function pointers without leaking runtime state into dispatch.

## Architecture Check

1. The handler contract should be an explicit function table, not a trait-object registry. Trait objects allow hidden per-handler state and indirection that the action framework does not need in Phase 1.
2. Registry uses a `Vec<ActionHandler>` indexed by `ActionHandlerId` for O(1) lookup.
3. Each lifecycle hook is a plain function pointer. That keeps dispatch deterministic, testable, and honest: handlers are static code selected by ID, not mini-objects with private state.

## What to Change

### 1. Create `worldwake-sim/src/action_handler.rs`

Define function pointer aliases plus an `ActionHandler` table:
```rust
pub type ActionStartFn =
    for<'w> fn(&ActionInstance, &mut WorldTxn<'w>) -> Result<Option<ActionState>, ActionError>;
pub type ActionTickFn =
    for<'w> fn(&ActionInstance, &mut WorldTxn<'w>) -> Result<ActionProgress, ActionError>;
pub type ActionCommitFn =
    for<'w> fn(&ActionInstance, &mut WorldTxn<'w>) -> Result<(), ActionError>;
pub type ActionAbortFn =
    for<'w> fn(&ActionInstance, &AbortReason, &mut WorldTxn<'w>) -> Result<(), ActionError>;

pub struct ActionHandler {
    pub on_start: ActionStartFn,
    pub on_tick: ActionTickFn,
    pub on_commit: ActionCommitFn,
    pub on_abort: ActionAbortFn,
}
```

Define supporting types:
```rust
pub enum ActionProgress {
    Continue,
    Complete,
}

pub enum ActionError {
    PreconditionFailed(String),
    ReservationUnavailable,
    InvalidTarget(EntityId),
    InternalError(String),
}

pub enum AbortReason {
    CommitConditionFailed(String),
    Interrupted(String),
    ExternalAbort(String),
}
```

### 2. Create `worldwake-sim/src/action_handler_registry.rs`

Define `ActionHandlerRegistry`:
- `register(handler: ActionHandler) -> ActionHandlerId`
- `get(id: ActionHandlerId) -> Option<&ActionHandler>`
- `len()` / `is_empty()`
- `iter() -> impl Iterator<Item = &ActionHandler>`

Iteration order is stable (insertion order).

**Note**: `ActionHandlerRegistry` is NOT serializable — it holds executable function pointers. It is reconstructed on load from the same static registration code. This is by design: handlers are code, not data.

### 3. Update `worldwake-sim/src/lib.rs`

Declare modules, re-export public types.

## Files to Touch

- `crates/worldwake-sim/src/action_handler.rs` (new)
- `crates/worldwake-sim/src/action_handler_registry.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- Concrete handler implementations for specific actions (later epics: trade, combat, etc.)
- Start gate or commit validation orchestration (E07ACTFRA-008, E07ACTFRA-009)
- KnowledgeView / affordance query (E07ACTFRA-006, E07ACTFRA-007)
- ActionDef (E07ACTFRA-003)

## Acceptance Criteria

### Tests That Must Pass

1. A test `ActionHandler` function table can be registered and retrieved by ID
2. Registry assigns sequential `ActionHandlerId` values
3. `get()` returns the correct handler for each registered ID
4. Handler `on_commit` can mutate world state through `WorldTxn` (integration test with real `WorldTxn` and free functions)
5. `ActionError` and `AbortReason` satisfy `Clone + Debug + Eq`
6. `ActionProgress` satisfies `Copy + Clone + Debug + Eq`
7. `ActionHandler` lifecycle hooks are callable through the retrieved registry entry
8. Existing suite: `cargo test --workspace`

### Invariants

1. Active actions never store the handler itself; executable dispatch is resolved from the action definition's handler linkage
2. Handlers mutate world state only through `WorldTxn`
3. Registry iteration order is stable (insertion order)
4. `ActionHandlerRegistry` is explicitly NOT serializable
5. Handler dispatch carries no hidden per-handler runtime state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_handler.rs` — trait bound assertions on supporting types
2. `crates/worldwake-sim/src/action_handler_registry.rs` — register/get, sequential IDs, mock handler integration test

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completed: 2026-03-09
- Changed vs. original plan:
  - Added `crates/worldwake-sim/src/action_handler.rs` with explicit lifecycle hook function-pointer aliases, `ActionHandler`, `ActionProgress`, `ActionError`, and `AbortReason`.
  - Added `crates/worldwake-sim/src/action_handler_registry.rs` with deterministic `Vec<ActionHandler>` storage and sequential `ActionHandlerId` assignment.
  - Re-exported the handler surface from `crates/worldwake-sim/src/lib.rs`.
- Deviations from original plan:
  - Replaced the proposed boxed trait-object registry with a plain function-table design. This removes hidden per-handler state and keeps dispatch tied directly to stable IDs.
  - Verified handler behavior with real `WorldTxn` mutation instead of only mock retrieval tests.
- Verification:
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace`
  - `cargo test --workspace`

Outcome amended: 2026-03-09
- Follow-up architectural refinement removed redundant `ActionHandlerId` storage from `ActionInstance`.
- The handler registry remains the executable lookup table, but active instances now reach it indirectly through `ActionDef`, keeping dispatch aligned with the action schema.
