# E15RUMWITDIS-017: Canonicalize Manual Event Construction Surfaces

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` manual event builder API, `worldwake-systems` perception emitters, and event construction tests
**Deps**: `archive/tickets/completed/E15RUMWITDIS-016.md`, `archive/specs/E15-rumor-witness-discovery.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/FOUNDATIONS.md`

## Problem

`E15RUMWITDIS-016` cleaned up the transaction-built event boundary, but the repository still has a second production event-construction path: manual `PendingEvent::new(...)` call sites that attach evidence and observed snapshots afterward with ad hoc `.with_*` mutation.

That remaining split is small but architecturally real:

1. transaction-built events now canonicalize metadata in `WorldTxn`
2. manual production events still canonicalize metadata at each caller
3. snapshot and evidence ownership rules are therefore still duplicated across event-building surfaces

The current code works, but this is exactly the kind of architectural fork that tends to spread. If more production systems follow the current perception pattern, event payload completeness and canonicalization rules will become caller-by-caller policy instead of a single builder policy.

## Assumption Reassessment (2026-03-14)

1. `crates/worldwake-systems/src/perception.rs` is the main production manual-event surface today. It still uses `PendingEvent::new(...)` followed by `.with_evidence(...)` and `.with_observed_entities(...)` for discovery and event-observation scenarios.
2. Other `PendingEvent::new(...)` call sites currently found in `worldwake-cli`, `worldwake-ai`, `worldwake-sim`, `worldwake-core`, and integration tests are primarily tests, replay/save-load fixtures, or tooling helpers rather than live production system emitters.
3. `crates/worldwake-core/src/event_record.rs` already has `PendingEvent::new_with_evidence(...)`, but there is still no canonical core builder that owns both evidence and observed-entity payload assembly for non-transaction emitters.
4. `WorldTxn` should remain the canonical builder for staged world mutations. This ticket should not blur the boundary by trying to force read-only/manual emitters through transactions.
5. The mismatch is therefore not “all `PendingEvent::new(...)` is bad.” The real issue is that production manual-event emitters still hand-roll canonical payload assembly instead of using a single core event-builder surface.

## Architecture Check

1. The cleaner architecture is to have exactly two canonical event-construction paths:
   - `WorldTxn` for staged world mutations
   - a dedicated manual-event builder for non-transaction emitters
2. This is cleaner than preserving today’s split where manual emitters construct a partial `PendingEvent` and then remember to mutate evidence and observed snapshots afterward at each call site.
3. A dedicated builder keeps payload completeness rules in one place, which is easier to reason about, easier to test, and less likely to drift as new event metadata fields are added.
4. No backwards-compatibility aliasing: this ticket should converge production manual emitters on one canonical builder path, not add more equally valid helper layers.

## What to Change

### 1. Add a canonical manual-event builder in core

Introduce a focused builder API in `crates/worldwake-core/src/event_record.rs` for non-transaction emitters that need to assemble complete event payloads without staging world mutations.

Acceptable shapes include:

```rust
PendingEvent::builder(...)
    .with_evidence(...)
    .with_observed_entities(...)
    .build()
```

or

```rust
PendingEvent::new_complete(...)
```

or an equivalent shape, provided it meets these requirements:

1. evidence ordering/deduplication stays canonical inside the builder
2. observed entities are attached through the same builder surface rather than caller-by-caller payload patching
3. the resulting API is clearly distinct from `WorldTxn`, not a compatibility shim layered on top of it

### 2. Migrate production perception emitters to the canonical builder

Update `crates/worldwake-systems/src/perception.rs` to stop constructing partial `PendingEvent` values and mutating them afterward.

Requirements:

1. discovery event emission should use the canonical manual-event builder
2. event-observation test and production helper paths in perception should use the same canonical manual-event builder when they need evidence or observed snapshots
3. payload semantics must stay identical to current behavior unless a current behavior is provably incomplete or inconsistent

### 3. Narrow remaining post-build setters to non-canonical surfaces

After the production migration:

1. no production system code should need `.with_evidence(...)` or `.with_observed_entities(...)` after partial construction
2. remaining `.with_*` use should be limited to tests, fixtures, or transitional helper code only if they cannot yet reasonably move
3. if the canonical builder fully supersedes those setters for production/manual use, consider removing or reducing those setters instead of preserving them indefinitely

Do not preserve redundant public APIs just because older tests happen to compile. If the builder fully replaces the post-build mutation surface cleanly, update the callers now.

## Files to Touch

- `crates/worldwake-core/src/event_record.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify if exports change)
- `crates/worldwake-systems/src/perception.rs` (modify)
- Additional production manual-event emitters if discovered during implementation
- Targeted test files covering manual-event construction and perception event emission

## Out of Scope

- Replacing `WorldTxn` as the transaction-built event builder
- Converting read-only tests, CLI helpers, or replay/save-load fixtures unless the chosen API change requires straightforward caller updates
- Changing perception semantics beyond canonicalizing how payloads are assembled
- Broad event-log schema redesign

## Acceptance Criteria

### Tests That Must Pass

1. Production manual-event emitters can build complete events without post-build mutation of a partial `PendingEvent`
2. Canonical manual-event construction preserves deterministic evidence ordering/deduplication
3. Canonical manual-event construction preserves explicit observed-entity payloads
4. Perception discovery and observation paths still emit the same evidence and observed snapshots after migration
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo test -p worldwake-systems`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`

### Invariants

1. Production event payload completeness rules live in canonical builders, not in scattered post-build mutation call sites
2. `WorldTxn` remains the canonical path for transaction-built events, while manual emitters use a distinct canonical non-transaction builder
3. No new backwards-compatibility alias path preserves multiple production ownership models for manual-event metadata assembly

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_record.rs` — add or update tests that prove the canonical manual-event builder preserves evidence and observed-entity payload canonicalization.
2. `crates/worldwake-systems/src/perception.rs` — add or adjust focused tests proving discovery and event-observation events still emit the same payloads through the new builder path.
3. Any affected production integration tests under `crates/worldwake-systems/tests/` — keep end-to-end E15 information behavior locked while the construction surface changes.

### Commands

1. `cargo test -p worldwake-core event_record`
2. `cargo test -p worldwake-systems perception`
3. `cargo test -p worldwake-systems e15_information_integration`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`
