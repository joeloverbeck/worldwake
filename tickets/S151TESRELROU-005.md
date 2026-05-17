# S151TESRELROU-005: Decision-history payload embedding + observer Section 3b rendering

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — two new summary types + payload-field extensions on `GoalCommittedPayload` and `GoalSuppressedPayload` + observer rendering
**Deps**: archive/tickets/S151TESRELROU-001.md

## Problem

S151's decision history needs to surface why a goal commit involved a particular witness's testimony or a particular route segment, and why a goal was suppressed due to source unreliability. Per Q3=(a) approved during reassessment, the contexts embed as optional fields on existing payload variants (following the `BeliefSnapshot` precedent at `decision_event_payload.rs:250-254`) rather than as new top-level enum variants.

## Assumption Reassessment (2026-05-17)

1. `DecisionEventPayload` at `crates/worldwake-core/src/decision_event_payload.rs:13` has 16 always-on always-emitted variants (lines 14-30 per Step 2 spot-check). `BeliefSnapshot` at lines 250-254 is the precedent for embedded summary types — `#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]`, embedded under `PlanInvalidatedPayload:298` and `BlockerRecordedPayload:484`.
2. `GoalCommittedPayload` at `decision_event_payload.rs:159-168` already carries `#[serde(default)] pub assumptions: Vec<PlanAssumptionRef>` (line 167) from S136 — the precedent for `#[serde(default)] Vec<_>` payload extensions.
3. `GoalCommittedPayload` construction sites (13 across `decision_event_payload.rs:582,917`, `save_load.rs:1087`, `agent_tick/planning.rs:1190,4023`, `golden_decision_payload.rs:100`, `golden_motive_sources.rs:42,172`, `observer.rs:6026,7817,7922,7949`). `GoalSuppressedPayload` construction: 1 site in `agent_tick/tests.rs:5477`. All need `testimony_trust_context: vec![]` (or equivalent) additions; `#[serde(default)]` means save deserialization stays compatible.
4. Observer Section 3b at `crates/worldwake-cli/src/bin/observer.rs:932` renders decision history as a markdown table at lines 939-940 with header `| Tick | Agent | Event | Payload Summary |`. Rendering hook at line 959 (`decision_payload_summary(payload, Some(world))`). Multi-line detail continuation pattern exists at lines 962-971 for `GoalCommitted` motive sources and `RepairApplied` breach details — new contexts follow the same continuation pattern.
5. Both summary types are core-resident; field types (`EntityId`, `TopicScope` from ticket 001, `Permille`, `Tick`, `RouteSegment` from `blocker_scope.rs:67`) all resolve to `worldwake-core` — no cross-crate-residence issue.

## Architecture Check

1. Per FND-29: decision history surfaces the *why* of commits and suppressions. Embedding contexts on existing payloads (instead of new top-level variants) avoids inflating always-on event volume — `BeliefSnapshot` precedent.
2. `Vec<_>` rather than `Option<_>` because a single goal commit may reference multiple witnesses or multiple traversed segments — vector shape matches the multi-context reality.
3. `#[serde(default)]` on every new field means existing save streams deserialize cleanly into empty vecs — no `SAVE_FORMAT_VERSION` bump needed in this ticket (deferred to ticket 010 alongside the runtime-store and component bumps).
4. Both summary types are `Copy + Eq + Hash + Ord + Serialize + Deserialize` per the `BeliefSnapshot` derive set — satisfies the existing payload struct's derive requirements.

## Verification Layers

1. Payload structural extension → focused unit tests in `decision_event_payload.rs#[cfg(test)]` constructing `GoalCommittedPayload` with non-empty contexts and asserting serde roundtrip preserves both contexts.
2. Save-load compatibility → `#[serde(default)]` round-trip test: serialize a `GoalCommittedPayload` without the new contexts (using the pre-S151 byte format), then deserialize and assert empty vecs.
3. Observer rendering → unit test that calls `decision_payload_summary` on a `GoalCommittedPayload` with both contexts populated, asserts the table cell + continuation rows match the expected markdown layout.

## What to Change

### 1. Add summary types in `crates/worldwake-core/src/decision_event_payload.rs`

Co-locate with `BeliefSnapshot` (around line 250):

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TestimonyTrustSummary {
    pub source: EntityId,
    pub topic: TopicScope,
    pub trust: Permille,
    pub observations: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RoutePreferenceSummary {
    pub segment: RouteSegment,
    pub preference: Permille,
    pub last_safe_tick: Option<Tick>,
    pub last_dangerous_tick: Option<Tick>,
}
```

Import `TopicScope` from ticket 001 and `RouteSegment` from `blocker_scope.rs`.

### 2. Extend `GoalCommittedPayload`

Append two `#[serde(default)] Vec<_>` fields after the existing fields:

```rust
#[serde(default)]
pub testimony_trust_context: Vec<TestimonyTrustSummary>,
#[serde(default)]
pub route_preference_context: Vec<RoutePreferenceSummary>,
```

### 3. Extend `GoalSuppressedPayload`

Append one `#[serde(default)] Vec<TestimonyTrustSummary>` field — used when the `GoalRejectionReason` or sibling omission enum indicates `TestimonySourceUnreliable` (the actual rejection-reason wiring lands in ticket 007).

```rust
#[serde(default)]
pub testimony_trust_context: Vec<TestimonyTrustSummary>,
```

### 4. Update construction sites

Append `testimony_trust_context: vec![], route_preference_context: vec![],` (and the single field for `GoalSuppressedPayload`) at each of the 13 + 1 sites listed in Assumption Reassessment item 3. The contexts stay empty in this ticket — population logic lands in ticket 006 (observation hook) for `GoalCommitted` and ticket 007 (suppression path) for `GoalSuppressed`.

### 5. Observer Section 3b rendering (`crates/worldwake-cli/src/bin/observer.rs`)

Extend the `decision_payload_summary()` function at line 959 (and any helper functions) to render the new contexts as continuation rows when non-empty, following the existing multi-line pattern at lines 962-971 (used today for `GoalCommitted` motive sources):

```
| 142 | merchant_42 | GoalCommitted | BuyCommodity(Grain) at MarketSquare |
|     |             |               | ↳ Trust: witness#17 RouteHazard p=320 obs=4 |
|     |             |               | ↳ Trust: witness#22 ResourceAvailability p=620 obs=7 |
|     |             |               | ↳ Route: (MarketSquare→Ferry) pref=410 last_safe=128 last_danger=- |
```

Use `↳` (or the file's existing continuation prefix — preserve the convention) for visual indent.

## Files to Touch

- `crates/worldwake-core/src/decision_event_payload.rs` (modify — two new summary types, two new fields on `GoalCommittedPayload`, one new field on `GoalSuppressedPayload`, two new construction sites at lines 582 and 917)
- `crates/worldwake-sim/src/save_load.rs` (modify — construction site at line 1087)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — construction sites at lines 1190, 4023)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — `GoalSuppressedPayload` site at line 5477)
- `crates/worldwake-ai/tests/golden_decision_payload.rs` (modify — construction at line 100)
- `crates/worldwake-ai/tests/golden_motive_sources.rs` (modify — construction sites at lines 42, 172)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — construction sites at lines 6026, 7817, 7922, 7949 + `decision_payload_summary` rendering extension)

## Out of Scope

- Populating `testimony_trust_context` on `GoalCommittedPayload` at commit time — ticket 006 (observation hook + planner integration)
- Populating `testimony_trust_context` on `GoalSuppressedPayload` at suppression time — ticket 007 (ranking damping + emission suppression)
- Diagnostics aggregator reading these contexts — ticket 009
- `SAVE_FORMAT_VERSION` bump — ticket 010

## Acceptance Criteria

### Tests That Must Pass

1. `TestimonyTrustSummary` and `RoutePreferenceSummary` satisfy `Copy + Hash + Ord + Serialize + Deserialize` at compile time.
2. `GoalCommittedPayload` with non-empty contexts round-trips through bincode without loss.
3. A pre-S151 byte payload (constructed without the new contexts) deserializes into a `GoalCommittedPayload` with empty `testimony_trust_context` and `route_preference_context` (verifying `#[serde(default)]`).
4. `decision_payload_summary` produces the expected continuation-row format when contexts are non-empty.
5. Existing suite: `cargo test --workspace`.

### Invariants

1. `#[serde(default)]` on every new field — pre-bump save streams deserialize cleanly.
2. Contexts are populated only by their owning tickets (006, 007); this ticket leaves all sites with `vec![]`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/decision_event_payload.rs#[cfg(test)]` — payload round-trip and `#[serde(default)]` backward-compat tests for both extended payloads.
2. `crates/worldwake-cli/src/bin/observer.rs` test (or sibling integration test) — rendering of populated contexts.

### Commands

1. `cargo test -p worldwake-core decision_event_payload`
2. `cargo test -p worldwake-cli observer`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`

Merge note: Ticket 010 bumps SAVE_FORMAT_VERSION 87→88 after this ticket lands; this ticket avoids its own bump via `#[serde(default)]` on all new payload fields.
