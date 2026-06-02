# AILIBBASE-001: Restore the baseline `worldwake-ai` library suite

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — AI planning/search/candidate/ranking test contracts, depending on reassessment
**Deps**: Discovered during `archive/tickets/AGEFOOREP-003.md` verification; reproduce on clean `HEAD` (`6d627d68`) before coding.

## Problem

`cargo test -p worldwake-ai` currently fails on the clean branch baseline, before any AGEFOOREP-003 scenario edits. This blocks using the package-level AI suite as broad proof for unrelated scenario/golden tickets.

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

## Architecture Check

Repair the failing AI contracts at their owning layers instead of weakening scenario-level proof or treating unrelated goldens as the broad gate. Reassess whether the tests are stale or production behavior regressed before editing assertions.

## Verification Layers

1. Candidate emission and quantity derivation -> focused `candidate_generation` unit test.
2. Sell/MoveCargo satisfaction and staging path -> focused `goal_model`, `search`, and `agent_tick` unit tests.
3. Theft motive arithmetic -> focused `ranking` unit test.
4. Broad package health -> `cargo test -p worldwake-ai`.

## What to Change

### 1. Reassess the seven failing tests

Confirm whether each failure is stale expectation, production regression, or shared setup drift.

### 2. Restore the package-level AI gate

Apply the narrowest owning fixes and rerun the failed focused tests before the full package command.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify if candidate emission is wrong or test fixture drift lives there)
- `crates/worldwake-ai/src/goal_model.rs` (modify if sell satisfaction is wrong or test fixture drift lives there)
- `crates/worldwake-ai/src/ranking.rs` (modify if theft motive arithmetic is wrong or expected value drift lives there)
- `crates/worldwake-ai/src/search/tests.rs` (modify if search expectations or setup drift live there)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify if runtime planning expectations or setup drift live there)

## Out of Scope

- AGEFOOREP-003 survival-theft scenario quantity/proof edits.
- Broad golden scenario retuning unless reassessment proves these library failures change authored scenario behavior.

## Acceptance Criteria

### Tests That Must Pass

1. Focused reruns for each repaired failing test.
2. `cargo test -p worldwake-ai`

### Invariants

1. Merchant stock/listing/cargo planning remains belief-backed and source/sink accountable.
2. Test repairs must not bypass planner/search legality, action preconditions, or ranking motive semantics.

## Test Plan

### New/Modified Tests

1. Existing failing focused tests listed in reassessment — update only after proving whether production or expectations are stale.

### Commands

1. `cargo test -p worldwake-ai <focused failing test selector>`
2. `cargo test -p worldwake-ai`
