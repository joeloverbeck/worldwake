# E15BSOCAIGOA-009: Golden social regression closure (T8 verified, T9 add, T10 deferred)

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None expected — golden test and report updates only unless the new regression test exposes a real bug
**Deps**: E15BSOCAIGOA-001 through E15BSOCAIGOA-005 (all Deliverable 1 implementation), E15BSOCAIGOA-006

## Problem

The original ticket assumed Tier 2 tests T8-T10 were still missing. That is no longer true. The autonomous Tell path is already covered in `golden_social.rs`, while the proposed T10 chain assumes an architecture that does not exist in the current engine: Tell transfers `BelievedEntityState`, but enterprise restock is driven by `DemandMemory`, not by social belief transfer. The remaining valid gap in this ticket's slice is an end-to-end suppression regression proving that survival needs suppress autonomous `ShareBelief` behavior.

## Assumption Reassessment (2026-03-15)

1. `GoalKind::ShareBelief`, `GoalKindTag::ShareBelief`, `PlannerOpKind::Tell`, `UtilityProfile::social_weight`, `emit_social_candidates()`, and ShareBelief ranking/suppression are already present in production code.
2. `crates/worldwake-ai/tests/golden_social.rs` already exists and already covers autonomous Tell end-to-end via `golden_agent_autonomously_tells_colocated_peer`, plus five other social scenarios.
3. T9 remains a useful golden regression because the suppression rule currently exists only as lower-level ranking coverage. We still want an end-to-end proof that hungry agents do not gossip before satisfying survival needs.
4. The original T10 assumption is incorrect for the current architecture. Tell currently propagates `BelievedEntityState` only. Merchant restock/enterprise behavior is driven by `DemandMemory` and `MerchandiseProfile`; there is no principled path today from "hearing about market demand" to `RestockCommodity`.
5. Forcing T10 through E15b by overloading Tell to carry trade-demand summaries would be an architectural regression. It would blur the boundary between belief snapshots and trade-memory/state carriers instead of introducing an explicit, first-class market-information artifact in a future spec.
6. `reports/golden-e2e-coverage-analysis.md` already mentions the existing social suite and must stay aligned with the actual test count and scenario list after this ticket lands.

## Note

This ticket now serves as closure for the still-missing end-to-end suppression regression, not as the implementation vehicle for a trade-information cascade. A future ticket can revisit market-demand communication only if the design introduces an explicit causal carrier for that information instead of aliasing it onto Tell's current belief payload model.

## Architecture Check

1. Appends one missing golden regression to existing `golden_social.rs`.
2. Updates `reports/golden-e2e-coverage-analysis.md` so the social coverage summary stays truthful after the new test lands.
3. Does not change production architecture unless the new regression exposes a real bug that must be fixed.
4. Explicitly defers the original T10 idea. A clean future design would likely require a first-class world-state carrier for market demand or demand reports, not a Tell payload expansion.

## What to Change

### 1. Add the missing suppression regression to golden_social.rs

**T9: `golden_survival_needs_suppress_social_goals`**
- Setup: Agent with high social_weight (Permille(900)) and fresh beliefs, but critically hungry. Food available at current location.
- Step simulation autonomously.
- Assert: Agent eats first (ConsumeOwnedCommodity outranks ShareBelief). Verify ShareBelief does not execute before hunger is addressed.
- Checks: Priority ordering (Critical/High > Low), determinism.

### 2. Keep T8 as verified existing coverage

`golden_agent_autonomously_tells_colocated_peer` already provides the end-to-end proof that autonomous `ShareBelief` generation, planning, Tell execution, and downstream listener replanning work without `InputQueue` injection. This ticket should not duplicate it.

### 3. Defer the original T10 idea instead of forcing it into E15b

Do not add `golden_information_cascade_enables_trade` under the current architecture. The engine does not currently translate told beliefs into `DemandMemory`, and that separation is desirable. If the project later wants market-demand propagation, it should be specified as a new causal carrier and implemented cleanly across the trade/information boundary.

### 4. Update the social coverage report

Update `reports/golden-e2e-coverage-analysis.md` so the social test count, suite totals, and scenario description match the post-ticket state.

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (modify — append T9)
- `reports/golden-e2e-coverage-analysis.md` (modify — keep social coverage truthful)

## Out of Scope

- Re-implementing or duplicating T8
- Tests T11-T13 (E15BSOCAIGOA-010)
- Any production redesign that teaches enterprise/restock directly from Tell payloads
- A market-demand information system; that needs its own spec and ticket
- Unrelated refactors in social, trade, or enterprise code

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_needs_suppress_social_goals` passes as a real golden E2E regression.
2. The scenario proves `ConsumeOwnedCommodity` executes before any `ShareBelief` action under critical hunger.
3. No social event is emitted before the agent has addressed the immediate hunger pressure.
4. The scenario verifies deterministic replay.
5. Existing social suite continues to pass under `cargo test -p worldwake-ai --test golden_social`.
6. Updated coverage report matches the actual social suite contents and totals.

### Invariants

1. `ShareBelief` never outranks Critical/High survival behavior in the real AI loop.
2. Autonomous social behavior remains purely AI-driven, with no manual queue injection.
3. Deterministic replay produces identical hashes for the new suppression regression.
4. The codebase retains a clean separation between social belief transfer and trade-demand memory.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — 1 new golden E2E test (`golden_survival_needs_suppress_social_goals`)
2. `reports/golden-e2e-coverage-analysis.md` — update social suite counts/summary

### Commands

1. `cargo test -p worldwake-ai --test golden_social`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Added `golden_survival_needs_suppress_social_goals` to `crates/worldwake-ai/tests/golden_social.rs`.
  - Updated `reports/golden-e2e-coverage-analysis.md` to reflect the seven social golden scenarios and the new suite total.
  - Re-scoped the ticket away from duplicate T8 work and away from the invalid T10 trade-information cascade assumption.
- Deviations from original plan:
  - T8 was already implemented before this ticket started, so it was treated as verified existing coverage rather than re-added.
  - The original T10 was not implemented. Current architecture intentionally keeps social Tell payloads (`BelievedEntityState`) separate from enterprise/trade demand carriers (`DemandMemory`). Forcing T10 into E15b would have coupled unrelated information substrates and weakened the design.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_social` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
