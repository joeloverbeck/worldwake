# FND01PHA1FOUALI-003: Replace Load Match-Arms with Physical Profiles

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new physical-profile structs and accessors in `items.rs`, delegated load lookups in `load.rs`, strengthened tests
**Deps**: None (independent, can parallelize with -001 and -002)

## Problem

`load_per_unit()` and `load_of_unique_item_kind()` in `load.rs` currently encode authoritative item load data as match-arm tables local to the load utility module. That makes physical item data live outside the item taxonomy it describes, which weakens traceability and makes future extension awkward. This violates Principle 2 (No Magic Numbers): the values exist, but not as named, item-owned physical data.

## Assumption Reassessment (2026-03-10)

1. `load_per_unit(commodity: CommodityKind) -> LoadUnits` in `crates/worldwake-core/src/load.rs` is currently the authoritative source for commodity load values. Match arms: Water=2, Firewood=3, Apple/Grain/Bread/Medicine/Coin/Waste=1.
2. `load_of_unique_item_kind(kind: UniqueItemKind) -> LoadUnits` in `crates/worldwake-core/src/load.rs` is currently the authoritative source for unique-item load values. Match arms: SimpleTool/Artifact=5, Weapon=10, Contract=1, OfficeInsignia=2, Misc=3.
3. `CommodityKind` has 8 variants: Apple, Grain, Bread, Water, Firewood, Medicine, Coin, Waste — confirmed at `items.rs:9-18`.
4. `UniqueItemKind` has 6 variants: SimpleTool, Weapon, Contract, Artifact, OfficeInsignia, Misc — confirmed at `items.rs:98-116`.
5. Both enums have `ALL` constants listing every variant — confirmed, useful for exhaustive and exact-mapping tests.
6. Existing exact-value tests live in `crates/worldwake-core/src/load.rs` under the module’s inline `#[cfg(test)]` block — confirmed. There is no separate dedicated `load` test file.
7. `crates/worldwake-core/src/lib.rs` explicitly re-exports `items` symbols; new public item-profile types will need to be added there if they should be available at the crate root.

## Architecture Reassessment

1. Moving the data into named profile structs is beneficial only if `items.rs` becomes the single authoritative home for physical item metadata. Simply hiding the same match arms behind a new method would be a weak improvement.
2. The stronger architecture is: item kinds own their physical metadata, and `load.rs` consumes that metadata for aggregate calculations (`load_of_lot`, container load, remaining capacity).
3. Within this ticket’s scope, profile accessors on `CommodityKind` and `UniqueItemKind` are a good step because they colocate the data with the taxonomy and make future expansion possible without expanding `load.rs`.
4. A full item-spec catalog would be even cleaner long-term if more physical fields appear, but that is broader than this ticket and is not required to remove the current architectural smell.
5. The tests should verify exact profile mappings, not only `> 0`; non-zero coverage alone is too weak to protect the invariant this ticket introduces.

## What to Change

### 1. Create `CommodityPhysicalProfile` struct in `items.rs`

In `crates/worldwake-core/src/items.rs` (alongside the types it describes):

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CommodityPhysicalProfile {
    pub load_per_unit: LoadUnits,
}
```

### 2. Create `UniqueItemPhysicalProfile` struct in `items.rs`

In `crates/worldwake-core/src/items.rs`:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct UniqueItemPhysicalProfile {
    pub load: LoadUnits,
}
```

### 3. Add `physical_profile()` on `CommodityKind` as the authoritative load source

```rust
impl CommodityKind {
    pub const fn physical_profile(self) -> CommodityPhysicalProfile {
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

### 4. Add `physical_profile()` on `UniqueItemKind` as the authoritative load source

```rust
impl UniqueItemKind {
    pub const fn physical_profile(self) -> UniqueItemPhysicalProfile {
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

### 7. Strengthen tests around the new source of truth

Add tests in `items.rs` that assert the exact physical profile returned for every `CommodityKind::ALL` and `UniqueItemKind::ALL` variant.

Add or update tests in `load.rs` to assert the load helpers delegate to the item-kind physical profiles, not their own separate tables.

### 8. Export new types from `crates/worldwake-core/src/lib.rs`

Ensure `CommodityPhysicalProfile` and `UniqueItemPhysicalProfile` are publicly accessible from the crate root.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — add structs and methods)
- `crates/worldwake-core/src/load.rs` (modify — delegate to profiles, adjust tests)
- `crates/worldwake-core/src/lib.rs` (modify — re-export new types if needed)

## Out of Scope

- Do NOT change `LoadUnits`, `Quantity`, or other numeric types.
- Do NOT change the load values in this ticket.
- Do NOT add fields beyond `load_per_unit` / `load` to the profile structs (future extensions are future tickets).
- Do NOT introduce a broader item-catalog refactor in this ticket.
- Do NOT touch `conservation.rs` or any container logic.

## Acceptance Criteria

### Tests That Must Pass

1. `CommodityPhysicalProfile` and `UniqueItemPhysicalProfile` structs exist and are public.
2. `CommodityKind::physical_profile()` and `UniqueItemKind::physical_profile()` methods exist.
3. `load_per_unit()` delegates to `CommodityKind::physical_profile().load_per_unit`.
4. `load_of_unique_item_kind()` delegates to `UniqueItemKind::physical_profile().load`.
5. Exact-mapping tests cover every commodity and unique-item variant through `physical_profile()`.
6. `load.rs` tests verify delegation to the new authoritative profile accessors.
7. Existing load behavior remains unchanged.
8. Existing suite: `cargo test -p worldwake-core`
9. Full suite: `cargo test --workspace`
10. `cargo clippy --workspace` clean.

### Invariants

1. `CommodityKind` and `UniqueItemKind` become the single source of truth for load metadata.
2. `load_per_unit()` and `load_of_unique_item_kind()` return identical values to before.
3. Conservation invariant (`verify_conservation`) unaffected.

## Test Plan

### New/Modified Tests

1. `items.rs::commodity_kind_physical_profiles_match_catalog` — exhaustive exact mapping for every commodity kind.
2. `items.rs::unique_item_kind_physical_profiles_match_catalog` — exhaustive exact mapping for every unique-item kind.
3. `load.rs` load-table tests updated to assert delegation against `physical_profile()` as well as preserved values.

### Commands

1. `cargo test -p worldwake-core -- load`
2. `cargo test -p worldwake-core -- items`
3. `cargo test -p worldwake-core`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

Implemented the ticket as a cleaner architectural step than the original draft implied:

- Added `CommodityKindSpec` and `UniqueItemKindSpec` catalogs in `items.rs`, with nested physical-profile data, so item metadata now has a single authoritative home per kind.
- Removed the separate item-kind helper accessors and made `load.rs` consume the canonical item specs directly for kind-level load lookups.
- Strengthened tests to assert exact spec mappings and delegation behavior, instead of only checking non-zero placeholder coverage.
- Re-exported the new profile types from the crate root.
