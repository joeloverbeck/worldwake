# S84SHBELOP-004: Golden test — ShareBelief succeeds under co-location

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — reassessment-only closeout
**Deps**: S84SHBELOP-001

## Problem

The original ticket assumed no golden E2E test yet proved the full local ShareBelief pipeline. Live reassessment showed that the repository already contains stronger golden coverage for this contract in `golden_emergent.rs`, including the S84-specific supply-depletion scenario and an additional same-place Tell chain.

## Assumption Reassessment (2026-04-10)

1. **`golden_social.rs` is not the owning surface for this contract**: it covers social omission, resend, and payload-focused social behavior, but the strongest existing ShareBelief E2E proofs already live in `crates/worldwake-ai/tests/golden_emergent.rs`.
2. **GoalKind::ShareBelief confirmed**: At `goal.rs:95-99` with fields `{ listener, topic, communication_class }`. Three dispatch variants at `goal_dispatch_decl.rs:507-533`.
3. **Shared boundary**: The relevant E2E proof surface is still the full agent decision pipeline: candidate generation → planning snapshot construction → search (`PlannerOpKind::Tell`) → action execution → tell commit → listener belief update.
4. **S84-specific golden already exists**: `run_supply_depletion_enables_share_belief` / `golden_supply_depletion_enables_share_belief` in `crates/worldwake-ai/tests/golden_emergent.rs` seeds a co-located speaker/listener pair at `VILLAGE_SQUARE`, asserts the `ShareBelief` candidate appears on the first post-refresh planning tick, waits for committed `tell`, and proves the listener's belief store updates from a `Report` source.
5. **Additional same-place ShareBelief coverage also exists**: the same-place office-holder Tell scenario in `crates/worldwake-ai/tests/golden_emergent.rs` proves same-place ShareBelief generation, committed `tell`, listener belief uptake, and downstream causal consequences after the Tell.
6. **Live GoalKind/operator contract matches the existing goldens**: `GoalKind::ShareBelief` uses `PlannerOpKind::Tell`, and `docs/planner-contracts.md` confirms ShareBelief has an explicit synthesized terminal root surface.
7. **Scenario isolation is already handled in the existing S84 golden**: the speaker and listener are co-located, `TellProfile` and `PerceptionProfile` are authored deliberately, and competing goals are bounded by the fixture rather than left to chance.

## Architecture Check

1. A golden E2E test is still the correct proof surface for this contract, but the repository already has that proof. Adding another scenario in `golden_social.rs` would duplicate existing coverage rather than fill a missing architectural gap.
2. No backward-compatibility shims. Because reassessment found the intended golden already landed, the correct outcome is to close the ticket without code changes.

## Verification Layers

1. Existing golden asserts ShareBelief candidate generation for a co-located listener -> decision trace
2. Existing golden asserts committed `tell` for the ShareBelief topic -> action trace
3. Existing golden asserts listener belief update from `Report` after Tell -> authoritative belief state
4. Existing fixture setup already authors the needed perception/tell profiles and local co-location constraints

## What to Change

### 1. Reassess the owning golden surface

Verify whether the full ShareBelief co-location chain is already covered in an existing `golden_*` suite before adding a new scenario. Reuse the strongest existing owning surface rather than duplicating it in `golden_social.rs`.

### 2. Confirm the existing S84 proof still covers the ticket contract

Confirm that the existing S84 golden proves:
- co-located speaker/listener setup
- `ShareBelief` candidate generation
- committed `tell` for the topic under test
- listener belief uptake through the Tell/report path

### 3. Close the ticket to the live ownership boundary

If the existing goldens already prove the contract, update this ticket to the confirmed owning suite and verification rather than adding a second scenario.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (read-only reassessment reference)
- `crates/worldwake-ai/tests/golden_social.rs` (read-only non-owner reference)
- `docs/generated/golden-e2e-inventory.md` (read-only reassessment reference)

## Out of Scope

- Adding a duplicate ShareBelief co-location scenario to `golden_social.rs`
- Testing ShareBelief when agents are NOT co-located (expected failure case)
- Testing tell action payload structure (already covered by existing social goldens)
- Testing multiple communication classes beyond the already proven live scenarios

## Acceptance Criteria

### Tests That Must Pass

1. Existing `golden_supply_depletion_enables_share_belief` still passes
2. Existing deterministic replay companion still passes
3. The owning golden inventory already lists the ShareBelief scenario

### Invariants

1. ShareBelief planning remains local/co-located in the owning golden setup
2. The existing S84 golden continues to prove the Tell commit and listener belief uptake path
3. Golden ownership remains with the stronger existing `golden_emergent.rs` scenarios rather than a new duplicate in `golden_social.rs`

## Test Plan

### New/Modified Tests

1. Existing `crates/worldwake-ai/tests/golden_emergent.rs` ShareBelief scenarios

### Commands

1. `cargo test -p worldwake-ai golden_supply_depletion_enables_share_belief`

## Outcome

Completed on 2026-04-10.

- Reassessed the claimed golden coverage gap against the live golden inventory and found that the repository already contains stronger ShareBelief co-location E2E coverage than this ticket proposed to add.
- Confirmed `crates/worldwake-ai/tests/golden_emergent.rs` already proves the S84 contract through `golden_supply_depletion_enables_share_belief`, including ShareBelief candidate generation, committed `tell`, and listener belief uptake from a report source.
- Confirmed `golden_emergent.rs` also contains an additional same-place office-holder Tell chain that exercises same-place ShareBelief generation and downstream consequence ordering.
- Closed the ticket without code changes because adding a second duplicate scenario in `golden_social.rs` would not prove a materially new contract.

## Verification Result

- Passed `cargo test -p worldwake-ai golden_supply_depletion_enables_share_belief`
