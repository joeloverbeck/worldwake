# E15RUMWITDIS-017: Canonicalize Manual Event Construction Surfaces

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` manual event builder API, `worldwake-systems` perception emitters, and event construction tests
**Deps**: `archive/tickets/completed/E15RUMWITDIS-016.md`, `archive/specs/E15-rumor-witness-discovery.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/FOUNDATIONS.md`

## Problem

`E15RUMWITDIS-016` cleaned up the transaction-built event boundary, but event metadata assembly is still split across multiple core/manual surfaces:

1. `WorldTxn::into_pending_event` canonicalizes evidence and observed snapshots through a constructor-plus-post-build chain
2. `PendingEvent` still exposes separate partial-construction and post-build mutation APIs (`new_with_evidence`, `.with_evidence(...)`, `.with_observed_entities(...)`)
3. the live production perception discovery emitter still patches evidence after `PendingEvent::new(...)`

The production risk is narrower than this ticket originally described, but it is still architecturally real: the core type still allows metadata ownership to remain fragmented between constructors and follow-up mutation. That means future non-transaction emitters can keep choosing ad hoc assembly paths instead of one canonical complete-event surface.

## Assumption Reassessment (2026-03-14)

1. `crates/worldwake-systems/src/perception.rs` is the only confirmed live production manual-event emitter still using post-build metadata mutation, and it currently does so for mismatch evidence only.
2. The `.with_observed_entities(...)` call sites found in `perception.rs` today are test scaffolding, not production emitters. Other non-test `PendingEvent::new(...)` call sites in `worldwake-cli`, `worldwake-ai`, `worldwake-sim`, and `worldwake-core` currently construct plain events without extra metadata.
3. `crates/worldwake-core/src/event_record.rs` already canonicalizes target/evidence ordering, but it still spreads complete-event assembly across `new`, `new_with_evidence`, and `.with_*` setters instead of a single constructor for full payloads.
4. `crates/worldwake-core/src/world_txn.rs` still assembles complete events through `PendingEvent::new_with_evidence(...).with_observed_entities(...)`, so the split also exists inside the canonical transaction path.
5. `WorldTxn` should remain the canonical builder for staged world mutations. This ticket should not blur the boundary by forcing read-only/manual emitters through transactions.
6. The mismatch is therefore not “all `PendingEvent::new(...)` is bad.” The real issue is that complete metadata assembly still lacks one canonical `PendingEvent` construction surface.

## Architecture Check

1. The cleaner architecture is to have exactly two canonical event-construction paths:
   - `WorldTxn` for staged world mutations
   - one canonical complete-event constructor on `PendingEvent` for non-transaction emitters and for `WorldTxn` finalization
2. This is cleaner than preserving today’s split where complete events are assembled through constructor-plus-mutation chains.
3. A single complete-event constructor keeps payload completeness rules in one place, which is easier to reason about, easier to test, and less likely to drift as new event metadata fields are added.
4. No backwards-compatibility aliasing: if the complete-event constructor supersedes `new_with_evidence` and the post-build setters cleanly, remove those redundant surfaces instead of preserving them indefinitely.

## What to Change

### 1. Add one canonical complete-event constructor in core

Introduce a focused API in `crates/worldwake-core/src/event_record.rs` for assembling a complete non-transaction event payload in one call.

Preferred shape:

```rust
PendingEvent::new_complete(...)
```

Requirements:

1. evidence ordering/deduplication stays canonical inside the constructor
2. observed entities are attached through the same constructor surface rather than caller-by-caller payload patching
3. `PendingEvent::new(...)` may remain as the zero-metadata convenience path only if it delegates to the canonical complete-event constructor
4. remove `new_with_evidence` and post-build `.with_*` setters if the new constructor makes them redundant

### 2. Migrate production perception emitters and transaction finalization

Update `crates/worldwake-systems/src/perception.rs` to stop constructing partial `PendingEvent` values and mutating them afterward.

Requirements:

1. discovery event emission should use the canonical manual-event builder
2. `WorldTxn::into_pending_event` should also finalize through the same canonical complete-event constructor rather than a constructor-plus-mutation chain
3. targeted perception test scaffolding that currently assembles explicit observed snapshots should move to the same canonical constructor as part of the migration
4. payload semantics must stay identical to current behavior unless a current behavior is provably incomplete or inconsistent

### 3. Remove redundant post-build metadata assembly surfaces

After the production migration:

1. no production system code should need `.with_evidence(...)` or `.with_observed_entities(...)` after partial construction
2. if the canonical constructor fully supersedes those setters, remove them and update all affected callers now
3. do not preserve redundant public APIs just because older tests happen to compile

## Files to Touch

- `crates/worldwake-core/src/event_record.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify if exports change)
- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-core/src/world_txn.rs` (modify)
- Additional production manual-event emitters if discovered during implementation
- Targeted test files covering manual-event construction and perception event emission

## Out of Scope

- Replacing `WorldTxn` as the transaction-built event builder
- Converting read-only tests, CLI helpers, or replay/save-load fixtures unless the chosen API change requires straightforward caller updates
- Changing perception semantics beyond canonicalizing how payloads are assembled
- Broad event-log schema redesign

## Acceptance Criteria

### Tests That Must Pass

1. Complete `PendingEvent` payloads can be built without post-build mutation of a partial `PendingEvent`
2. Canonical complete-event construction preserves deterministic evidence ordering/deduplication
3. Canonical complete-event construction preserves explicit observed-entity payloads
4. Perception discovery and observation paths still emit the same evidence and observed snapshots after migration
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo test -p worldwake-systems`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`

### Invariants

1. Complete event payload rules live in canonical constructors/builders, not in scattered post-build mutation call sites
2. `WorldTxn` remains the canonical path for transaction-built events, while complete `PendingEvent` assembly uses one distinct canonical non-transaction constructor
3. No new backwards-compatibility alias path preserves multiple ownership models for event metadata assembly

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_record.rs` — add or update tests that prove the canonical complete-event constructor preserves evidence and observed-entity payload canonicalization.
2. `crates/worldwake-systems/src/perception.rs` — add or adjust focused tests proving discovery and event-observation events still emit the same payloads through the new constructor path.
3. `crates/worldwake-core/src/world_txn.rs` or existing transaction-focused tests — prove transaction finalization preserves observed snapshots through the same constructor path.
4. Any affected production integration tests under `crates/worldwake-systems/tests/` — keep end-to-end E15 information behavior locked while the construction surface changes.

### Commands

1. `cargo test -p worldwake-core event_record`
2. `cargo test -p worldwake-systems perception`
3. `cargo test -p worldwake-systems e15_information_integration`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

1. The ticket scope was corrected before implementation: production `perception` only had a live evidence-only post-build mutation path, while explicit observed-entity post-build assembly was primarily a core `WorldTxn` finalization concern plus test scaffolding.
2. `PendingEvent` now has a single canonical complete-event constructor that assembles evidence and observed entities in one place, and `PendingEvent::new(...)` remains only as the empty-metadata convenience path delegating to it.
3. Redundant complete-event assembly surfaces were removed instead of preserved: `new_with_evidence`, `.with_evidence(...)`, and `.with_observed_entities(...)` were eliminated and all affected callers were updated.
4. `WorldTxn` finalization and perception discovery emission now use the canonical constructor directly, and the perception/event-record tests were updated to lock the invariant down.
