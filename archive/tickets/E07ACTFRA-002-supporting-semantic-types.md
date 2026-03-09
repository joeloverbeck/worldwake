# E07ACTFRA-002: Supporting Semantic Types

**Status**: COMPLETED
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
4. `EntityKind`, `CommodityKind`, `Quantity`, and `EntityId` already exist in `worldwake-core` and are exported from the crate root.
5. `worldwake-sim` currently contains only `action_ids.rs`, `action_status.rs`, and `lib.rs`; no existing `ActionDef`, affordance query, or action semantics module exists yet.
6. Downstream E07 tickets model bound targets as `Vec<EntityId>` on both `Affordance` and `ActionInstance`, so `TargetSpec` variants in this ticket must describe how to bind entity targets, not arbitrary non-entity payloads.
7. Serialized action schema should use fixed-width integers, not `usize`, to avoid architecture-dependent encoding in authoritative data.

## Architecture Check

1. These types are pure data — no logic, no trait objects, no closures. They encode *what* an action requires, not *how* it executes.
2. Phase 1 should define only concrete, evaluable variants that match the current action pipeline. Do not add `Custom(String)` or similar stringly escape hatches; they hide missing semantics instead of modeling them.
3. `TargetSpec` should only describe entity bindings. Relationship checks such as co-location belong in `Precondition`, not as pseudo-target variants.
4. `ReservationReq` should stay minimal in Phase 1. Reservation timing is derived from action start tick plus resolved action duration, so per-reservation tick counts are unnecessary here.

## What to Change

### 1. Create `worldwake-sim/src/action_semantics.rs`

Define the following types:

**`Constraint`** — actor eligibility check:
```rust
enum Constraint {
    ActorAlive,
    ActorHasControl,
    ActorAtPlace(EntityId),
    ActorHasCommodity { kind: CommodityKind, min_qty: Quantity },
    ActorKind(EntityKind),
}
```

**`TargetSpec`** — what targets an action requires:
```rust
enum TargetSpec {
    SpecificEntity(EntityId),                   // pre-bound target
    EntityAtActorPlace { kind: EntityKind },    // any entity of this kind co-located with actor
}
```

**`Precondition`** — checked at start and/or commit time:
```rust
enum Precondition {
    ActorAlive,
    TargetExists(u8),             // target at index must still exist
    TargetAtActorPlace(u8),       // target at index must be co-located with actor
    TargetKind { target_index: u8, kind: EntityKind },
}
```

**`ReservationReq`** — what reservations to acquire:
```rust
struct ReservationReq {
    target_index: u8,             // index into ActionDef.targets
}
```

**`DurationExpr`** — resolves to integer ticks:
```rust
enum DurationExpr {
    Fixed(u32),
}
```

Provide:
```rust
impl DurationExpr {
    pub const fn resolve(self) -> u32 { ... }
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

All types must derive `Clone, Eq, PartialEq, Debug, Serialize, Deserialize`. These Phase 1 types are all small deterministic value types and should derive `Copy` as well.

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
- String-based extension hooks such as `Custom(String)`; if later phases need new semantics, add new concrete variants and update all callers

## Acceptance Criteria

### Tests That Must Pass

1. All six types satisfy `Clone + Eq + Debug + Serialize + DeserializeOwned` (compile-time trait assertion)
2. `DurationExpr::Fixed(5)` resolves to `5u32` (a `resolve()` method)
3. All types survive bincode round-trip for each variant
4. No closure, trait object, `Box<dyn ...>`, or function pointer appears in any type
5. Target/precondition indices use a fixed-width integer type, not `usize`
6. Existing suite: `cargo test --workspace`

### Invariants

1. No `HashMap` or `HashSet` in new code
2. `DurationExpr` resolves to integer ticks only — no floats
3. All types contain only IDs, enums, and deterministic parameters
4. `TargetSpec` variants always describe entity bindings compatible with downstream `Vec<EntityId>` action target storage
5. Relationship checks such as co-location are encoded as preconditions, not as pseudo-target variants

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_semantics.rs` — trait assertions, bincode round-trips for each type/variant, `DurationExpr::resolve()` test, fixed-width target index coverage

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completed: 2026-03-09
- Changed vs. original plan:
  - Added `crates/worldwake-sim/src/action_semantics.rs` with concrete deterministic schema types for `Constraint`, `TargetSpec`, `Precondition`, `ReservationReq`, `DurationExpr`, and `Interruptibility`.
  - Added `DurationExpr::resolve()` and re-exported all semantic types from `crates/worldwake-sim/src/lib.rs`.
  - Corrected the ticket assumptions and scope before implementation:
    - removed string-based `Custom(String)` escape hatches
    - replaced architecture-dependent `usize` indices with `u8`
    - removed non-entity target semantics that did not fit downstream `Vec<EntityId>` target binding
    - moved co-location semantics into `Precondition` instead of `TargetSpec`
    - simplified `ReservationReq` so reservation timing derives from action duration instead of duplicating tick counts per reservation
- Deviations from original plan:
  - The implemented target vocabulary is intentionally narrower and cleaner than originally proposed. It matches the current `E07` architecture and avoids placeholder semantics that later tickets would have had to undo.
  - The code derives `Ord`, `PartialOrd`, and `Hash` in addition to the required traits because these value types are deterministic schema data and may need stable ordering in later action infrastructure.
- Verification:
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
