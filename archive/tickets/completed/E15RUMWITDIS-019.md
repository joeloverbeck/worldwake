# E15RUMWITDIS-019: Embed Shared EventPayload In PendingEvent And EventRecord

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` event record types, event-log consumers across workspace, and event-focused tests
**Deps**: `archive/tickets/completed/E15RUMWITDIS-018.md`, `archive/specs/E15-rumor-witness-discovery.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/FOUNDATIONS.md`

## Problem

`E15RUMWITDIS-018` removed positional-constructor brittleness by introducing `EventPayload`, but the event model still duplicates the same schema in three places:

1. `EventPayload`
2. `PendingEvent`
3. `EventRecord` (plus `event_id`)

That duplication is no longer a call-site API smell, but it is still a type-level architecture smell. Every payload field still has to be declared, copied, serialized, and migrated in multiple structs. That creates avoidable drift risk for future event-schema changes and weakens the “one canonical payload shape” invariant that `E15RUMWITDIS-018` was trying to establish.

## Assumption Reassessment (2026-03-14)

1. `crates/worldwake-core/src/event_record.rs` now defines `EventPayload`, `PendingEvent`, and `EventRecord`, but `PendingEvent` and `EventRecord` still duplicate all payload fields instead of owning a shared payload value.
2. `PendingEvent::from_payload(...)` and `EventRecord::from_payload(...)` already make `EventPayload` the canonical assembly surface. The remaining gap is storage shape and read API, not construction semantics.
3. Event consumers across core, sim, systems, and CLI still read duplicated fields directly (`record.tick`, `record.tags`, `record.evidence`, etc.). The cleanup must migrate those call sites to the new shared ownership surface. Current evidence does not show an AI-crate dependency on `EventRecord` field reads for this ticket.
4. `verification.rs` is affected because it iterates event-record deltas and cause refs directly, but it is a consumer adaptation task, not a separate event-model redesign.
5. The codebase does not need `Deref`, mirrored proxy fields, or compatibility aliases to bridge this change. Those would preserve the old parallel ownership model in a softer form. If the payload becomes canonical storage, consumers should read it explicitly.
6. The originally proposed command `cargo test -p worldwake-systems e15_information_integration` is not a valid way to verify the integration test file; it filters test names and matches zero tests. The correct targeted command is `cargo test -p worldwake-systems --test e15_information_integration`.

## Architecture Check

1. The cleaner long-term model is:
   - `PendingEvent { payload: EventPayload }`
   - `EventRecord { event_id: EventId, payload: EventPayload }`
   This makes `EventPayload` the single canonical schema, not just a constructor helper.
2. This is more robust than today’s design because schema evolution happens in one payload type rather than three mirrored structs and two conversion paths.
3. This is more extensible than adding convenience wrappers or `Deref` because explicit payload ownership makes field provenance obvious and keeps the model honest. Event identity (`event_id`) stays distinct from event payload, which is the correct separation.
4. No backwards-compatibility aliasing or shims: do not keep duplicated top-level payload fields on `PendingEvent`/`EventRecord`, and do not add `Deref<Target = EventPayload>` just to preserve old call sites invisibly.

## What to Change

### 1. Make EventPayload the canonical stored event schema

Refactor `crates/worldwake-core/src/event_record.rs` so that:

1. `PendingEvent` stores an `EventPayload` directly
2. `EventRecord` stores `event_id` plus an `EventPayload`
3. canonicalization remains owned by `PendingEvent::from_payload(...)`
4. `into_record(...)` moves the same payload into `EventRecord` instead of copying mirrored fields field-by-field

### 2. Migrate event consumers to explicit payload reads

Update event consumers and tests to read through the shared payload shape instead of duplicated top-level fields.

Requirements:

1. prefer explicit reads such as `record.payload.tick`, `record.payload.tags`, and `pending.payload.evidence`
2. if small helper methods are justified, they must be read-only helpers over the payload and must not recreate a second authoritative field surface
3. do not introduce `Deref`, mirrored getter spam for every field, or top-level alias fields on the wrapper structs

### 3. Keep serialization and event-log semantics stable

This ticket is an internal ownership refactor, not an event-schema redesign.

Requirements:

1. event meaning must remain unchanged for `EventLog`, replay, save/load, verification, CLI inspection, and perception
2. exact Rust/bincode field layout compatibility is not a goal here; if serialized bytes change because the ownership shape becomes cleaner, update the affected tests and let the new layout become canonical
3. `event_id` must remain the only field that lives outside the shared payload on `EventRecord`

## Files to Touch

- `crates/worldwake-core/src/event_record.rs` (modify)
- `crates/worldwake-core/src/event_log.rs` (modify)
- `crates/worldwake-core/src/verification.rs` (modify)
- `crates/worldwake-core/src/world_txn.rs` (modify if helper usage changes)
- `crates/worldwake-sim/src/` event consumers/tests that read event fields directly (modify as needed)
- `crates/worldwake-systems/src/` event consumers/tests that read event fields directly (modify as needed)
- `crates/worldwake-cli/src/handlers/events.rs` (modify)
- `crates/worldwake-ai/` is currently expected to be unaffected unless a direct event-record field read is discovered during implementation

## Out of Scope

- Changing event payload meaning or adding/removing payload fields
- Reworking event-log indexing policy
- Introducing builder patterns or compatibility adapters
- Broad non-event cleanup unrelated to payload ownership

## Acceptance Criteria

### Tests That Must Pass

1. `PendingEvent` and `EventRecord` store one canonical `EventPayload` shape rather than duplicating payload fields.
2. Canonical target/evidence ordering and deduplication remain unchanged after the storage refactor.
3. Event consumers across core, sim, systems, AI, and CLI still observe the same event semantics after migrating to explicit payload reads.
4. Existing suite: `cargo test -p worldwake-core event_record`
5. Existing suite: `cargo test -p worldwake-core event_log`
6. Existing suite: `cargo test -p worldwake-core verification`
7. Existing suite: `cargo test -p worldwake-systems perception`
8. Existing suite: `cargo test -p worldwake-systems --test e15_information_integration`
9. `cargo clippy --workspace --all-targets -- -D warnings`
10. `cargo test --workspace`

### Invariants

1. There is exactly one authoritative event payload schema type in core: `EventPayload`.
2. `PendingEvent` and `EventRecord` are wrappers around that schema, not parallel copies of it.
3. Event identity remains distinct from event payload.
4. No compatibility alias path preserves duplicated top-level payload ownership.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_record.rs` — update tests so they prove canonicalization and round-tripping through wrapper-owned `EventPayload` rather than duplicated fields.
2. `crates/worldwake-core/src/event_log.rs` — update event-log tests to validate stored-record behavior through the embedded payload shape.
3. `crates/worldwake-core/src/verification.rs` — update completeness/verification helpers that construct or inspect records so the payload wrapper contract is covered.
4. `crates/worldwake-systems/src/perception.rs` — update focused tests that inspect discovery/event observation records so they read through the shared payload shape.
5. `crates/worldwake-systems/tests/e15_information_integration.rs` and any affected CLI/sim tests — keep end-to-end event behavior locked while the ownership model changes.

### Commands

1. `cargo test -p worldwake-core event_record`
2. `cargo test -p worldwake-core event_log`
3. `cargo test -p worldwake-core verification`
4. `cargo test -p worldwake-systems perception`
5. `cargo test -p worldwake-systems --test e15_information_integration`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo test --workspace`

## Outcome

Actually changed:

1. `PendingEvent` now stores `payload: EventPayload` and `EventRecord` now stores `{ event_id, payload }`, making `EventPayload` the single authoritative event schema.
2. Event-log, verification, world-txn, CLI, sim, systems, and one AI golden test were migrated to explicit `record.payload.*` and `pending.payload.*` reads with no alias fields, `Deref`, or compatibility shims.
3. Existing focused tests were updated to assert the embedded-payload contract, and the full workspace now passes `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.

Changed from original plan:

1. The ticket originally assumed AI would likely be unaffected; workspace verification showed one AI golden test also read event targets directly, so it was updated as part of the same migration.
2. The ticket originally described serialization stability too loosely. The implementation kept event semantics stable while allowing the Rust/bincode field layout to change to match the cleaner ownership model.
