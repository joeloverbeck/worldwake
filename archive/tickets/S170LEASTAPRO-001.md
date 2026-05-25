# S170LEASTAPRO-001: RoutePreference safe-traversal event provenance

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — agent decision runtime (RoutePreference learning)
**Deps**: None

## Problem

Before this ticket, `RoutePreference::record_safe` (`crates/worldwake-core/src/route_preference.rs`) did not store an event id when recording safe traversals, while `record_dangerous` did. Audits could not answer "which event triggered this safe-traversal learning?" symmetrically. FND-22A's accountable-origin requirement was partial: the `RoutePreferenceEntry.last_traversal_event: Option<EventId>` field existed but was populated only on the dangerous branch.

## Assumption Reassessment (2026-05-25)

1. Before implementation, `RoutePreference::record_safe(segment, tick)` at `crates/worldwake-core/src/route_preference.rs` did not accept or store an `EventId`. `record_dangerous` already stored the event in the existing `RoutePreferenceEntry.last_traversal_event: Option<EventId>` field. The field was shared; the asymmetry was only in the write path.
2. Spec under audit: `archive/specs/S170-learned-state-provenance-hardening.md` D2. The only runtime call site was `record_route_preference_updates` at `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs`, which iterates `for _ in before_safe..after_entry.safe_trips`. The function already had `observation.provenance_event: EventId` in scope; the sibling `record_dangerous` loop uses the same id as fallback via `latest_threat_event_for_agent(...).unwrap_or(observation.provenance_event)`. No new event-flow surface was required.
3. The shared boundary under audit was `RoutePreference::record_safe`'s public signature. Adding an `EventId` parameter forced every call site in the workspace to supply one.
4. Existing focused tests exercising `record_safe` in `crates/worldwake-core/src/route_preference.rs`: `record_safe_and_dangerous_update_counts_and_timestamps`, `route_preference_uses_canonical_route_segment_key`, and `route_preference_bincode_round_trip_preserves_entries`. Test sites in worldwake-ai (`agent_tick/planning.rs`, `route_threat.rs`, `planning_snapshot.rs`, `tests/scenarios/route_preferences.rs`, `tests/scenarios/cognitive_archetypes_divergence.rs`, `tests/scenarios/scaled_contention.rs`) constructed `record_safe` calls and were updated to pass a synthesized event id. The drafted `decision_runtime.rs` call-site claim was stale; live reassessment found no `record_safe` call there.
5. No `SAVE_FORMAT_VERSION` bump: `last_traversal_event` already exists in `RoutePreferenceEntry`; the change only widens its population. Existing saves continue to load; new saves carry more `Some(EventId)` values where they previously had `None`.

## Architecture Check

1. Symmetric with the existing `record_dangerous(segment, event, tick)` signature — eliminates parallel-authority drift between safe and dangerous traversal recording. The field that holds the event id is already authoritative; only the write coverage is widening.
2. No backward-compatibility shim. The signature change forces every call site to supply an `EventId`; the compiler catches misses. The runtime fallback pattern (use `observation.provenance_event` when no per-traversal event tag exists) mirrors `record_dangerous`'s existing fallback, so no new precedent is set.

## Verified Layers

1. Symmetric provenance (FND-22A) → focused unit coverage on `RoutePreference::record_safe` asserting `last_traversal_event` is populated.
2. Authentic causal trigger (FND-29A — `provenance_event` is the event whose `RouteExperience` state-delta triggered the observation) → focused runtime coverage on `record_route_preference_updates` asserting the recorded `EventId` matches the triggering record's id.
3. Single-layer ticket — no cross-system invariant requires additional layer mapping beyond the focused-unit and focused-runtime surfaces above.

## Landed Changes

### 1. RoutePreference::record_safe signature

The landed `crates/worldwake-core/src/route_preference.rs` signature is:

```rust
pub fn record_safe(&mut self, segment: RouteSegment, event: EventId, tick: Tick) {
    let entry = self.entry(segment);
    entry.safe_traversals = entry.safe_traversals.saturating_add(1);
    entry.last_safe_tick = Some(tick);
    entry.last_traversal_event = Some(event);
}
```

### 2. Runtime call site

`crates/worldwake-ai/src/agent_tick/learned_state_observation.rs` now passes `observation.provenance_event` as the safe-traversal provenance parameter:

```rust
for _ in before_safe..after_entry.safe_trips {
    runtime
        .route_preference
        .record_safe(segment, observation.provenance_event, observation.tick);
}
```

### 3. Test sites in worldwake-ai

Updated every `record_safe(...)` invocation in:

- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/route_threat.rs`
- `crates/worldwake-ai/src/planning_snapshot.rs`
- `crates/worldwake-ai/tests/scenarios/route_preferences.rs`
- `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs`
- `crates/worldwake-ai/tests/scenarios/scaled_contention.rs`

Each call now passes a synthesized or contextually meaningful `EventId`.

### 4. Update inline tests in route_preference.rs

- `record_safe_and_dangerous_update_counts_and_timestamps`: now passes an `EventId` for the safe write.
- `route_preference_uses_canonical_route_segment_key`: now passes an `EventId` to `record_safe`.
- `route_preference_bincode_round_trip_preserves_entries`: now passes an `EventId`.
- `preference_is_neutral_below_minimum_traversals`: no `record_safe` call; no change.
- `preference_increases_decreases_and_decays_toward_neutral`: no `record_safe` call; no change.

## Landed Files

- `crates/worldwake-core/src/route_preference.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — test site at 4851)
- `crates/worldwake-ai/src/route_threat.rs` (modify — test sites at 567, 621)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — test site at 3261)
- `crates/worldwake-ai/tests/scenarios/route_preferences.rs` (modify)
- `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs` (modify)
- `crates/worldwake-ai/tests/scenarios/scaled_contention.rs` (modify)

## Out of Scope

- `LearnedOpportunitySource` enum or `OpportunityEntry` migration (`archive/tickets/S170LEASTAPRO-002.md`, now archived)
- `DiscrepancySource` enum or `DiscrepancyEntry` migration (ticket 003)
- `BlockerSource` enum or `Blocker` migration (ticket 004)
- `SAVE_FORMAT_VERSION` bump — no schema change in this ticket
- Restructuring the route-preference observation pipeline to emit per-traversal events (per spec Q3 resolution, the current `provenance_event` IS the authentic causal trigger — no restructure needed)

## Acceptance Result

### Tests That Passed Or Were Waived

1. Passed: `record_safe_populates_last_traversal_event` calls `record_safe(segment, EventId(42), Tick(5))` and asserts `entry.last_traversal_event == Some(EventId(42))`.
2. Passed: `record_safe_and_dangerous_update_counts_and_timestamps` now covers both safe and dangerous traversal provenance writes.
3. Passed: `records_safe_route_preference_provenance_from_route_experience_delta` asserts that a `RouteExperience` component-set delta with `safe_trips` increment produces a `RoutePreferenceEntry` whose `last_traversal_event` matches the triggering event id.
4. Passed: `cargo test -p worldwake-core route_preference` and `cargo test -p worldwake-ai`.
5. Waived for this per-ticket iteration: `./scripts/verify.sh`; the `implement-spec-tickets` harness final branch phase owns the full pre-PR verification gate before push after the full S170 family lands.

### Invariants

1. Every `RoutePreferenceEntry` written by `record_safe` after this ticket has `last_traversal_event = Some(_)`.
2. The recorded `EventId` for safe traversals is the same event whose `RouteExperience` state-delta triggered the observation (symmetric with `record_dangerous` semantics).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/route_preference.rs` — add `record_safe_populates_last_traversal_event`; update `record_safe_and_dangerous_update_counts_and_timestamps`, `route_preference_uses_canonical_route_segment_key`, `route_preference_bincode_round_trip_preserves_entries`.
2. `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs` — add a focused test asserting safe-side `last_traversal_event = Some(triggering_event_id)`.

### Commands Run

1. `cargo test -p worldwake-core route_preference`
2. `cargo test -p worldwake-ai`
3. `cargo fmt --all`

## Outcome

Completed on 2026-05-25.

- `RoutePreference::record_safe` now requires an `EventId` and stores it in `RoutePreferenceEntry.last_traversal_event`.
- `record_route_preference_updates` now passes the `RouteExperience` delta event id for safe traversal learning.
- Core and AI test call sites now pass explicit event ids, and route preference golden coverage asserts safe traversal provenance in the existing safe-route scenario.

## Deviations

- Live reassessment found no `record_safe` call in `crates/worldwake-ai/src/decision_runtime.rs`; that drafted file touch was removed from the landed scope.
- `./scripts/verify.sh` was not run for this per-ticket iteration. The full S170 harness finalization phase still owns that pre-PR gate before branch push.

## Verification Result

- Passed `cargo test -p worldwake-core route_preference`
- Passed `cargo test -p worldwake-ai learned_state_observation`
- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-ai`
- Waived `./scripts/verify.sh` for this per-ticket iteration because the `implement-spec-tickets` harness final branch phase owns the full pre-PR verification gate before push.
