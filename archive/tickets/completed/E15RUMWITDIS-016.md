# E15RUMWITDIS-016: Unify Event Metadata Ownership Under WorldTxn

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` event transaction shape, event emitters using `WorldTxn`, and event construction tests
**Deps**: `archive/tickets/completed/E15RUMWITDIS-013.md`, `archive/tickets/completed/E15RUMWITDIS-014.md`, `archive/tickets/completed/E15RUMWITDIS-015.md`, `specs/E15-rumor-witness-discovery.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/FOUNDATIONS.md`

## Problem

`E15RUMWITDIS-013` moved event-local observed snapshots into the append-only event record and centralized default snapshot capture in `WorldTxn::into_pending_event()`. That fixed the live-world rebuild bug in event-based perception, but one architectural gap remains:

1. `WorldTxn` still does not own event evidence metadata.
2. Some emitters still build an event with `txn.into_pending_event()` and then decorate it afterward with `.with_evidence(...)`.
3. Because metadata ownership is split, event-local snapshot capture cannot yet be fully centralized for evidence-linked entities.

The current code works, but it leaves the event-construction boundary partially duplicated. If that persists, more emitters will accumulate post-build event mutation and `WorldTxn` will stop being the single trustworthy place to understand what a committed event exposes.

## Assumption Reassessment (2026-03-14)

1. `crates/worldwake-core/src/world_txn.rs` now captures `observed_entities` centrally in `into_pending_event()`, but it still only has direct ownership of actor, targets, tags, deltas, visibility, and witness data.
2. `PendingEvent` in `crates/worldwake-core/src/event_record.rs` still supports post-construction decoration via `with_evidence(...)` and `with_observed_entities(...)`.
3. Production code still uses post-build decoration on the transaction path. The live transaction-built example is `crates/worldwake-systems/src/combat.rs`, which does `txn.into_pending_event().with_evidence(fatality.evidence)`.
4. `crates/worldwake-systems/src/perception.rs` still uses `with_evidence(...)` and `with_observed_entities(...)`, but those sites build `PendingEvent` manually rather than via `WorldTxn`. They are not evidence that multiple transaction-built ownership paths remain; they are the remaining manual-event surface.
5. No active sibling ticket in `tickets/` owns this boundary cleanup. `E15RUMWITDIS-013`, `E15RUMWITDIS-014`, and `E15RUMWITDIS-015` are already archived under `archive/tickets/completed/`.
6. `E15RUMWITDIS-013` intentionally corrected its own scope instead of solving this deeper ownership issue. This ticket is still the right place to finish the transaction-boundary cleanup rather than broadening unrelated archived work.

## Architecture Check

1. The clean architecture is for `WorldTxn` to own all event metadata that is part of the append-only causal record: actor, targets, deltas, tags, witnesses, visibility, evidence, and event-local observed snapshots.
2. This is cleaner than preserving a split model where a transaction creates “most of” an event and callers mutate the pending payload afterward. A single construction boundary is easier to reason about, easier to test, and harder to accidentally bypass.
3. Once evidence is transaction-owned, evidence-linked observed snapshots can also be captured centrally. That removes the last incentive to keep adding manual `with_observed_entities(...)` decoration for transaction-built events.
4. No backwards-compatibility aliasing: this should converge on one canonical `WorldTxn -> PendingEvent` construction path for transaction-built events, not preserve multiple equally valid metadata ownership models long-term.
5. Manual `PendingEvent::new(...)` remains a separate construction surface today. The durable architectural direction is still to keep metadata canonicalization inside the builder that owns the event payload, but this ticket only needs to finish that cleanup for `WorldTxn` paths.

## What to Change

### 1. Move event evidence ownership into `WorldTxn`

Extend `WorldTxn` so callers can attach event evidence before `into_pending_event()` / `commit()`.

Acceptable shapes include:

```rust
pub fn add_evidence(&mut self, evidence: EvidenceRef) -> &mut Self
pub fn extend_evidence(&mut self, evidence: impl IntoIterator<Item = EvidenceRef>) -> &mut Self
```

or an equivalent transaction-owned API.

Requirements:

1. Evidence ordering/deduplication must remain deterministic and match the `PendingEvent` contract.
2. `into_pending_event()` should emit the final canonical evidence list directly; callers should not need to mutate the returned `PendingEvent`.
3. `commit()` and `into_pending_event()` must continue to be the single authoritative event-finalization path for transaction-built events.

### 2. Capture observed snapshots after evidence is known

Once evidence is owned by `WorldTxn`, central snapshot capture should include evidence-linked entities in addition to actor, targets, and delta-linked entities.

That means:

1. `WorldTxn` observed-entity collection should include entities referenced by its owned evidence set.
2. `into_pending_event()` should capture `observed_entities` only after the final event metadata set is complete.
3. Manual `PendingEvent::new(...)` should remain available for non-transaction emitters, but transaction-built production events should no longer depend on `.with_evidence(...)` or `.with_observed_entities(...)` after `into_pending_event()`.

### 3. Migrate transaction-built emitters off post-build decoration

Update production emitters that currently do post-build decoration on transaction-created events so they use the transaction-owned evidence API instead.

Known first target:

- `crates/worldwake-systems/src/combat.rs`

If implementation reveals any other transaction-built production emitters still mutate `PendingEvent` afterward, migrate them in this ticket as well.

Do not expand this ticket into converting manual `PendingEvent::new(...)` call sites such as `crates/worldwake-systems/src/perception.rs` to transactions unless that becomes necessary to preserve invariants or remove duplicated production ownership logic.

### 4. Narrow the remaining role of manual `PendingEvent` decoration

Do not remove `PendingEvent::with_evidence(...)` or `with_observed_entities(...)` blindly if tests, CLI helpers, replay fixtures, or non-transaction emitters still need them.

But after this ticket:

1. production code using `WorldTxn` should not rely on those post-build setters
2. any remaining uses should be limited to manual-event construction surfaces
3. tests should make that ownership distinction explicit

## Files to Touch

- `crates/worldwake-core/src/world_txn.rs` (modify)
- `crates/worldwake-core/src/event_record.rs` (modify if event-builder APIs or tests need adjustment)
- `crates/worldwake-core/src/lib.rs` (modify if exports change)
- `crates/worldwake-systems/src/combat.rs` (modify)
- Additional transaction-built production emitters using `txn.into_pending_event().with_*` if discovered during implementation

## Out of Scope

- Confidence-policy work in `E15RUMWITDIS-014`
- Tell-profile runtime/planner cleanup in `E15RUMWITDIS-015`
- Redesigning all manual `PendingEvent::new(...)` sites into transactions
- Changing perception semantics beyond using the event payload already established in `E15RUMWITDIS-013`
- Broad replay/event-log schema redesign outside transaction metadata ownership

## Acceptance Criteria

### Tests That Must Pass

1. Transaction-built events can carry evidence without post-build `PendingEvent` mutation
2. `WorldTxn::into_pending_event()` includes evidence-linked entities in `observed_entities` when evidence references them
3. Production transaction-built emitters no longer use `txn.into_pending_event().with_evidence(...)`
4. Event evidence ordering/deduplication remains deterministic
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo test -p worldwake-systems`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`

### Invariants

1. Transaction-built event metadata is owned by `WorldTxn`, not split across transaction creation and post-build `PendingEvent` mutation
2. Event-local observed snapshot capture for transaction-built events runs against the final event metadata set, including evidence-linked entities
3. No new backwards-compatibility alias path preserves multiple production ownership models for transaction-built event metadata

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world_txn.rs` — add tests proving transaction-owned evidence is emitted deterministically and expands observed-entity capture to evidence-linked entities.
2. `crates/worldwake-core/src/event_record.rs` — keep or adjust event payload tests so manual `PendingEvent` construction still canonicalizes evidence ordering/deduplication.
3. `crates/worldwake-systems/src/combat.rs` — add or adjust focused tests proving combat still emits the same evidence while using the transaction-owned event metadata path.

### Commands

1. `cargo test -p worldwake-core world_txn`
2. `cargo test -p worldwake-core event_record`
3. `cargo test -p worldwake-systems combat`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What changed:
  - `WorldTxn` now owns event evidence directly via transaction APIs and emits canonicalized evidence from `into_pending_event()` / `commit()`.
  - Transaction-built observed-entity capture now includes entities referenced only through evidence, so the event snapshot is taken from the final metadata set.
  - The combat fatality path no longer mutates `PendingEvent` after `txn.into_pending_event()`; it commits through the canonical transaction boundary instead.
  - `PendingEvent` gained a direct constructor for canonical evidence-bearing manual events while keeping manual decoration available for non-transaction paths.
- Deviations from original plan:
  - No broader conversion of manual `PendingEvent::new(...)` sites such as perception was done; those remain explicitly out of scope.
  - `PendingEvent::with_evidence(...)` and `with_observed_entities(...)` were retained for manual-event surfaces.
- Verification results:
  - `cargo test -p worldwake-core world_txn`
  - `cargo test -p worldwake-core event_record`
  - `cargo test -p worldwake-systems combat`
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-systems`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
