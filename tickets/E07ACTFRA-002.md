# E07ACTFRA-002: Supporting Semantic Types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — defines the action schema vocabulary
**Deps**: E07ACTFRA-001 (action IDs)

## Problem

The action definition (ActionDef) requires six semantic types that encode actor constraints, target specifications, preconditions, reservation requirements, duration expressions, and interruptibility rules. These must be serializable enums/structs with no closures, trait objects, or non-deterministic data.

## Assumption Reassessment (2026-03-09)

1. Spec 3.7 lists 10 action semantics; this ticket covers the 6 that need dedicated types. The remaining 4 (name, visibility, causal_event_tags, handler) use existing types or IDs.
2. `VisibilitySpec` already exists in `worldwake-core/src/visibility.rs` — no need to redefine.
3. `ReservationId` and `TickRange` exist in `worldwake-core/src/ids.rs` — duration can resolve to tick counts.
4. `EntityId` from core is the only entity reference type needed in constraints and target specs.

## Architecture Check

1. These types are pure data — no logic, no trait objects, no closures. They encode *what* an action requires, not *how* it executes.
2. Enum variants should be minimal for Phase 1 but extensible (non-exhaustive where appropriate for future phases).

## What to Change

### 1. Create `worldwake-sim/src/action_semantics.rs`

Define the following types:

**`Constraint`** — actor eligibility check:
```rust
enum Constraint {
    MustBeAlive,
    MustBeAtPlace(EntityId),
    MustPossess { commodity: CommodityKind, min_qty: Quantity },
    MustHaveControl,  // actor must have active ControlSource
    Custom(String),    // extensibility for Phase 2+
}
```

**`TargetSpec`** — what targets an action requires:
```rust
enum TargetSpec {
    SamePlace,                    // target must be co-located with actor
    SpecificEntity(EntityId),     // pre-bound target
    EntityOfKind(EntityKind),     // any entity of this kind at actor's location
    Commodity { kind: CommodityKind, min_qty: Quantity },
}
```

**`Precondition`** — checked at start and/or commit time:
```rust
enum Precondition {
    TargetExists(usize),          // target at index must still exist
    TargetAtSamePlace(usize),     // target at index must be co-located
    ActorAlive,
    ReservationHeld(usize),       // reservation at index must be active
    Custom(String),
}
```

**`ReservationReq`** — what reservations to acquire:
```rust
struct ReservationReq {
    target_index: usize,          // index into ActionDef.targets
    duration_ticks: u32,
}
```

**`DurationExpr`** — resolves to integer ticks:
```rust
enum DurationExpr {
    Fixed(u32),
    // Future: variable durations based on skill, load, etc.
}
```

**`Interruptibility`** — controls interrupt behavior:
```rust
enum Interruptibility {
    NonInterruptible,
    InterruptibleWithPenalty,
    FreelyInterruptible,
}
```

All types must derive `Clone, Eq, PartialEq, Debug, Serialize, Deserialize`. Types that are small enough derive `Copy` as well.

### 2. Update `worldwake-sim/src/lib.rs`

Declare module, re-export all public types.

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- ActionDef struct itself (E07ACTFRA-003)
- Evaluation/checking logic for any of these types — they are pure data here
- Changes to worldwake-core
- VisibilitySpec (already exists in core)
- Handler or execution logic

## Acceptance Criteria

### Tests That Must Pass

1. All six types satisfy `Clone + Eq + Debug + Serialize + DeserializeOwned` (compile-time trait assertion)
2. `DurationExpr::Fixed(5)` resolves to `5u32` (a `resolve()` method)
3. All types survive bincode round-trip for each variant
4. No closure, trait object, `Box<dyn ...>`, or function pointer appears in any type
5. Existing suite: `cargo test --workspace`

### Invariants

1. No `HashMap` or `HashSet` in new code
2. `DurationExpr` resolves to integer ticks only — no floats
3. All types contain only IDs, enums, and deterministic parameters

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_semantics.rs` — trait assertions, bincode round-trips for each type/variant, `DurationExpr::resolve()` test

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`
