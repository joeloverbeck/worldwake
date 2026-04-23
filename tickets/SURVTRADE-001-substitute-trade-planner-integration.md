# SURVTRADE-001: Integrate Substitute Trade Selection Into the AcquireCommodity Pipeline

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI candidate generation / planning plus trade affordance integration
**Deps**: `docs/scenario-roadmap.md` row 9 `survival-trade`; archived `E11TRAECO-010-substitute-demand`

## Problem

`survival-trade.ron` now truthfully proves repeated local market trade under a survival-health contract, but row 9 still cannot land because authored `SubstitutePreferences` never become an explicit substitute trade choice in the live AI/runtime pipeline. The helper seam exists, yet the planner still asks for the originally desired commodity and the trade runtime still enumerates only concrete listed lots of that same commodity.

## Assumption Reassessment (2026-04-23)

1. `scenarios/survival-trade.ron` and `crates/worldwake-ai/tests/golden_survival_trade.rs` now prove a 1440-tick survival run where a buyer repeatedly purchases Bread from a listed merchant lot, but they do not prove substitute-driven commodity replacement.
2. `archive/tickets/completed/E11TRAECO-010-substitute-demand.md` already narrowed the substitute seam to explicit candidate selection in `crates/worldwake-systems/src/trade_actions.rs`; it explicitly left planner/GOAP integration out of scope.
3. The exact shared abstraction boundary under audit is `GoalKind::AcquireCommodity { commodity, purpose: SelfConsume }` flowing into trade affordance generation and plan construction for explicit `trade` actions.
4. The current affordance seam in `crates/worldwake-systems/src/trade_actions.rs` only enumerates payloads from concrete listed sale lots in the seller's `sale_kinds`; it never calls `select_substitute_trade_candidate`.
5. The current substitute helper already exists at `crates/worldwake-systems/src/trade_actions.rs::select_substitute_trade_candidate(...)` and returns a deterministic, valuation-approved local substitute candidate without mutating world state.
6. The current candidate/planner pipeline still treats `AcquireCommodity` as a request for one concrete commodity; reassessment did not find any live AI call site that turns substitute preferences into a new explicit trade request before planning or start-time validation.
7. The motivating scenario invariant is: when the desired self-consume commodity is unavailable or valuation-rejected, but a locally available substitute is accepted in stored preference order, the agent should explicitly pursue that substitute trade rather than idling, exploring unrelatedly, or waiting for the original commodity.
8. Ordering matters at the decision/planning layer, not only at event-log outcome time: the proof must distinguish "selected substitute trade branch" from a later accidental survival outcome through another lawful food path.
9. This ticket is not a commit-time payload rewrite ticket. The existing architecture requirement from `E11TRAECO-010` stands: substitute pursuit must become a new explicit trade proposal, not a hidden swap inside `commit_trade`.
10. Adjacent contradictions exposed by reassessment:
    - required consequence of this ticket: explicit substitute-aware trade affordance or planning integration for `AcquireCommodity(SelfConsume)`
    - future cleanup: broader substitute support for recipe-input or restock goals if row 9 only needs self-consume today
11. Mismatch + correction: the roadmap row originally grouped commodity valuation and substitute preferences together as one landing, but live proof now shows substitute preferences are the blocking seam while valuation is only supporting substrate in the current scenario.

## Architecture Check

1. The clean path is to keep substitute choice explicit at the goal/affordance layer: candidate generation or trade-affordance construction should ask the deterministic helper for an acceptable substitute and then emit a normal concrete trade plan for that substitute commodity.
2. This avoids hidden runtime mutation, keeps action traces truthful, and preserves symmetry between AI and any future human-initiated trade flow.

## Verification Layers

1. Substitute food unavailable -> decision trace shows `AcquireCommodity(SelfConsume)` selecting a substitute-backed trade branch rather than only the original commodity or a rival exploration branch.
2. Selected substitute branch -> action trace shows explicit `trade` against the substitute commodity's listed lot.
3. Successful substitute trade -> authoritative world state shows the substitute commodity and coin transfer at the buyer/seller seam.
4. Scenario isolation -> a roadmap golden or focused scenario names the excluded rival lawful branches so the substitute proof is not inferred from survival alone.
5. Lower-layer seam -> focused tests prove the substitute-aware affordance/planner bridge calls the helper without introducing commit-time payload rewriting.

## What to Change

### 1. Substitute-aware acquire/trade integration

Wire `GoalKind::AcquireCommodity { purpose: SelfConsume }` into an explicit substitute-selection path when the originally desired commodity lacks a viable local trade path but a deterministic substitute candidate is locally available and valuation-approved.

### 2. Truthful proof surfaces

Add focused coverage and a roadmap-compatible golden or scenario assertion that proves the substitute branch is selected for the intended causal reason, not merely that the agent survives somehow.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` and/or adjacent AcquireCommodity planning surfaces (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify if a public affordance bridge is needed)
- `crates/worldwake-ai/tests/` (modify/add focused proof)
- `docs/scenario-roadmap.md` (modify when the row can be truthfully landed)

## Out of Scope

- Commit-time rewriting of an already-started trade action payload
- New demand-memory observation recording
- Recipe-input or merchant-restock substitute planning unless reassessment proves row 9 needs them to land
- Broad market redesign outside the explicit substitute-trade seam

## Acceptance Criteria

### Tests That Must Pass

1. A focused AI/planner test proves substitute-backed `AcquireCommodity(SelfConsume)` selection when the preferred commodity is not the chosen viable trade path.
2. A trade/runtime test proves the emitted substitute trade remains an explicit normal trade action and transfers the substitute commodity authoritatively.
3. Existing suite: `cargo test -p worldwake-ai survival_trade_proves_live_market_branch -- --ignored --exact --test-threads=1`

### Invariants

1. Substitute pursuit remains an explicit new trade proposal; no hidden payload mutation is introduced.
2. Substitute selection remains deterministic, local, and valuation-approved.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/...` — prove substitute-backed `AcquireCommodity(SelfConsume)` branch selection at the decision/planning layer.
2. `crates/worldwake-systems/src/trade_actions.rs` or adjacent AI integration tests — prove the helper bridge emits explicit substitute trade terms without mutating commit-time payloads.
3. `crates/worldwake-ai/tests/golden_survival_trade.rs` — extend or complement the roadmap golden only if the substitute branch becomes the truthful row-landing proof.

### Commands

1. `cargo test -p worldwake-ai <focused substitute test>`
2. `cargo test -p worldwake-systems trade_actions`
3. `cargo test -p worldwake-ai survival_trade_proves_live_market_branch -- --ignored --exact --test-threads=1`
4. `cargo clippy --workspace --all-targets -- -D warnings`
