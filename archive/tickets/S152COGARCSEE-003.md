# S152COGARCSEE-003: PersonalityAssigned event tag and payload field

**Status**: COMPLETED
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
4. (Cumulative arithmetic / blast radius) `rg -n "EventPayload \{" crates/` returned **88 textual hits** before implementation, but several were non-literal matches such as `DecisionEventPayload`, `ContentionEventPayload`, the `EventPayload` type definition, or helper function blocks. The landed implementation updated the **78 actual `EventPayload { ... }` literals** with `personality_assigned_payload: None,`. None used `..Default::default()` spread syntax; `EventPayload` has no `Default` impl. Adding the field therefore broke literal construction until every actual construction site was updated in this ticket. High-density sites: `crates/worldwake-systems/src/perception.rs`, `crates/worldwake-core/src/event_record.rs`, and shared test/helper modules across crates.
5. (Save format) This ticket bumps `SAVE_FORMAT_VERSION 94 → 95` (adding a field to the bincode-serialized `EventPayload` breaks the format). Prefer adding the field with `#[serde(default)]` where serde-JSON paths exist, but note bincode save/load still requires the version bump regardless.
6. (Mismatch + correction) The original spec proposed four `blake3::Hash` snapshot fields; reassessment reduced this to a single `resolved_profile_hash: StateHash` carried on `PersonalityAssignedPayload` (defined in ticket 001). This ticket does not compute the hash — it only carries the payload.

## Architecture Check

1. A dedicated optional payload field matches the three existing payload precedents and keeps `PersonalityAssigned` semantically distinct from `DecisionEventPayload` (a spawn-time assignment is not a decision). The constructor churn is the same mechanical cost those precedents paid; it is purely additive (`None` at every existing `EventPayload` literal).
2. No backwards-compatibility shim: the save bump replaces the prior format; the new field is the single authoritative carrier of the assignment record (FND-28).

## Verified Layers

1. New variant is exhaustively handled / no match breaks -> passed `cargo build --workspace`.
2. `EventPayload` carries and round-trips the payload -> passed `cargo test -p worldwake-core event_payload`.
3. `EventView::personality_assigned_payload` returns the payload when set -> passed focused unit coverage for both `PendingEvent` and `EventRecord`; existing `None` payload literals compile and round-trip.
4. Emission/consumption of the payload at runtime is ticket 005's contract; this ticket proves only the carrier and accessor, so no decision/action-trace layer applies here.

## Landed Changes

### 1. Added the `EventTag` variant

Added `PersonalityAssigned` to `EventTag` (`event_tag.rs`) with a comment that ticket 005 owns the emission site. Updated the variant-count test inventory to 47 variants.

### 2. Added the `EventPayload` field

Added `pub personality_assigned_payload: Option<PersonalityAssignedPayload>` to `EventPayload` (`event_record.rs`) with `#[serde(default)]` for serde-backed paths. Updated all 78 actual `EventPayload` literal construction sites to add `personality_assigned_payload: None,`.

### 3. Added `EventView` accessor

Added `fn personality_assigned_payload(&self) -> Option<&PersonalityAssignedPayload>;` to the trait declaration and both `PendingEvent`/`EventRecord` impls.

### 4. Bumped save format

Bumped `SAVE_FORMAT_VERSION 94 → 95` (`crates/worldwake-sim/src/save_load.rs`) and added an event-log save/load round-trip test for an event carrying `PersonalityAssignedPayload`.

## Landed Files

- `crates/worldwake-core/src/event_tag.rs` (modify — variant + count test)
- `crates/worldwake-core/src/event_record.rs` (modify — field + accessor + impls + tests)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump + round-trip test)
- Actual `EventPayload { ... }` construction sites across `crates/worldwake-core`, `crates/worldwake-sim`, `crates/worldwake-systems`, `crates/worldwake-ai`, and `crates/worldwake-cli` (mechanical `personality_assigned_payload: None` additions).

## Out of Scope

- Emitting `PersonalityAssigned` at spawn (ticket 005 owns the only emission site; this ticket forward-declares the variant).
- Computing `resolved_profile_hash` (ticket 005).
- Observer rendering of the event (ticket 006).

## Acceptance Result

### Tests Passed

1. Workspace compiles with the new variant and field (`cargo build --workspace` passed).
2. An `EventPayload` with `personality_assigned_payload: Some(..)` round-trips through bincode unchanged.
3. `EventView::personality_assigned_payload` returns `Some` when set for both `PendingEvent` and `EventRecord`; existing `None` payload sites compile and round-trip.
4. Existing suite passed through `cargo test --workspace` and the full `./scripts/verify.sh` wrapper.

### Invariants

1. The new variant carries its payload only through the dedicated `EventPayload` field — no duplicate transport path (FND-28, single canonical path).
2. `SAVE_FORMAT_VERSION` strictly increases; the field is the only authoritative carrier of the assignment in the log (FND-29A append-only).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/event_record.rs` (`#[cfg(test)]`) — payload round-trip + accessor on both impls (mirror `event_payload_roundtrips_with_decision_payload` at `:895`).
2. `crates/worldwake-sim/src/save_load.rs` (`#[cfg(test)]`) — event-log round-trip carrying the payload.

### Commands Run

1. Passed `cargo test -p worldwake-core event_payload`
2. Passed `cargo test -p worldwake-sim save_to_bytes_roundtrip_preserves_personality_assigned_event_payloads`
3. Passed `cargo build --workspace`
4. Passed `cargo test --workspace`
5. Passed `./scripts/verify.sh`

Merge note: Ticket 003 bumps SAVE_FORMAT_VERSION 94→95; it must land after ticket 002 (which bumps 93→94). See the spec decomposition's Merge-Order Constraints.

## Outcome

Completed on 2026-05-20.

- Added the `EventTag::PersonalityAssigned` carrier tag for the S152 spawn-time assignment path.
- Added `EventPayload::personality_assigned_payload: Option<PersonalityAssignedPayload>` plus `EventView::personality_assigned_payload` on both pending and committed event views.
- Updated all actual `EventPayload` literal construction sites with `personality_assigned_payload: None`.
- Bumped `SAVE_FORMAT_VERSION` from `94` to `95` and proved save/load preserves an event-log `PersonalityAssignedPayload`.

## Deviations

- The reassessment's 88-site count came from textual `EventPayload {` matches. Implementation found and updated 78 actual `EventPayload` literals; the remaining textual hits were non-literal/type-name matches and were not legitimate construction sites.
- The first mechanical constructor pass was intentionally compiler-checked; it briefly overmatched non-`EventPayload` names such as `ContentionEventPayload` during implementation, then was corrected before any successful verification run.

## Verification Result

- Passed `cargo test -p worldwake-core event_payload`
- Passed `cargo test -p worldwake-sim save_to_bytes_roundtrip_preserves_personality_assigned_event_payloads`
- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `./scripts/verify.sh`
