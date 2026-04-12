# S99: Commodity Validation Helper Extraction

## Summary

Extract duplicated commodity validation helpers (`ensure_accessible_quantity` and `resolve_controlled_lots`) from four action handler modules in `worldwake-systems` into a shared `pub(crate)` module. Eliminates 4 copies of quantity-check logic and 3 copies of lot-resolution logic while preserving identical semantics. No behavioral changes, no new components, no cross-crate modifications.

## Phase and Status

- **Phase**: 7 (Adjunct — Architectural Debt Remediation)
- **Status**: COMPLETED

## Crates

- `worldwake-systems` (sole crate affected)

## Dependencies

- None. All referenced types (`World`, `WorldTxn`, `EntityId`, `CommodityKind`, `Quantity`) already exist in `worldwake-core`; `ActionError` in `worldwake-sim`.

## Design Goals

1. **DRY**: Eliminate maintenance risk from 4 identical `ensure_accessible_quantity` implementations and 3 identical `resolve_controlled_lots` implementations that could diverge silently.
2. **Preserve crate boundary**: The extraction is `pub(crate)` within `worldwake-systems` — no new public API surface, no cross-crate changes.
3. **Zero behavioral change**: Extracted functions have identical semantics to the originals.

## Non-Goals

- Changing the commodity validation logic itself.
- Exposing these helpers outside `worldwake-systems`.
- Refactoring `WorldTxn` or `World` APIs.
- Addressing the "Needs Investigation" signals from the architectural debt analysis (AI/Core coupling, `apply_planner_step` dispatch, TODO density).

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P4 (Persistent Identity and Explicit Transfer) | Aligned — consolidation reduces risk of one copy diverging from conservation semantics. No change to transfer logic. |
| P26 (Systems Interact Through State, Not Through Each Other) | Aligned — the shared module reads/mutates authoritative state through `World`/`WorldTxn`, not through sibling action modules. No cross-system coupling introduced. |
| P28 (No Backward Compatibility) | Aligned — the private functions are removed, not deprecated or aliased. |

## Deliverables

### D1: New module `crates/worldwake-systems/src/commodity_support.rs`

Declared as `mod commodity_support;` (not `pub mod`) in `lib.rs`, following the existing pattern of `contention_support`, `evidence_support`, and `inventory`.

Contains two `pub(crate)` functions:

```rust
/// Checks that `holder` controls at least `quantity` of `commodity`.
/// Returns `ActionError::AbortRequested(HolderLacksAccessibleCommodity)` on failure.
pub(crate) fn ensure_accessible_quantity(
    world: &World,
    holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> Result<(), ActionError>
```

Takes `&World` as the first parameter to avoid unnecessary coupling to `WorldTxn` for a read-only check. Three of four existing copies already take `&WorldTxn<'_>`, but since `WorldTxn` implements `Deref<Target=World>`, Rust auto-coerces `&WorldTxn` to `&World` at call sites — no adapter needed.

```rust
/// Resolves controlled lots of `commodity` at `place` owned by `holder`,
/// splitting lots as needed to yield exactly `quantity`. Returns the
/// selected `(lot_id, quantity)` pairs. `context` is used in the
/// internal-error message if lot accounting underflows.
pub(crate) fn resolve_controlled_lots(
    txn: &mut WorldTxn<'_>,
    holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
    place: EntityId,
    context: &str,
) -> Result<Vec<(EntityId, Quantity)>, ActionError>
```

Takes `&mut WorldTxn` because it performs lot splitting via `txn.split_lot()`. The `context: &str` parameter replaces the hardcoded error message strings that currently differ between callers ("bounty reward accounting underflowed" in artifact_actions vs. "controlled lot accounting underflowed" in justice/office_actions).

### D2: Migrate callers

Remove private `ensure_accessible_quantity` from:
- `artifact_actions.rs` (line ~760)
- `justice_actions.rs` (line ~661)
- `trade_actions.rs` (line ~1239)
- `office_actions.rs` (line ~1138)

Replace with `crate::commodity_support::ensure_accessible_quantity(...)`.

Remove private `resolve_controlled_lots` from:
- `artifact_actions.rs` (line ~779)
- `justice_actions.rs` (line ~680)
- `office_actions.rs` (line ~1157)

Replace with `crate::commodity_support::resolve_controlled_lots(...)`, passing the appropriate `context` string at each call site:
- artifact_actions: `"bounty reward accounting underflowed"`
- justice_actions: `"controlled lot accounting underflowed"`
- office_actions: `"controlled lot accounting underflowed"`

### D3: Module declaration

Add `mod commodity_support;` to `crates/worldwake-systems/src/lib.rs` in alphabetical position (between `combat` and `consult_record_actions`).

## FND-01 Section H

Not applicable — this spec introduces no new systems, actions, entities, or state. It is a pure code-organization refactor within a single crate.

## SystemFn Integration

Not applicable — no new system functions.

## Component Registration

Not applicable — no new components.

## Cross-System Interactions

None. The extracted functions are internal helpers within `worldwake-systems` that read/write authoritative state through `World`/`WorldTxn`. No cross-system coupling is introduced or changed.

## Profile-Driven Parameters

Not applicable — no agent-configurable parameters.

## Verification

1. `cargo clippy --workspace --all-targets -- -D warnings` — clean
2. `cargo test --workspace` — all existing tests pass unchanged
3. `cargo test -p worldwake-ai` — all golden tests pass (commodity transfers exercised by conservation enforcement in `golden_resilience.rs`)
4. Grep `worldwake-systems/src` for `fn ensure_accessible_quantity` and `fn resolve_controlled_lots` — each appears exactly once (in `commodity_support.rs`)

## Outcome

Completed on 2026-04-12.

- Added `crates/worldwake-systems/src/commodity_support.rs` with shared `pub(crate)` implementations of `ensure_accessible_quantity` and `resolve_controlled_lots`.
- Declared `mod commodity_support;` in `crates/worldwake-systems/src/lib.rs` without widening crate visibility.
- Replaced the duplicate private helper definitions in `artifact_actions.rs`, `justice_actions.rs`, `trade_actions.rs`, and `office_actions.rs` with shared-helper imports and preserved caller-specific underflow messages through the new `context` parameter on `resolve_controlled_lots`.
- Verified the landed refactor with `cargo test -p worldwake-systems`, `cargo test -p worldwake-ai`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
