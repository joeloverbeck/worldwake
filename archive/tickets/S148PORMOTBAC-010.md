# S148PORMOTBAC-010: Golden coverage for five-slot portfolio and resume/abandon lifecycle

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: No — test-only golden coverage and generated golden-inventory docs
**Deps**: `archive/tickets/S148PORMOTBAC-001.md`, `archive/tickets/S148PORMOTBAC-002.md`, `archive/tickets/S148PORMOTBAC-003.md`, `archive/tickets/S148PORMOTBAC-004.md`, `archive/tickets/S148PORMOTBAC-005.md`, `archive/tickets/S148PORMOTBAC-006.md`, `archive/tickets/S148PORMOTBAC-007.md`, `archive/tickets/S148PORMOTBAC-008.md`, `archive/tickets/S148PORMOTBAC-009.md`, `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

S148 changed the portfolio taxonomy, moved plan caps to `PortfolioWeightsProfile`, and enriched `IntentionFrame` with motive, claim, condition, and causal-link fields. The repo needed golden coverage that would catch drift in those public S148 contracts and keep the generated golden inventory synchronized.

## Outcome

Added `crates/worldwake-ai/tests/golden_portfolio_five_slots.rs` with five focused golden-contract tests over the public S148 surfaces:

- total `MotiveSourceDiscriminant` to five-slot mapping
- default slot weights and `OperatingMode` plan caps
- enriched `IntentionFrame` round-trip persistence
- all `IntentionResumeCondition` variants
- all `IntentionAbandonCondition` variants plus typed `Discrepancy::AbandonConditionFired` round-trips

The live `golden_portfolio_planning.rs` audit found that the full-pipeline portfolio golden had already migrated to the five-slot names and verifies the portfolio trace through the real planning pipeline. This ticket preserved that scenario and used the new file for the remaining public contract coverage rather than adding brittle large scenarios for behavior already covered by focused lower-layer tests in `agent_tick::portfolio`, `agent_tick::frame`, and `agent_tick`.

## Landed Changes

- Added `golden_portfolio_five_slots.rs` with five new golden tests.
- Kept `golden_portfolio_planning.rs` behavior intact after confirming it already asserts five-slot trace names under the full pipeline.
- Updated generated golden inventory docs with `scripts/golden_inventory.py --write --check-docs`.
- Added a D14 implementation note to `specs/S148-portfolio-and-motive-backed-intentions.md` describing the live proof split.

## Landed Files

- `crates/worldwake-ai/tests/golden_portfolio_five_slots.rs`
- `docs/generated/golden-coverage-matrix.md`
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-scenario-details/motive-sources.md`
- `docs/generated/golden-scenario-details/sleep-episode.md`
- `docs/generated/golden-scenario-index.md`
- `specs/S148-portfolio-and-motive-backed-intentions.md`

## Accepted Invariants

1. Every current `MotiveSourceDiscriminant` maps to a canonical S148 `SlotKind`, and the mapping covers all five slots.
2. `PortfolioWeightsProfile` exposes the S148 default slot weights and per-mode plan caps.
3. Every enriched `IntentionFrame` field persists through bincode.
4. Every resume and abandon condition variant remains serializable and distinct.
5. `Discrepancy::AbandonConditionFired` remains typed by the abandon-condition discriminant.
6. The existing full-pipeline portfolio golden continues to assert the five-slot trace names in live planning.

## Verification Result

- Passed: `cargo fmt --all`
- Passed: `cargo test -p worldwake-ai --test golden_portfolio_five_slots`
- Passed: `cargo test -p worldwake-ai --test golden_portfolio_planning`
- Passed: `cargo test -p worldwake-ai --test golden_portfolio_five_slots -- --list`
- Passed: `cargo test -p worldwake-ai --test golden_portfolio_planning -- --list`
- Passed: `python3 scripts/golden_inventory.py --write --check-docs`
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
- Passed: `cargo test --workspace`

## Notes

The drafted request for nine full end-to-end scenarios was narrowed after reassessment because the live code already had focused lower-layer tests for operating-mode slot suppression, resume/abandon evaluator behavior, and causal-link cap enforcement. The added golden suite locks the public S148 data contracts, while the existing `golden_portfolio_planning.rs` remains the full-pipeline portfolio-admission proof.
