# E15RUMWITDIS-020: Encapsulate Event Wrapper Storage Behind A Read-Only Event Surface

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` event wrapper API, event consumers across workspace, and event-focused tests
**Deps**: `archive/tickets/completed/E15RUMWITDIS-019.md`, `archive/specs/E15-rumor-witness-discovery.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/FOUNDATIONS.md`

## Problem

`E15RUMWITDIS-019` correctly made `EventPayload` the single authoritative event schema, but event consumers now reach through wrapper internals with `record.payload.*` and `pending.payload.*`.

That is cleaner than duplicated top-level fields, but it still couples the rest of the workspace to the wrapper storage shape instead of to event semantics. The current API boundary says “an event wrapper stores a payload field” rather than “an event exposes tick, cause, witnesses, deltas, and evidence.”

That representation leak has three costs:

1. Consumers now know both the wrapper type and its internal layout.
2. Any later storage refinement would require another workspace-wide migration.
3. The event model still lacks a canonical read surface, so wrappers are acting like transparent bags of fields rather than explicit event abstractions.

## Assumption Reassessment (2026-03-15)

1. `crates/worldwake-core/src/event_record.rs` now defines `PendingEvent { payload: EventPayload }` and `EventRecord { event_id, payload }`, so payload ownership has already been unified.
2. The representation leak is real, but the runtime read surface is narrower than a first pass suggests. Direct wrapper-internal reads in non-test code are currently concentrated in:
   - `crates/worldwake-core/src/event_log.rs`
   - `crates/worldwake-core/src/verification.rs`
   - `crates/worldwake-systems/src/perception.rs`
   - `crates/worldwake-cli/src/handlers/events.rs`
3. Many additional `.payload` call sites across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and `worldwake-cli` are test assertions or payload-construction paths rather than production read-path consumers. The implementation should update tests that become invalid, but it should not broaden scope into unrelated payload assembly cleanup.
4. The codebase still constructs events directly from `EventPayload` values, and that remains appropriate. The architectural gap is on the wrapper read boundary, not the payload assembly boundary.
5. This follow-up should not restore duplicated wrapper fields, `Deref`, or compatibility aliases. The goal is an explicit read-only event API, not a softer version of the old mirrored-field model.
6. `EventPayload` itself can remain the authoritative schema for construction and serialization while wrapper internals become non-public to the rest of the workspace.

## Architecture Check

1. The cleaner long-term model is to expose event semantics through a shared read-only API implemented by `PendingEvent` and `EventRecord`, while keeping wrapper storage private. That preserves one authoritative payload and gives the rest of the engine a stable semantic contract.
2. This is more robust than continuing with `record.payload.*` because callers stop depending on the wrapper’s exact internal representation.
3. This is more extensible than re-exposing top-level fields because borrowed read access does not recreate parallel ownership. It gives the engine a stable event interface without regressing to duplicated schema state.
4. A trait-based read surface is preferable to duplicating ad hoc inherent methods on both wrappers because it keeps consumer vocabulary aligned and reduces future drift.
5. No backwards-compatibility shims or alias fields: do not re-add mirrored fields, do not add `Deref<Target = EventPayload>`, and do not keep both `payload` field access and a semantic API as long-term public surfaces.

## What to Change

### 1. Introduce a canonical read-only event surface

Refactor `crates/worldwake-core/src/event_record.rs` so that event wrappers expose semantic reads without exposing storage internals.

Requirements:

1. Add a shared read-only event interface for `PendingEvent` and `EventRecord`.
2. The interface must cover the event data consumers actually need: tick, cause, actor, targets, evidence, place, deltas, observed entities, visibility, witness data, and tags.
3. `EventRecord` must continue to expose `event_id` distinctly from payload data.
4. Prefer a shared trait or similarly centralized abstraction over hand-copying the same methods independently across wrappers.
5. Keep the surface borrowed and semantic. Returning `&[EntityId]`, `&[StateDelta]`, `&BTreeSet<EventTag>`, and similar references is preferred over cloning or exposing the backing payload wholesale.

### 2. Make wrapper storage non-public

After the read surface exists:

1. make wrapper internals non-public outside the defining module
2. migrate workspace consumers off `record.payload.*` and `pending.payload.*`
3. keep `EventPayload` as the authoritative payload schema for event assembly and serialization

### 3. Migrate consumers to semantic event reads

Update event consumers and tests to use the new read-only event interface.

Requirements:

1. event-log indexing, verification, CLI inspection, and perception must read events through the semantic API
2. tests that exercise wrapper reads should assert behavior through that semantic surface rather than through wrapper internals
3. construction/serialization code that legitimately owns or moves `EventPayload` may continue to do so
4. if helper methods are added, they must be read-only borrows and must not introduce a second owned schema surface

## Files to Touch

- `crates/worldwake-core/src/event_record.rs` (modify)
- `crates/worldwake-core/src/event_log.rs` (modify)
- `crates/worldwake-core/src/verification.rs` (modify)
- `crates/worldwake-core/src/world_txn.rs` (modify only if tests need adjustment)
- `crates/worldwake-sim/src/` tests that currently read `record.payload.*` directly (modify only if visibility changes invalidate them)
- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-systems/src/` tests that currently read `record.payload.*` directly (modify only if visibility changes invalidate them)
- `crates/worldwake-cli/src/handlers/events.rs` (modify)
- `crates/worldwake-cli/tests/integration.rs` or `crates/worldwake-ai/` tests/consumers (modify only if visibility changes invalidate them)

## Out of Scope

- Changing event payload meaning or field set
- Reworking event-log indexing policy
- Replacing `EventPayload` as the authoritative construction schema
- Adding builder patterns, compatibility aliases, or duplicate wrapper-owned fields
- Broad cleanup unrelated to the event read boundary

## Acceptance Criteria

### Tests That Must Pass

1. Runtime event consumers no longer depend on `PendingEvent`/`EventRecord` wrapper internals for reads.
2. `PendingEvent` and `EventRecord` expose one shared semantic read surface over one authoritative `EventPayload`.
3. Wrapper internals are no longer public outside the core event module.
4. Test-only call sites are updated only where wrapper encapsulation requires it.
5. Existing suite: `cargo test -p worldwake-core event_record`
6. Existing suite: `cargo test -p worldwake-core event_log`
7. Existing suite: `cargo test -p worldwake-core verification`
8. Existing suite: `cargo test -p worldwake-systems perception`
9. Existing suite: `cargo test -p worldwake-systems --test e15_information_integration`
10. Existing suite: `cargo test -p worldwake-cli events`
11. `cargo clippy --workspace --all-targets -- -D warnings`
12. `cargo test --workspace`

### Invariants

1. `EventPayload` remains the only authoritative payload schema.
2. Event wrapper read access is semantic and borrowed, not a second owned or aliased field surface.
3. `event_id` remains distinct from payload data.
4. No compatibility alias path preserves direct wrapper-internal field access as a supported long-term API.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_record.rs` — add or update tests to prove the shared read-only event surface returns the same canonicalized payload data for both `PendingEvent` and `EventRecord`.
2. `crates/worldwake-core/src/event_log.rs` — update indexing/traversal tests to read event data through the semantic API instead of wrapper internals.
3. `crates/worldwake-core/src/verification.rs` — keep verification coverage locked while reading cause refs and deltas through the new event surface.
4. `crates/worldwake-systems/src/perception.rs` — update focused discovery and witness-observation tests to use the semantic event reads.
5. `crates/worldwake-cli/src/handlers/events.rs` — keep human-facing event inspection stable while the wrapper boundary is tightened.
6. Any additional test edits in sim/systems/CLI should be limited to assertions invalidated by wrapper encapsulation, not broader payload-construction rewrites.

### Commands

1. `cargo test -p worldwake-core event_record`
2. `cargo test -p worldwake-core event_log`
3. `cargo test -p worldwake-core verification`
4. `cargo test -p worldwake-systems perception`
5. `cargo test -p worldwake-systems --test e15_information_integration`
6. `cargo test -p worldwake-cli events`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Added a shared `EventView` read-only semantic surface in `crates/worldwake-core/src/event_record.rs` and re-exported it from `worldwake-core`.
  - Made `PendingEvent.payload` and `EventRecord.payload` private while keeping `EventPayload` as the sole owned event schema for construction and serialization.
  - Migrated runtime event readers in `event_log`, `verification`, CLI event handlers, `perception`, and `combat` to semantic borrowed reads.
  - Updated only the tests and helper assertions that broke because wrapper internals are now hidden, including affected core, sim, systems, CLI, and AI test files.
- Deviations from original plan:
  - The ticket initially framed the runtime scope mostly around `event_log`, `verification`, `perception`, and CLI. In practice, `combat` also had a real production read-path dependency on wrapper internals and was migrated as part of the same architectural boundary tightening.
  - No payload-schema changes, alias surfaces, or duplicate wrapper fields were introduced.
- Verification results:
  - Passed `cargo test -p worldwake-core event_record`
  - Passed `cargo test -p worldwake-core event_log`
  - Passed `cargo test -p worldwake-core verification`
  - Passed `cargo test -p worldwake-systems perception`
  - Passed `cargo test -p worldwake-systems --test e15_information_integration`
  - Passed `cargo test -p worldwake-cli events`
  - Passed `cargo clippy --workspace --all-targets -- -D warnings`
  - Passed `cargo test --workspace`
