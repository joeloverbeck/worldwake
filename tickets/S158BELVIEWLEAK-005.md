# S158BELVIEWLEAK-005: Lawful remote sale opportunity carrier for merchant return goldens

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — buyer-side remote sale opportunity carrier or merchant-selling golden rebind
**Deps**: archive/tickets/S158BELVIEWLEAK-001.md

## Problem

S158BELVIEWLEAK-001 closed the economic belief-view leak that let remote
`SaleListing` truth synthesize seller-backed trade plans. That made three
merchant-selling goldens invalid as active tests:
`merchant_return_revives_pending_purchase_agenda_entry`,
`seller_return_restores_displayed_listing_after_pending_revival`, and
`seller_return_completes_resumed_purchase_after_live_offer_refresh`.

Those scenarios still describe valuable lifecycle seams, but their setup relied
on inferred remote world seeding plus live `has_sale_listing` /
`seller_for_sale_lot` reads instead of a lawful carrier for "this seller was
offering bread there." They need either an explicit buyer-side remote sale
opportunity carrier or a truthful rebind to a local-observation setup that still
proves the pending/revival lifecycle.

## Assumption Reassessment (2026-05-21)

1. `crates/worldwake-ai/tests/scenarios/merchant_selling.rs` now keeps Scenario
   84 active as the S158 replacement proof:
   `remote_listing_belief_does_not_select_trade_branch_before_local_observation`
   asserts that inferred remote seller/lot beliefs do not select a seller-backed
   trade branch from current remote sale-listing truth.
2. The three seller-return lifecycle tests are marked `#[ignore]` with an S158
   reason because their original remote branch setup was no longer lawful after
   `PerAgentBeliefView` economic accessors stopped reading remote live sale
   listings.
3. Shared boundary under audit: buyer-side acquisition candidate evidence for
   remote seller-backed trade plans. The canonical post-S158 path must be an
   explicit belief, record, testimony, opportunity memory, or local observation
   carrier. It must not restore remote `SaleListing` world reads.
4. Intended invariant: seller-return pending/revival behavior remains covered,
   but only after a concrete lawful carrier creates the buyer's seller-backed
   trade expectation.
5. Live `GoalKind` under test: `AcquireCommodity { commodity: Bread, purpose:
   SelfConsume }`; exact operator surface is the selected `Travel -> Trade` or
   local `Trade` plan branch in `merchant_selling.rs`.
13. Adjacent contradiction: S158's parent spec says remote acquisition routes
    through existing opportunity-memory / `DemandObservation`, but the failing
    merchant-return goldens did not exercise such a buyer-side carrier. This
    ticket owns proving or correcting that carrier, not reopening remote
    sale-listing world reads.

## Architecture Check

1. The fix must preserve FND-14/FND-14A by making the remote sale opportunity
   inspectable as belief, testimony, record, learned opportunity, or local
   observation state before AI planning consumes it.
2. No backward-compatibility shim: do not add a fallback that reads current
   remote `SaleListing` state through `PerAgentBeliefView`.

## Verification Layers

1. Lawful carrier admits a seller-backed acquisition branch -> decision trace in
   `merchant_selling.rs`.
2. Seller departure parks the committed purchase into pending -> runtime agenda
   state.
3. Seller return revives or completes the purchase according to the original
   scenario seam -> runtime state, action trace, and authoritative inventory.
4. Remote sale-listing truth remains inaccessible without the carrier -> active
   Scenario 84 and S158BELVIEWLEAK-001 tests.

## What to Change

### 1. Choose the lawful carrier

Reassess whether an existing buyer-side opportunity memory, testimony, record, or
belief path already represents remote seller-backed sale opportunities. If it
does, seed the merchant-return fixtures through that path. If it does not, either
add the smallest explicit carrier justified by S158 or rebind the affected
goldens to local observation and record the remote carrier as out of scope.

### 2. Re-enable merchant-return coverage

Remove the S158 ignore markers from the three merchant-return tests once their
setup is lawful, or replace them with active tests that prove the same lifecycle
seams through the corrected carrier.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/merchant_selling.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify, if a buyer-side
  carrier is already present but not consumed)
- `crates/worldwake-core/src/*` / `crates/worldwake-sim/src/*` (modify only if
  reassessment proves a missing explicit carrier is in scope)

## Out of Scope

- Reopening remote live `SaleListing` reads in `PerAgentBeliefView`.
- Production-job, physical, or contention accessor closure (tickets 002-003).
- Documentation-only source-class contract work (ticket 004).

## Acceptance Criteria

### Tests That Must Pass

1. The three merchant-return tests are active again, or replaced by active tests
   with equal lifecycle coverage and truthful carrier setup.
2. `remote_listing_belief_does_not_select_trade_branch_before_local_observation`
   still passes.
3. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. Seller-backed remote trade planning requires a lawful knowledge carrier.
2. `PerAgentBeliefView` economic accessors still return empty/`None`/`false` for
   remote live sale-listing truth without that carrier.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/merchant_selling.rs` — re-enabled or
   replacement merchant-return lifecycle goldens.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai scenarios::merchant_selling::remote_listing_belief_does_not_select_trade_branch_before_local_observation -- --exact`
2. `cargo test -p worldwake-ai --test golden_ai <re-enabled-or-replacement-merchant-return-test> -- --exact`
3. `cargo test -p worldwake-ai`
