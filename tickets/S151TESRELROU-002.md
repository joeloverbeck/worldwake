# S151TESRELROU-002: TestimonyReliability + RoutePreference runtime stores on AgentDecisionRuntime

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — two new core data types and two new fields on `AgentDecisionRuntime` (ai-crate runtime state)
**Deps**: S151TESRELROU-001

## Problem

S151 needs two per-agent learned-state stores: `TestimonyReliability` (per-witness-per-topic confirmation/refutation counts) and `RoutePreference` (per-segment safe/dangerous traversal counts). Both are runtime-only AI state (not ECS components), living alongside the existing `agenda_state` and `exhaustion_cache` fields on `AgentDecisionRuntime` at `crates/worldwake-ai/src/decision_runtime.rs:153-188`.

## Assumption Reassessment (2026-05-17)

1. `AgentDecisionRuntime` at `crates/worldwake-ai/src/decision_runtime.rs:153` derives `Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize`. 13 explicit construction sites workspace-wide (`failure_handling.rs:2140`, `interrupts.rs:381`, `agent_tick/tests.rs` ×9, `agent_tick/planning.rs:3397`, `observer.rs:7184`); none use spread syntax. Adding 2 new `Default`-derived fields is below the 15-site load-bearing threshold (informational only), but each site needs explicit field-name additions because no spread syntax exists.
2. `ReliabilityRecord` at `crates/worldwake-core/src/experience.rs:77-95` is the precedent: keyed by `SourceKey { entity: EntityId, commodity: CommodityKind }`. `TestimonyReliabilityKey { source: EntityId, topic: TopicScope }` mirrors this shape.
3. `RouteSegment` at `crates/worldwake-core/src/blocker_scope.rs:67-81` is a struct with `from`/`to` `EntityId` fields and a `::new(from, to)` constructor that canonicalizes endpoint order — using it as a `BTreeMap` key gives direction-independent route keying.
4. Spec D3+D4 at `specs/S151-testimony-reliability-and-route-preferences.md:124-179` defines the store types, entry shapes, and derived `trust()` / `preference()` views. All field types resolve to `worldwake-core` symbols (TopicScope from ticket 001, EntityId/Tick/EventId/Permille/RouteSegment all core).
5. `TestimonyTrustProfile` and `RoutePreferenceProfile` (the configuration inputs for the derived views) land in ticket 003. The `trust()` and `preference()` impl bodies in this ticket may use placeholder formulas guarded by a TODO referencing ticket 003 — or the impls may land in ticket 003 alongside the profile types they consume. Prefer the latter (impl follows profile) so the formula and its profile parameters land together and reviewers see the calibration as a single diff.

## Architecture Check

1. Per FND-22A: per-agent learned summaries are legal even when abstract; per-agent location on `AgentDecisionRuntime` matches the agent-local-state requirement.
2. Runtime-only placement (not ECS components) follows the `AgendaState` precedent at `decision_runtime.rs:183` — these are emergent per-agent tracking structures, not scenario-authorable substrate.
3. Both new stores derive `Default` so `AgentDecisionRuntime`'s existing `#[derive(...Default)]` continues to work without per-construction-site initialization beyond field-name addition.
4. `provenance_events: Vec<EventId>` is bounded (default 8-entry ring) at write time to prevent unbounded growth — physical dampener per FND-11.

## Verification Layers

1. Store-API correctness (insert, update, decay, derived-view computation) → focused unit tests in each store's `#[cfg(test)]` block.
2. `AgentDecisionRuntime` deserializes pre-bump save bytes cleanly (because both new fields are `Default`-derived) → save-load round-trip unit test in the `worldwake-ai` crate.
3. Single-layer ticket from the runtime-state perspective — write paths (observation hook), read paths (ranking/travel-cost), and serialization (SAVE_FORMAT_VERSION bump) are owned by downstream tickets.

## What to Change

### 1. Add `crates/worldwake-core/src/testimony_reliability.rs` (new)

```rust
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

use crate::ids::{EntityId, EventId, Tick};
use crate::numerics::Permille;
use crate::topic_scope::TopicScope;

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct TestimonyReliability {
    entries: BTreeMap<TestimonyReliabilityKey, TestimonyReliabilityEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TestimonyReliabilityKey {
    pub source: EntityId,
    pub topic: TopicScope,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestimonyReliabilityEntry {
    pub direct_confirmations: u32,
    pub direct_refutations: u32,
    pub stale_claims: u32,
    pub contradicted_claims: u32,
    pub last_updated_tick: Tick,
    pub provenance_events: Vec<EventId>,    // bounded ring buffer (PROVENANCE_RING_CAPACITY)
}

const PROVENANCE_RING_CAPACITY: usize = 8;

impl TestimonyReliability {
    pub fn record_confirmation(&mut self, key: TestimonyReliabilityKey, event: EventId, tick: Tick) { /* ... */ }
    pub fn record_refutation(&mut self, key: TestimonyReliabilityKey, event: EventId, tick: Tick) { /* ... */ }
    pub fn record_stale(&mut self, key: TestimonyReliabilityKey, event: EventId, tick: Tick) { /* ... */ }
    pub fn record_contradiction(&mut self, key: TestimonyReliabilityKey, event: EventId, tick: Tick) { /* ... */ }
    pub fn get(&self, key: &TestimonyReliabilityKey) -> Option<&TestimonyReliabilityEntry> { self.entries.get(key) }
    pub fn iter(&self) -> impl Iterator<Item = (&TestimonyReliabilityKey, &TestimonyReliabilityEntry)> { self.entries.iter() }
}
```

The `trust()` view method on `TestimonyReliabilityEntry` lands in ticket 003 alongside `TestimonyTrustProfile`.

### 2. Add `crates/worldwake-core/src/route_preference.rs` (new)

```rust
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

use crate::blocker_scope::RouteSegment;
use crate::ids::{EventId, Tick};

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct RoutePreference {
    entries: BTreeMap<RouteSegment, RoutePreferenceEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutePreferenceEntry {
    pub safe_traversals: u32,
    pub dangerous_traversals: u32,
    pub last_safe_tick: Option<Tick>,
    pub last_dangerous_tick: Option<Tick>,
    pub last_traversal_event: Option<EventId>,
}

impl RoutePreference {
    pub fn record_safe(&mut self, segment: RouteSegment, tick: Tick) { /* ... */ }
    pub fn record_dangerous(&mut self, segment: RouteSegment, event: EventId, tick: Tick) { /* ... */ }
    pub fn get(&self, segment: &RouteSegment) -> Option<&RoutePreferenceEntry> { self.entries.get(segment) }
    pub fn iter(&self) -> impl Iterator<Item = (&RouteSegment, &RoutePreferenceEntry)> { self.entries.iter() }
}
```

The `preference()` view method on `RoutePreferenceEntry` lands in ticket 003 alongside `RoutePreferenceProfile`.

### 3. Re-export from `crates/worldwake-core/src/lib.rs`

```rust
pub mod testimony_reliability;
pub mod route_preference;
pub use testimony_reliability::{TestimonyReliability, TestimonyReliabilityEntry, TestimonyReliabilityKey};
pub use route_preference::{RoutePreference, RoutePreferenceEntry};
```

### 4. Add fields to `AgentDecisionRuntime` (`crates/worldwake-ai/src/decision_runtime.rs:153-188`)

Append two new fields alongside `agenda_state`:

```rust
pub testimony_reliability: TestimonyReliability,
pub route_preference: RoutePreference,
```

Both are `Default`-derived. `AgentDecisionRuntime` already derives `Default`, so `Default::default()` continues to work.

### 5. Update the 13 explicit construction sites

For each site, append `testimony_reliability: TestimonyReliability::default(), route_preference: RoutePreference::default(),` to the struct literal. Sites:

- `crates/worldwake-ai/src/failure_handling.rs:2140`
- `crates/worldwake-ai/src/interrupts.rs:381`
- `crates/worldwake-ai/src/agent_tick/tests.rs:3182, 3296, 3439, 3574, 3654, 5705, 6181, 7676, 8321`
- `crates/worldwake-ai/src/agent_tick/planning.rs:3397`
- `crates/worldwake-cli/src/bin/observer.rs:7184`

## Files to Touch

- `crates/worldwake-core/src/testimony_reliability.rs` (new)
- `crates/worldwake-core/src/route_preference.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/interrupts.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- `trust()` / `preference()` derived-view impl bodies — land in ticket 003 with the profile parameter types
- Update paths (observation hook that calls `record_*` methods) — ticket 006
- Consumer reads via `GoalBeliefView` accessors — ticket 004
- `SAVE_FORMAT_VERSION` bump — ticket 010

## Acceptance Criteria

### Tests That Must Pass

1. `TestimonyReliability::record_confirmation` increments counters and pushes the EventId into the bounded ring; ring eviction removes the oldest entry at capacity.
2. `RoutePreference::record_safe` and `record_dangerous` update the appropriate timestamps and counters.
3. Both stores round-trip cleanly via bincode (`Default`-derived defaults + serde derives).
4. `AgentDecisionRuntime::default()` returns empty stores for both new fields.
5. Existing suite: `cargo test --workspace` (compile + behavior preserved).

### Invariants

1. Per-agent isolation: each `AgentDecisionRuntime` holds its own stores; no shared state across agents (no global trust/preference singleton).
2. Provenance ring bounded at `PROVENANCE_RING_CAPACITY = 8` — no unbounded `Vec<EventId>` growth.
3. Determinism: same observation sequence on a fresh store produces identical entries (BTreeMap ordering preserved).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/testimony_reliability.rs#[cfg(test)]` — record_*, ring-eviction, default emptiness.
2. `crates/worldwake-core/src/route_preference.rs#[cfg(test)]` — record_safe/dangerous, timestamp updates, default emptiness.
3. Existing `agent_tick/tests.rs` constructions continue to compile after the field additions.

### Commands

1. `cargo test -p worldwake-core testimony_reliability route_preference`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh`
