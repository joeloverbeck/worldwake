# S04MERSELMAR-005: Listing cleanup in trade system tick

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — extend trade system tick with listing pruning
**Deps**: S04MERSELMAR-001

## Problem

`SaleListing` components must be pruned when they become invalid due to conditions outside the `staff_market` action lifecycle: the seller dies, leaves the place, loses possession of the lot, or removes the commodity from their `MerchandiseProfile.sale_kinds`. Without system-level cleanup, stale listings would persist and mislead buyers.

## Assumption Reassessment (2026-03-31)

1. `trade_system_tick()` in `crates/worldwake-systems/src/trade.rs` runs each tick as part of the Trade system slot. Currently ages `DemandMemory` entries. Confirmed via grep — `trade_system_tick` is exported and wired in `system_dispatch.rs`.
2. The tick execution order is `Needs -> Production -> Trade -> Combat -> FacilityQueue -> Politics -> Perception`. Trade system tick runs before Perception, so listing cleanup is visible to perception in the same tick. Confirmed in CLAUDE.md.
3. `SaleListing` (ticket 001) is on `ItemLot` entities. To check validity, the system needs: lot existence, lot possession, possessor alive/capable status, possessor effective place, lot commodity, possessor's `MerchandiseProfile.sale_kinds`.
4. All these checks are available through `World` state accessors (relations for possession, component queries for profiles, entity metadata for alive/dead).
5. No adjacent contradictions found.

## Architecture Check

1. Adding listing pruning to `trade_system_tick` follows the existing pattern of per-tick state maintenance (demand memory aging already lives there). Listing validity is trade-domain state, so the trade system is the natural owner.
2. No backwards-compatibility shims. This is additive to the trade system tick.
3. State-mediated cleanup: no cross-system calls needed. The trade system reads authoritative state (possession, place, alive status) written by other systems.

## Verification Layers

1. Dead seller listings pruned -> focused unit test (kill seller, verify SaleListing removed next tick)
2. Departed seller listings pruned -> focused unit test (move seller away, verify removal)
3. Lost-possession listings pruned -> focused unit test (transfer lot, verify removal)
4. Sale_kinds removal pruned -> focused unit test (remove commodity from profile, verify removal)
5. Valid listings preserved -> focused unit test (valid seller stays, listing persists)

## What to Change

### 1. Add `prune_invalid_listings()` in `trade.rs`

Iterate all entities with `SaleListing` component. For each, check:
- lot still exists (entity not archived)
- lot has a direct possessor
- possessor is alive (not dead/incapacitated)
- possessor is at the same effective place as the lot
- lot commodity is in possessor's `MerchandiseProfile.sale_kinds`
- possessor has `MerchandiseProfile`

Remove `SaleListing` from any lot that fails any check.

### 2. Call `prune_invalid_listings()` from `trade_system_tick()`

Add the call after demand memory aging but before the function returns.

## Files to Touch

- `crates/worldwake-systems/src/trade.rs` (modify — add pruning function and call from tick)

## Out of Scope

- Normal listing removal during `staff_market` commit/abort (ticket 003)
- Trade commit validation (ticket 009)
- AI-level listing visibility (ticket 004)
- Any listing creation logic (ticket 003)

## Acceptance Criteria

### Tests That Must Pass

1. Listings on lots possessed by dead sellers are removed within one tick
2. Listings on lots whose possessor left the place are removed within one tick
3. Listings on lots no longer directly possessed are removed within one tick
4. Listings on lots whose commodity was removed from `sale_kinds` are removed within one tick
5. Listings on lots with valid, co-located, alive sellers with correct `sale_kinds` persist
6. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. No stale listing survives more than one tick after the invalidating condition
2. Pruning is purely state-mediated — no cross-system calls
3. Pruning does not create or modify any entities — only removes `SaleListing` components

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/trade.rs` — focused tests for each pruning condition (dead, departed, unpossessed, wrong sale_kinds)
2. `crates/worldwake-systems/src/trade.rs` — focused test that valid listings are preserved

### Commands

1. `cargo test -p worldwake-systems -- trade`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- **Completion date**: 2026-04-01
- **What changed**: Added `is_listing_valid()` and `prune_invalid_listings()` in `crates/worldwake-systems/src/trade.rs`. Called from `trade_system_tick()` before demand memory aging. Added 5 focused tests covering dead seller, departed seller, unpossessed lot, removed sale_kinds, and valid listing preservation.
- **Deviations**: `prune_invalid_listings()` returns `()` instead of `Result<(), SystemError>` since it cannot fail (clippy pedantic). No other deviations.
- **Verification**: `cargo clippy --workspace --all-targets -- -D warnings` clean. `cargo test --workspace` all pass (0 failures).
