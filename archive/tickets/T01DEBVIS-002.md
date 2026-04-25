# T01DEBVIS-002: Force-directed place layout module

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: [archive/tickets/T01DEBVIS-001.md](T01DEBVIS-001.md)

## Problem

The visualizer needs deterministic, readable place positions for the canvas. Spec T01 §D9 specifies a hand-rolled Fruchterman-Reingold implementation with weighted ideal edge lengths (`k_e = k_base * travel_ticks_e`), 200 iterations with linear cooling, and BTreeMap-sorted iteration for within-platform determinism. The module is self-contained — it has no dependency on the rest of the visualizer crate and can be unit-tested in isolation.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The visualizer crate exists with a `src/` directory after T01DEBVIS-001 lands; this ticket adds `src/layout.rs` as a sibling module. Spec T01 §D9 declares the public API as `PlaceLayout::compute(places: &[EntityId], edges: &[(EntityId, EntityId, u32)], seed: u64) -> Self`.
2. `EntityId` is the canonical identity type at `crates/worldwake-core/src/ids.rs:44`; `rand_chacha::ChaCha8Rng` is a workspace-level dependency (used in `worldwake-sim`/`-core`). Reassessment 2026-04-25 confirmed both surfaces are unchanged.
3. Tooling-only ticket — pure module with no interaction with engine state, traces, or beliefs. No shared abstraction boundary.

## Architecture Check

1. Hand-rolled FR keeps the crate's external dep list minimal (`egui_graphs` brings transitive UI crate state and an internal RNG that would have to be wrapped). The implementation is ~80 LOC and well within review reach.
2. The layout's `ChaCha8Rng::seed_from_u64(seed)` is layout-local and never aliased onto the simulation's authoritative `DeterministicRng` (`crates/worldwake-sim/src/deterministic_rng.rs:13`). Per spec FOUNDATIONS Alignment table (P27): UI positions are caches, never authoritative state.

## Verification Layers

1. Layout determinism (within-platform) → focused unit test (`fr_layout_is_deterministic`) asserting bit-identical `Vec<(EntityId, Pos2)>` across two `compute` calls with the same `(places, edges, seed)`.
2. Topology fingerprint stability → focused unit test asserting input-vector order does not affect the fingerprint (BTreeMap-sorted internally).
3. Single-layer ticket — pure module, no decision/action/event-log surface to map. Per template item 6: additional layer mapping is not applicable.

## What to Change

### 1. Implement `PlaceLayout` and `PlaceLayout::compute`

Create `crates/worldwake-visualizer/src/layout.rs` with the public types and algorithm specified in T01 §D9:

- `pub struct PlaceLayout { positions: BTreeMap<EntityId, egui::Pos2>, topology_fingerprint: u64 }`.
- `pub fn compute(places: &[EntityId], edges: &[(EntityId, EntityId, u32)], seed: u64) -> Self`.
- Algorithm: seed positions from `ChaCha8Rng::seed_from_u64(seed)` inside `[0, 1000] × [0, 1000]`; compute `k_base = sqrt(area / n)` with `area = 1e6`; per-edge ideal length `k_e = k_base * travel_ticks_e`; 200 iterations of repulsive (`k_base² / |d|`) + attractive (`|d|² / k_e`) forces with linear cooling `t_i = t_0 * (1 - i / iterations)` and per-node displacement clamp; final centering at `(500, 500)`.
- Iteration order is BTreeMap-sorted for within-platform determinism. All `f32`; summation order fixed by node-ID sort. UI-only positions; cross-platform `f32` reproducibility is not required.

### 2. Topology fingerprint

Hash the sorted `(place_ids, edges)` tuple via a deterministic hash (`xxhash` if added as a dep, or a hand-rolled `wrapping_*` mix over sorted `EntityId` and edge tuples — keep dep-light). The fingerprint is consumed by T01DEBVIS-004 to decide whether to recompute layout on scenario reload.

### 3. Wire module into `lib.rs`

Add `pub mod layout;` to `crates/worldwake-visualizer/src/lib.rs`.

## Files to Touch

- `crates/worldwake-visualizer/src/layout.rs` (new)
- `crates/worldwake-visualizer/src/lib.rs` (modify — add module declaration)

## Out of Scope

- App integration / canvas rendering (T01DEBVIS-005).
- Cache invalidation logic on scenario reload (T01DEBVIS-004 owns the fingerprint check).
- Cross-platform `f32` reproducibility (explicit non-goal per FOUNDATIONS Alignment P27 in T01).
- Performance tuning for >30 places (explicit non-goal in T01 Non-Goals).

## Acceptance Criteria

### Tests That Must Pass

1. `fr_layout_is_deterministic` — same `(places, edges, seed)` produces a bit-identical positions map across two `compute` calls (within-platform).
2. `topology_fingerprint_stability` — fingerprint is unchanged when input vectors are passed in different orders (the same set yields the same fingerprint).
3. `topology_fingerprint_distinguishes_directed_edges` — reversing a directed edge changes the fingerprint.
4. Existing suite: `cargo test -p worldwake-visualizer` passes.

### Invariants

1. `PlaceLayout` positions are derived state; recomputation from `(places, edges, seed)` always reproduces them within-platform (per FND-27).
2. The module owns its `ChaCha8Rng` and never reads or writes the simulation's authoritative `DeterministicRng`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-visualizer/src/layout.rs` (`#[cfg(test)] mod tests`) — `fr_layout_is_deterministic`, `topology_fingerprint_stability`. Inline within the module since the file targets ~80 LOC of production + ~50 LOC of tests.

### Commands

1. `cargo test -p worldwake-visualizer layout::`
2. `cargo test -p worldwake-visualizer`
3. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-25.

- Added `crates/worldwake-visualizer/src/layout.rs` with deterministic Fruchterman-Reingold place layout, layout-local `ChaCha8Rng` seeding, fixed BTreeMap iteration order, weighted travel-tick edge lengths, final centering, and a dependency-light deterministic topology fingerprint over directed edge tuples.
- Exported the module from `crates/worldwake-visualizer/src/lib.rs`.
- Kept layout positions and topology fingerprint as derived UI/cache state only; no simulation state, event log, trace, or authoritative RNG path was touched.

## Deviations

- `PlaceLayout::positions` and `PlaceLayout::topology_fingerprint` landed as public fields so sibling modules in the later T01 visualizer tickets can consume the layout and fingerprint directly, matching the spec's downstream usage.
- The fingerprint uses a local `wrapping_*` FNV-style mix over sorted place IDs and directed edge tuples instead of adding an external hash dependency.

## Verification Result

- Passed `cargo test -p worldwake-visualizer --lib -- --list` to resolve exact focused selectors.
- Passed `cargo test -p worldwake-visualizer --lib layout::tests::fr_layout_is_deterministic -- --exact`.
- Passed `cargo test -p worldwake-visualizer --lib layout::tests::topology_fingerprint_stability -- --exact`.
- Passed `cargo test -p worldwake-visualizer --lib layout::tests::topology_fingerprint_distinguishes_directed_edges -- --exact`.
- Passed `cargo test -p worldwake-visualizer layout::`.
- Passed `cargo test -p worldwake-visualizer`.
- Passed `cargo clippy -p worldwake-visualizer --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh`, whose live gates are `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
