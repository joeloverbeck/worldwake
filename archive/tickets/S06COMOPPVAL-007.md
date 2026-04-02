# S06COMOPPVAL-007: Missing S06 golden coverage plus cross-layer agreement proof

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: S06COMOPPVAL-005, S06COMOPPVAL-006

## Problem

S06 is mostly implemented, but the remaining proof surface is no longer a pure "add one new golden file" task. Live code already has positive recipe-input emergence in [`golden_production.rs`](../crates/worldwake-ai/tests/golden_production.rs), while the still-unproven parts are:

1. missing negative/no-recipes golden coverage around indirect commodity opportunity
2. seller-side retention behavior at the strongest lower layer
3. an honest marginal-value boundary for the shared recipe-input scorer in trade valuation

The attempted same-snapshot agreement proof exposed a real contradiction: the shared scorer was giving recipe inputs positive retained value even when the actor held zero units, so trade valuation saw no improvement from receiving firewood and no loss from selling the last firewood. This ticket therefore owns the bounded production fix as well as the missing goldens.

## Assumption Reassessment (2026-04-02)

1. Golden test files live in `crates/worldwake-ai/tests/`. Relevant existing files: `golden_production.rs`, `golden_trade.rs`, `golden_supply_chain.rs`.
2. Golden tests require `PerceptionProfile` on agents that need to observe (CLAUDE.md).
3. `RecipeRegistry` and `RecipeDefinition` are in `worldwake-sim`. Tests can create registries with test recipes.
4. `CommodityValuationProfile` from ticket 001 must be set on agents that should reason about indirect value.
5. `KnownRecipes` component must include the relevant recipe IDs.
6. Existing golden test infrastructure supports `hash_world`, `hash_event_log`, `verify_authoritative_conservation`, `verify_live_lot_conservation` for determinism and conservation checks.
7. The positive baker/firewood chain is already covered in `golden_production.rs` by `golden_remote_acquire_commodity_recipe_input` plus its replay companion.
8. Multi-input, multi-step, and workstation-depth propagation already have focused unit proof in [`commodity_opportunity.rs`](../crates/worldwake-sim/src/commodity_opportunity.rs).
9. The still-missing proof surface is:
   - negative golden: unreachable workstation suppresses recipe-input behavior
   - negative golden: no-recipes / no-profile fallback suppresses indirect recipe behavior
   - focused lower-layer proof: seller retains enabling input when recipe opportunity dominates
   - focused lower-layer proof: absent recipe inputs no longer carry retained-value credit in trade valuation snapshots

## Architecture Check

1. Goldens should cover the planner/emergent behavior still missing after the existing S06 positive chain in `golden_production.rs`.
2. Seller-retention and the shared marginal-value correction are better proven first at the strongest lower layer (`trade_valuation.rs` / `commodity_opportunity.rs`) because they depend on the shared valuation substrate directly, not only on top-level scenario narration.
3. If those lower-layer proofs expose a real contradiction in the shared commodity-opportunity path, fix production code in-scope rather than weakening the proof.
4. No backward-compatibility shims.

## Verification Layers

1. Existing positive baker/firewood chain -> already covered by `golden_remote_acquire_commodity_recipe_input`
2. Unreachable workstation suppression -> decision trace / lack of recipe-input action chain
3. No-recipes or no-profile fallback -> decision trace / lack of recipe-input action chain
4. Seller retention -> focused authoritative trade-valuation proof
5. Shared marginal-value correction -> focused lower-layer proof that receiving an enabling input improves trade valuation while absent inputs do not already count as retained value
6. Deterministic replay -> state hash comparison for the new golden scenario

## What to Change

### 1. Add the missing negative S06 goldens

Setup:
- Baker-like agent with hunger, recipe knowledge, and a reachable or unreachable bread-production context as needed
- Reuse the existing recipe-input golden setup style from `golden_production.rs` instead of duplicating the already-covered positive remote-acquisition chain

Assert:
- Without a reachable mill, the agent does not generate or execute the recipe-input firewood path
- Without useful recipes / valuation profile, the agent does not gain indirect firewood motive from the recipe layer
- Add deterministic replay for at least one of the new negative scenarios

### 2. Add focused seller-retention proof at the trade-valuation layer

Setup:
- Seller has the last enabling input for a reachable, concretely useful recipe opportunity

Assert:
- Trade valuation refuses or prices against giving up that input when the recipe opportunity dominates
- Add the converse or balancing case only if needed to prove the boundary honestly

### 3. Correct the shared marginal-value boundary used by trade valuation

Setup:
- Reuse the shared `commodity_opportunity` / `trade_valuation` layer with reachable recipe opportunities

Assert:
- A held enabling input contributes retained recipe value
- An absent enabling input does not already count as retained value in the current snapshot
- Receiving that enabling input through trade improves the valuation snapshot
- Selling the last enabling input can now be rejected or priced against for the correct reason

## Files to Touch

- `tickets/S06COMOPPVAL-007.md`
- `crates/worldwake-ai/tests/golden_commodity_opportunity.rs` (new)
- `crates/worldwake-sim/src/trade_valuation.rs`
- `crates/worldwake-sim/src/commodity_opportunity.rs`
- `crates/worldwake-ai/tests/golden_harness/` only if helper extraction is truly needed

## Out of Scope

- Duplicating the already-covered positive remote recipe-input golden from `golden_production.rs`
- Re-proving multi-input, multi-step, and best-path propagation already covered by focused `commodity_opportunity.rs` tests
- Performance optimization of recipe propagation

## Acceptance Criteria

### Tests That Must Pass

1. Existing positive remote recipe-input golden still passes unchanged
2. A new golden proves the agent does not pursue recipe-input firewood when the required workstation is not believed reachable
3. A new golden proves no-recipes / no-profile fallback does not invent indirect recipe-input behavior
4. Focused trade valuation proves a seller retains an enabling input when the reachable recipe opportunity dominates
5. Focused lower-layer proof shows the shared recipe-input scorer now behaves as marginal value for trade snapshots: absent input contributes no retained value, received input improves the snapshot, and selling the last held input can be rejected
6. Deterministic replay holds for the new golden scenario
7. All existing AI tests pass: `cargo test -p worldwake-ai`
8. Full suite: `cargo test --workspace`
9. CI lint parity: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Deterministic replay: same seed, same inputs → identical hashes.
2. No duplicate AI-only and trade-only indirect-valuation logic is reintroduced.
3. No regressions in existing golden tests.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_commodity_opportunity.rs` (new) — missing negative S06 goldens
2. `crates/worldwake-sim/src/trade_valuation.rs` — seller-retention and marginal-value focused tests
3. `crates/worldwake-sim/src/commodity_opportunity.rs` — updated focused expectations for held-vs-absent input semantics
4. Replay companion tests for deterministic verification

### Commands

1. `cargo test -p worldwake-ai --test golden_commodity_opportunity` — targeted new tests
2. `cargo test -p worldwake-sim trade_valuation -- --nocapture` — focused shared-layer proof
3. `cargo test -p worldwake-sim commodity_opportunity -- --nocapture` — focused shared-scorer regression
4. `cargo test -p worldwake-ai` — AI regression
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-02
- What changed:
  - Added [golden_commodity_opportunity.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_commodity_opportunity.rs) with the missing negative S06 golden coverage for unreachable-workstation suppression and no-known-recipe suppression, plus deterministic replay for the unreachable-workstation case.
  - Updated [trade_valuation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/trade_valuation.rs) with focused proof that reachable recipe opportunity can justify accepting an enabling input, that sellers retain the last enabling input when the recipe opportunity dominates, and that the same commodity becomes sellable again when the recipe path is unreachable.
  - Corrected [commodity_opportunity.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/commodity_opportunity.rs) so absent recipe inputs no longer receive retained indirect value credit; shared valuation now reflects marginal value over accessible stock in trade snapshots.
- Deviations from original plan:
  - The ticket was corrected before coding from a pure golden-coverage ticket into a mixed-layer ticket after reassessment exposed a real shared marginal-value contradiction in production code.
  - The already-covered positive recipe-input emergence path in `golden_production.rs` was left unchanged and not duplicated here.
- Verification:
  - `cargo test -p worldwake-sim commodity_opportunity -- --nocapture`
  - `cargo test -p worldwake-sim trade_valuation -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_commodity_opportunity -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
