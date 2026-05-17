# S151TESRELROU-002: TestimonyReliability + RoutePreference runtime stores on AgentDecisionRuntime

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — two new core data types and two new fields on `AgentDecisionRuntime` (ai-crate runtime state)
**Deps**: archive/tickets/S151TESRELROU-001.md

## Problem

S151 needs two per-agent learned-state stores: `TestimonyReliability` (per-witness-per-topic confirmation/refutation counts) and `RoutePreference` (per-segment safe/dangerous traversal counts). Both are runtime-only AI state (not ECS components), living alongside the existing `agenda_state` and `exhaustion_cache` fields on `AgentDecisionRuntime` at `crates/worldwake-ai/src/decision_runtime.rs:153-188`.

## Assumption Reassessment (2026-05-17)

1. `AgentDecisionRuntime` at `crates/worldwake-ai/src/decision_runtime.rs:153` derives `Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize`. Live reassessment found 70 textual `AgentDecisionRuntime {` occurrences across `crates/worldwake-ai` and `crates/worldwake-cli`, not the drafted 13-site estimate. Most already used `..AgentDecisionRuntime::default()`; the only compiler-required full-literal fallout was the existing round-trip fixture in `crates/worldwake-ai/src/decision_runtime.rs`. The shared-struct constructor sweep stayed in scope.
2. `ReliabilityRecord` at `crates/worldwake-core/src/experience.rs:77-95` is the precedent: keyed by `SourceKey { entity: EntityId, commodity: CommodityKind }`. `TestimonyReliabilityKey { source: EntityId, topic: TopicScope }` mirrors this shape.
3. `RouteSegment` at `crates/worldwake-core/src/blocker_scope.rs:67-81` is a struct with `from`/`to` `EntityId` fields and a `::new(from, to)` constructor that canonicalizes endpoint order — using it as a `BTreeMap` key gives direction-independent route keying.
4. Spec D3+D4 at `specs/S151-testimony-reliability-and-route-preferences.md:124-179` defines the store types, entry shapes, and derived `trust()` / `preference()` views. All field types resolve to `worldwake-core` symbols (TopicScope from ticket 001, EntityId/Tick/EventId/Permille/RouteSegment all core).
5. `TestimonyTrustProfile` and `RoutePreferenceProfile` (the configuration inputs for the derived views) land in ticket 003. The `trust()` and `preference()` impl bodies remained deferred to ticket 003 alongside the profile types they consume, so the formula and its profile parameters are reviewed together.

## Architecture Check

1. Per FND-22A: per-agent learned summaries are legal even when abstract; per-agent location on `AgentDecisionRuntime` matches the agent-local-state requirement.
2. Runtime-only placement (not ECS components) follows the `AgendaState` precedent at `decision_runtime.rs:183` — these are emergent per-agent tracking structures, not scenario-authorable substrate.
3. Both new stores derive `Default` so `AgentDecisionRuntime`'s existing `#[derive(...Default)]` continues to work without per-construction-site initialization beyond field-name addition.
4. `provenance_events: Vec<EventId>` is bounded (default 8-entry ring) at write time to prevent unbounded growth — physical dampener per FND-11.

## Verified Layers

1. Store-API correctness (insert, update, decay, derived-view computation) → focused unit tests in each store's `#[cfg(test)]` block.
2. `AgentDecisionRuntime` preserves the S151 stores on current-format bincode round-trip and defaults both stores empty on `AgentDecisionRuntime::default()` → focused unit tests in the `worldwake-ai` crate. Version-boundary and old-save assertions remain owned by ticket 010.
3. Single-layer ticket from the runtime-state perspective — write paths (observation hook), read paths (ranking/travel-cost), and serialization (SAVE_FORMAT_VERSION bump) are owned by downstream tickets.

## Landed Changes

### 1. Added `crates/worldwake-core/src/testimony_reliability.rs`

The landed module defines `TestimonyReliability`, `TestimonyReliabilityKey`, `TestimonyReliabilityEntry`, and `PROVENANCE_RING_CAPACITY`. The store is backed by `BTreeMap<TestimonyReliabilityKey, TestimonyReliabilityEntry>`, records confirmation/refutation/stale/contradiction counters, updates `last_updated_tick`, retains a bounded EventId provenance ring, exposes `get()`, `iter()`, and `is_empty()`, and has focused unit/bincode tests.

The `trust()` view method on `TestimonyReliabilityEntry` remains owned by ticket 003 alongside `TestimonyTrustProfile`.

### 2. Added `crates/worldwake-core/src/route_preference.rs`

The landed module defines `RoutePreference` and `RoutePreferenceEntry`. The store is backed by `BTreeMap<RouteSegment, RoutePreferenceEntry>`, records safe and dangerous traversals, updates last-safe/last-danger ticks plus the last dangerous traversal event, exposes `get()`, `iter()`, and `is_empty()`, and has focused unit/bincode tests including canonical route-segment keying.

The `preference()` view method on `RoutePreferenceEntry` remains owned by ticket 003 alongside `RoutePreferenceProfile`.

### 3. Re-exported from `crates/worldwake-core/src/lib.rs`

`worldwake-core` now declares and re-exports the two modules plus `RoutePreference`, `RoutePreferenceEntry`, `PROVENANCE_RING_CAPACITY`, `TestimonyReliability`, `TestimonyReliabilityEntry`, and `TestimonyReliabilityKey`.

### 4. Added fields to `AgentDecisionRuntime` (`crates/worldwake-ai/src/decision_runtime.rs`)

`AgentDecisionRuntime` now has defaulted `testimony_reliability: TestimonyReliability` and `route_preference: RoutePreference` fields alongside the other runtime-only AI state. `AgentDecisionRuntime::default()` starts with both stores empty, and current-format bincode round-trip preserves populated entries.

### 5. Updated constructor fallout

The live constructor fallout was narrower than the initial textual sweep because most literals use `..AgentDecisionRuntime::default()`. The current-format bincode round-trip fixture now populates and asserts both new stores.

## Landed Files

- `crates/worldwake-core/src/testimony_reliability.rs` (new)
- `crates/worldwake-core/src/route_preference.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)

## Out of Scope

- `trust()` / `preference()` derived-view impl bodies — land in ticket 003 with the profile parameter types
- Update paths (observation hook that calls `record_*` methods) — ticket 006
- Consumer reads via `GoalBeliefView` accessors — ticket 004
- `SAVE_FORMAT_VERSION` bump — ticket 010

## Acceptance Result

### Tests Passed

1. `TestimonyReliability::record_confirmation`, `record_refutation`, `record_stale`, and `record_contradiction` increment counters, update ticks, and push EventIds into the bounded provenance ring.
2. Ring eviction removes the oldest testimony provenance entry at `PROVENANCE_RING_CAPACITY`.
3. `RoutePreference::record_safe` and `record_dangerous` update the appropriate timestamps, counters, and dangerous traversal event.
4. Both stores round-trip cleanly via bincode.
5. `AgentDecisionRuntime::default()` returns empty stores for both new fields, and current-format `AgentDecisionRuntime` bincode round-trip preserves populated S151 stores.
6. Existing suite: `cargo test --workspace` passed.

### Invariants

1. Per-agent isolation: each `AgentDecisionRuntime` holds its own stores; no shared state across agents (no global trust/preference singleton).
2. Provenance ring bounded at `PROVENANCE_RING_CAPACITY = 8` — no unbounded `Vec<EventId>` growth.
3. Determinism: same observation sequence on a fresh store produces identical entries (BTreeMap ordering preserved).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/testimony_reliability.rs#[cfg(test)]` — record_*, ring-eviction, default emptiness.
2. `crates/worldwake-core/src/route_preference.rs#[cfg(test)]` — record_safe/dangerous, timestamp updates, default emptiness.
3. `crates/worldwake-ai/src/decision_runtime.rs#[cfg(test)]` — current-format runtime bincode round-trip preserves populated S151 stores; default runtime starts with empty S151 stores.

### Commands Run

1. `cargo test -p worldwake-core testimony_reliability route_preference`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completed on 2026-05-17.

- Added `TestimonyReliability`, `TestimonyReliabilityKey`, and `TestimonyReliabilityEntry` in `worldwake-core`, with bounded provenance retention and focused bincode/unit coverage.
- Added `RoutePreference` and `RoutePreferenceEntry` in `worldwake-core`, keyed by canonical `RouteSegment`, with focused bincode/unit coverage.
- Re-exported the new S151 store types from `worldwake-core`.
- Added defaulted `testimony_reliability` and `route_preference` fields to `AgentDecisionRuntime`.
- Extended the existing `AgentDecisionRuntime` bincode round-trip fixture and default-state unit coverage for the new stores.

## Deviations

- The drafted 13 explicit `AgentDecisionRuntime` constructor-site count was stale. Live reassessment found 70 textual literals, but most used `..AgentDecisionRuntime::default()` and required no source edit; only the current-format round-trip fixture needed explicit field additions.
- The ticket originally implied pre-bump save-byte deserialization proof through serde defaults. This ticket proves current-format runtime serialization and default emptiness only; `SAVE_FORMAT_VERSION` and old-save boundary claims are owned by the now-archived `archive/tickets/S151TESRELROU-010.md`.
- `trust()` and `preference()` derived-view formulas intentionally remain deferred to the now-archived `archive/tickets/S151TESRELROU-003.md`, where the profile types land.

## Verification Result

- Passed `cargo test -p worldwake-core testimony_reliability`
- Passed `cargo test -p worldwake-core route_preference`
- Passed `cargo test -p worldwake-ai --lib decision_runtime::tests::agent_decision_runtime_bincode_round_trip_preserves_all_fields -- --exact`
- Passed `cargo test -p worldwake-ai --lib decision_runtime::tests::agent_decision_runtime_default_starts_with_empty_s151_stores -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
