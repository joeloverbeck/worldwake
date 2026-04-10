# S80EXPDRI-007: Restore negotiated supply-chain golden under exploration drive

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S80EXPDRI-004, S80EXPDRI-005

## Problem

Broadened verification for `S80EXPDRI-005` reproduces a real regression in `crates/worldwake-ai/tests/golden_supply_chain.rs`: the full negotiated supply-chain scenario no longer completes the consumer purchase. Focused tracing shows the consumer's new lawful exploration branch pulls it out of the market loop early, which delays the first trade attempt until the scenario is no longer proving the original restock -> negotiated trade contract cleanly. The exploration-drive work therefore exposed a stale golden fixture, not a production contradiction in trade or exploration runtime code. Until the golden is recalibrated to isolate the intended supply-chain branch, `cargo test -p worldwake-ai` remains red after the exploration-drive series.

## Assumption Reassessment (2026-04-10)

1. The failing proof surface is real and isolated: `cargo test -p worldwake-ai --test golden_supply_chain` now fails in both `golden_full_supply_chain_negotiated_restock_to_consumption` and `golden_full_supply_chain_negotiated_restock_to_consumption_replays_deterministically` at `crates/worldwake-ai/tests/golden_supply_chain.rs:1919`, asserting that the consumer should record `TradeAgreed` for apples at a negotiated price greater than one coin.
2. The failure was first surfaced by the broader `cargo test -p worldwake-ai` rerun during `S80EXPDRI-005`, then reproduced in the isolated target command above, so this is not broad-suite noise.
3. Shared abstraction boundary under audit: the supply-chain golden's intended buyer/seller branch in `crates/worldwake-ai/tests/golden_supply_chain.rs` versus the new lawful exploration fallback introduced by `S80EXPDRI-004`/`005`. The live question is whether the old full-chain setup still isolates restock -> trade, not whether production trade math is broken.
4. Focused proof changed the root-cause hypothesis twice. The consumer does eventually re-enter the trade path, so this is not a missing `AcquireCommodity` / `ExploreLocation` candidate-generation bug. A temporary trade-side pricing patch was also disproved: reverting it and setting the consumer's `ExplorationProfile.curiosity_weight` to `0` inside this scenario restores the original full-chain contract immediately.
5. The early divergence is a lawful competing branch under the current architecture. In the reproduced failing run, the consumer first travels away from `GeneralStore` through other village places before the merchant restock resolves. That makes this a scenario-isolation problem under `docs/golden-e2e-testing.md`, not a production contradiction.
6. The failing golden's contract predates the exploration-drive series and is still owned by the supply-chain surface, not by the exploration golden itself. This ticket therefore owns restoring the honest supply-chain proof surface by removing unrelated exploration from the scenario setup.

## Architecture Check

1. A dedicated follow-up is cleaner than folding the supply-chain regression into `S80EXPDRI-005` after the exploration golden and production contradiction were already fixed. The concern is a separate cross-system interaction between exploration fallback and negotiated trade.
2. The ticket should recalibrate the owning golden fixture rather than changing production trade or exploration behavior. The full-chain scenario is about merchant restock -> negotiated trade -> consumption, so removing unrelated lawful exploration from this one fixture is cleaner than mutating runtime logic.

## Verification Layers

1. Whether the consumer's lawful exploration branch is what breaks the old fixture -> decision trace plus action trace in `golden_supply_chain`.
2. Whether the recalibrated scenario still proves negotiated trade and hunger relief in the intended order -> action trace plus demand-memory / `TradeAgreed` evidence in the owning golden.
3. Whether the final world still reaches restock, trade, and hunger relief in the intended order -> golden E2E scenario assertions plus authoritative world state where the scenario already lawfully checks consequences.
4. Whether the change preserves exploration-drive behavior outside this isolated fixture -> focused exploration goldens from `crates/worldwake-ai/tests/golden_exploration.rs`.

## What to Change

### 1. Recalibrate the full-chain supply-chain fixture

Update `crates/worldwake-ai/tests/golden_supply_chain.rs` so the consumer does not lawfully peel off into exploration while this scenario is supposed to prove the merchant restock -> negotiated trade -> consumption chain. Use an explicit `ExplorationProfile` override in the fixture rather than changing runtime behavior.

### 2. Keep the owning proof surface aligned

Leave `golden_supply_chain.rs` as the owning proof surface for the full-chain contract, and keep the scenario comments/assertions aligned with the new isolation choice.

### 3. Re-verify exploration separately

Rerun `crates/worldwake-ai/tests/golden_exploration.rs` so the localized supply-chain fixture change does not mask or regress the exploration-drive contract itself.

## Files to Touch

- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — isolate the full-chain fixture from unrelated exploration and keep the owning proof aligned)

## Out of Scope

- New exploration features beyond recalibrating the stale supply-chain fixture exposed by the exploration-drive rollout
- Production trade-valuation changes not required by the focused proof
- Unrelated golden cleanups outside `golden_supply_chain` and the exploration proof surfaces this ticket depends on

## Acceptance Criteria

### Tests That Must Pass

1. `golden_supply_chain::golden_full_supply_chain_negotiated_restock_to_consumption`
2. `golden_supply_chain::golden_full_supply_chain_negotiated_restock_to_consumption_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_supply_chain`
4. `cargo test -p worldwake-ai --test golden_exploration`
5. `cargo test -p worldwake-ai`

### Invariants

1. Exploration remains a lawful self-care fallback rather than a scenario-specific override or hardcoded suppression.
2. The supply-chain golden continues to prove a concrete causal chain with traceable negotiation evidence instead of relying on hidden side conditions.
3. The fixture's exploration override is local to this scenario and exists only to remove an unrelated lawful branch from the proof surface.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_supply_chain.rs` — rerun the owning negotiated-trade proof after isolating the consumer from unrelated exploration.
2. `crates/worldwake-ai/tests/golden_exploration.rs` — rerun to ensure the localized fixture override does not regress the exploration-drive contract.

### Commands

1. `cargo test -p worldwake-ai --test golden_supply_chain`
2. `cargo test -p worldwake-ai --test golden_exploration`
3. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-04-10.

- Isolated the full-chain supply-chain golden in `crates/worldwake-ai/tests/golden_supply_chain.rs` by overriding the consumer's `ExplorationProfile` with `curiosity_weight: pm(0)` for this scenario only.
- Kept the merchant restock, negotiated trade, and consumption assertions unchanged; the scenario now proves that contract again without unrelated exploration pulling the consumer out of the market loop.
- Updated the scenario prose so the exploration override is documented as an explicit isolation choice rather than an unexplained fixture quirk.

## Deviations

- Reassessment changed the ticket from an assumed production trade/runtime fix to a golden-fixture correction. Focused proof showed the old failure was caused by a newly lawful exploration branch in the consumer setup, not by a production contradiction in trade negotiation or exploration runtime code.
- A temporary trade-side pricing patch was tested during diagnosis and then reverted once the isolated-fixture proof showed it was not required by the live contract.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_supply_chain`
- Passed `cargo test -p worldwake-ai --test golden_exploration`
- Passed `cargo test -p worldwake-ai`
