# E11TRAECO-006: Add `Trade(TradeActionPayload)` Variant to `ActionPayload`
**Status**: ✅ COMPLETED

## Summary
Define `TradeActionPayload` and add a `Trade(TradeActionPayload)` variant to the `ActionPayload` enum in `worldwake-sim`. This is the data payload carried by trade negotiation actions.

## Dependencies
- None (ActionPayload already exists with Harvest/Craft variants)

## Files to Touch
- `crates/worldwake-sim/src/action_payload.rs` — add `TradeActionPayload` struct and `Trade` variant to `ActionPayload`
- `crates/worldwake-sim/src/lib.rs` — re-export `TradeActionPayload`
- `crates/worldwake-systems/src/production_actions.rs` — update exhaustive payload matches so the new enum variant remains a handled non-match rather than a compile break

## Assumption Check
- Spec reference correction: this ticket follows `specs/E11-trade-economy.md`, not the non-existent `specs/E11-trade-exchange-merchant-restock.md`.
- Trade schema correction: `worldwake-core/src/trade.rs` already exists from E11TRAECO-002 through E11TRAECO-005, so this ticket only adds the simulation-layer action payload, not new trade-domain core schema.
- Blast-radius correction: the change is not truly single-file because `ActionPayload` is exhaustively matched in `crates/worldwake-systems/src/production_actions.rs`, and `worldwake-sim/src/lib.rs` is the crate re-export surface used by downstream crates.
- Test-surface correction: current `worldwake-sim` coverage only proves `Harvest` and `Craft` payload roundtrips. The new variant must extend that coverage and confirm existing payload extractors still reject non-matching variants cleanly.

## Out of Scope
- Trade action definition registration (E11TRAECO-007)
- Trade action handler (E11TRAECO-008)
- Valuation logic (E11TRAECO-007)
- Component changes in worldwake-core
- BeliefView changes

## Implementation Details

```rust
use worldwake_core::{CommodityKind, EntityId, Quantity};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradeActionPayload {
    pub counterparty: EntityId,
    pub offered_commodity: CommodityKind,
    pub offered_quantity: Quantity,
    pub requested_commodity: CommodityKind,
    pub requested_quantity: Quantity,
}
```

Add `Trade(TradeActionPayload)` to `ActionPayload` enum after `Craft`.

Update existing tests:
- `action_payload_satisfies_required_traits` must also cover `TradeActionPayload`
- Add bincode roundtrip test for `ActionPayload::Trade`
- Add coverage that existing harvest/craft payload extractors continue to reject unrelated payload variants after `Trade` is introduced

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-sim` — all existing tests pass
- `cargo test -p worldwake-systems production_actions` — payload extractor tests still pass with the new enum variant present
- New test: `TradeActionPayload` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned`
- New test: `ActionPayload::Trade(...)` bincode roundtrip
- Existing `ActionPayload::None`, `Harvest`, `Craft` tests still pass
- Existing harvest/craft payload extractors reject `ActionPayload::Trade` with the same internal-error contract used for other non-matching payloads

### Invariants That Must Remain True
- `ActionPayload` default remains `None`
- All existing variants unchanged
- No aliasing or compatibility shims are added; trade gets its own first-class payload variant
- `cargo clippy --workspace` clean

## Outcome
- Outcome amended: 2026-03-11
- Completion date: 2026-03-11
- What actually changed:
  - Added `TradeActionPayload` and `ActionPayload::Trade` in `crates/worldwake-sim/src/action_payload.rs`
  - Added typed payload accessors on `ActionPayload` (`as_harvest`, `as_craft`, `as_trade`) so downstream modules no longer open-code enum discrimination
  - Re-exported `TradeActionPayload` from `crates/worldwake-sim/src/lib.rs`
  - Updated `crates/worldwake-systems/src/production_actions.rs` to use `ActionPayload` accessors, so future payload variants do not require unrelated extractor matches to be edited
  - Corrected the ticket's spec reference and scope assumptions before implementation
  - Added test coverage for trade payload trait/roundtrip behavior, typed accessor behavior, and existing extractor rejection behavior
- Deviations from original plan:
  - The original ticket claimed a single-file change with no downstream impact; the real change required touching the sim re-export surface and production payload matches
  - The original acceptance criteria did not cover the enum-match blast radius in `production_actions`; that coverage was added
- Verification results:
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-systems production_actions -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
