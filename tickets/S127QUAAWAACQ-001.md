# S127QUAAWAACQ-001: AcquisitionQuantity struct

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — adds `AcquisitionQuantity` value type to `worldwake-core`
**Deps**: None

## Problem

S127 makes acquisition goals quantity-aware. The first compile-safe step is to land the `AcquisitionQuantity` value type in `worldwake-core` so subsequent tickets can extend `GoalKind::AcquireCommodity` with a `quantity` field, candidate generation can construct quantity intent, and decision traces can surface `desired_min`/`desired_target`/`horizon_ticks`. Without this type defined first, every other S127 ticket would have to inline its own draft.

## Assumption Reassessment (2026-04-26)

1. `crates/worldwake-core/src/goal.rs` exists and currently defines `GoalKind` with `AcquireCommodity { commodity, purpose }` (lines 28–31, confirmed during reassessment). `NonZeroU16`, `NonZeroU32` are stdlib types and derive `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug` plus `Serialize, Deserialize` via serde. `NonZeroU32::new(200).unwrap()` works in `const` context (stable since Rust 1.83).
2. `specs/S127-quantity-aware-acquisition.md` D1 prescribes the exact struct shape and the `single()` constructor with a default 200-tick horizon.
3. No existing `AcquisitionQuantity` type exists — `grep -rn "AcquisitionQuantity" crates/` returns 0 matches.

## Architecture Check

1. Defining the type before any consumer extends `GoalKind` keeps the migration linearizable: ticket 002 only has to add a field of an already-existing type, rather than introduce both the type and the field in one large change. This isolates compile failures.
2. No backwards-compatibility shim — `AcquisitionQuantity` is net-new with no existing implicit-quantity alias to preserve. The `single()` helper exists for migration ergonomics in ticket 002, not as a permanent compatibility surface.

## Verification Layers

1. Type derives compile and round-trip via bincode → focused unit test in `goal.rs` `#[cfg(test)]` block (target module's existing test infrastructure already exercises bincode round-trip for `GoalKind` variants — pattern at `goal.rs:395-398`).
2. `single()` constructor returns valid `NonZeroU16::MIN`, `NonZeroU16::MIN`, `NonZeroU32(200)` invariant: `desired_min <= desired_target` → focused unit test asserting `single().desired_min <= single().desired_target` and `single().horizon_ticks.get() == 200`.
3. Single-layer ticket (pure type addition, no runtime behavior) — no action trace, event-log delta, or world-state proof surface applies.

## What to Change

### 1. Add `AcquisitionQuantity` struct in `crates/worldwake-core/src/goal.rs`

Insert near the top of the file, alongside other `GoalKind` value types. Exact body per spec D1:

```rust
/// Quantity intent on an `AcquireCommodity` goal. The goal is satisfied
/// when the agent has obtained at least `desired_min` units; the planner
/// prefers plans projected to deliver `desired_target`. `horizon_ticks`
/// is consumed by the candidate emitter — it stops emitting when
/// `current_tick + horizon_ticks` no longer covers the projected
/// need-breach tick.
///
/// Invariant: `desired_min <= desired_target`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct AcquisitionQuantity {
    pub desired_min: NonZeroU16,
    pub desired_target: NonZeroU16,
    pub horizon_ticks: NonZeroU32,
}

impl AcquisitionQuantity {
    #[must_use]
    pub const fn single() -> Self {
        Self {
            desired_min: NonZeroU16::MIN,
            desired_target: NonZeroU16::MIN,
            horizon_ticks: NonZeroU32::new(200).unwrap(),
        }
    }
}
```

### 2. Re-export from `lib.rs`

Add `AcquisitionQuantity` to the existing `goal` module re-exports in `crates/worldwake-core/src/lib.rs` so downstream crates can import it directly.

### 3. Add focused unit tests

In the `#[cfg(test)]` block of `goal.rs`:

- `acquisition_quantity_single_invariant`: asserts `single().desired_min == single().desired_target` and `single().horizon_ticks.get() == 200`.
- `acquisition_quantity_bincode_roundtrip`: serializes a constructed `AcquisitionQuantity`, deserializes, asserts equality.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — add struct + tests)
- `crates/worldwake-core/src/lib.rs` (modify — add re-export)

## Out of Scope

- Adding `quantity` to `GoalKind::AcquireCommodity` — ticket 002.
- Candidate-generation use of `AcquisitionQuantity` — ticket 007.
- Belief-view accessors — ticket 004 / 005.
- Decision-trace surfacing — ticket 002.

## Acceptance Criteria

### Tests That Must Pass

1. `acquisition_quantity_single_invariant` — `single()` satisfies `desired_min <= desired_target`.
2. `acquisition_quantity_bincode_roundtrip` — bincode round-trip preserves all three fields.
3. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. `AcquisitionQuantity::single().desired_min == AcquisitionQuantity::single().desired_target` (single-unit canonical form).
2. `AcquisitionQuantity` derives `Copy` so embedding it in `GoalKind` (ticket 002) preserves `GoalKind::Copy`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` `#[cfg(test)]` block — two new focused tests as specified above.

### Commands

1. `cargo test -p worldwake-core acquisition_quantity`
2. `cargo test -p worldwake-core`
3. `cargo clippy -p worldwake-core --all-targets -- -D warnings`
