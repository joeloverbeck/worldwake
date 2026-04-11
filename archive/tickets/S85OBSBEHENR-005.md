# S85OBSBEHENR-005: Unknown location clarity for place entities

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S85 (Observer Behavioral Enrichment)

## Problem

The observer renders `last_known_place: None` as `"Unknown location"` for all entities. For place entities this is expected (places don't have a parent location), but the display is confusing and can mislead diagnosis of belief gaps.

## Assumption Reassessment (2026-04-10)

1. "Unknown location" rendered at `observer.rs:901` in the "Believed entity locations" section. The code iterates `store.known_entities` grouped by `state.last_known_place` at `observer.rs:888-895`. `BelievedEntityState` has `believed_kind: Option<EntityKind>` at `belief.rs:1322`. The belief store is available as `store` in the rendering loop at `observer.rs:890`.
2. S85 spec (Deliverable 5) describes this change. S77 (completed) added the `believed_kind` field to `BelievedEntityState`.
3. Single-layer ticket: observer-only formatting change. No shared abstraction boundary.

## Architecture Check

1. Uses the belief store's own `believed_kind` (not authoritative `world.entity_kind()`) to classify entities — consistent with the section being about **believed** entity locations. The change only affects the label for the `None` place group, branching on whether the entity is a believed place.
2. No backwards-compatibility aliasing or shims introduced.

## Verification Layers

1. Place entities show `"(place entity — no parent location)"` instead of `"Unknown location"` → focused unit test
2. Non-place entities with `last_known_place: None` still show `"Unknown location"` → focused unit test
3. Single-layer observer-only ticket; no action/planning/event-log layer mapping applicable.

## What to Change

### 1. Distinguish place entities in Unknown location rendering

At `observer.rs:898-902`, when rendering entities with `place_opt == None`, instead of labeling all as `"Unknown location"`, split the entities in the `None` group:

- Entities where `store.known_entities[entity].believed_kind == Some(EntityKind::Place)`: collect under label `"(place entity — no parent location)"`
- All other entities: collect under label `"Unknown location"` as before

This means the `None` group may produce two output lines if it contains both place entities and non-place entities.

### 2. Add unit tests

- Test with a belief store containing a place entity with `believed_kind: Some(EntityKind::Place)` and `last_known_place: None` — verify output shows `"(place entity — no parent location)"`
- Test with a non-place entity with `last_known_place: None` — verify output shows `"Unknown location"`
- Test with both types in the same `None` group — verify both labels appear

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Modifying simulation behavior or AI decision-making
- Changing how `believed_kind` is set during perception
- Enriching location display for entities with known locations
- Interactive observer features or live dashboards

## Acceptance Criteria

### Tests That Must Pass

1. New test: place entity with `last_known_place: None` renders as `"(place entity — no parent location)"`
2. New test: non-place entity with `last_known_place: None` renders as `"Unknown location"`
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Observer remains read-only — no mutation of world or belief state
2. Entities with `last_known_place: Some(place_id)` are rendered unchanged
3. Uses `believed_kind` from belief store, not authoritative `world.entity_kind()`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (inline tests) — verifies place vs non-place distinction in Unknown location rendering

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-10.

- Added observer-local helpers to keep believed-location rendering shared while splitting the `last_known_place: None` bucket into believed places vs. other entities using `BelievedEntityState::believed_kind`.
- The "Believed entity locations" section now renders believed places with `"(place entity — no parent location)"` and preserves `"Unknown location"` for non-place entities with no believed parent place.
- Added focused observer tests covering place-only, non-place-only, and mixed unknown-location groups.

## Verification Result

- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
