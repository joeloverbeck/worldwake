# GOLDHARN-001: Add Event-Log Assertion Helpers to Golden Harness

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `crates/worldwake-ai/tests/golden_harness/mod.rs`, `crates/worldwake-core/src/event_log.rs`, `crates/worldwake-core/src/delta.rs`

## Problem

Cross-system golden tests currently have to reconstruct authoritative mutation ordering by manually iterating raw event-log records and matching deltas inline. That creates repeated forensic boilerplate and makes mixed-layer assertions harder to read than they need to be.

## Assumption Reassessment (2026-03-18)

1. `crates/worldwake-ai/tests/golden_harness/mod.rs` already exposes high-value helpers for agents, offices, beliefs, and action tracing, but it does not expose reusable event-log search/assertion helpers.
2. `crates/worldwake-core/src/event_log.rs` provides the raw primitives needed (`len()`, `get()`, `events_by_tag()`), and `crates/worldwake-core/src/delta.rs` exposes typed `StateDelta`, `ComponentDelta`, and `RelationDelta`.
3. Recent cross-system work needed generic queries such as:
   - first event that sets `DeadAt` on entity X
   - first event that removes `OfficeHolder { office, holder }`
   - first event that adds `OfficeHolder { office, holder }`
   - ordering assertion that event A precedes event B
4. The gap is test-harness ergonomics, not missing authoritative data.

## Architecture Check

1. Adding reusable golden-harness event-log helpers is cleaner than duplicating raw event-log scans in every cross-system scenario. It keeps assertions declarative while preserving the append-only event log as the source of truth.
2. This belongs in the golden harness, not in production world/event code, because the need is assertion ergonomics for tests.
3. No backward-compatibility layers are introduced; this is a pure test helper improvement.

## What to Change

### 1. Add typed event-log helper functions

Extend `crates/worldwake-ai/tests/golden_harness/mod.rs` with small reusable helpers such as:

- `first_event_matching(...)`
- `event_sets_component(...)`
- `event_adds_relation(...)`
- `event_removes_relation(...)`
- `assert_event_order(...)`

The API should remain generic enough to support politics, queueing, social propagation, and future emergent tests.

### 2. Add harness-focused tests

Add focused harness tests proving the helpers correctly detect component and relation deltas and preserve append-order semantics.

### 3. Migrate at least one existing golden scenario

Replace one current ad hoc event-log scan in an existing golden test with the new helpers so the abstraction is proven useful immediately.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify to consume the helpers)

## Out of Scope

- Adding new production event-log APIs
- Adding new runtime/system trace sinks
- Rewriting every existing golden test to the new helper surface

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent`
2. `cargo test -p worldwake-ai --test golden_offices`
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Golden tests must be able to assert authoritative mutation ordering without manually open-coding raw event-log scans in each scenario.
2. The helper layer must remain a thin wrapper over typed event-log records and deltas, not a lossy summary cache.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_harness/mod.rs` — add helper-focused tests that prove component/relation delta detection and ordering.
2. `crates/worldwake-ai/tests/golden_emergent.rs` — replace one ad hoc event-log scan with the helper layer to prove practical use.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent`
2. `cargo test -p worldwake-ai --test golden_offices`
3. `cargo test -p worldwake-ai`

