# S39: Limited Side-Benefit Plan Scoring

**Status**: ✅ COMPLETED

## Summary

Allow plan selection to consider secondary benefits of a plan without modifying plan search. Currently plans are strictly single-goal — a trip to market to buy food cannot accrue benefit from also being near a place where the agent could sell surplus or deliver a report. This produces suboptimal multi-trip behavior where agents make separate journeys for goals that could be combined. Introduce lightweight post-search side-benefit detection that scores secondary goal satisfaction at plan destinations for tie-breaking in `select_best_plan()`.

## Source

Derived from ChatGPT architecture review Feature B (Limited multi-desire side-benefit scoring). The review explicitly warns against a full multi-desire planner. This spec follows that guidance: side-benefits are a scoring overlay on existing single-goal plans, not a search modification.

## Phase

Phase 4+: Economy & Trade

## Crates

- `worldwake-ai` (plan selection, new side_benefit module)
- `worldwake-core` (UtilityProfile extension)

## Dependencies

- S33 ✅ (opportunity-scoped goal identity — multi-desire awareness requires separate opportunity tracking to know which other goals the agent is pursuing)
- S22 ✅ (intention frames — side-benefit scoring respects frame commitment)

## FOUNDATIONS Alignment

- **P20** (Resource-Bounded Practical Reasoning): Real agents combine errands. A merchant traveling to market would naturally also sell surplus while there. Single-goal planning produces unrealistically inefficient behavior.
- **P5** (Simulate Carriers of Consequence): Combined trips create more interaction opportunities — an agent at market for food might also encounter a trader, hear a rumor, or witness an event. More co-location = more emergent interactions.
- **P22** (Agent Diversity): Per-agent `side_benefit_weight` on `UtilityProfile` means some agents are better at combining goals (efficient merchants) while others are single-minded (focused warriors).

## Design Goals

1. **Post-search scoring only**: Side-benefit detection happens AFTER plan search, not during. Search complexity is unchanged.
2. **Tie-breaking only**: Side-benefits never override primary goal priority class. They only break ties between plans of equal primary value.
3. **Lightweight detection**: Side-benefits are detected by checking if pending candidates have target places that appear in the plan path. No additional search.
4. **Per-agent weighting**: `side_benefit_weight` on `UtilityProfile` controls influence (P20).
5. **No plan modification**: The selected plan is executed as-is. Side-benefits are scoring input, not new plan steps.

## Deliverables

### 1. `SideBenefit` struct (worldwake-ai)

```rust
/// A secondary goal that could be pursued at a location the agent is already
/// visiting as part of the primary plan.
#[derive(Debug, Clone)]
pub struct SideBenefit {
    /// The secondary goal that could be satisfied.
    pub goal_key: GoalKey,
    /// The place where this benefit exists, which is on the plan's path.
    pub at_place: EntityId,
    /// Estimated value of pursuing this secondary goal.
    /// Derived from: candidate.motive_score * side_benefit_weight.value() as u32 / 1000
    pub estimated_value: u32,
}
```

### 2. `PlanValue` wrapper (worldwake-ai)

`PlanValue` is an internal scoring struct used only during plan selection. `select_best_plan()` continues to return `Option<PlannedPlan>` — callers are unaffected.

```rust
/// A scored plan including primary motive and secondary side-benefits.
/// Internal to plan selection; never stored or returned to callers.
#[derive(Debug, Clone)]
pub struct PlanValue {
    pub plan: PlannedPlan,
    pub priority_class: GoalPriorityClass,
    /// Motive score from the primary goal's ranking (u32, same type as RankedGoal.motive_score).
    pub primary_motive: u32,
    /// Detected side-benefits at locations along the plan path.
    pub side_benefits: Vec<SideBenefit>,
    /// Combined score: primary_motive + sum(side_benefit.estimated_value),
    /// capped at primary_motive * 3 / 2.
    pub total_value: u32,
}
```

### 3. Side-benefit detection (worldwake-ai, new `side_benefit.rs`)

```rust
/// Detect secondary goals that could be satisfied at locations in the plan path.
///
/// Scans the agent's pending ranked candidates for goals whose target place
/// appears in the plan's step sequence. Does NOT modify the plan or add steps.
pub fn detect_side_benefits(
    plan: &PlannedPlan,
    ranked_candidates: &[RankedGoal],
    primary_goal_key: &GoalKey,
    side_benefit_weight: Permille,
) -> Vec<SideBenefit>
```

Logic:
1. Collect all distinct places the plan visits: iterate `plan.steps`, filter to steps with `op_kind == PlannerOpKind::Travel`, extract `Authoritative(id)` targets as place EntityIds. Skip `Hypothetical` targets.
2. For each ranked candidate that is NOT the primary goal:
   - Extract the candidate's target place: first try `candidate.grounded.anchor` — if `OpportunityAnchor::Place(id)`, use that. Otherwise fall back to `candidate.grounded.key.place` (which is `Option<EntityId>`).
   - If the target place appears in the plan's visited places: create `SideBenefit { goal_key: candidate.grounded.key, at_place, estimated_value: candidate.motive_score * side_benefit_weight.value() as u32 / 1000 }`.
3. Deduplicate: at most one `SideBenefit` per `GoalKey`.
4. Cap at 3 side-benefits (avoid excessive scoring for agents with many pending goals).

### 4. `UtilityProfile` extension (worldwake-core)

Add `side_benefit_weight: Permille` to `UtilityProfile`:
- Default: `Permille(100)` (10% of secondary goal's motive counts as side-benefit).
- `Permille(0)`: No side-benefit scoring (single-minded agents).
- `Permille(500)`: Strong preference for combined trips (efficient merchants).

### 5. Integration in `select_best_plan()` (worldwake-ai)

Add `side_benefit_weight: Permille` as a new parameter to `select_best_plan()`, matching the existing pattern of passing per-agent parameters (like `default_switch_margin`, `frame_switch_margin`). Callers pass the agent's `UtilityProfile.side_benefit_weight`.

In `plan_selection.rs`, after plan search produces candidate plans:

1. For each plan, compute `PlanValue` with side-benefit detection.
2. `total_value = primary_motive + sum(side_benefit.estimated_value)`, capped at `primary_motive * 3 / 2` (side-benefits never more than 50% bonus). All values are `u32`.
3. Selection logic:
   - **Priority class comparison**: Unchanged. Higher priority class always wins regardless of side-benefits.
   - **Same priority class, same motive tier**: Use `total_value` for tie-breaking in the `compare_ranked_plans` sort.
   - **Goal switching margin**: Uses `primary_motive` only, not `total_value` (side-benefits don't trigger goal switches).
4. Return type remains `Option<PlannedPlan>` — `PlanValue` is used internally for scoring only.

### 6. Decision trace extension

Add side-benefit information to `SelectionTrace`:

```rust
pub struct SideBenefitTrace {
    pub plan_goal: GoalKey,
    pub side_benefits: Vec<SideBenefit>,
    pub total_value: u32,
    pub primary_motive: u32,
}
```

## Component Registration

No new ECS components. `side_benefit_weight` is a field on existing `UtilityProfile`.

## FND-01 Section H Analysis

### Information-path analysis
No new information paths. Side-benefit detection reads existing ranked candidates (already computed) and plan step targets (already computed). No new queries to belief view or world state.

### Positive-feedback analysis
No amplifying loops. Side-benefit scoring is a read-only overlay on existing plan selection. It does not create new goals, plans, or world state.

### Concrete dampeners
- Side-benefit cap: `total_value` capped at `primary_motive * 3 / 2` (150% of primary).
- Max 3 side-benefits per plan.
- `side_benefit_weight` per-agent control.
- Side-benefits never override priority class (survival > danger > normal hierarchy preserved).

### Stored state vs. derived read-model list
- **Stored**: `side_benefit_weight` on `UtilityProfile` (per-agent component).
- **Derived**: `SideBenefit` instances (computed during plan selection, never stored). `PlanValue` (computed during selection, never stored). `total_value` (computed, never stored).

## Tests

### Focused tests
- [ ] Side-benefit detected when pending candidate's target place matches plan destination
- [ ] Side-benefit NOT detected for the primary goal itself
- [ ] Side-benefit estimated_value = candidate.motive_score * side_benefit_weight.value() as u32 / 1000
- [ ] At most 3 side-benefits per plan
- [ ] total_value capped at 150% of primary_motive
- [ ] Side-benefits break ties between plans of equal primary motive
- [ ] Side-benefits NEVER override priority class (higher class always wins)
- [ ] Goal switching uses primary_motive only, not total_value
- [ ] Agents with side_benefit_weight = Permille(0) get no side-benefit scoring
- [ ] Different agents with different weights produce different plan selections for same candidates

### Golden tests
- [ ] Merchant with pending buy + sell goals at same market: prefers plan routing through market over alternative that only satisfies buy goal
- [ ] Deterministic replay companion

## Acceptance Criteria

1. Plans are scored for secondary benefits at locations along their path.
2. Side-benefits only break ties — never override primary goal priority class.
3. Detection is lightweight: reads existing candidates and plan paths, no additional search.
4. `side_benefit_weight` on `UtilityProfile` provides per-agent diversity.
5. Agents with `side_benefit_weight = Permille(0)` behave identically to pre-spec behavior.
6. Goal switching logic is unaffected (uses primary_motive only).
7. All existing golden tests pass unchanged.

## Outcome

- **Completed**: 2026-04-03
- **What changed**:
  - Added `side_benefit_weight` to `UtilityProfile` in `worldwake-core`.
  - Added the pure side-benefit substrate in `worldwake-ai` (`SideBenefit`, `PlanValue`, `detect_side_benefits`, `build_plan_value`).
  - Integrated side-benefit-aware tie-breaking into post-search plan selection while preserving priority-class ordering and primary-motive-based goal switching.
  - Extended selected-plan trace summaries with primary motive, total value, and side-benefit provenance.
  - Added the end-to-end merchant-market golden proof and deterministic replay companion showing a combined market trip wins because it also captures a lawful `SellCommodity(Firewood)` side benefit.
- **Deviations from original plan**:
  - The shipped golden scenario uses firewood rather than apples for the secondary sell-at-market benefit so the setup stays focused on side-benefit selection instead of directly satisfying the primary hunger need.
  - The final golden used simplified seller fixtures to avoid negotiation-price noise obscuring the selection contract.
  - Repo-global scenario numbering required assigning the new S39 proof to Scenario `95` and shifting the adjacent merchant scenario to `96`.
- **Verification results**:
  - `cargo test -p worldwake-core utility_profile::tests::utility_profile_roundtrips_through_bincode -- --exact --nocapture`
  - `cargo test -p worldwake-core component_tables::tests::insert_and_get_utility_profile -- --exact --nocapture`
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-ai --lib`
  - `cargo test -p worldwake-ai plan_selection -- --nocapture`
  - `cargo test -p worldwake-ai decision_trace -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_merchant_selling combined_market_trip_selected_for_side_benefit -- --exact --nocapture`
  - `cargo test -p worldwake-ai --test golden_merchant_selling combined_market_trip_selected_for_side_benefit_replays_deterministically -- --exact --nocapture`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-cli scenario::types::tests::test_scenario_def_deserialize_full -- --exact --nocapture`
  - `cargo test -p worldwake-cli`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
