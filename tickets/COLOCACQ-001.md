# COLOCACQ-001: Agent cannot acquire co-located unowned commodity lots when it owns none

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI candidate generation / commodity-acquisition opportunity anchoring (`worldwake-ai` source-composite + `worldwake-sim` commodity-opportunity / affordance surface)
**Deps**: S155 (Belief-View Boundary Correctness, COMPLETED — the change that unmasked this gap; see `archive/specs/S155-belief-view-boundary-correctness.md`)

## Problem

When an agent needs a commodity it does **not** own, and the only locally-reachable supply
is **co-located unowned item lots** (or a contended resource source), the planner can fail to
produce any acquisition plan and the agent idles with an elevated need.

Observed in the `survival-trade` golden (`survival_trade_proves_substitute_market_branch`):
Merchant Sera idles for the full guard window (ticks 1321–1381, max need 495 permille — thirst)
while controllable, co-located, unowned Water lots are present at the Market Square. The merchant
does not own Water, so `AcquireCommodity { commodity: Water }` is generated **anchored at the
merchant itself** (`OpportunityAnchor::Entity(<merchant>)`) and the plan search reports
`FrontierExhausted { expansions_used: 1 }` / `BudgetExhausted { expansions_used: 1 }`. No
"pick-up-then-consume the co-located unowned lot" opportunity is generated for Water during the
window, and the alternative (drawing from the Village Well) is blocked by facility contention with
the buyer.

This gap was previously **masked**: before S155, the merchant survived the same local-water
contention by routing to the remote South Orchard (apple is an authored thirst substitute) using
the orchard's **current authoritative location read through the per-agent belief view** — an
FND-14 remote-truth leak. S155 correctly closed that leak (`PerAgentBeliefView::effective_place`
no longer returns live world state for non-co-located entities), so the merchant no longer has an
illegitimate remote fallback and the latent acquisition gap surfaces as a stuck-idle window.

The merchant's overall behaviour is otherwise healthy across the 1440-tick run (committed actions:
drink 16, harvest:Harvest Water 132, eat 22, pick_up ~96; agent survives), so this is a localized
planning gap under contention, not a global failure.

## Assumption Reassessment (2026-05-20)

<!-- Verified during the S155 CI triage that produced this ticket. -->

1. **Failure reproduces** locally and on CI: `golden-survival / trade`
   (`survival_trade_proves_substitute_market_branch`) panics at
   `crates/worldwake-ai/tests/golden_harness/mod.rs:261` with
   `StuckIdleWindow { agent_name: "Merchant Sera", start_tick: 1321, end_tick: 1381, max_need_at_start: 495 }`.
   Repro: `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 'scenarios::survival_trade::survival_trade_proves_substitute_market_branch'`.
2. **Root layer is candidate generation, not authoritative validation.** Decision traces show the
   live goal is `GoalKind::AcquireCommodity { commodity: Water, purpose: SelfConsume }`. At t=1323
   the merchant's place held controllable co-located unowned Water lots (`can_control == true` via
   the existing co-located-unowned-item shortcut in `PerAgentBeliefView::can_control`), yet the only
   `AcquireCommodity(Water)` opportunity was anchored at the merchant
   (`OpportunityAnchor::Entity(<self>)`), which owns no Water, and frontier/budget-exhausted after a
   single expansion. The matching `AcquireCommodity(Apple)` succeeded at t=1318 only because the
   merchant **owns** apple sale-stock (self-anchored acquire binds to owned stock).
3. **S155 is correct and must not be reverted.** The unmasking change is
   `PerAgentBeliefView::effective_place` in `crates/worldwake-sim/src/per_agent_belief_view.rs`. The
   only entity whose belief-view place differs from world truth in this scenario is the remote Apple
   resource source (South Orchard Row, `EntityKind::Facility`). Reverting `effective_place` to the
   pre-S155 `knows_entity → world.effective_place` fallback makes the golden pass but reopens the
   FND-14 leak it closed — rejected.
4. **Intended invariant of the golden:** an agent must not idle for `>= 60` ticks with any need
   `> 300` permille (`assert_no_stuck_idle_windows`). The invariant is sound; the merchant should be
   able to satisfy thirst from co-located supply during contention without a remote fallback.
5. **Live `GoalKind` under test:** `AcquireCommodity` (self-consume). The current operator/affordance
   surface the scenario relies on is the `ResourceBarrier` terminal (`PlannerOpKind::Trade` /
   harvest) plus `pick_up` for unowned co-located lots. The gap is that for an unowned target
   commodity the acquire opportunity is anchored at the actor and does not enumerate co-located
   unowned lots (or a contention-aware harvest fallback) as alternative anchors.
6. **AI regression layer:** candidate generation (opportunity anchoring for `AcquireCommodity`),
   not `agent_tick`. Full golden E2E is required to confirm the fix because the bug only manifests
   mid-run under facility contention; a needs-only harness will not reproduce it.
7. **Ordering dependency:** the failure depends on transient facility contention (Merchant Sera and
   Buyer Nila both draining the capacity-24 Village Well, regen 3 ticks/unit) coinciding with the
   merchant owning no Water. Both agents are symmetric; the divergence is contention timing, not a
   priority-class asymmetry.
8. **Not a heuristic-weakening change.** The fix adds acquisition substrate (enumerate co-located
   unowned lots / contention-aware fallback for an unowned target commodity); it does not bypass an
   existing filter.
13. **Adjacent findings classified:**
    - `PerAgentBeliefView::can_control` omitting the FND-14A same-tick co-located authoritative-
      visibility case (it covers only unowned `ItemLot`/`UniqueItem`/`Container` via the shortcut,
      not co-located resource sources / owned co-located lots): a **separate** latent gap. It is NOT
      the cause of this failure (adding `has_authoritative_local_visibility` to the `can_control`
      gate left the window unchanged). Track separately only if a scenario surfaces it.
    - The static-fixture `effective_place` refinement (resolve immobile `Facility`/`Place` locations
      known via institutional belief) is architecturally sound but **inert** for this scenario
      (`knows_entity(orchard) == false` at the relevant ticks; the run was bit-identical). Not part
      of this ticket.
15. **Survivability envelope:** Village Well capacity 24, regen 3 ticks/unit, two heavy consumers.
    The merchant survives the run despite the window (drinks 16×), so the 60-tick wait is a guard
    violation rather than a death, but a longer or repeated contention window could threaten
    survival once the leaked remote fallback is gone.

## Architecture Check

1. The robust fix lets an agent acquire a needed commodity from **legitimately locally-reachable**
   supply (co-located unowned lots; contention-aware resource-source harvest) rather than depending
   on remote knowledge it should not have. This keeps S155's FND-14 leak fix intact and keeps the
   stuck-idle pathology guard intact, instead of weakening the guard or rebalancing the scenario
   around incorrect behaviour.
2. No backward-compatibility shim: this adds missing acquisition substrate; it does not restore the
   removed `effective_place` world-state read or add a dual path.

## Verification Layers

1. Merchant acquires + consumes co-located unowned Water during contention -> decision trace shows
   an `AcquireCommodity(Water)` opportunity anchored at a co-located unowned Water lot (or a
   contention-aware harvest) with `PlanSearchOutcome::Found`, plus an `action trace` `Committed`
   `pick_up`/`drink` in the formerly-idle window.
2. No stuck-idle window -> golden assertion `assert_no_stuck_idle_windows` passes for
   `survival_trade_proves_substitute_market_branch`.
3. FND-14 preserved -> S155 `effective_place`/`can_control` belief-view unit tests in
   `crates/worldwake-sim/src/per_agent_belief_view.rs` remain unchanged and pass (no remote-truth
   read reintroduced).

## What to Change

### 1. AcquireCommodity opportunity anchoring for unowned target commodities

Investigate `crates/worldwake-ai/src/source_composite.rs` (and the `worldwake-sim`
commodity-opportunity / `affordance_query.rs` surface it consumes) so that when the actor does not
own the target commodity, acquisition opportunities enumerate co-located unowned item lots as anchors
(pick-up-then-consume), in addition to the self/owned-stock anchor and the resource-source harvest.

### 2. Contention-aware harvest fallback (confirm during investigation)

Confirm whether the harvest path is suppressed purely by facility contention during the window, and
whether a contention-aware retry/queue interaction is also needed, or whether co-located-lot pickup
(change 1) is sufficient on its own.

## Files to Touch

- `crates/worldwake-ai/src/source_composite.rs` (modify — likely)
- `crates/worldwake-sim/src/commodity_opportunity.rs` (modify — likely)
- `crates/worldwake-sim/src/affordance_query.rs` (investigate)
- `crates/worldwake-ai/tests/scenarios/survival_trade.rs` (no behavioural edit expected; it is the
  proof surface)

## Out of Scope

- Any change to `PerAgentBeliefView::effective_place` or `can_control` (S155 is correct).
- Relaxing/recalibrating the `survival-trade` idle-window allowance.
- Rebalancing the `survival-trade.ron` Village Well capacity/regen.
- The separate `can_control` FND-14A co-location gap noted in Assumption Reassessment item 13.

## Acceptance Criteria

### Tests That Must Pass

1. `survival_trade_proves_substitute_market_branch` — no stuck-idle window; merchant satisfies
   thirst from co-located supply during the contention window.
2. `survival_trade_replays_deterministically` — still deterministic.
3. The full gated golden-survival family (engine/candidate-generation change): all scenarios via
   `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1`.
4. Existing suite: `./scripts/verify.sh`.

### Invariants

1. Agents never read non-co-located world state through the belief view (FND-14); S155 belief-view
   unit tests unchanged.
2. No stuck-idle window `>= 60` ticks with a need `> 300` permille in `survival-trade`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_trade.rs` — existing assertion is the proof; if a
   sharper unit-level proof of co-located-unowned acquisition is warranted, add a focused candidate-
   generation test in `worldwake-ai`/`worldwake-sim` for `AcquireCommodity` of an unowned commodity
   with co-located unowned lots present.
2. Regenerate `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (the
   `survival_baseline` diagnostics fixture also drifted under S155; see note below) intentionally,
   citing the behaviour change.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 'scenarios::survival_trade::survival_trade_proves_substitute_market_branch'`
2. `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1` (full gated golden-survival family)
3. `./scripts/verify.sh`

## Note: second CI failure on the S155 branch

The S155 branch also fails `golden-scenario-diagnostics / fixture`
(`golden_scenario_diagnostics_survival_baseline_fixture_is_stable`): the recorded diagnostics for the
`survival_baseline` scenario drifted because S155 changed agent behaviour. This is **intentional
drift** (a pure record of correct new belief-only behaviour), independent of the acquisition gap
above, and is regenerated via
`WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test --release -p worldwake-ai --test golden_ai scenario_diagnostics_fixture -- --ignored --test-threads=1`.
Per the triage decision (2026-05-20) it was deliberately **not** regenerated yet; regenerate it when
this ticket lands (or separately, citing S155) so the fixture reflects the final committed behaviour.
