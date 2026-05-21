# S158BELVIEWLEAK-001: Economic accessor leak closure

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `PerAgentBeliefView` economic accessors (belief-view read-model)
**Deps**: None

## Problem

`PerAgentBeliefView`'s economic accessors return current authoritative world
state for remote, non-co-located item-lots, leaking seller stock/listing truth
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
   (economic bullet) and the Source-Class Rule. Remote "where can I buy X"
   candidates already flow through the opportunity-memory / `DemandObservation`
   pathway in `crates/worldwake-ai/src/candidate_generation.rs` — this ticket does
   NOT add a believed-sale-listing surface; it gates these accessors to co-located
   lots only and relies on the existing pathway for remote acquisition.
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

## Verification Layers

1. Remote delist/restock does not change economic candidates → decision trace
   (`assert_no_*` style on candidate generation in `belief_wall_trap.rs`).
2. Remote delist/restock does not change the affordance set → affordance
   fingerprint (`affordance_fingerprint` helper, line 423).
3. AI and Human control sources see an identical lawful economic affordance set →
   control-source-swap fingerprint (pattern at
   `golden_belief_wall_trap_control_source_swap_preserves_affordances`, line 598).
4. Co-located displayed listing still produces the Trade affordance (negative
   control) → affordance fingerprint on a co-located fixture.

## What to Change

### 1. Gate the three economic accessors to co-located lots

In `crates/worldwake-sim/src/per_agent_belief_view.rs`:
- `has_sale_listing`, `seller_for_sale_lot`: return `false` / `None` unless the
  lot is co-located with the observing agent (reuse the co-location predicate used
  by `direct_container`, e.g. `has_authoritative_local_visibility`). For
  co-located lots, the existing `world` reads remain (lawful FND-14A physical
  observation of a displayed listing + present controller).
- `listed_sale_lots_at`: already scopes to a `place`; ensure the per-lot
  `world.*` reads only fire when that place is the agent's co-located place,
  returning empty otherwise.

### 2. Add economic goldens (failing-first)

In `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs`, add fixtures +
tests following the existing builder/assertion patterns:
- `golden_belief_wall_trap_remote_seller_delist_unseen` — remote seller delists
  after the agent's last observation; assert stale/unknown belief, no candidate
  retraction, no affordance change.
- `golden_belief_wall_trap_remote_seller_restock_unseen` — remote seller restocks
  unseen; assert no new acquire candidate until a carrier arrives.
- Extend the control-source-swap fingerprint to cover the economic scenarios.
- Negative control: co-located displayed listing still yields the Trade
  affordance for both control sources.
Each new leak golden must fail against current `main` and pass after section 1.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (modify)

## Out of Scope

- Production-job, load/capacity, contention accessors (tickets 002, 003).
- `can_control` / `believed_rights` social/control value reads (S158 Non-Goals;
  deferred to a future believed-rights spec).
- Any believed-sale-listing belief surface / `EntityBeliefAspect` addition.
- Doc updates to `planner-contracts.md` / `spec-drafting-rules.md` (ticket 004).

## Acceptance Criteria

### Tests That Must Pass

1. `golden_belief_wall_trap_remote_seller_delist_unseen` — stale/unknown belief,
   no candidate retraction, identical affordance fingerprint.
2. `golden_belief_wall_trap_remote_seller_restock_unseen` — no new acquire
   candidate without a lawful carrier.
3. Negative control: co-located displayed listing yields the Trade affordance for
   AI and Human control sources.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No economic accessor returns a value for a non-co-located lot from current
   world state; remote economic knowledge arrives only via belief/opportunity
   carriers (FND-14).
2. AI and Human control sources produce an identical lawful economic affordance
   set for every scenario (FND-19).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` — new remote-delist
   and remote-restock goldens + economic negative control + control-swap
   fingerprint extension; rationale: prove the economic leak is closed and not
   over-suppressed for co-located lots.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai belief_wall_trap` (confirm exact
   names with `cargo test -p worldwake-ai --test golden_ai -- --list`)
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `./scripts/verify.sh`
