# SURVTRADE-001: Integrate Substitute Trade Selection Into the AcquireCommodity Pipeline

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI candidate generation plus self-authoritative substitute-preference belief access
**Deps**: `docs/scenario-roadmap.md` row 9 `survival-trade`; archived `E11TRAECO-010-substitute-demand`

## Problem

`survival-trade.ron` now truthfully proves repeated local market trade under a survival-health contract, but row 9 still cannot land because authored `SubstitutePreferences` never become an explicit substitute trade choice in the live AI/runtime pipeline. The helper seam exists, yet the planner still asks for the originally desired commodity and the trade runtime still enumerates only concrete listed lots of that same commodity.

## Assumption Reassessment (2026-04-23)

1. `scenarios/survival-trade.ron` and `crates/worldwake-ai/tests/golden_survival_trade.rs` now prove a 1440-tick survival run where a buyer repeatedly purchases Bread from a listed merchant lot, but they do not prove substitute-driven commodity replacement.
2. `archive/tickets/completed/E11TRAECO-010-substitute-demand.md` already narrowed the substitute seam to explicit candidate selection in `crates/worldwake-systems/src/trade_actions.rs`; it explicitly left planner/GOAP integration out of scope.
3. The exact shared abstraction boundary under audit is `GoalKind::AcquireCommodity { commodity, purpose: SelfConsume }` flowing into trade affordance generation and plan construction for explicit `trade` actions.
4. The current affordance seam in `crates/worldwake-systems/src/trade_actions.rs` only enumerates payloads from concrete listed sale lots in the seller's `sale_kinds`; it never calls `select_substitute_trade_candidate`.
5. The current substitute helper already exists at `crates/worldwake-systems/src/trade_actions.rs::select_substitute_trade_candidate(...)` and returns a deterministic, valuation-approved local substitute candidate without mutating world state.
6. The live AI branch had no self-authoritative substitute-preference read on `GoalBeliefView`, so candidate generation could not emit a substitute-backed explicit `AcquireCommodity(SelfConsume)` trade goal from stored substitute order even though `trade_actions.rs` already had deterministic substitute valuation logic on the authoritative side.
7. The motivating scenario invariant is: when the desired self-consume commodity is unavailable or valuation-rejected, but a locally available substitute is accepted in stored preference order, the agent should explicitly pursue that substitute trade rather than idling, exploring unrelatedly, or waiting for the original commodity.
8. Ordering still matters at the decision/planning layer, but the strongest complete slice available on the live branch is earlier: explicit substitute-goal emission from candidate generation. The stronger "selected substitute branch wins row 9" claim remains follow-up work.
9. This ticket is not a commit-time payload rewrite ticket. The existing architecture requirement from `E11TRAECO-010` stands: substitute pursuit must become a new explicit trade proposal, not a hidden swap inside `commit_trade`.
10. Adjacent contradictions exposed by implementation:
    - required consequence of this ticket: explicit substitute-aware self-consume goal emission from candidate generation using stored substitute order and trade valuation approval
    - follow-up: row-9 selection/golden proof still needs a dedicated owner because the live branch can still surface rival same-category acquisition candidates after the substitute goal is emitted
11. Mismatch + correction: the truthful landed seam is narrower than the original row-landing draft. This ticket now owns the AI-side explicit substitute goal emission bridge; roadmap-row selection/golden proof moved to follow-up `SURVTRADE-002`.

## Architecture Check

1. The clean path is to keep substitute choice explicit at the goal/affordance layer: candidate generation or trade-affordance construction should ask the deterministic helper for an acceptable substitute and then emit a normal concrete trade plan for that substitute commodity.
2. This avoids hidden runtime mutation, keeps action traces truthful, and preserves symmetry between AI and any future human-initiated trade flow.

## Verification Layers

1. Self-authoritative substitute preferences -> AI candidate generation can read stored substitute order through `GoalBeliefView` without breaking the belief/planner boundary.
2. Preferred substitute bridge -> focused AI coverage proves self-consume candidate generation emits an explicit substitute commodity goal with seller-backed evidence when the preferred local commodity trade path is missing.
3. Valuation/order substrate -> existing lower-layer trade-actions coverage still proves substitute selection skips valuation-rejected earlier preferences for later acceptable ones.
4. CI-shaped safety -> workspace `clippy --all-targets -D warnings` stays green after widening the belief-view surface.

## What to Change

### 1. Substitute-aware self-consume goal emission

Wire `GoalKind::AcquireCommodity { purpose: SelfConsume }` candidate generation into an explicit substitute-selection path when the preferred local commodity trade path is missing and a deterministic substitute candidate is locally available and valuation-approved.

### 2. Truthful proof surfaces

Add focused AI coverage for the explicit substitute goal-emission seam and keep the stronger row-9 selection/golden proof out of this ticket.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify)

## Out of Scope

- Commit-time rewriting of an already-started trade action payload
- Goal-selection/ranking policy that suppresses every rival same-category substitute candidate
- Roadmap row-9 golden landing or `docs/scenario-roadmap.md` edits
- Recipe-input or merchant-restock substitute planning

## Acceptance Criteria

### Tests That Must Pass

1. A focused AI/planner test proves substitute-backed `AcquireCommodity(SelfConsume)` selection when the preferred commodity is not the chosen viable trade path.
2. Existing lower-layer trade-actions coverage proves the substitute helper still honors stored order while skipping valuation-rejected earlier candidates.
3. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Substitute pursuit remains an explicit new trade proposal; no hidden payload mutation is introduced.
2. Substitute selection remains deterministic, local, and valuation-approved.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — prove candidate generation emits the preferred substitute-backed explicit self-consume goal with seller-backed evidence.
2. `crates/worldwake-systems/src/trade_actions.rs` — preserve valuation/order substitute helper coverage at the lower trade seam.
3. `None — roadmap golden row-landing proof moved to follow-up SURVTRADE-002.`

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::unavailable_local_food_emits_preferred_substitute_trade_goal -- --exact`
2. `cargo test -p worldwake-systems trade_actions::tests::substitute_selection_skips_valuation_rejected_candidate_for_later_acceptable_one -- --exact`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-23.

- Added self-authoritative `SubstitutePreferences` access to the AI belief surface and `PerAgentBeliefView`.
- Added a goal-view trade helper bridge so AI candidate generation can reuse substitute valuation/order logic without introducing commit-time payload rewriting.
- `AcquireCommodity(SelfConsume)` candidate generation now emits an explicit preferred substitute commodity goal with seller-backed evidence when the preferred local trade path is missing.
- Created follow-up `SURVTRADE-002` for the still-open row-9 selection/golden proof seam.

## Deviations

- The truthful landed seam is narrower than the original ticket draft. This ticket does not land the stronger roadmap-row claim that the substitute branch is the selected/proved row-9 golden path; it lands the explicit AI substitute-goal emission substrate that the stronger proof depends on.

## Verification Result

- Passed `cargo test -p worldwake-ai candidate_generation::tests::unavailable_local_food_emits_preferred_substitute_trade_goal -- --exact`
- Passed `cargo test -p worldwake-systems trade_actions::tests::substitute_selection_skips_valuation_rejected_candidate_for_later_acceptable_one -- --exact`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
