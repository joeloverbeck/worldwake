# AILIBBASE-001: Restore the baseline `worldwake-ai` library suite

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — AI planning/search/candidate/ranking test contracts and one same-domain golden expectation
**Deps**: Discovered during `archive/tickets/AGEFOOREP-003.md` verification; reproduce on clean `HEAD` (`6d627d68`) before coding.

## Problem

Before this ticket, `cargo test -p worldwake-ai` failed on the clean branch baseline, before any AGEFOOREP-003 scenario edits. This blocked using the package-level AI suite as broad proof for unrelated scenario/golden tickets.

## Assumption Reassessment (2026-06-02)

1. The failure reproduces in a temporary clean worktree at `HEAD` (`6d627d68 Implemented AGEFOOREP-002.`), with no AGEFOOREP-003 edits applied.
2. The observed command is `cargo test -p worldwake-ai`.
3. The failing library tests are:
   - `agent_tick::tests::cargo_satisfaction_at_destination_while_carrying`
   - `agent_tick::tests::merchant_restock_requires_delivery_to_home_facility`
   - `agent_tick::tests::read_phase_runs_opportunity_compiler_before_candidate_generation`
   - `candidate_generation::tests::candidate_gen_quantity_aware_emission_derives_target_from_horizon`
   - `goal_model::tests::sell_commodity_not_satisfied_when_no_listed_lot`
   - `ranking::tests::survival_relevant_theft_uses_target_commodity_drive_priority_and_motive`
   - `search::tests::sell_search_for_remote_home_stock_moves_stores_and_stages_before_goal_satisfaction`
4. Shared abstraction boundary under audit: merchant stock/listing/cargo planning plus self-consume acquisition and theft ranking across `candidate_generation`, `goal_model`, `ranking`, `search`, and `agent_tick`.
5. This ticket is separate from AGEFOOREP-003 because that ticket only changes `scenarios/survival-theft.ron` and its golden harness; the same failures appear on the clean baseline.
6. Live reassessment split the failures into one production goal-satisfaction bug plus stale focused/golden expectations:
   - `GoalKind::SellCommodity::is_satisfied()` treated an at-market merchant with no listed stock and no local stock as satisfied. The landed fix requires listed stock, or local stock that exists but is non-saleable, before the sell goal can close.
   - The apple acquisition fixture had registered the apple resource source under the bread lookup key.
   - The opportunity-compiler read-phase fixture used the actor's already-held bread and a stale `MoveCargo` effect-op mapping, which conflicted with live self-consume suppression and the compiler's `AcquireCommodity`/`Trade` handoff.
   - The theft ranking aggregate motive had drifted to the live deterrence-scaled value `623_000`; the drive-provenance input remains the pure hunger input `567_000`.
   - The merchant-selling integration scenario proved a one-unit resumed purchase; its stale "three-coin unit-purchase" prose/assertion was corrected to the live one-coin purchase contract.

## Architecture Check

Repair the failing AI contracts at their owning layers instead of weakening scenario-level proof or treating unrelated goldens as the broad gate. Reassess whether the tests are stale or production behavior regressed before editing assertions.

## Verified Layers

1. Candidate emission and quantity derivation -> focused `candidate_generation` unit test.
2. Sell/MoveCargo satisfaction and staging path -> focused `goal_model`, `search`, and `agent_tick` unit tests.
3. Theft motive arithmetic -> focused `ranking` unit test.
4. Merchant-selling resumed purchase generated-golden contract -> focused `golden_ai` scenario and generated docs.
5. Broad package health -> `cargo test -p worldwake-ai`.

## Landed Changes

1. Repaired `SellCommodity` satisfaction so absence of listed stock is not enough to satisfy a sell goal when no local stock exists, while preserving the non-saleable-stock closure path.
2. Corrected stale candidate/read-phase/ranking fixtures to the live apple source lookup, opportunity-compiler `Trade` handoff, and deterrence-scaled theft motive.
3. Updated the merchant-selling golden scenario and generated golden docs from stale three-coin wording to the live one-unit, one-coin resumed-purchase contract.

## Landed Files

- `crates/worldwake-ai/src/candidate_generation.rs` (apple resource-source fixture drift)
- `crates/worldwake-ai/src/goal_model.rs` (sell satisfaction production fix)
- `crates/worldwake-ai/src/ranking.rs` (theft motive expectation drift)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (read-phase fixture drift)
- `crates/worldwake-ai/tests/scenarios/merchant_selling.rs` (same-domain golden contract drift exposed by the package gate)
- `docs/generated/golden-scenario-details/merchant-selling.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)

No source edit was needed in `crates/worldwake-ai/src/search/tests.rs`; the existing search expectation passed after the production sell-satisfaction fix.

## Outcome

Completed on 2026-06-02.

- Restored the `worldwake-ai` package baseline by fixing the sell-goal satisfaction contract and truthing stale AI fixture/golden expectations.
- Regenerated the merchant-selling golden docs after correcting the resumed-purchase scenario prose from the stale three-coin claim to the live one-unit, one-coin contract.

## Out of Scope

- AGEFOOREP-003 survival-theft scenario quantity/proof edits.
- Broad golden scenario retuning unless reassessment proves these library failures change authored scenario behavior.

## Acceptance Result

1. Focused reruns for the repaired failing tests passed.
2. `cargo test -p worldwake-ai` passed.

### Invariants Preserved

1. Merchant stock/listing/cargo planning remains belief-backed and source/sink accountable.
2. Test repairs did not bypass planner/search legality, action preconditions, or ranking motive semantics.

## Test Plan Result

### Focused Tests

1. Existing focused tests were repaired or restored at their owning seams; no duplicate test file was added.
2. The package gate exposed one same-domain `golden_ai` merchant-selling assertion, which was corrected and regenerated through the golden inventory docs.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib goal_model::tests::sell_commodity_not_satisfied_when_no_listed_lot -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::candidate_gen_quantity_aware_emission_derives_target_from_horizon -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::read_phase_runs_opportunity_compiler_before_candidate_generation -- --exact`
- Passed `cargo test -p worldwake-ai --lib ranking::tests::survival_relevant_theft_uses_target_commodity_drive_priority_and_motive -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::sell_search_for_remote_home_stock_moves_stores_and_stages_before_goal_satisfaction -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::merchant_restock_requires_delivery_to_home_facility -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::cargo_satisfaction_at_destination_while_carrying -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::sell_search_does_not_restage_non_saleable_unlisted_stock -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::merchant_selling::seller_return_completes_resumed_purchase_after_live_offer_refresh -- --exact`
- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-ai`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
