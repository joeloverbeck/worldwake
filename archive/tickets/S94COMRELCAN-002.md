# S94COMRELCAN-002: Implement commodity-relevance candidate filter

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — planner-internal filter only
**Deps**: archive/tickets/S94COMRELCAN-001.md

## Problem

The GOAP planner generates 1400-5700 candidates for commodity goals because the candidate pipeline filters by operation kind but not by commodity relevance. `AcquireCommodity(Water)` generates `MoveCargo(Waste)`, `Trade(Guard)`, `QueueForFacility(Mill)`, etc. — all irrelevant to water acquisition. This ticket adds a post-pipeline filter that retains only candidates whose targets relate to the goal's target commodity.

## Assumption Reassessment (2026-04-11)

1. `search_candidates()` at `crates/worldwake-ai/src/search/candidates.rs:101` returns `Vec<SearchCandidate>` after applying binding, blocked-facility, availability, and place-blocker filters. It has 11 parameters — the spec chose to add the new filter at the call site rather than widening this function.
2. `search_plan_with_trace_metadata()` at `crates/worldwake-ai/src/search/mod.rs:135` receives `recipes: &RecipeRegistry` as a parameter. The live pre-tactical boundary is: `search_candidates()` root candidates, then any `social_query_candidates()`, then the tactical filter. The commodity-relevance filter must run on that combined pre-tactical candidate set, not only on the `search_candidates()` return.
3. `PlannerOpSemantics` at `crates/worldwake-ai/src/planner_ops.rs:57` provides `op_kind: PlannerOpKind` for classifying candidates by operation type.
4. Belief view methods confirmed: `item_lot_commodity()` on `InventoryBeliefView` (planning_state.rs:2049), `resource_source()` on `FacilityBeliefView` (planning_state.rs:2111).
5. `SearchCandidate` struct at `candidates.rs:21` has `def_id: ActionDefId` and `targets: Vec<EntityId>` — the filter uses `def_id` to look up `PlannerOpKind` via `semantics_table`, and `targets` to resolve commodity kind.
6. Trade payloads: `SearchCandidate` has `payload_override: Option<ActionPayload>`, but `TradeActionPayload` does not carry a direct commodity field. Live trade candidates instead carry `sale_lot`, whose commodity can be resolved through `state.item_lot_commodity(payload.sale_lot)`.
7. Harvest and craft payloads already carry commodity-relevant data in the payload: `HarvestActionPayload.output_commodity`, and `CraftActionPayload.inputs` / `outputs`. Recipe lookup remains available, but the payload already exposes the live planner-side contract.
8. `QueueForFacilityUse` candidates use `QueueForFacilityUsePayload { intended_action }` rather than a facility commodity. For harvest/craft queue candidates, commodity relevance must be resolved from the queued action definition's payload in `ActionDefRegistry`, not just from the target facility's `resource_source()`.
9. The filter is a heuristic removal of provably-irrelevant candidates (not a heuristic addition). Per precision rule 12: the missing substrate is "commodity-relevance awareness in candidate generation." This ticket introduces that substrate. No unrelated regressions are opened because the filter only prunes candidates with positively-resolved non-matching commodities; unknown/unresolvable commodities pass (conservative default).

## Architecture Check

1. Placing the filter between `search_candidates()` and the tactical filter maintains clean separation: root filtering (binding, availability, place) → commodity relevance → tactical scoping (location). Each layer has a distinct concern.
2. No backward-compatibility shims. The filter applies unconditionally when the goal has a target commodity. No opt-out path or feature flag.
3. Queue candidate filtering must key off the queued `intended_action` payload contract rather than facility type alone. That keeps harvest and craft queue candidates aligned with the real operator they are reserving, instead of baking a brittle facility-category heuristic into the filter.

## Verification Layers

1. Commodity-irrelevant root candidates are pruned → decision trace shows `CommodityIrrelevant` entries for filtered root candidates
2. Commodity-relevant candidates are retained → focused unit tests with known candidate sets
3. Travel candidates always pass → focused unit test
4. Unknown/unresolvable commodity candidates pass (conservative default) → focused unit test
5. Non-commodity goals bypass the filter entirely → focused unit test with `Sleep` or similar goal
6. Single-layer ticket (planner-internal) — no authoritative state changes

## What to Change

### 1. Add `apply_commodity_relevance_filter()` function

In `crates/worldwake-ai/src/search/candidates.rs`:

Add a `pub(super)` function:

```rust
pub(super) fn apply_commodity_relevance_filter(
    candidates: &mut Vec<SearchCandidate>,
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    recipes: &RecipeRegistry,
    root_candidates: Option<&mut Vec<crate::decision_trace::RootCandidateTrace>>,
)
```

Filter logic per `PlannerOpKind`:
- **Travel**: Always pass
- **MoveCargo**: Pass if `state.item_lot_commodity(target)` == goal commodity
- **Trade**: Pass if the trade payload's `sale_lot` resolves to the goal commodity via `state.item_lot_commodity(payload.sale_lot)`
- **QueueForFacilityUse**: Resolve the queued `intended_action` via `ActionDefRegistry` and apply the same harvest/craft commodity checks that the intended action would use; unknown or non-harvest/craft intended actions pass conservatively
- **Harvest**: Pass if payload's `output_commodity` == goal commodity; if payload is absent or incomplete, fall back conservatively to belief-backed facility/resource resolution when possible
- **Craft**: Pass if payload's `inputs` or `outputs` contain the goal commodity; recipe lookup remains an acceptable fallback when only `recipe_id` is available
- **Heal, AskWitness, all others**: Always pass

Bypass: If `goal.key.kind.target_commodity(recipes)` returns `None`, return immediately without filtering.

Conservative default: If commodity kind cannot be positively resolved (method returns `None` or entity not in beliefs), the candidate passes.

Record filtered root candidates in `root_candidates` trace with `RootCandidateFilterReason::CommodityIrrelevant`. Candidates without root trace entries may still be filtered, but only traced root candidates must emit the new reason in this ticket.

### 2. Integrate filter into search pipeline

In `crates/worldwake-ai/src/search/mod.rs`, in `search_plan_with_trace_metadata()`:

After `search_candidates()` returns and any `social_query_candidates()` are appended, but before the tactical candidate filter runs, call `apply_commodity_relevance_filter()` on that combined candidate vec. Pass `registry`, `recipes`, `goal`, `&node.state`, `semantics_table`, and the trace sink.

## Files to Touch

- `crates/worldwake-ai/src/search/candidates.rs` (modify) — new filter function
- `crates/worldwake-ai/src/search/mod.rs` (modify) — integration call site
- `crates/worldwake-ai/src/search/tests.rs` (modify) — focused filter tests and pipeline integration coverage

## Out of Scope

- Modifying `matches_binding()` — operates at binding level, not commodity relevance
- Modifying the tactical candidate filter — location-based scoping is orthogonal
- Modifying `search_candidates()`'s parameter list
- Per-agent filter tuning via `CognitiveProfile`
- Golden test rewrites (ticket 003)
- Changing `CognitiveProfile` or `ExecutionBudget` parameters
- Addressing non-commodity goals (Sleep, Combat, etc.)

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit tests for `apply_commodity_relevance_filter()` covering each `PlannerOpKind` filter rule
2. New focused unit test for conservative default (unknown commodity passes)
3. New focused unit test for bypass condition (non-commodity goal)
4. Existing suite: `cargo test --workspace`

### Invariants

1. All positively-resolved commodity-relevant candidates are retained — the filter never removes a candidate whose resolved commodity matches the goal
2. Travel candidates always pass regardless of goal commodity
3. Unknown/unresolvable commodity kinds pass the filter (conservative default — no false negatives)
4. Non-commodity goals bypass the filter entirely (no candidates removed)
5. Filtered root candidates appear in decision traces with `CommodityIrrelevant` reason and both the candidate's and goal's commodity kinds

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/candidates.rs` (new `#[cfg(test)]` tests) — focused unit tests for the filter function covering: each PlannerOpKind rule, conservative default for unresolvable commodities, bypass for non-commodity goals, correct root-trace recording
2. `crates/worldwake-ai/src/search/tests.rs` (new tests) — integration-level tests using `search_plan_with_trace_metadata()` to verify the filter is called on the combined pre-tactical candidate set and produces expected candidate reductions

### Commands

1. `cargo test -p worldwake-ai commodity_relevance` — new focused tests
2. `cargo test -p worldwake-ai search` — search module integration
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean
4. `cargo test --workspace` — full regression

## Outcome

Completion date: 2026-04-11

Implemented the commodity-relevance root-candidate filter in `crates/worldwake-ai/src/search/candidates.rs` and integrated it into `search_plan_with_trace_metadata()` in `crates/worldwake-ai/src/search/mod.rs` before the tactical location filter. The filter now prunes positively-resolved non-matching `MoveCargo`, `Trade`, `Harvest`, `Craft`, and queued harvest/craft candidates while preserving conservative pass-through for unknown commodity resolution and non-commodity goals.

Focused coverage landed in `crates/worldwake-ai/src/search/tests.rs` for mismatched trade/move/craft pruning, conservative pass-through for travel and unknown commodity cases, non-commodity bypass, root-trace recording, and the tactical prerequisite override needed for two-phase `ProduceCommodity` search. The existing `search` integration tests now pass with the filter active, including the remote production path and trace-metadata scenarios. Broadened verification also required a minimal expectation update in `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` so one pre-existing S93 treat-wounds snapshot can admit its now-lower-pressure `FrontierExhausted` path while ticket `S94COMRELCAN-003` retains ownership of the full golden rewrite.

## Deviations

1. The landed helper takes the active tactical goal in addition to the root goal so commodity pruning can follow the live tactical commodity during two-phase search. Without that, `ProduceCommodity(Bread)` incorrectly pruned lawful `AcquirePrerequisite(Firewood)` pickup candidates and broke existing remote production search tests.
2. A minimal fallout adjustment was required in `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs`: one pre-existing S93 golden now reaches `FrontierExhausted` instead of `BudgetExhausted` after candidate pruning reduced search pressure. Ticket `S94COMRELCAN-003` still owns the full golden rewrite; this change only keeps `cargo test --workspace` green.

## Verification Result

1. `cargo test -p worldwake-ai commodity_relevance`
2. `cargo test -p worldwake-ai search_trace_metadata_records_two_phase_strategic_and_landmark_details`
3. `cargo test -p worldwake-ai search`
4. `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots kael_treat_wounds_vara_at_dusty_trail_budgets_exhaust`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`
