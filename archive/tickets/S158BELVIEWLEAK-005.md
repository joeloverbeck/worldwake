# S158BELVIEWLEAK-005: Merchant return local-observation lifecycle rebind

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planning-path pending-repair preservation plus merchant-selling golden rebind
**Deps**: archive/tickets/S158BELVIEWLEAK-001.md

## Problem

S158BELVIEWLEAK-001 closed the economic belief-view leak that let remote
`SaleListing` truth synthesize seller-backed trade plans. That made three
merchant-selling goldens invalid as active tests:
`merchant_return_revives_pending_purchase_agenda_entry`,
`seller_return_restores_displayed_listing_after_pending_revival`, and
`seller_return_completes_resumed_purchase_after_live_offer_refresh`.

Those scenarios still described valuable lifecycle seams, but their S158 ignore
reason over-described the problem as missing remote sale-opportunity substrate.
Live reassessment showed their setup could remain lawful as local observation
coverage once the fixture waited for an adopted runtime trade plan before moving
the seller away.

## Assumption Reassessment (2026-05-21)

1. `crates/worldwake-ai/tests/scenarios/merchant_selling.rs` now keeps Scenario
   84 active as the S158 replacement proof:
   `remote_listing_belief_does_not_select_trade_branch_before_local_observation`
   asserts that inferred remote seller/lot beliefs do not select a seller-backed
   trade branch from current remote sale-listing truth.
2. Before this ticket, the three seller-return lifecycle tests were marked
   `#[ignore]` with an S158 reason because their original remote branch setup
   was no longer lawful after `PerAgentBeliefView` economic accessors stopped
   reading remote live sale listings.
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
14. Live correction: the three ignored merchant-return tests already seed the
    buyer at `VILLAGE_SQUARE` with the listed lot and seller, so the lawful
    carrier for this lifecycle proof is same-tick local physical observation,
    not a new remote sale-listing belief surface.
15. Runtime contradiction found during TDD: the tests moved the seller after a
    selected planning trace but before the selected trade plan was adopted into
    `AgentDecisionRuntime.current_plan`. Once the fixture waited for a real
    runtime trade plan, `clear_current_plan` still needed to preserve that plan
    as `PendingRepairContext` before parking the committed goal.
16. Corrected boundary: this ticket restores the merchant-return lifecycle
    goldens through local observation and fixes the planning-path clear seam.
    A remote buyer-side sale-opportunity carrier remains out of scope and was
    not introduced.

## Architecture Check

1. The fix must preserve FND-14/FND-14A by making the remote sale opportunity
   inspectable as belief, testimony, record, learned opportunity, or local
   observation state before AI planning consumes it.
2. No backward-compatibility shim: do not add a fallback that reads current
   remote `SaleListing` state through `PerAgentBeliefView`.

## Verified Layers

1. Lawful carrier admits a seller-backed acquisition branch -> decision trace in
   `merchant_selling.rs`.
2. Seller departure parks the committed purchase into pending -> runtime agenda
   state.
3. Seller return revives or completes the purchase according to the original
   scenario seam -> runtime state, action trace, and authoritative inventory.
4. Remote sale-listing truth remains inaccessible without the carrier -> active
   Scenario 84 and S158BELVIEWLEAK-001 tests.

## Implementation Result

### 1. Rebound the lawful carrier

The three merchant-return goldens now use local observation as the lawful carrier:
the buyer starts co-located with the displayed bread lot and seller, and the
fixture waits until the selected trade branch is adopted into runtime state
before moving the seller away.

No remote `SaleListing` read, believed-sale-listing surface, testimony record, or
new opportunity-memory carrier was added.

### 2. Re-enabled merchant-return coverage

Removed the S158 ignore markers from:
- `merchant_return_revives_pending_purchase_agenda_entry`
- `seller_return_restores_displayed_listing_after_pending_revival`
- `seller_return_completes_resumed_purchase_after_live_offer_refresh`

Added `runtime_has_trade_plan()` so all three fixtures prove seller departure
after a real runtime `Trade` plan exists, not merely after a planning trace shows
a candidate branch.

### 3. Fixed planning-path pending repair preservation

`clear_current_plan()` now snapshots the existing `current_plan` into
`PendingRepairContext` when none exists yet. This preserves a failed trade plan
for later counterparty-triggered revival even when the plan is cleared by the
planning path before an active action failure records repair context.

## Landed Files

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modified)
- `crates/worldwake-ai/tests/scenarios/merchant_selling.rs` (modified)
- `docs/generated/golden-scenario-details/merchant-selling.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated source-line updates)

## Out of Scope

- Reopening remote live `SaleListing` reads in `PerAgentBeliefView`.
- Production-job, physical, or contention accessor closure (tickets 002-003).
- Documentation-only source-class contract work (ticket 004).

## Acceptance Result

### Tests That Passed

1. The three merchant-return tests are active again with truthful local
   observation setup.
2. `remote_listing_belief_does_not_select_trade_branch_before_local_observation`
   still passes.
3. Existing suite passed: `cargo test -p worldwake-ai --quiet`.

### Invariants

1. Seller-backed remote trade planning requires a lawful knowledge carrier.
2. `PerAgentBeliefView` economic accessors still return empty/`None`/`false` for
   remote live sale-listing truth without that carrier.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` —
   `clear_current_plan_snapshots_current_trade_plan_for_pending_repair`.
2. `crates/worldwake-ai/tests/scenarios/merchant_selling.rs` — re-enabled
   merchant-return lifecycle goldens and tightened their setup to require an
   adopted runtime trade plan.

### Commands That Passed

1. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::clear_current_plan_snapshots_current_trade_plan_for_pending_repair -- --exact`
2. `cargo test -p worldwake-ai --test golden_ai scenarios::merchant_selling::merchant_return_revives_pending_purchase_agenda_entry -- --exact`
3. `cargo test -p worldwake-ai --test golden_ai scenarios::merchant_selling::seller_return_restores_displayed_listing_after_pending_revival -- --exact`
4. `cargo test -p worldwake-ai --test golden_ai scenarios::merchant_selling::seller_return_completes_resumed_purchase_after_live_offer_refresh -- --exact`
5. `cargo test -p worldwake-ai --test golden_ai scenarios::merchant_selling::remote_listing_belief_does_not_select_trade_branch_before_local_observation -- --exact`
6. `cargo test -p worldwake-ai --quiet`
7. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

Completed on 2026-05-21.

What changed:
- Restored the three merchant-return lifecycle goldens as active tests.
- Rebound their setup to a lawful local-observation seam: the buyer observes the
  seller/listed lot locally before seller departure.
- Fixed the planning-path clear seam so an adopted trade plan is retained as
  pending repair context before being parked.
- Regenerated golden docs after the ignored tests became active.

Deviation from the original plan:
- No buyer-side remote sale-opportunity carrier was added. The covered lifecycle
  did not need one once the fixture was corrected to wait for a runtime trade
  plan; remote live sale-listing truth remains inaccessible without a lawful
  carrier.

## Verification Result

1. Passed: `cargo test -p worldwake-ai --lib agent_tick::planning::tests::clear_current_plan_snapshots_current_trade_plan_for_pending_repair -- --exact`
2. Passed: `cargo test -p worldwake-ai --test golden_ai scenarios::merchant_selling::merchant_return_revives_pending_purchase_agenda_entry -- --exact`
3. Passed: `cargo test -p worldwake-ai --test golden_ai scenarios::merchant_selling::seller_return_restores_displayed_listing_after_pending_revival -- --exact`
4. Passed: `cargo test -p worldwake-ai --test golden_ai scenarios::merchant_selling::seller_return_completes_resumed_purchase_after_live_offer_refresh -- --exact`
5. Passed: `cargo test -p worldwake-ai --test golden_ai scenarios::merchant_selling::remote_listing_belief_does_not_select_trade_branch_before_local_observation -- --exact`
6. Passed: `cargo test -p worldwake-ai --quiet`
7. Passed: `python3 scripts/golden_inventory.py --write --check-docs`
