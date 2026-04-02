# S39LIMSIDBEN-003: Golden proof for combined-trip side-benefit selection

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: [archive/tickets/S39LIMSIDBEN-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S39LIMSIDBEN-002.md), [specs/S39-limited-side-benefit-plan-scoring.md](/home/joeloverbeck/projects/worldwake/specs/S39-limited-side-benefit-plan-scoring.md), [archive/specs/S04-merchant-selling-market-presence.md](/home/joeloverbeck/projects/worldwake/archive/specs/S04-merchant-selling-market-presence.md)

## Problem

After the substrate and AI-selection integration land, the repo still needs an end-to-end proof that lawful side-benefit scoring actually changes plan choice in a concrete scenario. Current trade and merchant goldens cover selling, listing, restocking, and source-reliability reranking, but none prove that an agent with both buy and sell pressure prefers the combined market trip because the selected plan also satisfies a secondary destination benefit.

## Assumption Reassessment (2026-04-02)

1. The golden gap is still real. Generated inventory and scenario docs in [`golden-e2e-inventory.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-e2e-inventory.md) and [`golden-scenario-map.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-scenario-map.md) show merchant-selling coverage in scenarios `75`-`87` and trade-side coverage through scenario `94`, but no existing scenario proves “same trip satisfies primary goal while tie-breaking on a secondary destination benefit.”
2. The nearest existing live goldens are in [`golden_merchant_selling.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_merchant_selling.rs) and [`golden_trade.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs). They cover listing persistence, market selling, restock/return, and seller-reranking, but they do not prove side-benefit scoring as the reason a combined market trip wins.
3. The exact shared abstraction boundary under audit is the AI end-to-end planning contract after `002`: ranked primary goals and side-benefit-aware `select_best_plan()` should produce a selected plan whose path lawfully reaches the market for the primary reason and also captures the secondary benefit at that same destination.
4. The live `GoalKind` surface this scenario depends on is still merchant/trade opportunity planning rather than multi-goal search. The scenario should isolate a primary acquisition or restock branch plus a secondary sell-at-market benefit, not a planner that explicitly searches for two goals at once.
5. The intended invariant is selection-and-execution agreement on the same lawful combined-trip branch. The golden must not treat unrelated lawful alternatives as failures if they arise from higher priority class, missing seller knowledge, or absent sale stock; those branches should be excluded from setup.
6. This remains a golden-only ticket if `002` lands cleanly. If the new scenario exposes a production contradiction in selection, plan binding, or execution, the ticket must be corrected before implementation rather than silently left as “tests only.”
7. Ordering for this ticket is decision and execution ordering, not event-log timing alone: the selected side-benefit branch should appear in the decision trace first, and the executed path/world state should confirm that the same market route actually ran.
8. Mismatch + correction: the spec names a generic combined-trip proof, but the live reusable surfaces strongly suggest this belongs with the merchant-market goldens rather than a new planner-only test file. This ticket therefore targets the existing merchant/trade golden area and the generated doc refresh, not a new abstract harness family.

## Architecture Check

1. Proving the behavior in the existing merchant/trade golden surface is cleaner than inventing a synthetic harness because the contract is inherently cross-system: candidate pressure, plan selection, market travel, and sell/acquire behavior all need to line up in one lawful scenario.
2. No backwards-compatibility shims are introduced. This ticket only adds the missing emergent proof and refreshes the generated golden docs.

## Verification Layers

1. The winning branch is chosen because of side-benefit-aware plan selection rather than a missing competing branch -> decision trace in the golden scenario
2. The executed path actually follows the selected market-routed plan -> action trace / selected next-step assertions in the golden scenario
3. The scenario reaches authoritative world-state consequences consistent with the combined market trip -> authoritative world state and/or event-log delta assertions in the golden test
4. Deterministic replay preserves the same side-benefit-driven route choice -> replay companion golden
5. Additional lower-layer mapping is not required here because `002` already owns the focused selection and trace substrate proof; this ticket owns the end-to-end emergent contract

## What to Change

### 1. Add the combined-trip golden scenario

Extend the existing merchant/trade golden surface with a scenario where an agent has a primary market-directed acquisition/replenishment reason and a secondary lawful sell-at-market benefit at the same destination, then prove that the side-benefit-aware selection path prefers that combined trip.

### 2. Add deterministic replay coverage

Add the replay companion proving the same combined-trip selection and downstream execution remain deterministic across reruns.

### 3. Refresh generated golden docs

Run the inventory refresh so [`golden-e2e-inventory.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-e2e-inventory.md), [`golden-scenario-map.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-scenario-map.md), and [`golden-coverage-matrix.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-coverage-matrix.md) reflect the new scenario.

## Files to Touch

- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/docs/generated/golden-e2e-inventory.md` (modify)
- `/home/joeloverbeck/projects/worldwake/docs/generated/golden-scenario-map.md` (modify)
- `/home/joeloverbeck/projects/worldwake/docs/generated/golden-coverage-matrix.md` (modify)

## Out of Scope

- New production changes outside the side-benefit selection work already owned by `S39LIMSIDBEN-002`
- A second golden proving every possible side-benefit combination; one concrete merchant-market emergent chain is enough here
- Refreshing unrelated golden docs beyond the inventory/doc outputs produced by the required script

## Acceptance Criteria

### Tests That Must Pass

1. A merchant/trade golden proves a combined market trip wins because the selected path lawfully captures a secondary destination benefit in addition to the primary goal.
2. The replay companion proves the same result deterministically.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The scenario must prove side-benefit selection on top of the existing single-goal planner; it must not rely on adding explicit multi-goal search or handcrafted extra plan steps.
2. Generated golden docs must remain in sync with the new scenario and replay companion.

## Test Plan

### New/Modified Tests

1. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_merchant_selling.rs` — end-to-end proof that a lawful combined market trip wins because of side-benefit scoring and executes that same route.
2. `/home/joeloverbeck/projects/worldwake/docs/generated/golden-e2e-inventory.md`, `/home/joeloverbeck/projects/worldwake/docs/generated/golden-scenario-map.md`, and `/home/joeloverbeck/projects/worldwake/docs/generated/golden-coverage-matrix.md` — generated doc refresh for the new scenario.

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling -- --nocapture`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`
