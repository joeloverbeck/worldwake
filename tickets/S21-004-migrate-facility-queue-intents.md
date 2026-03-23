# S21-004: Migrate facility queue intents from runtime to FacilityQueueIntents component

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI crate runtime reads/writes of queued_facility_intents migrated to World/WorldTxn
**Deps**: S21-001

## Problem

`AgentDecisionRuntime.queued_facility_intents: BTreeMap<EntityId, QueuedFacilityIntent>` holds per-agent facility use intentions that affect contention resolution. This state is lost on save/load. After S21-001 defines the `FacilityQueueIntents` component, this ticket migrates all reads and writes to the authoritative component.

## Assumption Reassessment (2026-03-23)

1. `queued_facility_intents` field is at `decision_runtime.rs` line 97.
2. Read/write sites confirmed via grep `queued_facility_intents` in AI crate:
   - `agent_tick/observation.rs` (lines 172, 184, 190, 290, 299) — `reconcile_committed_facility_queue_intents()` and facility access signature comparison
   - `decision_runtime.rs` — field definition, Default impl (empty BTreeMap)
3. This is the smallest of the three migration tickets — `queued_facility_intents` is only accessed in the observation module for reconciliation and dirty detection.
4. Independent of S21-002 (journey) and S21-003 (active goal). Can be done in parallel after S21-001.

## Architecture Check

1. `FacilityQueueIntents` wraps the same `BTreeMap<EntityId, QueuedFacilityIntent>` structure. The component is a thin wrapper that provides `Default` (empty map) for agents without intents.
2. No backward-compatibility shims.

## Verification Layers

1. All `queued_facility_intents` reads go through `world.get_component_facility_queue_intents(agent)` → grep for `runtime.queued_facility_intents` returns zero hits
2. All writes go through `txn.set_component_facility_queue_intents(agent, ...)` → grep for `runtime.queued_facility_intents` assignment returns zero hits
3. No behavioral change → all golden tests pass with unchanged hashes
4. Single-layer: observation module unit tests + golden E2E coverage

## What to Change

### 1. Migrate reads in agent_tick/observation.rs

- `reconcile_committed_facility_queue_intents()`: read `FacilityQueueIntents` from World/WorldTxn
- Facility access signature comparison: read intents from component

### 2. Migrate writes in agent_tick/observation.rs

- Intent addition/removal: read-modify-write through `txn.set_component_facility_queue_intents(agent, updated)`
- Intent clearing: `txn.clear_component_facility_queue_intents(agent)` or set to default

### 3. Remove field from AgentDecisionRuntime

Delete `queued_facility_intents` from struct (line 97). Update `Default` impl.

### 4. Update unit tests

Tests that set `runtime.queued_facility_intents` must use the component on a test World.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — remove `queued_facility_intents` field)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — read/write through World/WorldTxn)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — update fixtures if affected)

## Out of Scope

- Journey commitment migration (S21-002)
- Active goal migration (S21-003)
- New save/load golden tests (S21-005)
- Any changes to `worldwake-core` (S21-001)
- Any changes to `worldwake-sim` or `worldwake-systems`
- Facility queue contention system logic (the FacilityQueue system itself reads the component — that's a future concern only if a FacilityQueue system exists)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all unit tests and golden tests pass
2. `cargo test -p worldwake-ai --test golden_determinism` — replay hashes unchanged
3. `cargo clippy --workspace` — no new warnings

### Invariants

1. Zero occurrences of `runtime.queued_facility_intents` in the codebase (grep verification)
2. `AgentDecisionRuntime` no longer has a `queued_facility_intents` field
3. All facility intent writes go through `WorldTxn`, producing `ComponentDelta` entries
4. No golden test behavioral changes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/observation.rs` or `tests.rs` — updated fixtures to use `FacilityQueueIntents` component

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
