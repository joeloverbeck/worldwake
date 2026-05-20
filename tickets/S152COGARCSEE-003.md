# S152COGARCSEE-003: PersonalityAssigned event tag and payload field

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new `EventTag` variant, new `EventPayload` field, save format bump
**Deps**: archive/tickets/S152COGARCSEE-001.md, archive/tickets/S152COGARCSEE-002.md

## Problem

Archetype assignment must be replayable and inspectable through the append-only event log (FND-22A, FND-29A). S152 adds an `EventTag::PersonalityAssigned` variant and carries `PersonalityAssignedPayload` as a new optional field on the shared `EventPayload`, following the established optional-payload convention (`ContentionEventPayload`, `DecisionEventPayload`, `ArtifactTransitionPayload`). The emission site itself is ticket 005; this ticket lands the tag, the field, and the accessor so the format is ready.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `EventTag` is a `worldwake-core` enum at `crates/worldwake-core/src/event_tag.rs` (46 variants; the spec's original `event_log.rs` location was corrected during reassessment — `crates/worldwake-sim/src/event_log.rs` does not exist). It derives `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`; adding a unit variant is derive-safe. No `PersonalityAssigned` variant exists today.
2. `EventPayload` (`crates/worldwake-core/src/event_record.rs:48`) carries optional payloads as `Option<…>` fields (e.g. `contention_event_payload`, `decision_payload`, `artifact_transition_payload`). The `EventView` trait declares accessors at `event_record.rs:24-26` with two impl sites (`:121`, `:183`). The new field needs a trait-decl accessor plus both impls.
3. Mixed-layer boundary under audit: the serialized `EventPayload` struct shape and the `EventView` accessor contract. `PersonalityAssignedPayload` is defined in ticket 001.
4. (Cumulative arithmetic / blast radius) `EventPayload` has **88 literal `EventPayload { … }` construction sites** workspace-wide (`rg -c "EventPayload \{" crates/`), and **none use `..Default::default()` spread syntax**; `EventPayload` has no `Default` impl. Adding a field therefore breaks every one of the 88 sites — each must add `personality_assigned_payload: None,`. Intermediate states do not compile, so the field addition and all 88 site updates land in this single ticket. High-density sites: `crates/worldwake-systems/src/perception.rs` (21), `crates/worldwake-core/src/event_record.rs` (15, mostly tests).
5. (Save format) This ticket bumps `SAVE_FORMAT_VERSION 94 → 95` (adding a field to the bincode-serialized `EventPayload` breaks the format). Prefer adding the field with `#[serde(default)]` where serde-JSON paths exist, but note bincode save/load still requires the version bump regardless.
6. (Mismatch + correction) The original spec proposed four `blake3::Hash` snapshot fields; reassessment reduced this to a single `resolved_profile_hash: StateHash` carried on `PersonalityAssignedPayload` (defined in ticket 001). This ticket does not compute the hash — it only carries the payload.

## Architecture Check

1. A dedicated optional payload field matches the three existing payload precedents and keeps `PersonalityAssigned` semantically distinct from `DecisionEventPayload` (a spawn-time assignment is not a decision). The 88-site churn is the same mechanical cost those precedents paid; it is purely additive (`None` at every existing site).
2. No backwards-compatibility shim: the save bump replaces the prior format; the new field is the single authoritative carrier of the assignment record (FND-28).

## Verification Layers

1. New variant is exhaustively handled / no match breaks -> `cargo build --workspace` after the variant lands (compile-surface proof).
2. `EventPayload` carries and round-trips the payload -> event-log serialization round-trip test (event-log delta surface).
3. `EventView::personality_assigned_payload` returns the payload when set, `None` otherwise -> focused unit test on both impl sites.
4. Emission/consumption of the payload at runtime is ticket 005's contract; this ticket proves only the carrier and accessor, so no decision/action-trace layer applies here.

## What to Change

### 1. Add the `EventTag` variant

Add `PersonalityAssigned` to `EventTag` (`event_tag.rs`). Document at definition that the emission-site owner is ticket 005 (forward-declared variant). Update the variant-count test constant.

### 2. Add the `EventPayload` field

Add `pub personality_assigned_payload: Option<PersonalityAssignedPayload>` to `EventPayload` (`event_record.rs`). Update all 88 literal construction sites to add `personality_assigned_payload: None,`.

### 3. `EventView` accessor

Add `fn personality_assigned_payload(&self) -> Option<&PersonalityAssignedPayload>;` to the trait decl and both impls (`event_record.rs:24-26`, `:121`, `:183`).

### 4. Save format bump

Bump `SAVE_FORMAT_VERSION 94 → 95` (`crates/worldwake-sim/src/save_load.rs`) and add an event-log round-trip test for an event carrying the payload.

## Files to Touch

- `crates/worldwake-core/src/event_tag.rs` (modify — variant + count test)
- `crates/worldwake-core/src/event_record.rs` (modify — field + accessor + impls + tests)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump + round-trip test)
- All 88 `EventPayload { … }` construction sites — enumerate with `rg -n "EventPayload \{" crates/` during implementation; known clusters: `crates/worldwake-systems/src/perception.rs`, `crates/worldwake-core/src/{event_record.rs,event_log.rs,contention_event.rs,canonical.rs,verification.rs,world_txn.rs,decision_event_payload.rs}`, `crates/worldwake-sim/src/{simulation_state.rs,interrupt_abort.rs,compaction.rs,tick_action.rs,tick_step.rs}`, `crates/worldwake-ai/src/{agent_tick/*,scenario_diagnostics/aggregator.rs}`, `crates/worldwake-systems/src/travel_actions.rs`, `crates/worldwake-systems/tests/e15_information_integration.rs`

## Out of Scope

- Emitting `PersonalityAssigned` at spawn (ticket 005 owns the only emission site; this ticket forward-declares the variant).
- Computing `resolved_profile_hash` (ticket 005).
- Observer rendering of the event (ticket 006).

## Acceptance Criteria

### Tests That Must Pass

1. Workspace compiles with the new variant and field (all 88 sites updated).
2. An `EventPayload` with `personality_assigned_payload: Some(..)` round-trips through bincode unchanged.
3. `EventView::personality_assigned_payload` returns `Some` when set and `None` for events that don't set it.
4. Existing suite: `cargo test --workspace`

### Invariants

1. The new variant carries its payload only through the dedicated `EventPayload` field — no duplicate transport path (FND-28, single canonical path).
2. `SAVE_FORMAT_VERSION` strictly increases; the field is the only authoritative carrier of the assignment in the log (FND-29A append-only).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_record.rs` (`#[cfg(test)]`) — payload round-trip + accessor on both impls (mirror `event_payload_roundtrips_with_decision_payload` at `:895`).
2. `crates/worldwake-sim/src/save_load.rs` (`#[cfg(test)]`) — event-log round-trip carrying the payload.

### Commands

1. `cargo test -p worldwake-core event_payload`
2. `cargo build --workspace` (confirms all 88 sites updated)
3. `./scripts/verify.sh`

Merge note: Ticket 003 bumps SAVE_FORMAT_VERSION 94→95; it must land after ticket 002 (which bumps 93→94). See the spec decomposition's Merge-Order Constraints.
