# E04ITECON-009: Generalize Lot Transfer Provenance Beyond Trade

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` item provenance model, transfer helpers, trade/bribe transfer callers
**Deps**: [archive/tickets/E04ITECON-006-lot-algebra.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E04ITECON-006-lot-algebra.md), [archive/tickets/E05RELOWN-005-ownership-possession-apis.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E05RELOWN-005-ownership-possession-apis.md), [archive/tickets/completed/E16OFFSUCFAC-006.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/E16OFFSUCFAC-006.md), [specs/E17-crime-theft-justice.md](/home/joeloverbeck/projects/worldwake/specs/E17-crime-theft-justice.md), [specs/S04-merchant-selling-market-presence.md](/home/joeloverbeck/projects/worldwake/specs/S04-merchant-selling-market-presence.md)

## Problem

`LotOperation::Traded` currently does double duty:

1. it records that a lot changed hands
2. it implies the high-level reason for that handoff was trade

That coupling is already too coarse. E16 bribery now transfers real goods, but the current implementation has to append `LotOperation::Traded` just to record a lawful transfer. That makes lot history semantically wrong. It also sets the project up for more distortion when E17 theft, fines, confiscation, taxes, tribute, or other institutional transfers arrive.

The clean architecture is to keep lot provenance focused on lot-lineage facts and let the event log carry the higher-level causal meaning. A lot should record that it was transferred; the attached event should say whether that transfer happened through trade, bribery, theft, or some later mechanism.

## Assumption Reassessment (2026-03-15)

1. `LotOperation` currently contains `Created`, `Split`, `Merge`, `Produced`, `Consumed`, `Destroyed`, `Spoiled`, `Transformed`, and `Traded` in [items.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/items.rs) — confirmed.
2. `ProvenanceEntry` already stores `event_id`, so lot history can point to the event record that explains why the transfer happened — confirmed.
3. Trade commit logic in [trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs) appends `LotOperation::Traded` directly — confirmed.
4. Bribe transfer logic in [office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) also appends `LotOperation::Traded` directly — confirmed, and this is the concrete architecture bug.
5. The remaining active E16 tickets do not own this issue:
   - [tickets/E16OFFSUCFAC-007.md](/home/joeloverbeck/projects/worldwake/tickets/E16OFFSUCFAC-007.md) is succession-system work
   - [tickets/E16OFFSUCFAC-008.md](/home/joeloverbeck/projects/worldwake/tickets/E16OFFSUCFAC-008.md) is public-order derivation
   - [tickets/E16OFFSUCFAC-009.md](/home/joeloverbeck/projects/worldwake/tickets/E16OFFSUCFAC-009.md) is AI planning/candidate generation
6. `ProvenanceEntry` does already carry `event_id`, but current action-layer callers append transfer provenance with `event_id: None`, and [world_txn.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs) does not backfill the committed event id during `commit()`. The previous draft overstated the current linkage behavior.
7. `specs/S04-merchant-selling-market-presence.md` says trade should append traded provenance, but that spec is about preserving trade-specific meaning, not about blessing `Traded` as the generic marker for every transfer. The current codebase can satisfy that intent more robustly by using transfer provenance plus trade-tagged event linkage.

## Architecture Check

1. `LotOperation` should describe physical lot-lineage transitions, not encode every social or institutional cause. The lineage fact here is "this lot was transferred."
2. High-level causal meaning already belongs in the append-only event log through tags and payload context. Reusing that existing channel is cleaner than proliferating provenance operations like `Bribed`, `FinePaid`, `Stolen`, or `Taxed`.
3. Replacing `Traded` with a general transfer operation is more extensible than adding one new enum variant per transfer source. It prevents provenance taxonomy bloat before E17 and later institution work amplify the problem.
4. The cleaner long-term architecture is not just "rename the enum and keep appending raw provenance from system code." Transfer provenance should be recorded through a transaction-owned API in core, so lawful transfer callers stop constructing provenance entries by hand.
5. No backward-compatibility aliases or dual semantics: do not keep both "generic transfer" and "pretend trade transfer" paths alive.

## What to Change

### 1. Generalize the lot provenance operation

In [items.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/items.rs):

- replace `LotOperation::Traded` with `LotOperation::Transferred`
- update `LotOperation::ALL`, serialization tests, and enum-coverage tests accordingly
- keep `ProvenanceEntry` shape unchanged unless implementation proves that a helper field is strictly necessary

Rationale:
- the provenance entry already has `event_id`
- the event record already carries tags like `Trade`, `Transfer`, `Social`, `Political`, `Coercion`
- that is enough to recover transfer cause without bloating `LotOperation`

### 2. Add a canonical transfer-provenance API in `WorldTxn`

Create a single helper on the core mutation surface, preferably in [world_txn.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs), for appending transfer provenance correctly.

Minimum target shape:

```rust
pub fn append_transfer_provenance(
    &mut self,
    lot_id: EntityId,
    amount: Quantity,
) -> Result<(), WorldError>
```

The helper should:

- append a `ProvenanceEntry` with `operation: LotOperation::Transferred`
- use the current transaction tick
- ensure the resulting provenance entry is linked to the committed event id instead of leaving `event_id: None`
- centralize transfer provenance creation so system-layer callers no longer hand-roll `ProvenanceEntry`

If the actual transaction/event implementation requires a slightly different internal shape, keep the public API small and transfer-specific rather than adding another free-form provenance escape hatch. A small `EventLog::next_id()` or equivalent transaction-internal finalization step is in scope if that is the cleanest way to attach the eventual event id before emit.

### 3. Convert all lawful transfer callers away from trade-specific provenance

Update current callers so they use the canonical transfer helper:

- [trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs)
- [office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs)

Trade remains trade because the emitted event is tagged `Trade` and `Transfer`.
Bribery remains bribery because the emitted event is tagged `Social` and `Transfer`.
The lot provenance itself should just say the lot transferred.

### 4. Lock in the event-plus-provenance contract with tests

Add tests proving:

- trade appends `Transferred` provenance, links it to the committed event id, and still emits trade-tagged events
- bribe appends `Transferred` provenance, links it to the committed event id, and still emits social/transfer-tagged events
- no caller appends a trade-specific provenance operation for non-trade actions

If a utility/helper test is needed in core, add it there instead of duplicating assertions in each caller module.

## Files to Touch

- [crates/worldwake-core/src/items.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/items.rs) (modify)
- [crates/worldwake-core/src/world_txn.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs) (modify)
- [crates/worldwake-systems/src/trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs) (modify)
- [crates/worldwake-systems/src/office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) (modify)

## Out of Scope

- adding one provenance variant per future transfer cause
- redesigning `EventRecord` or the event-log storage model beyond the minimal surface needed to link transfer provenance to the committed event id
- introducing any compatibility layer that accepts both `Traded` and `Transferred`
- changing trade valuation, bribery loyalty math, theft legality, or office politics behavior
- implementing E17 theft/justice itself

## Acceptance Criteria

### Tests That Must Pass

1. `LotOperation` no longer contains `Traded`; the transfer lineage operation is `Transferred`.
2. Trade appends `Transferred` provenance to transferred lots.
3. Bribe appends `Transferred` provenance to transferred lots.
4. Trade- and bribe-appended transfer provenance entries carry the committed event id instead of `None`.
5. Trade still emits `Trade` and `Transfer` event tags.
6. Bribe still emits the existing social/transfer event tags and witness semantics.
7. Core provenance enum tests and serialization tests pass after the rename.
8. `cargo test -p worldwake-core`
9. `cargo test -p worldwake-systems`
10. `cargo clippy --workspace --all-targets -- -D warnings`
11. `cargo test --workspace`

### Invariants

1. Lot provenance records concrete lot-lineage facts, not overloaded social-policy interpretations.
2. High-level cause stays event-mediated through event tags and linked event records.
3. Transfer provenance written during action commits must link to the actual committed event record.
4. No dual semantics: after this change, non-trade transfers must not write trade-specific provenance.
5. Conservation and existing transfer behavior remain unchanged.

## Test Plan

### New/Modified Tests

1. [crates/worldwake-core/src/items.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/items.rs) — update enum coverage and serialization tests for `Transferred`.
2. [crates/worldwake-core/src/world_txn.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs) — add a focused test if needed for transaction-owned transfer provenance finalization/event-id linkage.
3. [crates/worldwake-systems/src/trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs) — assert successful trade records `Transferred` provenance with the committed event id while keeping trade event tags.
4. [crates/worldwake-systems/src/office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) — assert bribery records `Transferred` provenance with the committed event id while keeping obligation/perception behavior.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Actual implementation vs. original draft:

- `LotOperation::Traded` was replaced with `LotOperation::Transferred`.
- A canonical `WorldTxn::append_transfer_provenance()` path was added so transfer callers stop constructing provenance entries by hand.
- Transfer provenance written during action commits now links to the committed event id through transaction-owned finalization at commit time.
- Trade and bribe transfers now use the canonical core helper while preserving their existing event tags and behavior.
- Focused tests were strengthened to assert both the generalized provenance operation and committed event linkage.
