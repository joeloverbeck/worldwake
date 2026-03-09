# E02WORTOP-005: Prototype World Builder

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E02WORTOP-003 (Topology storage), E02WORTOP-004 (Route/pathfinding for connectivity validation)

## Problem

The simulation needs a fixed prototype world graph matching spec 4.1. A builder function must produce a deterministic, connected micro-world with 12-20 place nodes, stable IDs, and intentional connectivity for the playable scenario.

## Assumption Reassessment (2026-03-09)

1. `Topology`, `Place`, `PlaceTag`, `TravelEdge`, `TravelEdgeId`, and `Route` already exist in `crates/worldwake-core/src/topology.rs` from archived tickets E02WORTOP-001 through E02WORTOP-004 — confirmed.
2. `Topology` currently has private storage plus validating `add_place` / `add_edge` mutation APIs. The builder should compose those APIs rather than bypass them or introduce aliasing/mutable field access.
3. Spec 4.1 requires place categories and total node count, but does not mandate exact place names beyond those roles. The original ticket over-specified canonical names; this ticket should require a fixed manifest chosen in code, with stable deterministic names, rather than pretending the spec already chose the exact labels.
4. `Topology` serialization and stable hashing are explicitly deferred to E02WORTOP-006. This ticket must not depend on byte-level serialization tests to prove builder determinism.
5. Builder output must still be stable: same code path => same place IDs, edge IDs, place metadata, and graph structure. No randomness in the builder.
6. Every prototype place must have at least one incoming and one outgoing edge — spec requirement.

## Architecture Check

1. `build_prototype_world() -> Topology` should live with the topology module for now. A separate public builder type or ID factory is unnecessary until there is more than one topology manifest to compose.
2. The implementation should use a fixed in-code manifest for places and edges, then populate `Topology` through `add_place` / `add_edge`. That keeps legality checks centralized and is cleaner than constructing raw maps.
3. Place IDs should come from a fixed sequence of `EntityId { slot: n, generation: 0 }`; edge IDs should similarly use a fixed sequential `TravelEdgeId` order. This should remain a local implementation detail, not a new public API.
4. The graph should be hand-designed to be intentionally connected and navigable, with a clear low-danger village core and a riskier forest/bandit route. It should also be strongly connected so later movement systems can route in both directions without hidden one-way traps.
5. All `danger` and `visibility` values should be fixed `Permille` constants. The function remains pure: no external state, no RNG, no I/O.

## What to Change

### 1. Add `build_prototype_world()` function

Located in `topology.rs`.

Creates a fixed manifest with 12-20 places total. The manifest must include at least one place for each required spec role:

- village core
- farm/orchard
- general store
- inn or communal house
- ruler's hall
- barracks/guard post
- latrine/toilet facility
- crossroads
- forest route
- bandit camp

Additional connective places (for example gate, trail, road, or field nodes) may fill the remaining slots, but they must be intentional parts of the playable topology rather than filler.

The exact names are implementation-defined, but once chosen they must be deterministic and stable.

Edges connect these into a navigable graph. Exact edge list is implementation detail but must satisfy:
- Every place has >= 1 incoming and >= 1 outgoing edge.
- The graph is strongly connected (every place reachable from every other via directed edges).
- Travel times are reasonable integers (e.g., 1-10 ticks for village internal, 5-20 for longer routes).
- Danger/visibility values differentiate safe village roads from dangerous forest paths.

### 2. Re-export builder from `lib.rs`

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (modify — add `build_prototype_world()` and any helpers)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `build_prototype_world`)

## Out of Scope

- Random world generation — this is a fixed prototype.
- Population spawning (agents, NPCs) — that's E03+.
- Item placement — that's E04+.
- Any gameplay logic — later epics.
- Topology serialization round-trip — E02WORTOP-006.

## Acceptance Criteria

### Tests That Must Pass

1. `build_prototype_world()` creates 12-20 places — spec test.
2. Every prototype place has at least one incoming and one outgoing edge — spec test.
3. Graph traversal from every place reaches every other place (strong connectivity) — spec-aligned builder test.
4. Builder is deterministic: two calls produce identical `Topology` values by structural equality. Byte-level serialization/hash checks remain in E02WORTOP-006.
5. All `danger` values are valid `Permille` (0..=1000) — inherently enforced by type but tested.
6. All `visibility` values are valid `Permille` (0..=1000).
7. All `travel_time_ticks >= 1` for every edge.
8. Place names are non-empty strings.
9. The manifest covers every spec-required role via tags (for example at least one `Village`, `Farm`, `Store`, `Inn`, `Hall`, `Barracks`, `Latrine`, `Crossroads`, `Forest`, and `Camp` place, with connective tags as needed).
10. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. Builder output is stable: same code path => same place IDs, edge IDs, place metadata, and graph structure — spec invariant.
2. No randomness in the builder function (no RNG parameter, no system calls).
3. No floating-point values anywhere in the output.
4. The generated graph matches the spec 4.1 role list and node-count range without inventing extra mandatory requirements not present in the spec.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/topology.rs` — builder tests: place count, strong connectivity, deterministic equality, role/tag coverage, and edge invariant checks.

### Commands

1. `cargo test -p worldwake-core build_prototype`
2. `cargo test -p worldwake-core topology`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Corrected the ticket before implementation to match the current codebase and `specs/E02-world-topology.corrected.md`, especially removing the premature serialization/hash requirement and replacing over-specified canonical place names with a fixed-but-implementation-defined manifest.
  - Added `build_prototype_world()` in `crates/worldwake-core/src/topology.rs` as a manifest-driven builder that composes the existing validating `Topology::add_place` / `Topology::add_edge` APIs instead of introducing a parallel construction path.
  - Added a 12-place deterministic prototype topology with stable `EntityId` / `TravelEdgeId` assignment, bidirectional reachability, and explicit danger/visibility gradients between the village and forest/bandit route.
  - Re-exported `build_prototype_world` from `crates/worldwake-core/src/lib.rs`.
  - Added focused builder tests for determinism, strong connectivity, required-role coverage, per-place incoming/outgoing edges, and travel stat/risk-gradient invariants.
- Deviations from original plan:
  - The original ticket required byte-identical topology serialization to prove determinism, but `Topology` serialization belongs to E02WORTOP-006 and is not implemented yet. This ticket now verifies determinism by structural equality, which is the correct boundary for the current architecture.
  - The original ticket treated a local `EntityId` factory as a distinct deliverable and hardcoded exact place names as if the spec required them. That was simplified: stable ID assignment remains a private implementation detail, and the builder now follows spec roles without inventing extra canonical naming requirements.
- Verification results:
  - `cargo test -p worldwake-core build_prototype` passed.
  - `cargo test -p worldwake-core topology` passed.
  - `cargo fmt --all` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
