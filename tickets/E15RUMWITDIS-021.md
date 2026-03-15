# E15RUMWITDIS-021: Generalize Event Helper Read APIs From EventRecord To EventView

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core`/`worldwake-systems`/`worldwake-cli` helper signatures and event-focused tests
**Deps**: `archive/tickets/completed/E15RUMWITDIS-020.md`, `archive/specs/E15-rumor-witness-discovery.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/FOUNDATIONS.md`

## Problem

`E15RUMWITDIS-020` established `EventView` as the semantic read boundary for events and hid wrapper storage internals. That fixed the main architectural leak, but several helper functions still accept `&EventRecord` even when they only read semantic event fields and do not use `event_id`.

That leaves a smaller but still unnecessary coupling point:

1. Read-only helpers still encode the concrete wrapper type in their signature instead of the semantic event contract they actually need.
2. `PendingEvent` and `EventRecord` now share a read surface, but some helper APIs cannot reuse that common surface because their signatures remain wrapper-specific.
3. Future event read utilities will drift toward `EventRecord` by habit unless the helper boundary is tightened now.

## Assumption Reassessment (2026-03-15)

1. `crates/worldwake-core/src/event_record.rs` now exports `EventView`, and both `PendingEvent` and `EventRecord` implement it — confirmed.
2. `EventRecord` still uniquely owns `event_id`, so functions that use causal identity, indexing, or logging by event id must remain `EventRecord`-specific — corrected scope.
3. Some helper functions currently take `&EventRecord` only to read semantic fields such as tags, visibility, witnesses, place, actor, or observed entities. These are the intended targets for cleanup.
4. This ticket should not broaden into replacing all `&EventRecord` uses. If a helper needs `event_id`, it should keep `EventRecord` in its contract.
5. This follow-up is about helper boundaries, not about adding new event data, alias traits, blanket wrappers, or compatibility shims.

## Architecture Check

1. A helper that only reads semantic event fields should depend on `EventView`, not on `EventRecord`. That is the cleanest expression of the actual contract.
2. This is more robust than keeping `EventRecord` in signatures because it prevents `event_id` from leaking into APIs that do not conceptually require it.
3. This is more extensible than adding parallel helper overloads for `PendingEvent` and `EventRecord`; one trait-bounded helper keeps the semantic boundary singular.
4. No backwards-compatibility layering: do not add duplicate helper variants such as `foo_from_record()` beside `foo()`. Replace the concrete signature with the semantic one and update callers.

## What to Change

### 1. Audit helper signatures that only need semantic event reads

Inspect helper functions added or retained after `E15RUMWITDIS-020` and identify any that:

1. accept `&EventRecord`
2. do not read `event_id`
3. only consume fields already covered by `EventView`

These helpers should be considered in-scope for signature tightening.

### 2. Replace concrete helper contracts with `EventView`-based contracts

For each in-scope helper:

1. change the function signature from `&EventRecord` to `&impl EventView` or an equivalent generic bound
2. keep the helper read-only
3. continue reading through semantic accessors only
4. avoid introducing extra trait aliases, wrapper adapters, or duplicate helper families

If a helper needs to remain nameable in trait bounds across modules, a generic type parameter such as `fn helper<E: EventView + ?Sized>(event: &E)` is acceptable.

### 3. Preserve `EventRecord` specificity only where identity is part of the contract

Do not generalize functions that:

1. use `event_id`
2. participate in indexing or causal traversal keyed by event id
3. serialize or persist full records rather than read semantic event data

The goal is not maximal genericity. The goal is an honest contract boundary.

## Files to Touch

- `crates/worldwake-core/src/event_record.rs` (modify only if helper ergonomics or docs need adjustment)
- `crates/worldwake-core/src/` event helper modules that still accept `&EventRecord` for semantic reads only (modify as needed)
- `crates/worldwake-systems/src/perception.rs` (modify if helper signatures still use `EventRecord` where `EventView` is sufficient)
- `crates/worldwake-cli/src/handlers/events.rs` (modify if any local helpers remain overly concrete)
- affected tests in core/systems/CLI (modify as needed)

## Out of Scope

- Changing `EventView` field coverage
- Reopening wrapper storage visibility
- Removing `event_id` from `EventRecord`
- Generalizing APIs that truly depend on `event_id`
- Adding compatibility overloads, alias traits, or helper wrappers to preserve old concrete signatures
- Broad event-model refactors unrelated to helper parameter contracts

## Acceptance Criteria

### Tests That Must Pass

1. Any helper that only needs semantic event reads no longer requires `&EventRecord` in its signature.
2. Helpers that rely on `event_id` remain explicitly `EventRecord`-based.
3. No duplicate helper APIs are added just to support both `EventRecord` and `EventView`.
4. Existing suite: `cargo test -p worldwake-core event_record`
5. Existing suite: `cargo test -p worldwake-core event_log`
6. Existing suite: `cargo test -p worldwake-systems perception`
7. Existing suite: `cargo test -p worldwake-cli events`
8. `cargo clippy --workspace --all-targets -- -D warnings`
9. `cargo test --workspace`

### Invariants

1. `EventView` remains the semantic read boundary for event data.
2. `EventRecord` remains the type that owns `event_id`; that identity is not abstracted away where it is semantically required.
3. No compatibility aliases or parallel helper families are introduced.
4. Helper genericity remains read-only and does not create a second owned event schema.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_record.rs` — add or update focused tests only if helper-generic refactors require new compile-time or behavior coverage for `EventView`-based helper usage.
   Rationale: keep the shared event read boundary explicit and prevent accidental regression toward `EventRecord`-only helpers.
2. `crates/worldwake-systems/src/perception.rs` — update tests if helper signatures are generalized there.
   Rationale: perception is the most likely runtime reader to benefit from `EventView`-generic helpers because many local helpers only read semantic event data.
3. `crates/worldwake-cli/src/handlers/events.rs` — update tests if local formatting helpers are generalized.
   Rationale: preserve human-facing event rendering while tightening helper contracts.

### Commands

1. `cargo test -p worldwake-core event_record`
2. `cargo test -p worldwake-core event_log`
3. `cargo test -p worldwake-systems perception`
4. `cargo test -p worldwake-cli events`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`
