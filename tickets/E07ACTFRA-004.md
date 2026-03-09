# E07ACTFRA-004: ActionState + ActionInstance

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — defines serializable active action state
**Deps**: E07ACTFRA-001 (IDs, ActionStatus)

## Problem

Active actions need a serializable representation that can survive save/load and replay. `ActionInstance` tracks the full lifecycle state of a running action. `ActionState` provides handler-local persistent storage. Neither may hold references, closures, or transient data.

## Assumption Reassessment (2026-03-09)

1. `ActionStatus` from E07ACTFRA-001 provides: Pending, Active, Committed, Aborted, Interrupted.
2. `ReservationId` exists in `worldwake-core/src/ids.rs`.
3. `Tick` and `EntityId` exist in core.
4. The spec explicitly forbids borrowed references, closure-captured transient state, and function pointers in `ActionInstance`.

## Architecture Check

1. `ActionState` is a serializable enum that handlers use to persist intermediate data. Phase 1 starts with a minimal variant set (Empty + generic key-value for flexibility).
2. `ActionInstance` is a flat struct with no `&` references. All entity/ID links are by value.
3. Both types are fully serializable — the combination must survive bincode round-trip.

## What to Change

### 1. Create `worldwake-sim/src/action_state.rs`

Define `ActionState`:
```rust
enum ActionState {
    Empty,
    Data(BTreeMap<String, ActionStateValue>),
}
```

Where `ActionStateValue` is:
```rust
enum ActionStateValue {
    Integer(i64),
    Entity(EntityId),
    Text(String),
    Quantity(Quantity),
}
```

Both derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 2. Create `worldwake-sim/src/action_instance.rs`

Define `ActionInstance`:
```rust
pub struct ActionInstance {
    pub instance_id: ActionInstanceId,
    pub def_id: ActionDefId,
    pub handler_id: ActionHandlerId,
    pub actor: EntityId,
    pub targets: Vec<EntityId>,
    pub start_tick: Tick,
    pub remaining_ticks: u32,
    pub status: ActionStatus,
    pub reservation_ids: Vec<ReservationId>,
    pub local_state: Option<ActionState>,
}
```

Must derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 3. Update `worldwake-sim/src/lib.rs`

Declare modules, re-export public types.

## Files to Touch

- `crates/worldwake-sim/src/action_state.rs` (new)
- `crates/worldwake-sim/src/action_instance.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- ActionInstance storage/management (tracked by the scheduler in E08)
- Tick progression or status transitions (E07ACTFRA-009)
- Start gate logic (E07ACTFRA-008)
- Handler execution that reads/writes ActionState (E07ACTFRA-005)
- ActionDef (E07ACTFRA-003)

## Acceptance Criteria

### Tests That Must Pass

1. `ActionInstance` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned`
2. `ActionState` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned`
3. An `ActionInstance` with `local_state: Some(ActionState::Data(...))` survives bincode round-trip with all fields preserved
4. An `ActionInstance` with `local_state: None` also round-trips correctly
5. `ActionInstance` contains no `&` references, no `Box<dyn ...>`, no function pointers (verified by compilation — the derives enforce this)
6. Existing suite: `cargo test --workspace`

### Invariants

1. No borrowed references in ActionInstance
2. No closure-captured transient state
3. All fields are serializable and replay-safe
4. No `HashMap` or `HashSet`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_state.rs` — trait assertions, bincode round-trip for each variant
2. `crates/worldwake-sim/src/action_instance.rs` — trait assertions, bincode round-trip with various local_state values

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`
