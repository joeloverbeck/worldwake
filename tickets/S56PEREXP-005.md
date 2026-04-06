# S56PEREXP-005: Scenario integration for `PlaceVisibilityProfile`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — scenario loading, PlaceDef
**Deps**: S56PEREXP-003

## Problem

After S56PEREXP-003 registers `PlaceVisibilityProfile` as a component, scenario authors need a way to set it on places via RON scenario files. Without this, all places default to zero concealment and the modulation system has no environmental input.

## Assumption Reassessment (2026-04-06)

1. `PlaceDef` at `crates/worldwake-cli/src/scenario/types.rs:36-42` has `name: String` and `tags: Vec<PlaceTag>`. Both fields are scenario-definable.
2. `build_topology()` at `crates/worldwake-cli/src/scenario/mod.rs:130-191` creates places and inserts them into `Topology`. Place entity IDs are assigned manually from slot indices (line 139), not via `txn.create_entity()`.
3. Components can be set on place entity IDs via `WorldTxn` — confirmed by `set_component_bandit_camp` usage in tests.
4. `PlaceDef` derives `Clone, Debug, Deserialize` — the `Option<PlaceVisibilityProfile>` field needs `PlaceVisibilityProfile` to also derive `Deserialize` (already required for component registration).
5. The `build_topology()` function receives `&mut Topology` but does NOT currently have a `WorldTxn` — components like `BanditCamp` on places are set elsewhere. Need to verify where place component assignment happens and follow the same pattern.

## Architecture Check

1. Optional field on `PlaceDef` with `#[serde(default)]` — places without `visibility_profile` in RON get `None`, meaning zero concealment. Clean degradation.
2. Follows the `AgentDef` pattern for optional profile fields. No backwards-compatibility shims.

## Verification Layers

1. PlaceDef deserialization with and without visibility_profile -> RON parsing test
2. Place component set when profile is present -> integration test
3. Place component absent when profile is None -> default behavior unchanged
4. Single-layer ticket (scenario loading) — no decision/action trace needed.

## What to Change

### 1. Add optional field to `PlaceDef`

In `crates/worldwake-cli/src/scenario/types.rs`:

```rust
pub struct PlaceDef {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<PlaceTag>,
    #[serde(default)]
    pub visibility_profile: Option<PlaceVisibilityProfile>,
}
```

Import `PlaceVisibilityProfile` from `worldwake_core`.

### 2. Wire component assignment in scenario loading

Find where place components (like `BanditCamp`) are set on place entity IDs and follow the same pattern. If it happens in `spawn_entities()` or a separate function, add:

```rust
if let Some(vis) = &place_def.visibility_profile {
    txn.set_component_place_visibility_profile(place_id, vis.clone())?;
}
```

### 3. Add import to `Cargo.toml` if needed

Verify `crates/worldwake-cli/Cargo.toml` already depends on `worldwake-core` (it should, since it uses many core types).

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)

## Out of Scope

- Updating existing scenario `.ron` files to include visibility profiles — that's part of golden test scenarios (S56PEREXP-006)
- Default visibility profiles for existing places

## Acceptance Criteria

### Tests That Must Pass

1. RON deserialization of `PlaceDef` with `visibility_profile: Some(PlaceVisibilityProfile { base_concealment: Permille(400) })` succeeds
2. RON deserialization of `PlaceDef` without `visibility_profile` succeeds (defaults to `None`)
3. Place entity has `PlaceVisibilityProfile` component after spawn when profile is in RON
4. Place entity does NOT have `PlaceVisibilityProfile` component when profile is absent from RON
5. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Existing scenario files continue to parse without changes (serde default)
2. `PlaceVisibilityProfile` is only set on Place entities

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/` — scenario loading test with and without visibility profile

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy --workspace --all-targets -- -D warnings`
