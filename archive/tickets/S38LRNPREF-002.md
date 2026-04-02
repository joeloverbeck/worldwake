# S38LRNPREF-002: Memory eviction and post-load pruning

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S38LRNPREF-001

## Problem

Experience stores grow unbounded without eviction. Agents need capacity limits and staleness eviction to prevent unbounded memory growth (P11 dampener). Post-load pruning must remove references to dead entities after deserialization.

## Assumption Reassessment (2026-04-02)

1. Existing eviction pattern in `AgentBeliefStore::enforce_limits()` at `crates/worldwake-core/src/belief.rs:112-131` — evicts by staleness first, then by capacity (oldest `observed_tick`). Our eviction follows the same pattern.
2. `RouteExperience`, `SourceReliability`, `PreferenceProfile` exist after S38LRNPREF-001.
3. `memory_retention_ticks: u64` and `route_memory_capacity: u32` / `source_memory_capacity: u32` on `PreferenceProfile` control eviction bounds.
4. Binary eviction only — no gradual decay. Matches `PerceptionProfile` pattern.
5. No existing post-load pruning infrastructure for experience components. The `World` type has `is_alive(entity)` for dead-entity checks.
6. `BTreeMap` iteration is deterministic (sorted by key), ensuring deterministic eviction when multiple records share the oldest tick.

## Architecture Check

1. Eviction as methods on the component types (e.g., `RouteExperience::enforce_limits`) keeps the logic co-located with the data, matching how `AgentBeliefStore` handles its own eviction. Cleaner than a separate system function.
2. No backward-compatibility shims.

## Verification Layers

1. Staleness eviction removes old records → focused unit test with controlled tick values
2. Capacity eviction removes oldest when full → focused unit test with known insertion order
3. Post-load pruning removes dead entities → focused unit test with dead EntityId/TravelEdgeId
4. Single-layer ticket (worldwake-core eviction logic); no cross-system verification needed.

## What to Change

### 1. Eviction methods on RouteExperience

Add `pub fn enforce_limits(&mut self, current_tick: Tick, profile: &PreferenceProfile)` to `RouteExperience`:
- Remove entries where `current_tick.0 - entry.last_travel_tick.0 > profile.memory_retention_ticks`.
- If `self.edges.len() > profile.route_memory_capacity as usize`, remove entries with oldest `last_travel_tick` until within capacity.

### 2. Eviction methods on SourceReliability

Add `pub fn enforce_limits(&mut self, current_tick: Tick, profile: &PreferenceProfile)` to `SourceReliability`:
- Remove entries where `current_tick.0 - entry.last_attempt_tick.0 > profile.memory_retention_ticks`.
- If `self.sources.len() > profile.source_memory_capacity as usize`, remove entries with oldest `last_attempt_tick` until within capacity.

### 3. Post-load pruning methods

Add `pub fn prune_dead_edges(&mut self, is_valid_edge: impl Fn(&TravelEdgeId) -> bool)` to `RouteExperience` — removes entries for edges that no longer exist in the topology.

Add `pub fn prune_dead_sources(&mut self, is_alive: impl Fn(&EntityId) -> bool)` to `SourceReliability` — removes entries for entities that are dead.

## Files to Touch

- `crates/worldwake-core/src/experience.rs` (modify — add eviction and pruning methods)

## Out of Scope

- Calling eviction from action handlers (S38LRNPREF-004, 005 will call `enforce_limits` after recording)
- Calling post-load pruning from the save/load pipeline (integration deferred to S38LRNPREF-008 or a follow-up)
- Ranking logic (S38LRNPREF-006, 007)

## Acceptance Criteria

### Tests That Must Pass

1. Stale route records evicted when `current_tick - last_travel_tick > memory_retention_ticks`
2. Oldest route records evicted when `edges.len() > route_memory_capacity`
3. Stale source records evicted when `current_tick - last_attempt_tick > memory_retention_ticks`
4. Oldest source records evicted when `sources.len() > source_memory_capacity`
5. `prune_dead_edges` removes entries for invalid edge IDs
6. `prune_dead_sources` removes entries for dead entity IDs
7. Eviction is deterministic (same input → same eviction order)
8. Existing suite: `cargo test --workspace`

### Invariants

1. After `enforce_limits`, `edges.len() <= route_memory_capacity` and `sources.len() <= source_memory_capacity`
2. No records older than `memory_retention_ticks` survive eviction
3. Eviction is binary — no gradual decay applied

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/experience.rs` (modify test module) — eviction focused tests: staleness, capacity, combined, edge cases (empty store, zero capacity), pruning

### Commands

1. `cargo test -p worldwake-core experience`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

- **Completed**: 2026-04-02
- **What changed**:
  - added `RouteExperience::enforce_limits` in `crates/worldwake-core/src/experience.rs` for binary staleness eviction plus oldest-record capacity eviction
  - added `SourceReliability::enforce_limits` in `crates/worldwake-core/src/experience.rs` with the matching staleness and capacity behavior
  - added `RouteExperience::prune_dead_edges` and `SourceReliability::prune_dead_sources` in `crates/worldwake-core/src/experience.rs`
  - expanded the focused `experience.rs` test module to cover stale-record eviction, capacity eviction, deterministic oldest-tick tie-breaking, and dead-reference pruning
- **Deviations from original plan**:
  - none; the ticket remained a single-layer `worldwake-core` data-method slice, and save/load pipeline integration stayed out of scope
- **Verification**:
  - `cargo test -p worldwake-core experience`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
