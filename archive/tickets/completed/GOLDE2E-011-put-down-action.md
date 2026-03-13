# GOLDE2E-011: Put-Down Action (Inventory Management)

**Status**: ✅ COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None required
**Deps**: None

## Problem

The original ticket assumed the golden suite was missing a real-AI-loop proof for `put_down`, and that merchant restock or cargo-delivery behavior should naturally exercise that action.

That assumption is not correct for the current architecture.

The codebase already has:
- a concrete `put_down` transport action and handler in [transport_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/transport_actions.rs);
- focused transport tests that execute `put_down` directly, including scheduler/replay coverage in [e10_production_transport_integration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/tests/e10_production_transport_integration.rs);
- planner/runtime tests proving the actual cargo-delivery invariant: `MoveCargo` is satisfied once controlled stock reaches the destination place, even if the merchant is still carrying that stock locally.

What was stale was the ticket and the report backlog item, not the engine.

## Report Reference

Backlog item **P15** in [golden-e2e-coverage-analysis.md](/home/joeloverbeck/projects/worldwake/reports/golden-e2e-coverage-analysis.md) (Tier 3, composite score 2). This ticket resolves that stale backlog item by correcting the documented assumptions and scope.

## Assumption Reassessment (2026-03-13)

1. `put_down` exists and is already covered below the golden layer:
   - action registration/handler tests in [transport_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/transport_actions.rs);
   - integration/replay coverage in [e10_production_transport_integration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/tests/e10_production_transport_integration.rs).
2. The AI does not currently need a generic reason to call `put_down` for merchant restock. The implemented cargo architecture treats destination-local controlled stock as sufficient to satisfy `GoalKind::MoveCargo`.
3. That invariant is already explicit in focused AI tests:
   - [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) proves `MoveCargo` is satisfied when destination stock is local.
   - [agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs) proves cargo satisfaction at destination while still carrying.
4. `golden_merchant_restock_return_stock` in [golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs) already covers the behavior-level emergent loop the architecture promises: leave home, acquire stock, carry it back to the home market, and satisfy the restock path.
5. A new golden test that requires `put_down` through the AI loop would not verify the current design. It would instead force a new architectural promise that the system does not presently make.

## Architecture Check

1. The current architecture is cleaner than the ticket assumed:
   - `put_down` is a valid transport primitive;
   - `MoveCargo` is a destination-delivery goal;
   - merchant restock does not depend on ground-dropping inventory at the destination.
2. Forcing AI restock to end in `put_down` would only be more robust if the design also introduced stronger place-local storage semantics such as explicit stock containers, seller-access rules, and trade depending on deposited inventory rather than on agent-controlled local stock.
3. That broader redesign could be worthwhile in the future, but adding a golden test that implies such a redesign without changing the underlying architecture would be misleading and brittle.
4. The better long-term design, if this area is expanded, is:
   - keep `put_down` as a concrete primitive;
   - only make AI depend on it when a higher-level world rule genuinely requires deposited stock;
   - model that requirement through explicit containers, custody, and access, not through a test-only expectation.

## Engine-First Mandate

If future work changes merchant restock or local selling to require explicit deposited stock, the engine should be updated directly and the golden suite should then add a behavior-level `put_down` proof. This ticket does not justify that architectural change today.

## What to Change

### 1. Correct the ticket scope

Record that the original requested golden scenario does not match the current cargo architecture.

### 2. Remove the stale backlog item from the report

Update [golden-e2e-coverage-analysis.md](/home/joeloverbeck/projects/worldwake/reports/golden-e2e-coverage-analysis.md) so it no longer treats `put_down` as missing golden coverage for the present architecture.

### 3. Keep code and tests unchanged unless a real gap is found

Reassessment did not uncover an engine bug or a missing invariant. Existing focused tests already cover the critical cargo semantics.

## Files to Touch

- [golden-e2e-coverage-analysis.md](/home/joeloverbeck/projects/worldwake/reports/golden-e2e-coverage-analysis.md)
- [GOLDE2E-011-put-down-action.md](/home/joeloverbeck/projects/worldwake/tickets/GOLDE2E-011-put-down-action.md) (then archive)

## Out of Scope

- Changing `MoveCargo` to require `put_down`
- Adding a redundant golden scenario that contradicts current cargo goal semantics
- Adding compatibility aliases or shims around cargo delivery semantics
- Designing explicit stock-container architecture without a separate ticket/spec

## Acceptance Criteria

### Tests That Must Pass

1. Focused cargo-semantics tests still pass.
2. Relevant golden merchant-restock coverage still passes.
3. Full workspace tests pass.
4. `cargo clippy --workspace` passes.
5. [golden-e2e-coverage-analysis.md](/home/joeloverbeck/projects/worldwake/reports/golden-e2e-coverage-analysis.md) no longer lists P15 as a missing golden scenario.

### Invariants

1. No backward-compatibility shims or alias paths are introduced.
2. `MoveCargo` remains destination-local delivery, not implicit forced `put_down`.
3. Golden coverage stays aligned with actual architecture rather than imagined future behavior.

## Post-Implementation

Archive this ticket with an Outcome section documenting that the work was a reassessment-and-correction pass, not an engine change.

## Test Plan

### New/Modified Tests

- None. Reassessment found the relevant invariants already covered by existing focused tests and the existing merchant restock golden scenario.

### Commands

1. `cargo test -p worldwake-ai move_cargo_satisfied_when_destination_stocked`
2. `cargo test -p worldwake-ai cargo_satisfaction_at_destination_while_carrying`
3. `cargo test -p worldwake-ai golden_merchant_restock_return_stock`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

### Completion date

2026-03-13

### What actually changed

- Corrected the ticket assumptions to match the implemented cargo architecture and current test coverage.
- Removed the stale P15 backlog item from [golden-e2e-coverage-analysis.md](/home/joeloverbeck/projects/worldwake/reports/golden-e2e-coverage-analysis.md).
- Kept engine code and tests unchanged because the requested `put_down`-via-AI behavior is not part of the current design contract.

### Deviations from the original plan

- The original plan proposed a new golden test that would prove `put_down` through the real AI loop.
- Reassessment showed that this would be a behavior-architecture mismatch rather than a genuine coverage improvement:
  - `put_down` already has lower-layer coverage;
  - current `MoveCargo` semantics intentionally complete at destination-local control;
  - existing merchant-restock golden coverage already proves the behavior the system actually promises.

### Verification results

- `cargo test -p worldwake-ai move_cargo_satisfied_when_destination_stocked`
- `cargo test -p worldwake-ai cargo_satisfaction_at_destination_while_carrying`
- `cargo test -p worldwake-ai golden_merchant_restock_return_stock`
- `cargo test --workspace`
- `cargo clippy --workspace`
