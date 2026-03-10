# E11TRAECO-001: Add `LotOperation::Traded` Variant

## Summary
Add a `Traded` variant to the `LotOperation` enum in `worldwake-core/src/items.rs` so that lots transferred through trade carry provenance tracking.

## Files to Touch
- `crates/worldwake-core/src/items.rs` — add `Traded` variant to `LotOperation` enum, update `ALL` array

## Out of Scope
- Trade action handler logic (E11TRAECO-008)
- Trade system tick (E11TRAECO-009)
- Any other changes to `items.rs` besides `LotOperation`
- Component registration changes
- `ActionPayload` changes

## Implementation Details
1. Add `Traded` to `LotOperation` enum after `Transformed`.
2. Update `LotOperation::ALL` array to include `Traded` and adjust the const length from 8 to 9.

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-core` — all existing tests pass (no regressions)
- Existing `LotOperation` bincode roundtrip tests still pass
- `LotOperation::ALL` length equals 9

### Invariants That Must Remain True
- `LotOperation` derives remain: `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`
- `LotOperation::ALL` is exhaustive (contains every variant)
- No existing variant is removed or renamed
