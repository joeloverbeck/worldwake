# E11TRAECO-001: Add `LotOperation::Traded` Variant

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — item provenance taxonomy extension in `worldwake-core`
**Deps**: None

## Problem

E11 trade provenance needs a first-class lot lineage operation for ownership transfers caused by trade. Without an explicit `LotOperation::Traded`, later trade handling would either overload another operation or encode trade provenance indirectly, which would make lineage queries less precise and weaken the append-only provenance model.

## Assumption Reassessment (2026-03-11)

1. `LotOperation` currently lives in [crates/worldwake-core/src/items.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/items.rs) with 8 variants and a matching `ALL` array of length 8 — confirmed.
2. This repository already has explicit `LotOperation` coverage in the same file under `items::tests`, not just implicit workspace coverage — confirmed:
   - `lot_operation_all_is_canonical_variant_list`
   - `lot_operation_variants_roundtrip_through_bincode`
   - `lot_operation_ordering_is_deterministic`
   - `lot_operation_trait_bounds`
3. The original ticket scope was too narrow. Adding a new enum variant without updating those explicit tests would leave the ticket incomplete even if the code compiled.
4. There is currently no trade implementation yet in non-ticket code that appends provenance on ownership transfer — confirmed. That behavior belongs to downstream trade-handler work in E11TRAECO-008, not this ticket.
5. The current architecture does not yet provide a generic "ownership transfer with provenance" helper. That is acceptable for now; forcing such an abstraction in this ticket would be premature because trade is the first consumer and the correct shared boundary should be decided with the actual transfer workflow in hand.

## Architecture Check

1. Adding a dedicated `Traded` variant is better than the current architecture because trade provenance becomes concrete state rather than an overloaded interpretation of `Produced`, `Transformed`, or raw relation changes.
2. Appending the variant at the end preserves the existing declaration order and keeps the taxonomy easy to extend without aliasing or compatibility shims.
3. This ticket should stay narrow. Introducing a provenance helper or speculative transfer abstraction now would couple `worldwake-core` to trade behavior before the actual handler semantics are implemented.
4. A likely future improvement, once E11TRAECO-008 is implemented, is a small world-level helper for relation transfer plus provenance append if that logic would otherwise duplicate across trade and other ownership-transfer actions. That should be evaluated then, not guessed here.

## What to Change

### 1. Extend `LotOperation`

In [crates/worldwake-core/src/items.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/items.rs):

- add `Traded` after `Transformed`
- update `LotOperation::ALL` from length 8 to length 9

### 2. Update and strengthen the explicit `LotOperation` tests

In the existing `items.rs` test module:

- update the canonical variant list test to include `Traded`
- keep the roundtrip coverage exhaustive through `LotOperation::ALL`
- keep deterministic ordering coverage valid with the new terminal variant
- add one direct provenance roundtrip example using `LotOperation::Traded` so the ticket verifies the intended provenance operation, not just the enum in isolation

## Files to Touch

- [crates/worldwake-core/src/items.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/items.rs) (modify — enum, `ALL`, focused tests)

## Out of Scope

- Do NOT implement trade action handling.
- Do NOT append provenance during ownership transfer yet.
- Do NOT introduce generic transfer/provenance helpers in `worldwake-core` in this ticket.
- Do NOT modify `ActionPayload`, trade systems, or component registration.
- Do NOT change any existing `LotOperation` name or semantics besides adding `Traded`.

## Acceptance Criteria

### Tests That Must Pass

1. `LotOperation` includes `Traded` as a distinct variant.
2. `LotOperation::ALL` is exhaustive and has length 9.
3. `items::tests::lot_operation_all_is_canonical_variant_list` includes `Traded`.
4. `items::tests::lot_operation_variants_roundtrip_through_bincode` still passes exhaustively through `LotOperation::ALL`.
5. `items::tests::lot_operation_ordering_is_deterministic` still passes with the new variant.
6. A provenance roundtrip test covers a `ProvenanceEntry` using `LotOperation::Traded`.
7. `cargo test -p worldwake-core`
8. `cargo test --workspace`
9. `cargo clippy --workspace`

### Invariants

1. `LotOperation` derives remain unchanged.
2. No existing variant is removed, renamed, or reordered before `Traded`.
3. No compatibility aliases or deprecated paths are introduced.
4. `worldwake-core` remains trade-agnostic: this ticket only extends the provenance taxonomy.

## Test Plan

### New/Modified Tests

1. `items::tests::lot_operation_all_is_canonical_variant_list` — modified to require `Traded` in the canonical order.
2. `items::tests::lot_operation_variants_roundtrip_through_bincode` — modified indirectly by expanding `LotOperation::ALL` so exhaustive enum coverage includes `Traded`.
3. `items::tests::lot_operation_ordering_is_deterministic` — modified indirectly by expanding `LotOperation::ALL`.
4. `items::tests::provenance_entry_roundtrips_through_bincode` — modified so one direct example uses `LotOperation::Traded`.

### Commands

1. `cargo test -p worldwake-core lot_operation`
2. `cargo test -p worldwake-core provenance_entry_roundtrips_through_bincode`
3. `cargo test -p worldwake-core`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added `LotOperation::Traded` to [crates/worldwake-core/src/items.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/items.rs).
  - Expanded `LotOperation::ALL` from 8 to 9 entries.
  - Updated the canonical `LotOperation` test list to include `Traded`.
  - Kept exhaustive enum roundtrip and deterministic-order coverage via the existing `LotOperation::ALL`-driven tests.
  - Updated `provenance_entry_roundtrips_through_bincode` so one direct example uses `LotOperation::Traded`.
- Deviations from original plan:
  - The ticket was corrected first because the original scope understated the real code surface. This repo already had explicit `LotOperation` tests in `items.rs`, so the ticket now treats those test updates as part of the required work.
  - No generic transfer/provenance helper was introduced. That remains a better decision for the later trade-handler ticket, where the real ownership-transfer workflow will exist.
- Verification results:
  - `cargo test -p worldwake-core lot_operation` passed.
  - `cargo test -p worldwake-core provenance_entry_roundtrips_through_bincode` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
