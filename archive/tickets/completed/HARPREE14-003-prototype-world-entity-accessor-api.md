# HARPREE14-003: Prototype world entity accessor API

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes -- new public prototype-place identifier API in topology
**Deps**: None (Wave 1, independent)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-B03

## Problem

`crates/worldwake-ai/tests/golden_e2e.rs` duplicates the private `prototype_entity()` helper from `topology.rs` because there is no public, named way to reference prototype-world places. That leaves the test coupled to raw slot arithmetic instead of to an explicit prototype-place identifier.

## Assumption Reassessment (2026-03-11)

1. `prototype_entity()` exists in `crates/worldwake-core/src/topology.rs` and is private -- confirmed.
2. `crates/worldwake-ai/tests/golden_e2e.rs` defines its own local `prototype_entity()` helper and hard-coded `VILLAGE_SQUARE` / `ORCHARD_FARM` constants -- confirmed.
3. The prototype world already has a single manifest source of truth in `PROTOTYPE_PLACE_SPECS`; the proposed `PrototypeWorldEntities` struct would introduce a second parallel naming surface that must be kept in sync manually -- confirmed.
4. I found no second current test file duplicating the helper. The verified immediate problem is `golden_e2e.rs`, not a broader existing pattern across the tree.

## Architecture Check

1. The public API should expose a prototype-place identifier, not raw slot arithmetic.
2. A public enum such as `PrototypePlace` is cleaner than a `PrototypeWorldEntities` struct:
   - it avoids duplicating the manifest as a second public data shape,
   - it scales naturally when the prototype world grows,
   - it preserves a single-source mapping from semantic place identity to `EntityId`.
3. The accessor should remain prototype-specific rather than adding generic `Topology` lookup-by-name. String lookup would be weaker, less explicit, and more typo-prone than a typed place identifier.
4. No backward-compatibility shims or alias APIs. Replace the local test helper outright.

## What to Change

### 1. Add a public `PrototypePlace` enum to `topology.rs`

```rust
pub enum PrototypePlace {
    VillageSquare,
    OrchardFarm,
    GeneralStore,
    CommonHouse,
    RulersHall,
    GuardPost,
    PublicLatrine,
    NorthCrossroads,
    ForestPath,
    BanditCamp,
    SouthGate,
    EastFieldTrail,
}
```

### 2. Add a public accessor for the enum

Preferred shape:

```rust
pub const fn prototype_place_entity(place: PrototypePlace) -> EntityId
```

This can use the existing slot mapping internally. `prototype_entity()` itself does not need to become public.

### 3. Keep the manifest as the source of truth

Route both `build_prototype_world()` and the public accessor through the same semantic mapping where practical. Do not create a second hand-maintained list of named public fields.

### 4. Update golden e2e to use the typed accessor

Remove the local `prototype_entity()` helper and raw slot constants. Use `prototype_place_entity(PrototypePlace::...)`.

### 5. Add direct core tests for the new API

Add topology tests that verify the public prototype-place accessor resolves to the expected place IDs / names in the built prototype world. The bug is API absence; the fix should be covered in `worldwake-core`, not only indirectly in AI e2e.

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (modify -- add enum + accessor + tests)
- `crates/worldwake-core/src/lib.rs` (modify -- re-export public topology API)
- `crates/worldwake-ai/tests/golden_e2e.rs` (modify -- use typed accessor, remove local helper)

## Out of Scope

- Changing `build_prototype_world()` behavior or place definitions
- Adding new places to the prototype world
- Modifying entity allocation order
- Adding generic string-based topology lookup APIs
- Sweeping other tests to adopt the new API unless they already need named prototype places in this ticket

## Acceptance Criteria

### Tests That Must Pass

1. Golden e2e passes with identical state hashes.
2. `cargo test -p worldwake-core topology` passes, including new accessor coverage.
3. `cargo test --workspace` passes.
4. `cargo clippy --workspace` passes with no new warnings.

### Invariants

1. `build_prototype_world()` behavior unchanged.
2. Entity allocation order unchanged.
3. Golden e2e state hashes identical.
4. The public API exposes semantic prototype place identity without exposing raw slot arithmetic.
5. No behavioral change of any kind.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/topology.rs` -- add tests covering `prototype_place_entity(PrototypePlace::...)`.
2. `crates/worldwake-ai/tests/golden_e2e.rs` -- modify to use `prototype_place_entity()` instead of a local helper.

### Commands

1. `cargo test -p worldwake-core topology` (targeted API coverage first)
2. `cargo test -p worldwake-ai --test golden_e2e` (targeted downstream integration)
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

Completion date: 2026-03-11

What actually changed:
- Added a public typed prototype-place API in `worldwake-core` using `PrototypePlace` plus `prototype_place_entity(...)`.
- Reworked the internal prototype-world manifest to use `PrototypePlace` identifiers directly instead of raw slot numbers.
- Replaced the duplicated `prototype_entity()` helper in `crates/worldwake-ai/tests/golden_e2e.rs` with the new public accessor.
- Added direct topology tests covering prototype-place-to-entity resolution and resolution into the built prototype world.

Deviations from original plan:
- Did not add a `PrototypeWorldEntities` struct.
- Did not make the private `prototype_entity()` helper public.
- Used a typed enum-based API instead because it keeps one semantic source of truth and avoids maintaining a second parallel public struct for the same prototype manifest.

Verification results:
- `cargo test -p worldwake-core topology` passed.
- `cargo test -p worldwake-ai --test golden_e2e` passed.
- `cargo test --workspace` passed.
- `cargo clippy --workspace` passed.
