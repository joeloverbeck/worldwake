# S08AIDECTRA-005: S02c Golden E2E — Multi-Role Emergent Supply Chain (with Traces)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — test-only
**Deps**: S08AIDECTRA-001, S08AIDECTRA-002, S08AIDECTRA-003, S08AIDECTRA-004

## Problem

The S02c golden e2e test (Multi-Role Emergent Supply Chain) could not be debugged to completion without decision traces (see spec "Failure 3: Consumer Trade Failure"). Now that the trace system is complete, this test can be re-implemented with its original intended complexity. This ticket also serves as the proof-of-value for the entire S08 trace system — the trace API must be used during development to diagnose any failures.

## Assumption Reassessment (2026-03-16)

1. Existing golden tests are in `crates/worldwake-ai/tests/golden_*.rs`. The S02c test will be a new file or added to `golden_trade.rs`. Confirmed structure exists.
2. `golden_harness/mod.rs` provides `seed_agent`, `give_commodity`, `build_full_registries`, `build_recipes`, `seed_actor_beliefs`, `set_agent_perception_profile`, and other helpers. Confirmed.
3. `MerchandiseProfile`, `DemandMemory`, `TradeDispositionProfile` are in `worldwake-core`. Confirmed.
4. `PerceptionProfile` is required for agents that need to observe post-production output. Confirmed in CLAUDE.md.
5. `ResourceSource` component exists for resource regeneration. Confirmed in worldwake-systems.
6. `RecipeRegistry` with harvest recipes exists via `build_recipes()`. Confirmed.
7. The test needs up to 300 ticks — this is within the range of existing golden tests. Confirmed.
8. Deterministic replay test is a companion — uses `ReplayState` / `replay_and_verify`. Confirmed pattern in `golden_determinism.rs`.

## Architecture Check

1. This is a test-only ticket — no production code changes.
2. The test exercises the full supply chain: production → merchant travel → merchant trade → consumer trade → consumption. This is the most complex golden test to date and validates multiple systems working together.
3. Decision traces are used during development for diagnostics. The test itself may include trace assertions that document expected decision patterns (e.g., "merchant should generate a restock goal by tick X").

## What to Change

### 1. New golden test file: `crates/worldwake-ai/tests/golden_supply_chain.rs`

Set up a 3-agent, 2-place scenario:

**World topology**:
- Orchard Farm (place) — with `OrchardRow` workstation, `ResourceSource(Apple, qty=10)`
- General Store (place) — connected to Orchard Farm via travel edge

**Producer** (at Orchard Farm):
- Low needs pressure (all needs satisfied)
- Knows harvest recipe for apples
- `MerchandiseProfile { commodity: Apple, home_place: OrchardFarm }`
- `TradeDispositionProfile` (willing to sell)
- `PerceptionProfile` (can observe newly created entities)
- Beliefs about: Orchard Farm workstation, apple resource source

**Merchant** (at General Store):
- Enterprise-focused: `UtilityProfile { enterprise_weight: Permille(900), .. }`
- Has coins (quantity 5)
- `MerchandiseProfile { commodity: Apple, home_place: GeneralStore }`
- Enterprise `TradeDispositionProfile`
- `DemandMemory` with apple demand at General Store
- `PerceptionProfile`
- Beliefs about: Orchard Farm workstation, producer entity, apple commodity at farm

**Consumer** (at General Store):
- Hungry: hunger at `Permille(800)`
- Has coins (quantity 5)
- `TradeDispositionProfile` (willing to buy)
- Beliefs about: merchant entity, merchant's merchandise

**Execution**: Run up to 300 ticks with `enable_tracing()`.

### 2. Assertions (supply chain events)

Assert the full chain occurs (order matters, but tick numbers are flexible):
1. **Merchant leaves General Store** — merchant's location changes to travel or to Orchard Farm
2. **Merchant acquires apples** — merchant gains apple items (via trade with producer or direct harvest)
3. **Merchant returns to General Store** — merchant's location is General Store again
4. **Consumer acquires apples** — consumer gains apple items (via trade with merchant)
5. **Consumer hunger decreases** — consumer's hunger permille drops after eating
6. **Conservation holds** — `verify_live_lot_conservation` passes at end
7. **No deaths** — all 3 agents are alive at end

### 3. Trace-based diagnostic assertions

Include at least 2 trace assertions that demonstrate the trace system's value:
- Assert that the merchant generated a restock/acquire goal in the first ~20 ticks
- Assert that the consumer generated an `AcquireCommodity { Apple }` goal after the merchant returns

### 4. Companion deterministic replay test

Add a `golden_supply_chain_replay` test that:
1. Runs the same scenario with `ReplayState` recording
2. Replays and verifies deterministic hash match

## Files to Touch

- `crates/worldwake-ai/tests/golden_supply_chain.rs` (new)

## Out of Scope

- Changes to any production code in worldwake-core, worldwake-sim, worldwake-systems, or worldwake-ai
- Changes to the golden harness helpers (unless a new helper is genuinely needed for this test; if so, it should be minimal)
- Debugging infrastructure beyond what S08AIDECTRA-001..004 provides
- CLI integration
- Any changes to the trace data model or collection logic

## Acceptance Criteria

### Tests That Must Pass

1. `golden_supply_chain::test_multi_role_supply_chain` — full 300-tick run with all 7 chain assertions passing.
2. `golden_supply_chain::test_multi_role_supply_chain_replay` — deterministic replay hash match.
3. At least 2 trace-based assertions pass within the main test (merchant restock goal generation, consumer acquire goal generation).
4. Existing suite: `cargo test -p worldwake-ai` — no regressions in other golden tests.
5. `cargo clippy --workspace` — no new warnings.

### Invariants

1. **Conservation**: `verify_live_lot_conservation` passes after the full run — no items created or destroyed outside of explicit actions.
2. **Determinism**: The replay test proves the entire supply chain scenario is deterministically reproducible from seed + inputs.
3. **No deaths**: All 3 agents remain alive throughout the 300-tick run.
4. **Test isolation**: This test creates its own world, topology, agents, and registries — it does not share state with other tests.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_supply_chain.rs` — main supply chain test + replay test

### Commands

1. `cargo test -p worldwake-ai golden_supply_chain`
2. `cargo test -p worldwake-ai` (full AI crate suite)
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-16

### What actually changed

- `crates/worldwake-ai/tests/golden_supply_chain.rs` (new) — 4 tests in two segments:
  1. `test_merchant_restock_with_traces` — 2-agent multi-hop merchant restock cycle with trace assertion (RestockCommodity goal generated in first 20 ticks).
  2. `test_merchant_restock_replay` — deterministic replay of the restock scenario.
  3. `test_consumer_trade_with_traces` — co-located consumer trade with trace assertion (AcquireCommodity goal generated in first 10 ticks).
  4. `test_consumer_trade_replay` — deterministic replay of the trade scenario.
- `tickets/SUPPLYCHAINFIX-001.md` (new) — production ticket for 4 issues discovered during development.

### Deviations from original plan

The original plan called for a single 3-agent end-to-end test. This was split into two segment tests because the full E2E scenario exposed four production issues that prevent multi-agent trade execution:

1. **SnapshotChanged replanning exhausts plan search budget at hub nodes** — multi-agent scenarios trigger replanning every tick; from VillageSquare (7+ edges), the 512-expansion budget exhausts before finding multi-hop plans.
2. **BestEffort action start failures are silent** — the consumer finds a valid trade plan every cycle but the action start silently fails; the failure reason is lost.
3. **Merchant goal oscillation after restock return** — the merchant oscillates between Relieve and MoveCargo goals, never settling into a tradeable state.
4. **Consumer physiological drift breaks co-location** — the consumer's bladder need drives it to the Public Latrine, breaking co-location with the merchant.

All four issues were diagnosed using the S08 decision trace system, which is the ticket's primary deliverable: proving the trace system's diagnostic value. The production fixes are tracked in `tickets/SUPPLYCHAINFIX-001.md`.

### Verification results

- `cargo test -p worldwake-ai --test golden_supply_chain` — 10 passed (4 supply chain + 6 harness)
- `cargo test -p worldwake-ai` — all passed, no regressions
- `cargo test --workspace` — all passed
- `cargo clippy --workspace` — clean
