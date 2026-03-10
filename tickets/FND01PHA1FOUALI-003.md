# FND01PHA1FOUALI-003: Replace Load Match-Arms with Physical Profiles

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new structs in items.rs, refactored delegation in load.rs
**Deps**: None (independent, can parallelize with -001 and -002)

## Problem

`load_per_unit()` and `load_of_unique_item_kind()` in `load.rs` use match-arm tables that assign load values via magic numbers embedded in function bodies. These values have no named data structure — they're invisible to introspection and untraceable. This violates Principle 2 (No Magic Numbers).

## Assumption Reassessment (2026-03-10)

1. `load_per_unit(commodity: CommodityKind) -> LoadUnits` at `load.rs:7-18` — confirmed. Match arms: Water=2, Firewood=3, Apple/Grain/Bread/Medicine/Coin/Waste=1.
2. `load_of_unique_item_kind(kind: UniqueItemKind) -> LoadUnits` at `load.rs:30-38` — confirmed. Match arms: SimpleTool/Artifact=5, Weapon=10, Contract=1, OfficeInsignia=2, Misc=3.
3. `CommodityKind` has 8 variants: Apple, Grain, Bread, Water, Firewood, Medicine, Coin, Waste — confirmed at `items.rs:9-18`.
4. `UniqueItemKind` has 6 variants: SimpleTool, Weapon, Contract, Artifact, OfficeInsignia, Misc — confirmed at `items.rs:98-116`.
5. Both enums have `ALL` constants listing every variant — confirmed, useful for exhaustive tests.
6. Existing load tests validate exact values — confirmed, these must continue passing unchanged.

## Architecture Check

1. Named structs (`CommodityPhysicalProfile`, `UniqueItemPhysicalProfile`) make load values introspectable and traceable. Future systems can extend profiles with additional physical properties (volume, fragility, etc.) without new match arms.
2. Existing public API (`load_per_unit()`, `load_of_unique_item_kind()`) remains unchanged — callers don't need modification.
3. No backward-compatibility shims — the match arms move inside profile methods, no aliases needed.

## What to Change

### 1. Create `CommodityPhysicalProfile` struct

In `crates/worldwake-core/src/items.rs` (alongside the types it describes):

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CommodityPhysicalProfile {
    pub load_per_unit: LoadUnits,
}
```

### 2. Create `UniqueItemPhysicalProfile` struct

In `crates/worldwake-core/src/items.rs`:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct UniqueItemPhysicalProfile {
    pub load: LoadUnits,
}
```

### 3. Add `physical_profile()` method on `CommodityKind`

```rust
impl CommodityKind {
    pub fn physical_profile(self) -> CommodityPhysicalProfile {
        let load_per_unit = match self {
            Self::Water => LoadUnits(2),
            Self::Firewood => LoadUnits(3),
            Self::Apple | Self::Grain | Self::Bread
            | Self::Medicine | Self::Coin | Self::Waste => LoadUnits(1),
        };
        CommodityPhysicalProfile { load_per_unit }
    }
}
```

### 4. Add `physical_profile()` method on `UniqueItemKind`

```rust
impl UniqueItemKind {
    pub fn physical_profile(self) -> UniqueItemPhysicalProfile {
        let load = match self {
            Self::SimpleTool | Self::Artifact => LoadUnits(5),
            Self::Weapon => LoadUnits(10),
            Self::Contract => LoadUnits(1),
            Self::OfficeInsignia => LoadUnits(2),
            Self::Misc => LoadUnits(3),
        };
        UniqueItemPhysicalProfile { load }
    }
}
```

### 5. Refactor `load_per_unit()` to delegate

In `load.rs`:
```rust
pub fn load_per_unit(commodity: CommodityKind) -> LoadUnits {
    commodity.physical_profile().load_per_unit
}
```

### 6. Refactor `load_of_unique_item_kind()` to delegate

In `load.rs`:
```rust
pub fn load_of_unique_item_kind(kind: UniqueItemKind) -> LoadUnits {
    kind.physical_profile().load
}
```

### 7. Add exhaustive profile coverage test

New test asserting every `CommodityKind::ALL` variant returns a profile with `load_per_unit > LoadUnits(0)`, and every `UniqueItemKind::ALL` variant returns a profile with `load > LoadUnits(0)`.

### 8. Export new types from `crates/worldwake-core/src/lib.rs`

Ensure `CommodityPhysicalProfile` and `UniqueItemPhysicalProfile` are publicly accessible.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — add structs and methods)
- `crates/worldwake-core/src/load.rs` (modify — delegate to profiles)
- `crates/worldwake-core/src/lib.rs` (modify — re-export new types if needed)

## Out of Scope

- Do NOT change `LoadUnits`, `Quantity`, or other numeric types.
- Do NOT change the values — same numbers, just moved into named structs.
- Do NOT add fields beyond `load_per_unit` / `load` to the profile structs (future extensions are future tickets).
- Do NOT modify any callers of `load_per_unit()` or `load_of_unique_item_kind()`.
- Do NOT touch `conservation.rs` or any container logic.

## Acceptance Criteria

### Tests That Must Pass

1. `CommodityPhysicalProfile` and `UniqueItemPhysicalProfile` structs exist and are public.
2. `CommodityKind::physical_profile()` and `UniqueItemKind::physical_profile()` methods exist.
3. `load_per_unit()` delegates to `CommodityKind::physical_profile().load_per_unit`.
4. `load_of_unique_item_kind()` delegates to `UniqueItemKind::physical_profile().load`.
5. Exhaustive coverage test: every variant returns non-zero profile.
6. All existing load tests pass unchanged (same values, same API).
7. Existing suite: `cargo test -p worldwake-core`
8. Full suite: `cargo test --workspace`
9. `cargo clippy --workspace` clean.

### Invariants

1. `load_per_unit()` and `load_of_unique_item_kind()` return identical values to before.
2. Conservation invariant (`verify_conservation`) unaffected.
3. No callers of the existing public API need changes.

## Test Plan

### New/Modified Tests

1. `items.rs::every_commodity_kind_has_nonzero_physical_profile` — exhaustive coverage.
2. `items.rs::every_unique_item_kind_has_nonzero_physical_profile` — exhaustive coverage.

### Commands

1. `cargo test -p worldwake-core -- load`
2. `cargo test -p worldwake-core -- items`
3. `cargo test --workspace && cargo clippy --workspace`
