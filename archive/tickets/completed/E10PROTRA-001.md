# E10PROTRA-001: WorkstationTag enum + RecipeId newtype in worldwake-core

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new shared Phase 2 types in core
**Deps**: E08 (Phase 1 complete), Phase 2 shared schema extraction step

## Problem

E10 deliverables require two foundational production-schema types that must live in `worldwake-core`, because later core components (`KnownRecipes`, workstation marker components) and the sim-layer recipe registry will both depend on them. Putting `RecipeId` or `WorkstationTag` in `worldwake-sim` would force an invalid `worldwake-core -> worldwake-sim` dependency once those core components are added. These are part of the Phase 2 shared schema extraction boundary that E10 depends on, even though `RecipeId` itself is not listed explicitly in the minimum Step 7a bullets.

## Assumption Reassessment (2026-03-10)

1. No `WorkstationTag` type exists in the codebase — confirmed.
2. No `RecipeId` type exists in the codebase — confirmed.
3. `EntityKind::Facility` already exists and is the downstream kind for workstation entities, but this ticket itself does not register any components against it.
4. `worldwake-core` already uses focused topic modules for shared schema with colocated tests (for example `drives.rs`, `wounds.rs`, `items.rs`). Creating `production.rs` is consistent with current structure and gives later E10 core components a stable home.
5. `RecipeId` must live in core (not sim) because `KnownRecipes` is planned as a core component that references it; the real boundary issue is preserving the crate dependency graph, not avoiding a `sim <-> systems` circular import.

## Architecture Check

1. `production.rs` should become the shared home for production-domain core schema (`WorkstationTag`, `RecipeId`, then later `ResourceSource`, `KnownRecipes`, `WorkstationMarker`, `ProductionJob`). Keeping those types together is cleaner than scattering them across unrelated core modules.
2. `WorkstationTag` is a catalog-like enum analogous to `CommodityKind`: a classification tag, not a component. It will be consumed by `RecipeDefinition` in sim and by workstation-related core components in later tickets.
3. `RecipeId` is a simple shared identifier newtype over `u32`. It belongs in core because both sim registries and later core components depend on it.
4. No backwards-compatibility shims needed — these are brand new types.

## What to Change

### 1. New module `crates/worldwake-core/src/production.rs`

```rust
/// Tag identifying what kind of workstation an entity is.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WorkstationTag {
    Forge,
    Loom,
    Mill,
    ChoppingBlock,
    WashBasin,
    OrchardRow,
    FieldPlot,
    // Extend as recipes require
}

impl WorkstationTag {
    pub const ALL: [Self; 7] = [
        Self::Forge,
        Self::Loom,
        Self::Mill,
        Self::ChoppingBlock,
        Self::WashBasin,
        Self::OrchardRow,
        Self::FieldPlot,
    ];
}

/// Identifies a recipe definition in the RecipeRegistry.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub u32);
```

### 2. Export from `crates/worldwake-core/src/lib.rs`

Add `pub mod production;` and re-export `WorkstationTag` and `RecipeId`.

## Files to Touch

- `crates/worldwake-core/src/production.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Component registration (no components added in this ticket)
- Transport schema (`CarryCapacity`, `InTransitOnEdge`) — handled separately in E10PROTRA-003
- `RecipeDefinition` or `RecipeRegistry` (E10PROTRA-006)
- `KnownRecipes` component (E10PROTRA-004)
- Workstation marker component (E10PROTRA-005 or later)
- Any action logic
- Any systems crate changes

## Acceptance Criteria

### Tests That Must Pass

1. `WorkstationTag` variants satisfy `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.
2. `RecipeId` satisfies the same trait bounds.
3. `WorkstationTag::ALL` is a canonical, deterministic variant list.
4. Both round-trip through bincode.
5. `RecipeId(0) < RecipeId(1)` (ordering works).
6. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All types are deterministic (`Ord`, no `HashMap`).
2. No floating-point types used.
3. No external dependencies beyond `serde`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` — trait bounds, canonical variant list, serialization round-trip, ordering

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - Added `crates/worldwake-core/src/production.rs` with `WorkstationTag` and `RecipeId`.
  - Added `WorkstationTag::ALL` as a canonical deterministic variant list.
  - Re-exported both types from `crates/worldwake-core/src/lib.rs`.
  - Added module-local tests covering trait bounds, canonical ordering, and bincode round-trips.
  - Corrected the ticket’s dependency rationale before implementation so it matches the current crate boundaries and shared-schema architecture.
- Deviations from original plan:
  - The ticket text was corrected first to reflect the real architectural constraint: preserving `worldwake-core` as the shared schema boundary for later core components, rather than preventing a `worldwake-sim`/`worldwake-systems` circular import.
  - Added a canonical `WorkstationTag::ALL` list because that pattern is useful for deterministic catalog-like enums and supports later registry/testing work.
  - To satisfy repository-wide lint finalization, a small unrelated cleanup was made in `crates/worldwake-systems/tests/e09_needs_integration.rs` for pre-existing clippy violations.
- Verification results:
  - `cargo test -p worldwake-core` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo test --workspace` ✅
