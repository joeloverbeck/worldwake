# S138OPPCOM-011: Attribute opportunity-retained travel pruning decisions

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — enriches travel-pruning trace records and selected-plan provenance formatting
**Deps**: archive/tickets/S138OPPCOM-007.md (opportunity-aware travel pruning)

## Problem

S138OPPCOM-007 made `prune_travel_away_from_goal_with_expansion_trace` retain travel candidates that would otherwise be pruned when a destination's opportunity salience multiplied by `CognitiveProfile.detour_budget_permille` meets the detour cost threshold. The existing trace surface records retained and pruned destinations, but it does not say whether a retained farther destination survived because it was already within the best travel cost or because the opportunity budget allowed the detour.

That leaves the spec's debugging promise incomplete: `archive/specs/S138-opportunity-compiler.md` says travel-detour budget mis-tuning should be inspectable because observer/provenance surfaces expose detour decisions with attribution.

## Assumption Reassessment (2026-05-11)

1. Before this ticket, `TravelSuccessorTrace` in `crates/worldwake-ai/src/decision_trace.rs` recorded destination, perceived travel cost pieces, remaining travel ticks, and projected total cost, but no retention/pruning reason.
2. `TravelPruningTrace` already travels through `SearchExpansionSummary.travel_pruning` and `SelectedPlanSearchProvenance.root_travel_pruning`; this ticket should enrich that existing trace path rather than adding a parallel debug subsystem.
3. Before this ticket, `format_selected_plan_search_provenance` rendered retained and pruned travel successors without any opportunity salience, budget, threshold, or reason information.
4. S138OPPCOM-007's focused pruning tests prove behavior, but they do not prove trace attribution for opportunity-retained detours.
5. The exact shared abstraction boundary is the planner search trace/provenance surface for spatial pruning, not authoritative action execution or observer opportunity rendering.

## Architecture Check

1. This follows FND-29: developers can inspect why a travel branch survived pruning rather than inferring it from code and local arithmetic.
2. The fix should enrich the existing `TravelPruningTrace` / selected-plan provenance path, preserving one canonical trace surface for travel-pruning decisions.
3. No backwards-compatibility aliasing or parallel trace path is introduced; downstream tests/renderers should update to the new trace shape.

## Verification Layers

1. Opportunity-retained detour attribution -> focused search/heuristic unit test asserting the retained successor records an opportunity-derived reason and the salience/budget/threshold arithmetic.
2. Ordinary retained travel candidate attribution -> focused unit test asserting the non-detour or within-best-cost candidate remains distinguishable from opportunity-retained candidates.
3. Human-readable selected-plan provenance -> decision-trace formatting test asserting opportunity-retained branches render with attribution.

## What to Change

### 1. Enrich travel-pruning trace data

Modify `crates/worldwake-ai/src/decision_trace.rs` to add a typed reason/attribution surface to travel successors, for example:

```rust
pub enum TravelPruningAttribution {
    WithinBestCost,
    OpportunityDetour {
        salience_permille: u32,
        detour_budget_permille: Permille,
        cost_increase: u32,
        cost_threshold: u32,
    },
    PrunedAsAwayFromGoal,
}
```

Use the exact final shape that fits the live trace structs, but keep the attribution typed and deterministic.

### 2. Populate attribution in pruning

Modify `crates/worldwake-ai/src/search/heuristic.rs` so opportunity-retained candidates record the opportunity salience, per-agent detour budget, cost increase, and threshold that caused retention.

### 3. Render attribution in selected-plan provenance

Update `format_selected_plan_search_provenance` and nearby decision-trace tests so retained/pruned travel branches expose the reason in the human-readable trace summary.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — trace type and formatting/tests)
- `crates/worldwake-ai/src/search/heuristic.rs` (modify — populate attribution)
- `crates/worldwake-ai/src/search/tests.rs` (modify — focused pruning attribution tests)

## Out of Scope

- Changing travel-pruning behavior — S138OPPCOM-007 already landed the decision rule
- Observer Section 3a opportunity list rendering — owned by S138OPPCOM-009
- Golden/E2E scenario coverage — owned by `archive/tickets/S138OPPCOM-010.md`

## Acceptance Criteria

### Tests That Must Pass

1. New/updated focused test: an opportunity-retained farther travel candidate records opportunity-derived attribution with salience, budget, cost increase, and threshold.
2. New/updated focused test: a retained candidate that is not opportunity-derived records ordinary within-best-cost attribution.
3. New/updated decision-trace formatting test: selected-plan provenance renders the opportunity-retained reason.
4. Existing focused pruning tests from S138OPPCOM-007 continue to pass.
5. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. Trace attribution is derived from the same arithmetic used by pruning; no second heuristic or approximate recomputation is introduced.
2. The trace remains deterministic and uses typed numeric fields, not formatted strings as source data.
3. No new planner behavior or pruning threshold change lands in this ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — attribution assertions for ordinary retained and opportunity-retained travel successors.
2. `crates/worldwake-ai/src/decision_trace.rs` — selected-plan provenance formatting assertion.

### Commands

1. `cargo test -p worldwake-ai --lib prune_travel`
2. `cargo test -p worldwake-ai --lib decision_trace::tests::summary_planning_includes_attempt_anchor -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-11.

- Added typed `TravelPruningAttribution` data to `TravelSuccessorTrace` on the existing `TravelPruningTrace` / selected-plan provenance path.
- Populated attribution at the pruning decision point from the same arithmetic used to retain opportunity detours: summed opportunity salience, per-agent detour budget, cost increase, and threshold.
- Rendered the attribution in selected-plan search provenance as `within_best_cost`, `opportunity_detour(...)`, or `pruned_as_away_from_goal`.
- Tightened focused pruning and decision-trace tests so ordinary retained, opportunity-retained, and pruned travel successors are distinguishable.

## Deviations

- The decision-trace proof used the exact live unit test `decision_trace::tests::summary_planning_includes_attempt_anchor`; `cargo test -p worldwake-ai --lib decision_trace -- --list` was used as selector discovery, then narrowed to the proof that renders selected-plan search provenance.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib prune_travel -- --list`
- Passed `cargo test -p worldwake-ai --lib prune_travel`
- Passed `cargo test -p worldwake-ai --lib decision_trace -- --list`
- Passed `cargo test -p worldwake-ai --lib decision_trace::tests::summary_planning_includes_attempt_anchor -- --exact`
- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
