# E10PROTRA-001: WorkstationTag enum + RecipeId newtype in worldwake-core

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new shared Phase 2 types in core
**Deps**: E08 (Phase 1 complete), Phase 2 shared schema extraction step

## Problem

E10 deliverables require two foundational types that must live in `worldwake-core` so both `worldwake-sim` (RecipeRegistry) and `worldwake-systems` (production actions, workstation reservation) can depend on them without circular imports. These are part of the Phase 2 shared schema extraction (Step 7a in IMPLEMENTATION-ORDER).

## Assumption Reassessment (2026-03-10)

1. No `WorkstationTag` type exists in the codebase — confirmed.
2. No `RecipeId` type exists in the codebase — confirmed.
3. `EntityKind::Facility` already exists and is the correct kind for workstation entities.
4. The crate follows the pattern of placing domain enums/newtypes in focused modules (e.g., `items.rs` has `CommodityKind`, `ids.rs` has `EntityId`).
5. `RecipeId` must live in core (not sim) because `KnownRecipes` is a core component that references it.

## Architecture Check

1. `WorkstationTag` is an enum analogous to `CommodityKind` — a classification tag, not a component. It will be used inside `RecipeDefinition` and as a workstation marker component in a later ticket.
2. `RecipeId` is a simple `u32` newtype analogous to `ActionDefId`. It must live in core because `KnownRecipes` (a core component on Agent entities) stores a collection of recipe IDs.
3. No backwards-compatibility shims needed — these are brand new types.

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
- `RecipeDefinition` or `RecipeRegistry` (E10PROTRA-006)
- `KnownRecipes` component (E10PROTRA-004)
- Workstation marker component (E10PROTRA-005 or later)
- Any action logic
- Any systems crate changes

## Acceptance Criteria

### Tests That Must Pass

1. `WorkstationTag` variants satisfy `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.
2. `RecipeId` satisfies the same trait bounds.
3. Both round-trip through bincode.
4. `RecipeId(0) < RecipeId(1)` (ordering works).
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All types are deterministic (`Ord`, no `HashMap`).
2. No floating-point types used.
3. No external dependencies beyond `serde`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` — trait bounds, serialization round-trip, ordering

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
