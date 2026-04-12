# S99COMVALHEL-001: Extract commodity validation helpers into shared module

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

Four identical private copies of `ensure_accessible_quantity` and three identical private copies of `resolve_controlled_lots` exist across `artifact_actions.rs`, `justice_actions.rs`, `trade_actions.rs`, and `office_actions.rs` in `worldwake-systems`. Silent divergence between copies is a maintenance risk — a fix to one copy may not propagate to the others.

## Assumption Reassessment (2026-04-12)

1. **Symbol existence and counts**: `fn ensure_accessible_quantity` exists as a private function in `artifact_actions.rs:760`, `justice_actions.rs:661`, `trade_actions.rs:1239`, `office_actions.rs:1138` (4 copies). `fn resolve_controlled_lots` exists as a private function in `artifact_actions.rs:779`, `justice_actions.rs:680`, `office_actions.rs:1157` (3 copies). `trade_actions.rs` does not have `resolve_controlled_lots`. All confirmed via `grep -c` during reassessment.
2. **Signature equivalence**: All 4 copies of `ensure_accessible_quantity` have identical bodies. Three take `txn: &WorldTxn<'_>`, one (`justice_actions.rs`) takes `world: &World`. The proposed shared signature uses `&World` — valid because `WorldTxn` implements `Deref<Target=World>` (`world_txn.rs:1925`). All 3 copies of `resolve_controlled_lots` have identical signatures taking `txn: &mut WorldTxn<'_>`. The only body difference is the error message string: `"bounty reward accounting underflowed"` in `artifact_actions.rs` vs. `"controlled lot accounting underflowed"` in `justice_actions.rs` and `office_actions.rs`.
3. **Shared boundary**: The extraction is `pub(crate)` within `worldwake-systems` — no cross-crate API changes, no crate boundary under audit. The lot-splitting method is `WorldTxn::split_lot()` (`world_txn.rs:505`), not `split_item_lot`. `ActionError` is defined in `worldwake-sim/src/action_handler.rs:280`.

## Architecture Check

1. A single `pub(crate)` module follows the existing `contention_support` / `evidence_support` / `inventory` pattern in `worldwake-systems` — internal shared helpers without public API surface. This is cleaner than 4/3 independent copies because a single definition is the canonical fix site.
2. No backwards-compatibility aliasing — the private functions are deleted, not deprecated or re-exported.

## Verification Layers

1. Semantic equivalence of extracted functions → existing workspace tests (no behavioral change, identical logic)
2. No stale private copies remain → post-implementation grep for `fn ensure_accessible_quantity` and `fn resolve_controlled_lots` across `worldwake-systems/src` must show exactly 1 match each (in `commodity_support.rs`)
3. Single-layer ticket — no cross-system or mixed-layer invariants apply. This is a pure code-organization refactor.

## What to Change

### 1. Create `crates/worldwake-systems/src/commodity_support.rs`

Add two `pub(crate)` functions:

```rust
pub(crate) fn ensure_accessible_quantity(
    world: &World,
    holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> Result<(), ActionError>
```

Body identical to any existing copy (they are all equivalent). Uses `&World` to avoid unnecessary coupling to `WorldTxn` for a read-only check.

```rust
pub(crate) fn resolve_controlled_lots(
    txn: &mut WorldTxn<'_>,
    holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
    place: EntityId,
    context: &str,
) -> Result<Vec<(EntityId, Quantity)>, ActionError>
```

Body identical to existing copies except the hardcoded error message string is replaced with the `context` parameter.

### 2. Declare module in `lib.rs`

Add `mod commodity_support;` in alphabetical position between `combat` and `consult_record_actions`.

### 3. Migrate callers and delete originals

In each file, replace the private function with an import from `crate::commodity_support`:

- **`artifact_actions.rs`**: Delete private `ensure_accessible_quantity` (~line 760) and `resolve_controlled_lots` (~line 779). Replace call sites with `crate::commodity_support::ensure_accessible_quantity(...)` and `crate::commodity_support::resolve_controlled_lots(..., "bounty reward accounting underflowed")`.
- **`justice_actions.rs`**: Delete private `ensure_accessible_quantity` (~line 661) and `resolve_controlled_lots` (~line 680). Replace call sites with shared versions, passing `"controlled lot accounting underflowed"` as context.
- **`trade_actions.rs`**: Delete private `ensure_accessible_quantity` (~line 1239). Replace call site with shared version. No `resolve_controlled_lots` to migrate.
- **`office_actions.rs`**: Delete private `ensure_accessible_quantity` (~line 1138) and `resolve_controlled_lots` (~line 1157). Replace call sites with shared versions, passing `"controlled lot accounting underflowed"` as context.

## Files to Touch

- `crates/worldwake-systems/src/commodity_support.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add `mod commodity_support;`)
- `crates/worldwake-systems/src/artifact_actions.rs` (modify — delete 2 private fns, update call sites)
- `crates/worldwake-systems/src/justice_actions.rs` (modify — delete 2 private fns, update call sites)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — delete 1 private fn, update call site)
- `crates/worldwake-systems/src/office_actions.rs` (modify — delete 2 private fns, update call sites)

## Out of Scope

- Changing the commodity validation logic itself
- Exposing these helpers outside `worldwake-systems`
- Refactoring `WorldTxn` or `World` APIs
- Addressing other architectural debt signals (AI/Core coupling, `apply_planner_step` dispatch, TODO density)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — all existing tests pass unchanged (zero behavioral change)
2. `cargo test -p worldwake-ai` — all golden tests pass (commodity transfers exercised by conservation enforcement)
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean

### Invariants

1. Each of `ensure_accessible_quantity` and `resolve_controlled_lots` has exactly one definition in `worldwake-systems/src` (in `commodity_support.rs`)
2. No `pub` or `pub(super)` visibility on the new module — only `mod commodity_support;` (not `pub mod`)
3. Conservation semantics unchanged — identical logic, identical error variants

## Test Plan

### New/Modified Tests

1. None — pure structural refactor; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-systems` — targeted crate tests
2. `cargo test -p worldwake-ai` — golden tests exercising commodity transfers
3. `cargo clippy --workspace --all-targets -- -D warnings` — lint clean
4. `cargo test --workspace` — full suite

## Outcome

Completed on 2026-04-12.

- Added `crates/worldwake-systems/src/commodity_support.rs` with shared `pub(crate)` implementations of `ensure_accessible_quantity` and `resolve_controlled_lots`.
- Declared `mod commodity_support;` in `crates/worldwake-systems/src/lib.rs` without widening crate visibility.
- Replaced the duplicate private helper definitions in `artifact_actions.rs`, `justice_actions.rs`, `trade_actions.rs`, and `office_actions.rs` with shared-helper imports.
- Preserved the existing underflow context strings at each `resolve_controlled_lots` call site by passing the caller-specific message into the shared helper.
- Verified post-implementation that `fn ensure_accessible_quantity` and `fn resolve_controlled_lots` each now have exactly one definition in `worldwake-systems/src` (in `commodity_support.rs`).

## Verification Result

- Passed `cargo test -p worldwake-systems`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
