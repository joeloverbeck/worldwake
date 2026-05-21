# S158BELVIEWLEAK-001: Economic accessor leak closure

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `PerAgentBeliefView` economic accessors (belief-view read-model)
**Deps**: None

## Problem

Before this ticket, `PerAgentBeliefView`'s economic accessors returned live
authoritative world state for remote, non-co-located item-lots, leaking seller
stock/listing truth
into AI planning and the human CLI action menu (both share `get_affordances()`).
An agent can "know" a remote market delisted or restocked without any perception,
testimony, or record — a direct FND-14 violation that also breaks player/AI
symmetry (FND-19). S158 D1 (economic), proven by S158 D4 economic goldens.

## Assumption Reassessment (2026-05-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The three economic accessors read `world` with no co-location/belief gate,
   confirmed in `crates/worldwake-sim/src/per_agent_belief_view.rs`:
   `has_sale_listing` (line 2125) → `self.world.has_component_sale_listing(lot)`;
   `seller_for_sale_lot` (2113) → reads `world` sale-listing + stock-assignment,
   returns `facility_controller_at`; `listed_sale_lots_at` (2094) → filters
   belief-based `entities_at` but reads `world.has_component_sale_listing` /
   `world.get_component_stock_assignment` per lot. `direct_container` /
   `direct_possessor` (1832–1848) are already correctly gated and are NOT touched.
2. Source authority is `specs/S158-belief-view-remote-truth-leak-closure.md` D1
   (economic bullet) and the Source-Class Rule. Live implementation confirmed
   that inferred remote seller/lot beliefs can still emit generic acquisition
   evidence, but seller-backed remote trade binding had been relying on the
   leaky `has_sale_listing` / `seller_for_sale_lot` accessors. This ticket does
   NOT add a believed-sale-listing surface; it gates these accessors to
   co-located lots only. The merchant-return lifecycle coverage gap exposed by
   removing the leak was later closed by `archive/tickets/S158BELVIEWLEAK-005.md`.
3. Shared boundary under audit: the `MarketBeliefView` / economic accessor surface
   of `PerAgentBeliefView` consumed by `affordance_query.rs` and trade candidate
   generation. The gate predicate is co-location of the lot with the observing
   agent (FND-14A physical observation), mirroring the existing
   `has_authoritative_local_visibility` gate used by `direct_container`.
4. Intended invariant (restate before trusting scenario narrative): a remote
   seller delisting or restocking unseen must NOT change the agent's economic
   candidates, plans, or affordance set until a lawful carrier (local observation,
   testimony, record, or opportunity memory) arrives.
5. Live `GoalKind` under audit: `AcquireCommodity` / `RestockCommodity` trade
   candidates. The exact surface is the economic accessor reads feeding
   `affordance_query.rs` Trade affordance enumeration; reassessment confirmed
   remote trade opportunities route through opportunity memory, not these reads.
6. Intended verification layer: golden E2E in
   `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (decision-trace +
   affordance-fingerprint assertions), full action registries required (trade).
13. Adjacent contradiction: `seller_for_sale_lot` also exposes facility
    controller identity (a social fact). For co-located lots a present controller
    is observable (FND-14A); remote controller identity is gated out with the
    rest. No separate believed-controller surface is introduced here — that is
    consistent with S158's Non-Goals (social/control value-belief-backing
    deferred). Treated as a required consequence of this ticket, not a new bug.

## Architecture Check

1. Gating in place (return empty/`None`/`false` for non-co-located lots) keeps
   the belief view a derived read-model over lawful same-tick physical
   observation, with no new stored state and no `Sourced<T>` machinery (S158
   defers that). Remote acquisition keeps working through the existing
   opportunity-memory path, so no behavior is lost — only unlawful omniscience.
2. No backward-compatibility shim: the leaking `world.*` reads are replaced in
   place behind the co-location gate; no parallel `believed_sale_listing` method
   is added (FND-28).

## Verified Layers

1. Remote delist/restock does not change economic candidates → decision trace
   (`assert_no_*` style on candidate generation in `belief_wall_trap.rs`).
2. Remote delist/restock does not change the affordance set → affordance
   fingerprint (`affordance_fingerprint` helper, line 423).
3. AI and Human control sources see an identical lawful economic affordance set →
   control-source-swap fingerprint (pattern at
   `golden_belief_wall_trap_control_source_swap_preserves_affordances`, line 598).
4. Co-located displayed listing still produces the Trade affordance (negative
   control) → affordance fingerprint on a co-located fixture.

## Implementation Result

### 1. Gated the three economic accessors to co-located lots

In `crates/worldwake-sim/src/per_agent_belief_view.rs`:
- `has_sale_listing` now returns `false` for non-co-located lots.
- `seller_for_sale_lot` now returns `None` for non-co-located lots before
  reading sale-listing or stock-assignment state.
- `listed_sale_lots_at` now returns empty unless the requested place is the
  observing agent's current effective place.
Co-located displayed listings still use the authoritative physical observation
path.

### 2. Added and updated economic proof surfaces

Implemented proof as:
- `per_agent_belief_view::tests::remote_listed_sale_lot_does_not_read_live_sale_listing`
  for the direct accessor boundary.
- `golden_belief_wall_trap_remote_sale_listing_does_not_leak_live_truth` for the
  golden E2E belief-wall suite.
- `merchant_selling::remote_listing_belief_does_not_select_trade_branch_before_local_observation`
  to replace the old remote seller-backed branch expectation with the corrected
  S158 boundary.

Three merchant-return lifecycle goldens were marked ignored with an explicit
S158 reason because their original setup depended on the remote sale-listing leak
to create a seller-backed purchase branch. `archive/tickets/S158BELVIEWLEAK-005.md`
later restored that coverage through a lawful local-observation rebind.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modified same-domain fallout:
  remote seller belief retention no longer implies remote acquisition from live
  sale-listing truth)
- `crates/worldwake-ai/tests/scenarios/merchant_selling.rs` (modified
  same-domain fallout and follow-up handoff)
- `docs/generated/*golden*` (regenerated golden inventory/docs)
- `archive/tickets/S158BELVIEWLEAK-005.md` (completed follow-up)

## Out of Scope

- Production-job, load/capacity, contention accessors (tickets 002, 003).
- `can_control` / `believed_rights` social/control value reads (S158 Non-Goals;
  deferred to a future believed-rights spec).
- Any believed-sale-listing belief surface / `EntityBeliefAspect` addition.
- Doc updates to `planner-contracts.md` / `spec-drafting-rules.md` (ticket 004).

## Acceptance Criteria

### Tests That Passed

1. `per_agent_belief_view::tests::remote_listed_sale_lot_does_not_read_live_sale_listing`
   — remote economic accessors return empty/`None`/`false` while existing
   container/possessor behavior remains unchanged.
2. `golden_belief_wall_trap_remote_sale_listing_does_not_leak_live_truth` —
   known-but-remote sale listing, seller identity, and listing presence do not
   leak through the golden belief-wall path.
3. `merchant_selling::remote_listing_belief_does_not_select_trade_branch_before_local_observation`
   — inferred remote seller/lot belief does not select a seller-backed trade
   branch from current remote sale-listing truth.
4. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. No economic accessor returns a value for a non-co-located lot from current
   world state; remote economic knowledge arrives only via belief/opportunity
   carriers (FND-14).
2. AI and Human control sources produce an identical lawful economic affordance
   set for every scenario (FND-19).

## Verification Result

1. Passed: `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::remote_listed_sale_lot_does_not_read_live_sale_listing -- --exact`
2. Passed: `cargo test -p worldwake-ai --test golden_ai scenarios::belief_wall_trap::golden_belief_wall_trap_remote_sale_listing_does_not_leak_live_truth -- --exact`
3. Passed: `cargo test -p worldwake-ai --lib agent_tick::tests::expired_remote_seller_belief_remains_until_perception_refresh_without_acquisition_leak -- --exact`
4. Passed: `cargo test -p worldwake-ai --lib agent_tick::tests::perception_refresh_preserves_remote_seller_belief_without_acquisition_leak -- --exact`
5. Passed: `cargo test -p worldwake-ai --lib agent_tick::tests::perception_refresh_evicts_remote_seller_belief_below_activation_threshold_without_acquisition_leak -- --exact`
6. Passed: `cargo test -p worldwake-ai --test golden_ai scenarios::merchant_selling::remote_listing_belief_does_not_select_trade_branch_before_local_observation -- --exact`
7. Passed: `python3 scripts/golden_inventory.py --write --check-docs`
8. Passed: `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-05-21.

What changed:
- Closed the remote economic accessor leak in `PerAgentBeliefView`.
- Added a golden belief-wall regression for known-but-remote sale listings.
- Corrected same-domain AI tests that had treated remote seller belief as a live
  sale opportunity.
- Replaced the old merchant-selling remote branch expectation with an active
  S158 no-leak assertion.
- Marked three merchant-return lifecycle goldens ignored with an explicit S158
  reason and created `archive/tickets/S158BELVIEWLEAK-005.md` to restore that
  coverage through a lawful carrier. Outcome amended: 2026-05-21 — the follow-up
  later restored the coverage through a lawful local-observation rebind.

Deviation from the original plan:
- The final proof is not split into separate delist/restock goldens. The landed
  lower-layer and golden tests prove the stronger source-class invariant directly:
  known remote sale-listing truth is not visible through the economic accessors.
- The full CI clippy / `./scripts/verify.sh` gates are left to the harness
  pre-push phase; the ticket iteration proof ran the focused tests, generated
  golden docs check, and `cargo test -p worldwake-ai`.
