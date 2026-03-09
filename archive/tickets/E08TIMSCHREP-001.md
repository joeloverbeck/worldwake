# E08TIMSCHREP-001: Closed `SystemId` enum and immutable system-order manifest

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new scheduler-order types in `worldwake-sim`
**Deps**: E07 (action framework complete)

## Problem

The scheduler needs a stable, deterministic ordering of simulation systems. Without a closed `SystemId` set and an immutable manifest, execution order can drift through ad hoc registration, insertion order, or future "temporary" aliases, violating determinism (Spec 9.2).

## Assumption Reassessment (2026-03-09)

1. `worldwake-sim` exists and currently exposes only action-framework modules — confirmed in `crates/worldwake-sim/src/lib.rs`.
2. No scheduler, replay, `SystemId`, or system-order manifest exists yet — confirmed by repository search.
3. `worldwake-systems` currently contains only crate scaffolding, but its crate-level contract already names the planned simulation systems: needs, production, trade, combat, perception, politics — confirmed in `crates/worldwake-systems/src/lib.rs`.
4. Because the execution set is known conceptually but not implemented yet, a closed enum is a better fit than an open numeric ID newtype. The order should be explicit data, but the legal identifiers themselves should be compile-time constrained.

## Architecture Check

1. `SystemId` should be a closed enum, not `SystemId(u32)`.
   A numeric newtype is useful when IDs are allocated or data-driven. System phases here are neither. A closed enum makes illegal IDs unrepresentable and forces order changes to be deliberate code changes.
2. `SystemManifest` should own an immutable ordered collection.
   A wrapper over `Box<[SystemId]>` is a better fit than a mutable `Vec<SystemId>` because the manifest is constructed once, then only read by the scheduler.
3. The manifest should validate duplicates at construction.
   Duplicate phases in the order list would be a real scheduler bug, so construction must reject them.
4. This ticket should stay focused on ordering metadata only.
   It must not grow dynamic registration, dispatch tables, or system-function mapping. Those belong with scheduler tick execution.

## What to Change

### 1. New type: `SystemId`

Define a serializable, ordered enum representing the canonical scheduler phases that the simulation systems crate already commits to:

```rust
pub enum SystemId {
    Needs,
    Production,
    Trade,
    Combat,
    Perception,
    Politics,
}
```

Requirements:
- Derive `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`
- Implement `Display` with stable lowercase names such as `"needs"` and `"combat"`
- Preserve declaration-order sorting semantics so canonical order is explicit in code

### 2. New type: `SystemManifest`

Define an immutable manifest wrapper that:
- owns `Box<[SystemId]>`
- rejects duplicate entries on construction
- preserves insertion order
- provides `ordered_ids() -> &[SystemId]`
- provides a canonical constructor for the default simulation order

Recommended surface:

```rust
pub struct SystemManifest {
    ordered_ids: Box<[SystemId]>,
}

impl SystemManifest {
    pub fn new(ids: impl Into<Vec<SystemId>>) -> Result<Self, SystemManifestError>;
    pub fn canonical() -> Self;
    pub fn ordered_ids(&self) -> &[SystemId];
}
```

Add a small local error type for duplicate detection. Do not introduce a dependency for this.

## Files to Touch

- `crates/worldwake-sim/src/system_manifest.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Actual system registration or dispatch logic (that belongs with scheduler tick execution)
- Mapping `SystemId` to concrete system functions
- Any `worldwake-systems` implementation work
- The scheduler struct itself

## Acceptance Criteria

### Tests That Must Pass

1. `SystemId` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Display + Serialize + Deserialize`
2. `SystemId` bincode round-trips correctly
3. `SystemId` display strings are stable for every variant
4. `SystemManifest::new` rejects duplicate `SystemId` entries
5. `SystemManifest` preserves insertion order via `ordered_ids()`
6. `SystemManifest::canonical` returns the fixed scheduler order committed by this ticket
7. `SystemManifest` round-trips through bincode
8. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. The legal system identifiers are compile-time bounded; no ad hoc numeric IDs
2. The manifest is immutable after construction
3. No `HashMap` or `HashSet` in the new types
4. Canonical scheduler order is explicit and reviewable in one place

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/system_manifest.rs` (inline `#[cfg(test)]`) — trait bounds, display, bincode, duplicate rejection, canonical order, insertion-order preservation

### Commands

1. `cargo test -p worldwake-sim system_manifest`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completed: 2026-03-09
- Changed vs. original plan:
  - Added `crates/worldwake-sim/src/system_manifest.rs` with a closed `SystemId` enum, immutable `SystemManifest`, and duplicate-validation error type.
  - Re-exported the new scheduler-ordering types from `crates/worldwake-sim/src/lib.rs`.
  - Added focused unit coverage for trait bounds, stable display strings, bincode round-trips, duplicate rejection, insertion-order preservation, and canonical order.
- Deviations from original plan:
  - Replaced the proposed open numeric `SystemId(u32)` with a closed enum. The codebase has a fixed conceptual system set rather than allocated IDs, so the enum is stricter, cleaner, and harder to misuse.
  - Stored manifest contents as `Box<[SystemId]>` rather than a mutable wrapper over `Vec<SystemId>` to reflect the real lifecycle: build once, read many.
- Verification:
  - `cargo test -p worldwake-sim system_manifest`
  - `cargo clippy --workspace`
  - `cargo test --workspace`

Outcome amended: 2026-03-09
- Added `SystemId::ALL` as the canonical compile-time system list and rewired `SystemManifest::canonical()` to use it.
- This keeps later scheduler dispatch work aligned with the closed-enum design and avoids growing a separate implicit system list elsewhere.
