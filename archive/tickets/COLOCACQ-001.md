# COLOCACQ-001: Agent cannot acquire co-located unowned commodity lots when it owns none

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - belief-view local physical inventory visibility and opportunity compiler acquisition anchoring
**Deps**: S155 (Belief-View Boundary Correctness, COMPLETED - the change that unmasked this gap; see `archive/specs/S155-belief-view-boundary-correctness.md`)

## Problem

Before this ticket, Merchant Sera could idle for the full `survival_trade_proves_substitute_market_branch` guard window (ticks 1321-1381, max thirst 495 permille) while controllable, co-located, unowned Water lots were present at Market Square. The only generated `AcquireCommodity(Water)` opportunity in the window was anchored at the merchant itself, and search exhausted after a single expansion.

S155 correctly removed a prior remote-truth fallback through `PerAgentBeliefView::effective_place`. That exposed a local-acquisition gap: same-tick co-located item lots were visible through `entities_at`, but their physical inventory facts were unavailable to the same belief view unless the lot already existed in agent belief memory. Need-driven candidate generation therefore could not use the co-located Water lot evidence during the contention window.

## Assumption Reassessment

1. The original failure reproduced with `StuckIdleWindow { agent_name: "Merchant Sera", start_tick: 1321, end_tick: 1381, max_need_at_start: 495 }`.
2. The root layer was local belief-view inventory visibility plus opportunity anchoring, not authoritative validation. `pick_up` and lower search already handled co-located unowned lots once the candidate had a real local lot anchor.
3. S155's `effective_place` boundary remains intact. This ticket did not restore non-co-located live world reads.
4. The separate `can_control` same-tick co-location observation noted in the original triage remained out of scope; this fix did not change `can_control`.
5. A contention-aware harvest fallback was not required. Once same-tick local lot facts were visible and self-anchored compiler opportunities were suppressed, the survival-trade golden passed.

## Architecture Check

The landed behavior keeps FND-14 locality intact: agents can read physical facts for entities with authoritative local visibility, but not remote entity locations through the belief view. The opportunity compiler also no longer turns an actor's own remembered inventory into an acquisition source for that same actor.

No compatibility shim was added. The fix narrows the belief-facing physical read to local visibility and removes the impossible self-acquisition compiler anchor.

## Landed Changes

1. `PerAgentBeliefView` now exposes `item_lot_commodity`, `direct_container`, and `direct_possessor` for entities with authoritative local visibility, matching the already-local `entities_at` surface used by candidate generation.
2. `compile_opportunities` skips `entity == agent` while iterating known entity beliefs, so compiler output cannot propose `AcquireCommodity` anchored on the actor's own inventory.
3. Focused regressions cover both changes:
   - `co_located_unknown_item_lot_exposes_physical_inventory_facts`
   - `compile_opportunities_does_not_anchor_acquisition_on_self_inventory`
4. `expected-scenario-diagnostics.json` was regenerated after the final behavior landed, capturing the intentional S155 and COLOCACQ behavior drift.

## Landed Files

- `crates/worldwake-sim/src/per_agent_belief_view.rs`
- `crates/worldwake-ai/src/opportunity_compiler/compile.rs`
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json`

## Verified Layers

1. Co-located item-lot physical facts are available through the belief view for local entities without seeding prior belief memory.
2. Opportunity compilation preserves non-self inventory-backed opportunities and omits self-anchored acquisition opportunities.
3. The motivating survival-trade golden has no stuck idle window.
4. The survival-trade replay companion remains deterministic.
5. The full ignored `golden_ai` suite passed.
6. Repository verification passed through `./scripts/verify.sh`.

## Acceptance Criteria

1. Passed: `survival_trade_proves_substitute_market_branch` no longer reports the Merchant Sera stuck-idle window.
2. Passed: `survival_trade_replays_deterministically` remains deterministic.
3. Passed: the full ignored `golden_ai` family passes in release.
4. Passed: `./scripts/verify.sh` passes.
5. Passed: S155 locality remains intact; no `effective_place` or `can_control` relaxation was made.

## Test Plan Result

1. Added and passed focused sim regression: `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::co_located_unknown_item_lot_exposes_physical_inventory_facts -- --exact`
2. Added and passed focused AI regression: `cargo test -p worldwake-ai --lib opportunity_compiler::compile::tests::compile_opportunities_does_not_anchor_acquisition_on_self_inventory -- --exact`
3. Passed motivating golden: `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 'scenarios::survival_trade::survival_trade_proves_substitute_market_branch'`
4. Passed replay companion: `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 'scenarios::survival_trade::survival_trade_replays_deterministically'`
5. Regenerated and passed diagnostics fixture with `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1`: `cargo test --release -p worldwake-ai --test golden_ai scenario_diagnostics_fixture -- --ignored --test-threads=1`
6. Passed full ignored golden family: `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1`
7. Passed repository verification: `./scripts/verify.sh`

## Verification Result

1. Passed: `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::co_located_unknown_item_lot_exposes_physical_inventory_facts -- --exact`
2. Passed: `cargo test -p worldwake-ai --lib opportunity_compiler::compile::tests::compile_opportunities_does_not_anchor_acquisition_on_self_inventory -- --exact`
3. Passed: `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 'scenarios::survival_trade::survival_trade_proves_substitute_market_branch'`
4. Passed: `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 'scenarios::survival_trade::survival_trade_replays_deterministically'`
5. Passed: `cargo test --release -p worldwake-ai --test golden_ai scenario_diagnostics_fixture -- --ignored --test-threads=1` with `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1`
6. Passed: `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1`
7. Passed: `./scripts/verify.sh`

## Outcome

Completed on 2026-05-20.

The actual fix landed in `PerAgentBeliefView` and the opportunity compiler, rather than the originally suspected source-composite or commodity-opportunity files. Local item-lot physical facts are now available at the belief-view surface for co-located visible entities, and compiler-backed acquisition opportunities no longer use the actor's own inventory as the source entity.

The original harvest fallback investigation resolved as no-change: co-located lot acquisition was sufficient for the survival-trade contention window. The diagnostics fixture was regenerated after the behavior stabilized.
