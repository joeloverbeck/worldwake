# E07ACTFRA-003: ActionDef + ActionDefRegistry

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — defines the action definition model
**Deps**: E07ACTFRA-001 (IDs), E07ACTFRA-002 (semantic types)

## Problem

Every action in the simulation must be defined by a static `ActionDef` that encodes all 10 semantics from spec 3.7. These definitions are registered in a deterministic static order. The registry provides lookup by `ActionDefId` and deterministic iteration.

## Assumption Reassessment (2026-03-09)

1. Spec 3.7 requires 10 semantics: actor constraints, targets, preconditions, reservation requirements, duration, interruptibility, commit conditions, effects (handler), visibility, causal event emission.
2. `VisibilitySpec` exists in `worldwake-core/src/visibility.rs` — use it directly.
3. `EventTag` exists in `worldwake-core/src/event_tag.rs` — use `BTreeSet<EventTag>` for causal tags.
4. `ActionHandlerId` from E07ACTFRA-001 links to effect logic (the handler).
5. Action definitions are static data — they do not change at runtime.

## Architecture Check

1. `ActionDef` is a plain struct with no optional shortcut fields — all 10 semantics are mandatory. This prevents "default away" mistakes.
2. `ActionDefRegistry` uses a `Vec<ActionDef>` indexed by `ActionDefId(u32)` for O(1) lookup with deterministic insertion-order iteration.
3. No `HashMap` — the registry is an indexed vector.

## What to Change

### 1. Create `worldwake-sim/src/action_def.rs`

Define `ActionDef`:
```rust
pub struct ActionDef {
    pub id: ActionDefId,
    pub name: String,
    pub actor_constraints: Vec<Constraint>,
    pub targets: Vec<TargetSpec>,
    pub preconditions: Vec<Precondition>,
    pub reservation_requirements: Vec<ReservationReq>,
    pub duration: DurationExpr,
    pub interruptibility: Interruptibility,
    pub commit_conditions: Vec<Precondition>,
    pub visibility: VisibilitySpec,
    pub causal_event_tags: BTreeSet<EventTag>,
    pub handler: ActionHandlerId,
}
```

Must derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 2. Create `worldwake-sim/src/action_def_registry.rs`

Define `ActionDefRegistry`:
- `register(def: ActionDef) -> ActionDefId` — appends and returns the assigned ID
- `get(id: ActionDefId) -> Option<&ActionDef>` — O(1) lookup
- `iter() -> impl Iterator<Item = &ActionDef>` — deterministic insertion-order iteration
- `len()` / `is_empty()`

The registry validates that `def.id` matches the next sequential index on registration.

Must derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.

### 3. Update `worldwake-sim/src/lib.rs`

Declare modules, re-export `ActionDef` and `ActionDefRegistry`.

## Files to Touch

- `crates/worldwake-sim/src/action_def.rs` (new)
- `crates/worldwake-sim/src/action_def_registry.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- Evaluating constraints, preconditions, or commit conditions (E07ACTFRA-007/008)
- ActionInstance or active action state (E07ACTFRA-004)
- Handler execution logic (E07ACTFRA-005)
- KnowledgeView (E07ACTFRA-006)
- Populating the registry with concrete game actions (later epics)

## Acceptance Criteria

### Tests That Must Pass

1. `ActionDef` has exactly 12 fields — one for each of the 10 semantics plus `id` and `name`
2. `ActionDef` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned`
3. `ActionDefRegistry` round-trips through bincode with registered defs preserved
4. `register()` assigns sequential `ActionDefId` values starting from 0
5. `get()` returns the correct def for each registered ID
6. `iter()` returns defs in registration order
7. Registration panics or errors if `def.id` does not match expected next index
8. All ten semantics are required — no field is `Option` (type model enforces completeness)
9. Existing suite: `cargo test --workspace`

### Invariants

1. No `HashMap` or `HashSet` in registry storage
2. ActionDef fields are not optional — all 10 semantics are mandatory
3. Registry iteration order equals registration order (deterministic)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_def.rs` — trait assertions, field count enforcement, bincode round-trip
2. `crates/worldwake-sim/src/action_def_registry.rs` — register/get/iter, sequential ID assignment, order preservation, bincode round-trip, mismatched-ID rejection

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`
