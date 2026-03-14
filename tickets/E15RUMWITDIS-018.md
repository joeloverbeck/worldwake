# E15RUMWITDIS-018: Replace Wide Event Constructors With Typed Payload Specs

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` event construction API and affected event-construction call sites/tests
**Deps**: `archive/tickets/completed/E15RUMWITDIS-017.md`, `archive/specs/E15-rumor-witness-discovery.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/FOUNDATIONS.md`

## Problem

`E15RUMWITDIS-017` correctly converged complete `PendingEvent` assembly onto one canonical constructor, but that constructor now carries the entire event payload as a long positional argument list. That is cleaner than fragmented builder-plus-mutation ownership, but it is still brittle:

1. complete event payload assembly depends on argument ordering rather than named structure
2. adding or reordering payload fields increases call-site fragility across production and tests
3. `EventRecord::new(...)` still mirrors the same wide constructor shape, so the API smell is duplicated

This is not a functional bug today, but it is a real architectural risk. Event payloads are foundational simulation records; they should be explicit, hard to misuse, and resilient to field growth.

## Assumption Reassessment (2026-03-14)

1. `crates/worldwake-core/src/event_record.rs` now has a single canonical complete-event constructor, `PendingEvent::new_complete(...)`, but it still requires a wide positional parameter list and is explicitly exempted from Clippy’s `too_many_arguments`.
2. `crates/worldwake-core/src/world_txn.rs` and `crates/worldwake-systems/src/perception.rs` are now the main production callers of that complete constructor, so any replacement must keep those paths straightforward and not reintroduce post-build metadata mutation.
3. `EventRecord::new(...)` still exists as a parallel wide constructor, which means the API duplication problem is not fully solved by only changing `PendingEvent`.
4. The current architecture does not need another builder layer. The cleaner next step is a typed event payload/spec value that makes fields explicit while preserving the two-path ownership model established in `E15RUMWITDIS-017`.

## Architecture Check

1. A typed event payload/spec struct is cleaner than a wide positional constructor because it makes event fields explicit at the call site and reduces the chance of silent argument-order mistakes as the schema evolves.
2. This is cleaner than adding another fluent builder. The code already decided that complete-event ownership should be canonical and direct; a typed payload preserves that without reintroducing fragmented assembly.
3. `WorldTxn` should still own transaction-built event finalization, and non-transaction emitters should still own manual payload population. The improvement here is API shape, not ownership semantics.
4. No backwards-compatibility aliasing: if the typed payload/spec cleanly replaces the wide constructors, remove the old constructor surfaces instead of keeping both.

## What to Change

### 1. Introduce a typed complete-event payload/spec in core

Add a dedicated type in `crates/worldwake-core/src/event_record.rs` for complete event payload assembly. Acceptable names include `PendingEventSpec`, `PendingEventPayload`, or equivalent.

Requirements:

1. it must carry the same complete payload currently passed to `PendingEvent::new_complete(...)`
2. it must keep evidence canonicalization and target canonicalization owned by `PendingEvent` construction
3. field names must be explicit at construction sites rather than relying on positional ordering
4. it must preserve deterministic authoritative data structures (`Vec`, `BTreeMap`, `BTreeSet`)

### 2. Converge `PendingEvent` and `EventRecord` construction on the typed payload

Update `PendingEvent` and `EventRecord` construction so complete records are built from the typed payload/spec rather than wide argument lists.

Requirements:

1. `PendingEvent::new(...)` may remain as the small convenience path for zero-metadata events only if it delegates through the typed payload/spec
2. `EventRecord::new(...)` should either be removed or rewritten to use the same typed payload/spec rather than preserving a second wide constructor
3. no constructor-plus-mutation fallback path may be reintroduced

### 3. Update production callers and focused tests

Migrate the production callers and tests changed by `E15RUMWITDIS-017` to the typed payload/spec surface.

Requirements:

1. `WorldTxn::into_pending_event` should remain a direct finalization path with explicit payload fields
2. `perception` manual discovery emission should remain explicit and compact
3. tests should prove the typed payload/spec preserves the same event semantics as the current constructor

## Files to Touch

- `crates/worldwake-core/src/event_record.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify if exports change)
- `crates/worldwake-core/src/world_txn.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify)
- Targeted event-construction tests in `worldwake-core` and `worldwake-systems`

## Out of Scope

- Reintroducing fluent event builders or post-build metadata setters
- Changing event-log schema or serialized field meanings
- Changing perception or transaction semantics beyond API-shape migration
- Broad cleanup of unrelated wide constructors elsewhere in the workspace

## Acceptance Criteria

### Tests That Must Pass

1. Complete event payloads can be assembled through a typed payload/spec without positional constructor ambiguity.
2. Canonical evidence ordering/deduplication and target canonicalization remain unchanged.
3. `WorldTxn` and manual perception emitters still produce the same event records after migration.
4. Existing suite: `cargo test -p worldwake-core event_record`
5. Existing suite: `cargo test -p worldwake-systems perception`
6. Existing suite: `cargo test -p worldwake-core world_txn`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`

### Invariants

1. There remains exactly one canonical complete-event payload shape shared by transaction finalization and manual event emitters.
2. Event construction does not regress to post-build metadata mutation or parallel compatibility APIs.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_record.rs` — add or update tests proving the typed payload/spec still yields canonical `PendingEvent` and `EventRecord` construction.
2. `crates/worldwake-core/src/world_txn.rs` — keep transaction finalization tests proving observed entities and evidence survive the API migration unchanged.
3. `crates/worldwake-systems/src/perception.rs` — keep focused discovery and observed-snapshot tests proving manual emitters still construct the same payloads.

### Commands

1. `cargo test -p worldwake-core event_record`
2. `cargo test -p worldwake-core world_txn`
3. `cargo test -p worldwake-systems perception`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`
