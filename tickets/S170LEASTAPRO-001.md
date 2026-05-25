# S170LEASTAPRO-001: RoutePreference safe-traversal event provenance

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — agent decision runtime (RoutePreference learning)
**Deps**: None

## Problem

`RoutePreference::record_safe` (`crates/worldwake-core/src/route_preference.rs:85-89`) does not store an event id when recording safe traversals, while `record_dangerous` (lines 91-96) does. Audits cannot answer "which event triggered this safe-traversal learning?" symmetrically. FND-22A's accountable-origin requirement is partial — the `RoutePreferenceEntry.last_traversal_event: Option<EventId>` field at line 20 exists but is populated only on the dangerous branch.

## Assumption Reassessment (2026-05-25)

1. `RoutePreference::record_safe(segment, tick)` at `crates/worldwake-core/src/route_preference.rs:85-89` does not currently accept or store an `EventId`. `record_dangerous` at lines 91-96 already stores the event in the existing `RoutePreferenceEntry.last_traversal_event: Option<EventId>` field (line 20). The field is shared; the asymmetry is only in the write path.
2. Spec under audit: `specs/S170-learned-state-provenance-hardening.md` D2. The only runtime call site is `record_route_preference_updates` at `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs:215-219`, which iterates `for _ in before_safe..after_entry.safe_trips`. The function already has `observation.provenance_event: EventId` in scope (line 74); the sibling `record_dangerous` loop (lines 220-230) uses the same id as fallback via `latest_threat_event_for_agent(...).unwrap_or(observation.provenance_event)`. No new event-flow surface is required.
3. The shared boundary under audit is `RoutePreference::record_safe`'s public signature — adding an `EventId` parameter to the public method. Every call site in the workspace must supply one.
4. Existing focused tests exercising `record_safe` in `crates/worldwake-core/src/route_preference.rs`: `record_safe_and_dangerous_update_counts_and_timestamps:136`, `route_preference_uses_canonical_route_segment_key:152`, `route_preference_bincode_round_trip_preserves_entries:167`, `preference_is_neutral_below_minimum_traversals:180`, `preference_increases_decreases_and_decays_toward_neutral:196`. Test sites in worldwake-ai (`agent_tick/planning.rs:4851`, `route_threat.rs:567/621`, `planning_snapshot.rs:3261`, `decision_runtime.rs:600`, `tests/scenarios/route_preferences.rs:35/104`, `tests/scenarios/cognitive_archetypes_divergence.rs:234`, `tests/scenarios/scaled_contention.rs:144`) construct `record_safe` calls and must pass a synthesized event id.
5. No `SAVE_FORMAT_VERSION` bump: `last_traversal_event` already exists in `RoutePreferenceEntry`; the change only widens its population. Existing saves continue to load; new saves carry more `Some(EventId)` values where they previously had `None`.

## Architecture Check

1. Symmetric with the existing `record_dangerous(segment, event, tick)` signature — eliminates parallel-authority drift between safe and dangerous traversal recording. The field that holds the event id is already authoritative; only the write coverage is widening.
2. No backward-compatibility shim. The signature change forces every call site to supply an `EventId`; the compiler catches misses. The runtime fallback pattern (use `observation.provenance_event` when no per-traversal event tag exists) mirrors `record_dangerous`'s existing fallback, so no new precedent is set.

## Verification Layers

1. Symmetric provenance (FND-22A) → focused unit coverage on `RoutePreference::record_safe` asserting `last_traversal_event` is populated.
2. Authentic causal trigger (FND-29A — `provenance_event` is the event whose `RouteExperience` state-delta triggered the observation) → focused runtime coverage on `record_route_preference_updates` asserting the recorded `EventId` matches the triggering record's id.
3. Single-layer ticket — no cross-system invariant requires additional layer mapping beyond the focused-unit and focused-runtime surfaces above.

## What to Change

### 1. RoutePreference::record_safe signature

In `crates/worldwake-core/src/route_preference.rs:85-89`, change the signature to:

```rust
pub fn record_safe(&mut self, segment: RouteSegment, event: EventId, tick: Tick) {
    let entry = self.entry(segment);
    entry.safe_traversals = entry.safe_traversals.saturating_add(1);
    entry.last_safe_tick = Some(tick);
    entry.last_traversal_event = Some(event);
}
```

### 2. Runtime call site

In `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs:215-219`, pass `observation.provenance_event` as the new parameter:

```rust
for _ in before_safe..after_entry.safe_trips {
    runtime
        .route_preference
        .record_safe(segment, observation.provenance_event, observation.tick);
}
```

### 3. Test sites in worldwake-ai

Update every `record_safe(...)` invocation in:

- `crates/worldwake-ai/src/agent_tick/planning.rs:4851`
- `crates/worldwake-ai/src/route_threat.rs:567, 621`
- `crates/worldwake-ai/src/planning_snapshot.rs:3261`
- `crates/worldwake-ai/src/decision_runtime.rs:600` (test sub-module context)
- `crates/worldwake-ai/tests/scenarios/route_preferences.rs:35, 104`
- `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs:234`
- `crates/worldwake-ai/tests/scenarios/scaled_contention.rs:144`

Pass a synthesized `EventId` (e.g., `EventId(0)`, or a contextually-meaningful id where the test already names an event).

### 4. Update inline tests in route_preference.rs

- `record_safe_and_dangerous_update_counts_and_timestamps` (line 136): pass an `EventId` (e.g., `EventId(7)`) and assert `entry.last_traversal_event == Some(EventId(7))` for the safe write — currently this test asserts only for the dangerous write.
- `route_preference_uses_canonical_route_segment_key` (line 152): pass an `EventId` to `record_safe`.
- `route_preference_bincode_round_trip_preserves_entries` (line 167): pass an `EventId`.
- `preference_is_neutral_below_minimum_traversals` (line 180): no `record_safe` call (constructs entry directly) — no change.
- `preference_increases_decreases_and_decays_toward_neutral` (line 196): no `record_safe` call — no change.

## Files to Touch

- `crates/worldwake-core/src/route_preference.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — test site at 4851)
- `crates/worldwake-ai/src/route_threat.rs` (modify — test sites at 567, 621)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — test site at 3261)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — test site at 600)
- `crates/worldwake-ai/tests/scenarios/route_preferences.rs` (modify)
- `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs` (modify)
- `crates/worldwake-ai/tests/scenarios/scaled_contention.rs` (modify)

## Out of Scope

- `LearnedOpportunitySource` enum or `OpportunityEntry` migration (ticket 002)
- `DiscrepancySource` enum or `DiscrepancyEntry` migration (ticket 003)
- `BlockerSource` enum or `Blocker` migration (ticket 004)
- `SAVE_FORMAT_VERSION` bump — no schema change in this ticket
- Restructuring the route-preference observation pipeline to emit per-traversal events (per spec Q3 resolution, the current `provenance_event` IS the authentic causal trigger — no restructure needed)

## Acceptance Criteria

### Tests That Must Pass

1. New: `record_safe_populates_last_traversal_event` — call `record_safe(segment, EventId(42), Tick(5))`, assert `entry.last_traversal_event == Some(EventId(42))`.
2. Updated: `record_safe_and_dangerous_update_counts_and_timestamps` — both safe and dangerous traversals now populate `last_traversal_event`.
3. New (runtime): focused test in `learned_state_observation.rs` test module asserting that a `RouteExperience`-component-set state delta with `safe_trips` increment produces a `RoutePreferenceEntry` whose `last_traversal_event` matches the triggering event id.
4. Existing suite: `cargo test -p worldwake-core route_preference` and `cargo test -p worldwake-ai`.

### Invariants

1. Every `RoutePreferenceEntry` written by `record_safe` after this ticket has `last_traversal_event = Some(_)`.
2. The recorded `EventId` for safe traversals is the same event whose `RouteExperience` state-delta triggered the observation (symmetric with `record_dangerous` semantics).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/route_preference.rs` — add `record_safe_populates_last_traversal_event`; update `record_safe_and_dangerous_update_counts_and_timestamps`, `route_preference_uses_canonical_route_segment_key`, `route_preference_bincode_round_trip_preserves_entries`.
2. `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs` — add a focused test asserting safe-side `last_traversal_event = Some(triggering_event_id)`.

### Commands

1. `cargo test -p worldwake-core route_preference`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`
