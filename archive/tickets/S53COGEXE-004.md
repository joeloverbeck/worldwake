# S53COGEXE-004: Behavioral validation conformance test

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S53COGEXE-003

## Problem

The spec's Behavioral Validation Contract claims `ExecutionBudget` changes should not change goal selection when `CognitiveProfile` is held constant. But S97 already proves `max_node_expansions` is behavior-changing. The live golden harness does not expose a single reusable "run all goldens and extract goal sequences" API, so this ticket must implement the strongest honest in-process conformance slice the current harness can support: representative decision-trace comparisons plus the existing S97-style divergence proof.

## Assumption Reassessment (2026-04-05)

1. S97 at `crates/worldwake-ai/tests/golden_reasoning_diversity.rs:124` — "Search Depth Drives Multi-Step Plan Divergence". Proves `max_node_expansions` changes agent behavior. Confirmed.
2. After ticket 003, all agents have `CognitiveProfile` and `ExecutionBudget` as separate components.
3. The live golden suite is spread across separate `golden_*.rs` test binaries with no shared callable scenario registry and no canonical cross-suite "goal selection sequence" extractor. Corrected: this ticket owns a curated in-process conformance set over reusable harness setups, not a subprocess or log-parsing "all goldens" runner.
4. ExecutionBudget fields and minimum sensible values:
   - `max_node_expansions: 50` (minimum that still allows basic planning)
   - `beam_width: 3` (minimum width)
   - `snapshot_travel_horizon: 2` (minimum horizon)
   - `max_prerequisite_locations: 1` (minimum prerequisites)
5. The ticket now separates two proof surfaces:
   - explicit positive divergence proof for `max_node_expansions`
   - representative non-divergence checks for `beam_width` and `max_prerequisite_locations`
6. Focused reassessment during implementation showed `snapshot_travel_horizon = 2` already suppresses goal selection on a one-hop remote-acquire boundary. Corrected: this ticket must record `snapshot_travel_horizon` as behavior-changing too, not pretend it passed conformance.
7. This ticket still does not reclassify fields. If the conformance slice confirms a field is behavior-changing, that is follow-up work.

## Architecture Check

1. The conformance test is an in-process harness test over curated reusable scenario families, not a subprocess meta-run over the entire golden suite.
2. Decision traces are the canonical comparison surface. The test compares selected goals and, where needed, selected plan shape from the same tick boundary under identical `CognitiveProfile`.
3. If violations are found, the result is a documented reclassification recommendation — not a production code change within this ticket. Reclassification (moving a field from `ExecutionBudget` to `CognitiveProfile`) would be a follow-up ticket.
4. This establishes an honest regression gate for the reusable conformance scenarios the current harness can actually support.

## Verification Layers

1. Representative conformance scenarios run at default vs targeted reduced budgets → decision traces show whether selection or plan shape changes
2. `max_node_expansions` divergence is proved explicitly on the multi-step search-depth scenario
3. `snapshot_travel_horizon` divergence is proved explicitly on a one-hop remote-acquire scenario
4. `beam_width` and `max_prerequisite_locations` are validated on representative bounded scenarios only
5. Single-layer ticket (test-only) — no cross-system verification needed beyond the test suite itself

## What to Change

### 1. Add conformance test module

Create a conformance test in `crates/worldwake-ai/tests/conformance_execution_budget.rs`:

**Approach**:
1. Build a simple local-consume harness where a hungry agent already owns bread. Compare default vs reduced-budget selected-goal sequences to prove that immediate local needs behavior survives budget compression.
2. Build a bounded multi-step craft harness mirroring the S97 topology shape: one remote prerequisite location, one pickup, one craft step. Compare default vs targeted reductions of:
   - `beam_width = 3`
   - `max_prerequisite_locations = 1`
   Assert the selected goal and selected-plan shape stay the same.
3. Build a one-hop remote-acquire harness and prove that reducing `snapshot_travel_horizon = 2` changes selection from the same planning boundary. This records the field as behavior-changing and not conformance-safe.
4. Add an explicit positive divergence test for `max_node_expansions`, cross-referencing the S97 search-depth scenario contract. This test should prove that reducing `max_node_expansions` changes plan selection from the same planning boundary, so the field is behavior-changing and not conformance-safe.

**Output**: The file itself documents which `ExecutionBudget` fields are representative-scenario safe and which are already known to be behavior-changing.

### 2. Document reclassification findings

After the tests land, document in comments:
- which `ExecutionBudget` fields are representative-scenario safe
- which fields are already known to be behavior-changing and therefore reclassification candidates
- the S97 cross-reference for `max_node_expansions`

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

1. New conformance tests compare representative reusable scenarios at default and reduced `ExecutionBudget`
2. Tests explicitly prove `max_node_expansions` changes behavior from the same planning boundary
3. Tests explicitly prove `snapshot_travel_horizon` changes behavior from the same planning boundary
4. Tests explicitly prove the representative scenarios do not change goal selection or bounded plan shape under reduced `beam_width` and `max_prerequisite_locations`
5. Existing suite: `cargo test --workspace`

### Invariants

1. `CognitiveProfile` values are identical between compared runs — only `ExecutionBudget` changes
2. Goal and plan comparison uses the same seed and initial state for both runs
3. Deterministic: same seed → same comparison result

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/conformance_execution_budget.rs` — Representative conformance tests for split execution-budget fields plus explicit `max_node_expansions` divergence proof

### Commands

1. `cargo test -p worldwake-ai -- conformance`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completed**: 2026-04-05
- **What changed**:
  - Added [`conformance_execution_budget.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/conformance_execution_budget.rs) as the bounded in-process conformance suite for the split-profile contract.
  - Proved the full minimum-budget bundle preserves immediate local `ConsumeOwnedCommodity` selection on a simple local-needs boundary.
  - Proved `beam_width` and `max_prerequisite_locations` preserve selected goal and bounded multi-step plan shape on the reusable S97-style craft/search scenario.
  - Proved `max_node_expansions` is behavior-changing on the bounded multi-step search boundary and `snapshot_travel_horizon` is behavior-changing on a one-hop remote-acquire boundary.
  - Corrected the ticket boundary from an impossible "run all goldens" meta-test to the strongest honest reusable harness contract the current test architecture supports.
- **Deviations from original plan**:
  - The original ticket and spec prose treated `snapshot_travel_horizon` as a presumed safe engine knob. Focused conformance falsified that assumption, so the finished ticket records it as behavior-changing instead of forcing it into the safe bucket.
  - Because the live golden architecture has no universal callable scenario API, the completed work landed as a curated in-process conformance suite rather than a subprocess over every `golden_*.rs` file.
  - Follow-up ticket [`S53COGEXE-005`](/home/joeloverbeck/projects/worldwake/tickets/S53COGEXE-005.md) was created to reclassify the two behavior-changing fields from `ExecutionBudget` into `CognitiveProfile`.
- **Verification**:
  - `cargo test -p worldwake-ai --test conformance_execution_budget -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
