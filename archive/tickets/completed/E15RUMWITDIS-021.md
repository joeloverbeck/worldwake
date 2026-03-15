# E15RUMWITDIS-021: Generalize Event Helper Read APIs From EventRecord To EventView

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-systems` perception helper signatures and focused perception tests
**Deps**: `archive/tickets/completed/E15RUMWITDIS-020.md`, `archive/specs/E15-rumor-witness-discovery.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/FOUNDATIONS.md`

## Problem

`E15RUMWITDIS-020` established `EventView` as the semantic read boundary for events and hid wrapper storage internals. That fixed the main architectural leak, but several helper functions still accept `&EventRecord` even when they only read semantic event fields and do not use `event_id`.

That leaves a smaller but still unnecessary coupling point:

1. Read-only helpers still encode the concrete wrapper type in their signature instead of the semantic event contract they actually need.
2. `PendingEvent` and `EventRecord` now share a read surface, but some helper APIs cannot reuse that common surface because their signatures remain wrapper-specific.
3. Future event read utilities will drift toward `EventRecord` by habit unless the helper boundary is tightened now.

## Assumption Reassessment (2026-03-15)

1. `crates/worldwake-core/src/event_record.rs` exports `EventView`, and both `PendingEvent` and `EventRecord` implement it — confirmed.
2. A current workspace scan shows only three non-test helper signatures still typed as `&EventRecord`, all in `crates/worldwake-systems/src/perception.rs`:
   - `resolve_witnesses`
   - `social_observations_for_event`
   - `social_kind`
3. Those helpers only read semantic event fields such as visibility, witness data, place, actor, targets, observed entities, and tags. They do not use `event_id`.
4. `crates/worldwake-cli/src/handlers/events.rs` is already on the semantic side; it reads through `EventView` methods on concrete records obtained from the log and does not expose the helper leak this ticket was written to address.
5. No current `worldwake-core` helper signature needs this follow-up. `EventRecord` should remain concrete where identity-bearing APIs such as `EventLog::get` or `event_id`-specific logic are part of the contract.
6. This ticket should stay narrowly scoped to helper boundaries. It should not add new event data, alias traits, blanket wrappers, compatibility shims, or a broader generic sweep across modules that are already architecturally honest.

## Architecture Check

1. For private read-only helpers that only inspect event semantics, `EventView` is the honest contract and should replace `EventRecord`.
2. This is more robust than keeping `EventRecord` in those signatures because it prevents accidental coupling to `event_id` and storage-specific assumptions in perception logic.
3. This is more extensible than adding parallel helper overloads for `PendingEvent` and `EventRecord`; one trait-bounded helper keeps the semantic boundary singular without widening the public API.
4. Because the affected helpers are private to `perception.rs`, the cleanest implementation is a local generic or `&impl EventView` signature change. No new abstraction layer is warranted.
5. No backwards-compatibility layering: do not add duplicate helper variants such as `foo_from_record()` beside `foo()`. Replace the concrete signature with the semantic one and update callers/tests directly.

## What to Change

### 1. Tighten the three remaining helper signatures in `perception.rs`

Change the following private helpers from `&EventRecord` to an `EventView`-based contract:

1. `resolve_witnesses`
2. `social_observations_for_event`
3. `social_kind`

These are in scope because they are semantic readers only and do not use `event_id`.

### 2. Keep the refactor local and read-only

For each in-scope helper:

1. change the signature from `&EventRecord` to `&impl EventView` or an equivalent generic bound
2. keep the helper read-only
3. continue reading only through semantic accessors
4. avoid introducing trait aliases, wrapper adapters, duplicate helper families, or broader module refactors

### 3. Strengthen tests around the semantic boundary

Add or update focused tests in `perception.rs` to prove the generalized helpers still behave correctly when given semantic event readers and to guard against future regression back to `EventRecord`-only helper contracts.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify)
- focused tests in `crates/worldwake-systems/src/perception.rs` (modify/add as needed)

## Out of Scope

- Changing `EventView` field coverage
- Reopening wrapper storage visibility
- Removing `event_id` from `EventRecord`
- Generalizing `EventLog`, CLI handlers, or other APIs that already have an honest `EventRecord` or log-based contract
- Adding compatibility overloads, alias traits, or helper wrappers to preserve old concrete signatures
- Broad event-model refactors unrelated to the three private perception helper contracts

## Acceptance Criteria

### Tests That Must Pass

1. The three private semantic-read helpers in `crates/worldwake-systems/src/perception.rs` no longer require `&EventRecord` in their signatures.
2. Event APIs that truly depend on record identity remain explicitly `EventRecord`-based.
3. No duplicate helper APIs are added just to support both `EventRecord` and `EventView`.
4. Existing suite: `cargo test -p worldwake-systems perception`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`

### Invariants

1. `EventView` remains the semantic read boundary for event data.
2. `EventRecord` remains the type that owns `event_id`; that identity is not abstracted away where it is semantically required.
3. No compatibility aliases or parallel helper families are introduced.
4. The helper genericity stays local to perception and does not create a second owned event schema or a new public abstraction layer.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` — add or update focused tests around witness resolution and social observation classification with `EventView`-generic helper usage.
   Rationale: this is the only remaining production surface where helper contracts are overly concrete, so tests should pin the corrected boundary directly.

### Commands

1. `cargo test -p worldwake-systems perception`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Reassessed the ticket against the current codebase and narrowed scope from a cross-crate helper sweep to three private semantic-read helpers in `crates/worldwake-systems/src/perception.rs`.
  - Generalized `resolve_witnesses`, `social_observations_for_event`, and `social_kind` from `&EventRecord` to `EventView`-based read contracts.
  - Added a focused perception regression test proving those helpers work with `PendingEvent`, which locks the semantic boundary instead of only exercising `EventRecord`.
- Deviations from original plan:
  - No `worldwake-core` or `worldwake-cli` code changes were needed; the original ticket overstated the active leak surface.
  - Verification still included representative core/CLI targeted suites plus full workspace checks, but the implementation remained local to perception because widening it would not improve the architecture.
- Verification results:
  - Passed `cargo test -p worldwake-systems perception`
  - Passed `cargo test -p worldwake-core event_log`
  - Passed `cargo test -p worldwake-cli events`
  - Passed `cargo clippy --workspace --all-targets -- -D warnings`
  - Passed `cargo test --workspace`
