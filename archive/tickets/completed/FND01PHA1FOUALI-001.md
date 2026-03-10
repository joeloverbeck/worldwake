# FND01PHA1FOUALI-001: Remove Route Danger/Visibility Scores from TravelEdge

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — topology.rs struct, constructor, accessors, prototype builder
**Deps**: None (independent, can parallelize with -002 and -003)

## Problem

`TravelEdge` stores `danger: Permille` and `visibility: Permille` — abstract scores with no causal grounding. These violate Principle 3 (Concrete State Over Abstract Scores): route danger should emerge from which entities are physically present on the route, not from a stored number.

## Assumption Reassessment (2026-03-10)

1. `TravelEdge` struct at `topology.rs:40-48` has fields `danger: Permille` and `visibility: Permille` — confirmed.
2. `TravelEdge::new()` at `topology.rs:51-73` accepts `danger: Permille` and `visibility: Permille` parameters — confirmed.
3. Accessor methods `danger()` and `visibility()` exist at `topology.rs:95-101` — confirmed.
4. `PrototypeEdgeSpec` at `topology.rs:335-343` has `danger: u16` and `visibility: u16` fields — confirmed.
5. `build_prototype_world()` at `topology.rs:292-326` uses `prototype_permille(spec.danger)` and `prototype_permille(spec.visibility)` — confirmed.
6. The prototype-world score assertions live in `build_prototype_world_edges_have_valid_stats_and_risk_gradient` near the end of `topology.rs`, not in a test named `prototype_world_dangerousness_varies_by_location` — confirmed, must be replaced with structural assertions only.
7. Test `travel_edge_roundtrips_with_permille_fields` tests serde round-trip including danger/visibility — confirmed, must be updated and renamed.
8. Test helpers `edge_with_ticks(...)` and `RawTravelEdge` currently depend on `Permille` solely to construct or deserialize `TravelEdge` fixtures — confirmed, must be updated as part of the cleanup.
9. No code outside `topology.rs` reads `danger()` or `visibility()` on `TravelEdge` — confirmed via grep across `crates/` (only docs/spec/ticket text reference it outside this file).

## Architecture Check

1. Removing stored abstract scores is better than the current architecture. `TravelEdge` is a structural topology primitive, so embedding authored risk/visibility scores mixes navigation facts with simulation judgments that should emerge from concrete state.
2. Route danger/visibility should eventually derive from route occupants, witnesses, and other local state introduced in later epics; until then, the clean design is to have no score at all rather than preserve a misleading abstraction.
3. No backward-compatibility shims — all references are removed cleanly (Principle 13).

## What to Change

### 1. Remove fields from `TravelEdge` struct

Remove `danger: Permille` and `visibility: Permille` from the struct definition at `topology.rs:40-48`.

### 2. Remove from `TravelEdge::new()` constructor

Remove `danger` and `visibility` parameters from the constructor signature at `topology.rs:51-73`. Remove field assignments.

### 3. Remove accessor methods

Remove `pub fn danger(&self) -> Permille` and `pub fn visibility(&self) -> Permille` at `topology.rs:95-101`.

### 4. Update `PrototypeEdgeSpec`

Remove `danger: u16` and `visibility: u16` fields from the struct at `topology.rs:335-343`. Update all entries in the prototype edge table to remove these fields.

### 5. Update `build_prototype_world()`

Remove `prototype_permille(spec.danger)` and `prototype_permille(spec.visibility)` from edge construction calls inside `build_prototype_world()`.

### 6. Update all `TravelEdge::new()` call sites

Remove `danger` and `visibility` arguments from every call to `TravelEdge::new()` in tests and production code.

### 7. Remove/update tests

- Replace `build_prototype_world_edges_have_valid_stats_and_risk_gradient` with a structural prototype-world test that validates only invariants that still belong on topology edges after this change (for example, positive travel time and valid endpoint references).
- Update `travel_edge_roundtrips_with_permille_fields` to remove danger/visibility from expected serialized form. Rename to `travel_edge_serde_roundtrip` since it no longer tests permille fields specifically.
- Update `travel_edge_construction_accepts_minimum_valid_ticks` to assert only the remaining `TravelEdge` fields.
- Update helper `edge_with_ticks(...)` to stop manufacturing removed score values.
- Update `RawTravelEdge` and `travel_edge_deserialization_rejects_zero_ticks` so deserialization coverage still checks the `NonZeroU32` invariant without obsolete fields.
- Keep all structural/connectivity/pathfinding tests intact.

### 8. Remove `Permille` import if unused

If `Permille` is no longer used in `topology.rs` after this change, remove the import and the `prototype_permille(...)` helper. `Permille` should disappear from this module entirely if the cleanup is complete.

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (modify)

## Out of Scope

- Do NOT implement concrete route presence (deferred to E10/E11/E12).
- Do NOT modify `Place`, `PlaceTag`, `Route`, `Topology`, or Dijkstra pathfinding.
- Do NOT touch any file outside `topology.rs`.
- Do NOT add any replacement for danger/visibility.
- Do NOT modify `Permille` or `numerics.rs`.
- Do NOT preserve old serialized field layouts or add serde aliasing/compatibility behavior.

## Acceptance Criteria

### Tests That Must Pass

1. `TravelEdge` has no `danger` or `visibility` fields (compile-time enforcement).
2. No accessor methods `danger()` or `visibility()` exist on `TravelEdge`.
3. `PrototypeEdgeSpec` has no `danger` or `visibility` fields.
4. No test references `danger` or `visibility` on edges.
5. Existing suite: `cargo test -p worldwake-core`
6. Full suite: `cargo test --workspace`
7. `cargo clippy --workspace` clean.

### Invariants

1. All structural topology tests (connectivity, pathfinding, place graph) continue to pass unchanged.
2. `TravelEdge` retains: `id`, `from`, `to`, `travel_time_ticks`, `capacity`.
3. `build_prototype_world()` still produces a valid topology with 12 places and directed edges.
4. Serde round-trip for `TravelEdge` still works (with updated format and no compatibility shim for removed fields).

## Test Plan

### New/Modified Tests

1. `topology.rs::travel_edge_serde_roundtrip` — renamed from permille-specific test, updated to exclude danger/visibility fields.
2. `topology.rs::travel_edge_construction_accepts_minimum_valid_ticks` — updated to assert only structural edge fields.
3. `topology.rs::travel_edge_deserialization_rejects_zero_ticks` — fixture updated to the new serialized shape.
4. `topology.rs::build_prototype_world_edges_have_valid_stats_and_risk_gradient` — replaced with a topology-only invariant test after score removal.

### Commands

1. `cargo test -p worldwake-core -- topology`
2. `cargo test -p worldwake-core`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Actually changed:
- Removed `danger` and `visibility` from `TravelEdge`, its constructor, and its accessors.
- Removed `danger` and `visibility` from `PrototypeEdgeSpec` and deleted the `prototype_permille(...)` helper.
- Updated all in-file helpers and serde fixtures to the new structural `TravelEdge` shape.
- Replaced the prototype score-gradient test with a topology-only invariant test that validates edge endpoints, adjacency registration, and positive travel time.

Changed from the original plan:
- The implementation stayed entirely inside `crates/worldwake-core/src/topology.rs` as expected.
- The prototype-world test that needed replacement was `build_prototype_world_edges_have_valid_stats_and_risk_gradient`, not `prototype_world_dangerousness_varies_by_location`.
- No extra compatibility or aliasing was added for the removed serialized fields; the serialized shape changed directly, per Principle 13.
