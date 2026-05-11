# S138OPPCOM-007: Opportunity-aware travel pruning

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — extends `prune_travel_away_from_goal_with_expansion_trace` with two new params; changes detour-allow semantics under non-empty opportunity sets
**Deps**: 003 (CognitiveProfile.detour_budget_permille), 006 (PerceivedOpportunityIndex populated per-tick)

## Problem

S138 makes travel pruning opportunity-aware: a detour that would normally be pruned (because `remaining_travel_ticks` increases) is allowed when the salience-weighted opportunity yield along the detour path exceeds the cost increase by `detour_budget_permille`. The existing pruning function at `crates/worldwake-ai/src/search/heuristic.rs:248` (`prune_travel_away_from_goal_with_expansion_trace`) gains two new parameters and a new branch. At default profiles, opportunity-derived detours are conservative (`detour_budget_permille = 150`); behavior reduces to the existing prune semantics when the opportunity index is empty.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage: `crates/worldwake-ai/src/search/heuristic.rs` has inline tests adjacent to the function at line 248; the sibling `prune_travel_away_from_goal` at line 231 is the parameter-free variant retained for callers that don't need expansion traces.
2. Spec/doc reference: `specs/S138-opportunity-compiler.md` deliverable section "Travel-pruning extension (in `search/heuristic.rs`)".
3. Caller surface: `prune_travel_away_from_goal_with_expansion_trace` has a single caller in `crates/worldwake-ai/src/search/candidates.rs` (per agent finding). The single-caller surface means parameter threading is mechanical.
4. `PerceivedOpportunityIndex` becomes available per-tick after ticket 006 lands; before 006, the index is `Default::default()` (empty), and the new branch never fires — workspace builds fine even in intermediate state. This ticket's effective behavior activates only when 006 is also landed.
5. Heuristic-removal discipline (precision-rules.md §12): this ticket does NOT remove or weaken the existing prune heuristic. It adds an opportunity-derived bypass under explicit per-agent budget control. The existing prune behavior is preserved when the opportunity index is empty or salience contributions are zero.

## Architecture Check

1. The detour-allow rule is per-agent (`CognitiveProfile.detour_budget_permille`) — FND-22 agent diversity preserved: two agents with the same opportunity perception rank detours differently based on profile.
2. Deterministic sum: salience contributions along the detour path are summed in `BTreeMap`-stable order (the underlying `PerceivedOpportunityIndex.by_place` is `BTreeMap`).
3. No backward-compatibility shim: the new parameters are required; the single caller updates atomically. No optional wrapping.
4. FND-3 concrete-state preserved: the detour decision is computed from typed opportunity salience and a typed permille budget, not from an abstract "interestingness score".

## Verification Layers

1. Empty opportunity index: function returns identical results to today's pre-S138 behavior — focused unit test asserting structural identity of `TravelPruneOutcome` for matched inputs
2. Detour allowed when opportunity salience × budget exceeds cost increase — focused unit test
3. Detour pruned when salience × budget is insufficient — focused unit test
4. Per-agent budget effect: two agents with different `detour_budget_permille` make different prune decisions on the same opportunity set — focused unit test
5. Determinism: same inputs produce byte-identical outputs across runs — focused unit test

## What to Change

### 1. Extend function signature

Modify `crates/worldwake-ai/src/search/heuristic.rs:248`:

```rust
pub(super) fn prune_travel_away_from_goal_with_expansion_trace(
    candidates: &mut Vec<SearchCandidate>,
    expansion_candidates: Option<&mut Vec<crate::decision_trace::ExpansionCandidateTrace>>,
    current_place: EntityId,
    goal_places: &[EntityId],
    snapshot: &PlanningSnapshot,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    detour_budget_permille: Permille,                                       // NEW
    opportunity_index: &crate::opportunity_compiler::PerceivedOpportunityIndex,  // NEW
) -> Option<crate::decision_trace::TravelPruningTrace>
```

### 2. Implement the detour-allow branch

Inside the function body, when a candidate would be pruned (`remaining_travel_ticks > current_min`), evaluate the opportunity-salience contribution along the candidate's detour path:

```rust
let detour_salience = sum_salience_along_detour(opportunity_index, candidate_path);
let cost_increase = remaining_travel_ticks.saturating_sub(current_min);
let weighted_salience = (detour_salience.value() as u32) * (detour_budget_permille.value() as u32);
let cost_threshold = (cost_increase as u32) * 1000;
if weighted_salience >= cost_threshold {
    // Allow detour — keep the candidate
    continue;
}
// Otherwise prune as before
```

The exact salience-summation function (`sum_salience_along_detour`) iterates the detour path's places via `opportunity_index.by_place` in `BTreeMap`-stable order. Implementation detail to confirm during /implement-ticket.

### 3. Update the single caller

Modify `crates/worldwake-ai/src/search/candidates.rs`: pass `cognitive_profile.detour_budget_permille` and the per-tick `PerceivedOpportunityIndex` (threaded from observation phase per ticket 006) into the call.

## Files to Touch

- `crates/worldwake-ai/src/search/heuristic.rs` (modify — function signature + new branch + tests)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — caller update; thread params from agent_tick context)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (likely modify — ensure `PerceivedOpportunityIndex` reaches the search layer; confirm wiring during implementation)

## Out of Scope

- The parameter-free sibling `prune_travel_away_from_goal` at line 231 — unchanged
- New action types — none introduced
- Cross-agent opportunity sharing — spec Non-Goal

## Acceptance Criteria

### Tests That Must Pass

1. New test: empty `PerceivedOpportunityIndex` produces results structurally identical to pre-S138 behavior for the same inputs
2. New test: a high-salience opportunity along a detour with budget `detour_budget_permille = 150` allows the detour when `salience × 150 >= cost_increase × 1000` (sum-based deterministic comparison)
3. New test: low-salience opportunity along a detour does not exceed the budget, candidate is pruned
4. New test: two agents with `detour_budget_permille = 150` vs `300` make different decisions on the same opportunity set
5. New test: determinism — repeat the test with reversed insertion order on the underlying `BTreeMap`, assert identical results
6. Existing tests in `heuristic.rs` continue to pass
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. With `PerceivedOpportunityIndex::default()` (empty), the function's outputs match pre-S138 outputs byte-for-byte
2. Detour-allow decisions are deterministic across runs (no `HashMap` iteration, no float comparison, no wall-clock)
3. The existing prune heuristic is preserved as the default branch; opportunity-derived bypass is additive

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/heuristic.rs` (inline `#[cfg(test)]`) — 5 new tests per Acceptance Criteria

### Commands

1. `cargo test -p worldwake-ai search::heuristic`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
