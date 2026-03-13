# E13DECARC-018: Search regression coverage for consume-target commodity matching

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — AI-layer search/planner regression coverage
**Deps**: `archive/tickets/completed/E13DECARC-010-planner-op-kinds-and-semantics-table.md`, `archive/tickets/completed/E13DECARC-012-plan-search-and-selection.md`, `archive/tickets/completed/GOLDENE2E-010-three-way-need-competition.md`

## Problem

The recent GOLDENE2E-010 work fixed a real planner bug: hypothetical consume transitions were commodity-blind, so a `drink` step could satisfy a bread-consume goal in planning. The fix is covered at the transition layer and by a golden runtime scenario, but there is still no focused `search_plan` regression proving that bounded search itself cannot assemble an invalid consume plan from mismatched local stock.

That leaves an important test gap at the actual caller boundary for `apply_hypothetical_transition()`. If a future change regresses target binding or search candidate filtering, the current test set could fail late or indirectly instead of catching the planner error at the search layer.

## Assumption Reassessment (2026-03-13)

1. `apply_hypothetical_transition()` in `crates/worldwake-ai/src/planner_ops.rs` now rejects `PlannerOpKind::Consume` transitions whose target commodity does not match `GoalKind::ConsumeOwnedCommodity { commodity }` (confirmed).
2. `search_plan()` in `crates/worldwake-ai/src/search.rs` is the direct caller path that builds bounded plans from affordances plus planner-only candidates (confirmed).
3. Existing `search.rs` tests cover successful local consume and adjacent travel-then-consume, but they do **not** cover the negative case where only the wrong consumable is locally controllable (confirmed).
4. The nearby pickup-oriented `search.rs` tests exercise `AcquireCommodity` satisfaction/progress behavior, not `ConsumeOwnedCommodity` target matching. They should not be treated as consume-regression coverage for this bug.
5. `AcquireCommodity` search paths are already covered elsewhere. This ticket should stay focused on the regression actually exposed by GOLDENE2E-010 rather than reopening broader consume/acquire semantics.

## Architecture Check

1. A focused `search_plan` regression is cleaner than relying only on a low-level transition test plus a broad golden scenario. It tests the exact search-layer contract without duplicating runtime behavior.
2. This ticket does not add compatibility shims or fallback logic. It only locks in the corrected planner behavior at the proper abstraction boundary.
3. Keeping this regression in `search.rs` is better than hiding it in a golden test because the failure would then point directly at the planner/search seam instead of surfacing as a distant AI-loop symptom.

## What to Change

### 1. Add a negative search regression for mismatched local consumables

In `crates/worldwake-ai/src/search.rs`, add a test that:

- builds a planning snapshot for an agent with a local, controllable `Water` lot but no `Bread`
- uses goal `ConsumeOwnedCommodity { commodity: Bread }`
- confirms `search_plan(...)` returns `None`

This must prove the search layer cannot produce a one-step local consume plan for the wrong commodity.

### 2. Keep a positive control for lawful local bread consume planning

In the same test module, keep a clear positive control showing that:

- local `Bread` still produces a valid consume plan
- the regression remains narrowly scoped to commodity matching rather than accidentally disabling lawful local consume planning

The existing `search_returns_one_step_consume_plan_for_local_food` test already covers the positive path clearly, so this ticket should prefer strengthening that seam with a new negative regression rather than adding a redundant duplicate.

### 3. Document the planner boundary this regression protects

Add a short test comment or naming clarification explaining that the regression protects the `search_plan -> apply_hypothetical_transition` seam, not the runtime interrupt/commit ordering behavior already covered by GOLDENE2E-010.

## Files to Touch

- `crates/worldwake-ai/src/search.rs` (modify)

## Out of Scope

- Reworking planner transition semantics beyond regression coverage
- New golden scenarios
- Changes to `rank_candidates()`
- Changes to runtime interrupt behavior

## Acceptance Criteria

### Tests That Must Pass

1. A `ConsumeOwnedCommodity { Bread }` search attempt returns `None` when the only local controllable consumable is `Water`
2. A lawful local bread-consume search path still returns a valid plan
3. Existing suite: `cargo test -p worldwake-ai search`
4. Existing suite: `cargo test --workspace`
5. Existing suite: `cargo clippy --workspace`

### Invariants

1. Search must not treat different consumable commodities as interchangeable for consume goals
2. No compatibility shim or special-case runtime fallback is introduced
3. Deterministic planner data structures remain `BTreeMap` / `BTreeSet` only

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` — add a negative regression for mismatched local consume targets; rely on the existing positive local-consume test as the control unless a tiny clarification is needed for readability

### Commands

1. `cargo test -p worldwake-ai search`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-13
- **What actually changed**:
  - Corrected the ticket assumptions first: `search.rs` had positive local-consume coverage, but the nearby pickup tests were for `AcquireCommodity`, not consume-target matching.
  - Added `search_returns_none_when_only_wrong_local_consumable_is_controllable` in `crates/worldwake-ai/src/search.rs` to lock the `search_plan -> apply_hypothetical_transition` seam against cross-commodity consume regressions.
  - Added a tiny shared consumable-lot test helper so the new negative case could reuse the same local fixture style as the existing positive consume coverage.
- **Deviations from original plan**:
  - No planner or runtime code changed. The current architecture is already the cleaner long-term design: commodity validation lives in `apply_hypothetical_transition()`, and `search.rs` now asserts that its caller boundary respects that contract.
  - The ticket now explicitly relies on the existing `search_returns_one_step_consume_plan_for_local_food` test as the positive control instead of duplicating a second bread-success test.
- **Verification results**:
  - `cargo test -p worldwake-ai search`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
