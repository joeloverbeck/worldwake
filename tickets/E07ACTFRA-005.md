# E07ACTFRA-005: Action Handler Trait + HandlerRegistry

**Status**: PENDING
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

## Architecture Check

1. The handler trait uses a trait object approach: `Box<dyn ActionHandler>` stored in the registry. This is acceptable because handlers are *static logic*, not serialized state. The `ActionInstance` stores only the `ActionHandlerId`, not the handler itself.
2. Registry uses a `Vec<Box<dyn ActionHandler>>` indexed by `ActionHandlerId` for O(1) lookup.
3. Alternative considered: function pointer table. Rejected because trait objects allow handlers to carry static configuration and are more ergonomic for multi-method dispatch (on_start, on_tick, on_commit, on_abort).

## What to Change

### 1. Create `worldwake-sim/src/action_handler.rs`

Define the `ActionHandler` trait:
```rust
pub trait ActionHandler: Send + Sync {
    fn on_start(&self, instance: &ActionInstance, txn: &mut WorldTxn) -> Result<Option<ActionState>, ActionError>;
    fn on_tick(&self, instance: &ActionInstance, txn: &mut WorldTxn) -> Result<ActionProgress, ActionError>;
    fn on_commit(&self, instance: &ActionInstance, txn: &mut WorldTxn) -> Result<(), ActionError>;
    fn on_abort(&self, instance: &ActionInstance, reason: &AbortReason, txn: &mut WorldTxn) -> Result<(), ActionError>;
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
- `register(handler: Box<dyn ActionHandler>) -> ActionHandlerId`
- `get(id: ActionHandlerId) -> Option<&dyn ActionHandler>`
- `len()` / `is_empty()`

Iteration order is stable (insertion order).

**Note**: `ActionHandlerRegistry` is NOT serializable — it holds trait objects. It is reconstructed on load from the same static registration code. This is by design: handlers are code, not data.

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

1. A test handler implementing `ActionHandler` can be registered and retrieved by ID
2. Registry assigns sequential `ActionHandlerId` values
3. `get()` returns the correct handler for each registered ID
4. Handler `on_commit` can mutate world state through `WorldTxn` (integration test with a mock handler)
5. `ActionError` and `AbortReason` satisfy `Clone + Debug + Eq`
6. `ActionProgress` satisfies `Copy + Clone + Debug + Eq`
7. Existing suite: `cargo test --workspace`

### Invariants

1. Active actions store `ActionHandlerId`, never the handler itself
2. Handlers mutate world state only through `WorldTxn`
3. Registry iteration order is stable (insertion order)
4. `ActionHandlerRegistry` is explicitly NOT serializable

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_handler.rs` — trait bound assertions on supporting types
2. `crates/worldwake-sim/src/action_handler_registry.rs` — register/get, sequential IDs, mock handler integration test

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`
