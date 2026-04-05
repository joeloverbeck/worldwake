# S53COGEXE-004: Behavioral validation conformance test

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S53COGEXE-003

## Problem

The spec's Behavioral Validation Contract claims `ExecutionBudget` changes should not change goal selection when `CognitiveProfile` is held constant. But S97 already proves `max_node_expansions` is behavior-changing. This ticket implements the concrete reclassification gate: run all golden tests with `ExecutionBudget` at minimum sensible values and identify which fields violate the contract.

## Assumption Reassessment (2026-04-05)

1. S97 at `crates/worldwake-ai/tests/golden_reasoning_diversity.rs:124` — "Search Depth Drives Multi-Step Plan Divergence". Proves `max_node_expansions` changes agent behavior. Confirmed.
2. After ticket 003, all agents have `CognitiveProfile` and `ExecutionBudget` as separate components.
3. Golden tests in `crates/worldwake-ai/tests/golden_*.rs` — the full suite that must be checked.
4. ExecutionBudget fields and minimum sensible values:
   - `max_node_expansions: 50` (minimum that still allows basic planning)
   - `beam_width: 3` (minimum width)
   - `snapshot_travel_horizon: 2` (minimum horizon)
   - `max_prerequisite_locations: 1` (minimum prerequisites)
5. The conformance test is a diagnostic — it identifies fields that need reclassification, not a pass/fail gate on the ticket itself. The spec explicitly acknowledges `max_node_expansions` may be reclassified.

## Architecture Check

1. The conformance test is a meta-test: it runs existing golden scenarios with modified ExecutionBudget values and checks whether goal selection changes. It does not modify production code.
2. If violations are found, the result is a documented reclassification recommendation — not a code change within this ticket. Reclassification (moving a field from ExecutionBudget to CognitiveProfile) would be a follow-up ticket.
3. This test establishes an ongoing regression gate: future ExecutionBudget changes that cause goal-selection divergence are caught automatically.

## Verification Layers

1. Conformance test runs all goldens at minimum budget → test output documents which fields cause goal-selection changes
2. Fields identified as behavior-changing → documented for reclassification
3. Fields confirmed as compression-safe → documented as validated engine knobs
4. Single-layer ticket (test-only) — no cross-system verification needed beyond the test itself.

## What to Change

### 1. Add conformance test module

Create a conformance test in `crates/worldwake-ai/tests/` (e.g., `conformance_execution_budget.rs` or add to an existing test file):

**Approach**:
For each golden test scenario that uses AI agents:
1. Run the scenario with default ExecutionBudget → record goal selection sequence (list of GoalKind chosen per agent per tick).
2. Run the same scenario with minimum ExecutionBudget (`max_node_expansions: 50, beam_width: 3, snapshot_travel_horizon: 2, max_prerequisite_locations: 1`) → record goal selection sequence.
3. Compare: if the goal selection sequence differs (different GoalKind chosen at any decision point), flag the scenario and the likely violating field.

**Output**: The test documents which fields are confirmed compression-safe vs behavior-changing. If `max_node_expansions` causes divergence (expected given S97), it is recorded for reclassification.

### 2. Document reclassification findings

After the test runs, add a comment or test annotation documenting:
- Which ExecutionBudget fields are confirmed compression-safe
- Which fields caused goal-selection divergence and should be reclassified as Cognitive
- S97 cross-reference for `max_node_expansions`

### 3. If reclassification needed

If the conformance test confirms `max_node_expansions` (or any other field) must be reclassified:
- Document the finding in the test output
- Create a follow-up ticket for the reclassification (move field from ExecutionBudget to CognitiveProfile, update all consumers)
- Do NOT implement the reclassification in this ticket

## Files to Touch

- `crates/worldwake-ai/tests/conformance_execution_budget.rs` (new)

## Out of Scope

- Actually reclassifying fields (follow-up ticket if violations found)
- Changing ExecutionBudget defaults
- Changing any planner algorithm
- Production code changes

## Acceptance Criteria

### Tests That Must Pass

1. Conformance test runs all golden scenarios at default and minimum ExecutionBudget
2. Test documents which fields are compression-safe vs behavior-changing
3. If `max_node_expansions` causes goal-selection divergence (expected), the finding is recorded
4. Test does not fail — it is diagnostic, not a hard gate. Divergence is an expected outcome that triggers reclassification, not a test failure.
5. Existing suite: `cargo test --workspace`

### Invariants

1. CognitiveProfile values are identical between default and minimum runs — only ExecutionBudget changes
2. Goal selection comparison uses the same seed and initial state for both runs
3. Deterministic: same seed → same comparison result

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/conformance_execution_budget.rs` — Conformance test comparing goal selection under default vs minimum ExecutionBudget for all golden scenarios

### Commands

1. `cargo test -p worldwake-ai -- conformance`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
