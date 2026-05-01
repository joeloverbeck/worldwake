# S130SURRECFRO-002: Core types — HypothesisKind, SurveyMemory, SurveyRecorded event

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `HypothesisKind` enum, `SurveyRecord`/`SurveyMemory` module, `EventTag::SurveyRecorded`, `SurveyRecordedPayload`
**Deps**: `archive/tickets/S130SURRECFRO-001.md`, spec `specs/S130-survey-records-frontier-disconfirmation.md` D1, D3, D5

## Problem

S130 introduces three tightly co-resident core types: `HypothesisKind` (an enum naming what an exploring agent expected to find), `SurveyMemory` (a per-agent ECS component holding `SurveyRecord` entries), and `EventTag::SurveyRecorded` with payload struct. Downstream tickets — D2 ExploreLocation extension, D4 component registration, D6 perception writes, D7 ranking damping — all consume these types, so they need to land together as the foundation pass.

## Assumption Reassessment (2026-05-02)

1. `GoalKind` lives at `crates/worldwake-core/src/goal.rs:11` with derives `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`; new `HypothesisKind` enum must satisfy these (specifically `Copy`).
2. `CommodityKind` at `crates/worldwake-core/src/items.rs:10-21` derives `Copy + Eq + Ord + Hash`; embedding it in `HypothesisKind::MayContainCommodity { commodity }` satisfies `GoalKind`'s `Copy` derive.
3. `EventTag` at `crates/worldwake-core/src/event_tag.rs:7-51` has 43 existing variants and is consumed via `txn.add_tag(...)` and tag-membership tests — no exhaustive matches in `worldwake-ai`/`worldwake-systems`/`worldwake-cli`. Adding `SurveyRecorded` (44th variant) has no downstream blast radius beyond the new emission site (added in ticket 007).
4. `decision_event_payload` module at `crates/worldwake-core/src/decision_event_payload.rs` is the existing home for analog `*Payload` structs (`GoalCommittedPayload`, `SleepEpisodeStartedPayload`).
5. Component-residence constraint: `with_component_schema_entries!` macro at `crates/worldwake-core/src/component_schema.rs:3` references types via `crate::TypeName`, so `SurveyMemory` must live in `worldwake-core` for ticket 004's registration to compile.
6. `SurveyMemory::enforce_limits` body reads `profile.survey_memory_retention_ticks` (added in ticket 001) — 001 must land first.
7. New unit tests added in this ticket only — no existing focused/unit, runtime, or golden coverage exercises `SurveyMemory` or `HypothesisKind` (both net-new).

## Architecture Check

1. Co-locating the three additions in one ticket reduces review hops — all are pure additive types in core with no cross-crate consumer until subsequent tickets, and the ticket boundary is the "foundation types" boundary in the dependency chain.
2. `HypothesisKind` is a value type embedded in `GoalKind` (FND-3 — concrete state, not abstract score). `SurveyMemory` is a per-agent ECS component (FND-22A — concrete learned state with explicit acquisition, decay, replacement). `SurveyRecorded` is an authoritative causal event tag (FND-29A — append-only history).
3. No backward-compatibility shims — net-new types with no prior surface to alias.
4. `Vec<SurveyRecord>` storage matches existing project convention for bounded learned-state collections (`WoundList.wounds: Vec<Wound>`, `DemandMemory.observations: Vec<DemandObservation>`); iteration order is insertion-order; determinism preserved via `find()`'s `max_by_key`.

## Verification Layers

1. `SurveyMemory::record` replace-vs-append semantics → focused unit test: same-(place, hypothesis) replaces; distinct (place, hypothesis) appends.
2. `SurveyMemory::record` capacity-eviction semantics → focused unit test: oldest-tick entry evicted on overflow.
3. `SurveyMemory::find` returns freshest matching record → focused unit test with multiple entries for same (place, hypothesis) at different ticks.
4. `SurveyMemory::enforce_limits` retention pruning → focused unit tests: entries older than `survey_memory_retention_ticks` dropped; entries within retention preserved.
5. Single-layer ticket — no SystemFn, no decision/action trace, no event-log emission yet (those land in 006, 007, 008). Pure type and method validation.

## What to Change

### 1. `HypothesisKind` enum

In `crates/worldwake-core/src/goal.rs`, add:

```rust
/// What an exploring agent expects to find at the target place.
/// Drives both ranking input and arrival-time hypothesis evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum HypothesisKind {
    MayContainCommodity { commodity: CommodityKind },
    MayContainLatrine,
    MayContainWashBasin,
    MayContainSleepSite,
    Proactive,
}
```

### 2. `SurveyRecord` and `SurveyMemory` module

Create `crates/worldwake-core/src/survey_memory.rs` per spec D3:

- `SurveyRecord { place, hypothesis, found, confidence, recorded_tick }` (Copy + Serialize + Deserialize).
- `SurveyMemory { entries: Vec<SurveyRecord> }` with `Default`, `impl Component for SurveyMemory`.
- Methods: `find(&self, place, hypothesis) -> Option<&SurveyRecord>` (uses `max_by_key`), `record(&mut self, record, capacity)` (replace-same-key, append-distinct, evict-oldest on overflow), `enforce_limits(&mut self, current_tick: Tick, profile: &CognitiveProfile)` (retains entries within `profile.survey_memory_retention_ticks`).

Add `pub mod survey_memory;` and `pub use survey_memory::{SurveyRecord, SurveyMemory};` to `crates/worldwake-core/src/lib.rs`.

### 3. `EventTag::SurveyRecorded` and `SurveyRecordedPayload`

Add the `SurveyRecorded` variant to the `EventTag` enum at `crates/worldwake-core/src/event_tag.rs` (now 44 variants).

Add `SurveyRecordedPayload` to `crates/worldwake-core/src/decision_event_payload.rs`:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SurveyRecordedPayload {
    pub surveyor: EntityId,
    pub place: EntityId,
    pub hypothesis: HypothesisKind,
    pub found: bool,
    pub confidence: Permille,
}
```

Re-export `SurveyRecordedPayload` from `lib.rs` if other `*Payload` types follow that pattern.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — add `HypothesisKind`)
- `crates/worldwake-core/src/survey_memory.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — `pub mod` + re-exports)
- `crates/worldwake-core/src/event_tag.rs` (modify — add `SurveyRecorded` variant)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify — add `SurveyRecordedPayload`)

## Out of Scope

- `GoalKind::ExploreLocation` field addition and the ~65 destructure/construction site updates (ticket 003)
- `SurveyMemory` ECS registration in `component_schema.rs` and `create_agent`/`spawn_agent` insertion (ticket 004)
- `GoalBeliefView::survey_memory()` accessor (ticket 004)
- `SAVE_FORMAT_VERSION` bump (ticket 004 — bumped alongside the registration that introduces the new save-bound component)
- Calling `SurveyMemory::record` from perception or `SurveyMemory::enforce_limits` from a SystemFn (tickets 007, 008)
- Reading `SurveyMemory` in ranking damping (ticket 006)

## Acceptance Criteria

### Tests That Must Pass

1. New: `survey_memory_record_replaces_same_place_hypothesis_entry`.
2. New: `survey_memory_record_appends_distinct_place_or_hypothesis_entries`.
3. New: `survey_memory_record_evicts_oldest_on_capacity_overflow`.
4. New: `survey_memory_find_returns_freshest_matching_record`.
5. New: `survey_memory_enforce_limits_drops_entries_older_than_retention`.
6. New: `survey_memory_enforce_limits_keeps_entries_within_retention`.
7. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. `HypothesisKind` derives `Copy + Hash + Eq + Ord` so `GoalKind`'s existing trait bounds are preserved when ticket 003 embeds it as a field.
2. `SurveyMemory.entries` traversal is insertion-order; `find()` returns deterministic freshest record under equal `recorded_tick` (uses `max_by_key`, which is stable for tied keys with insertion-order traversal).
3. `EventTag::SurveyRecorded` is consumed only via `txn.add_tag(...)` and tag-membership tests — no exhaustive match site needs updating.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/survey_memory.rs` (`#[cfg(test)]` block) — 6 new focused unit tests covering `record`, `find`, `enforce_limits` (per Acceptance Criteria 1–6).

### Commands

1. `cargo test -p worldwake-core survey_memory`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
