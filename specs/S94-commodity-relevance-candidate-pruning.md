# S94 — Commodity-Relevance Candidate Pruning

**Status**: DRAFT
**Phase**: 7 (Adjunct — Simulation Remediation)
**Crates**: `worldwake-ai`
**Dependencies**: S90 (completed), S93 (completed)
**Supersedes**: None (addresses root cause diagnosed in S93 golden tests)

## Problem Statement

Six golden e2e tests in `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` (S93) prove that the GOAP planner budget-exhausts on simple acquisition and healing goals in realistic entity densities. Guard Theron dies of hunger at tick 422 because `AcquireCommodity(Water)` generates 2085 candidates — at a location where Water is physically present. Merchant Vara's `TreatWounds` generates 5739 candidates at depth 3.

The candidate pipeline filters by operation KIND (`ACQUIRE_OPS = [Travel, Trade, QueueForFacilityUse, Harvest, Craft, MoveCargo]`) but performs **no commodity-relevance filtering on targets**. The `matches_binding()` method (goal_model.rs:1594) returns `true` for all auxiliary ops — Travel, Trade, MoveCargo, QueueForFacilityUse, Harvest, Craft, etc. — regardless of whether the candidate's target relates to the goal's commodity.

Consequently, `AcquireCommodity(Water)` generates candidates for every known entity: `MoveCargo(Waste)`, `MoveCargo(Sword)`, `MoveCargo(Bow)`, `Trade(Guard)`, `QueueForFacility(Mill)`, `QueueForFacility(Loom)`, etc. In late game, Waste entity accumulation (7-16 items) directly amplifies this, as each Waste generates MoveCargo, Trade, and Harvest affordances despite being irrelevant to the goal.

**Evidence source**: S93 golden tests, `reports/simulation-observer-report.md` (seed 7777, 1440 ticks).

**Candidate counts from S93 golden tests**:

| Test | Goal | Tick | Expansions | Candidates | Entities known |
|------|------|------|-----------|------------|----------------|
| merchant_vara_water_thornwall | AcquireCommodity(Water) | 11 | 300 | 1483 | 12 |
| guard_theron_water_thornwall | AcquireCommodity(Water) | 25 | 224 | 2085 | 14 |
| merchant_vara_apple_dusty_trail | AcquireCommodity(Apple) | 85 | 300 | 2511 | 12 |
| kael_water_thornwall_late | AcquireCommodity(Water) | 411 | 224 | 2657 | 16 (7 Waste) |
| merchant_vara_treat_wounds | TreatWounds | 456 | 300 | 5739 | 12 (9 Waste) |
| kael_treat_wounds_vara | TreatWounds | 471 | 224 | 4151 | 16 (9 Waste) |

## Design Goals

- Reduce candidate counts by 60-90% for commodity-specific goals by filtering non-travel candidates whose targets don't relate to the goal's commodity
- Ensure all 6 S93 golden budget-exhaustion tests find plans within budget after the fix
- Rewrite all 12 S93 golden tests (6 active + 6 ignored) into regression guards with zero ignored tests
- Preserve all causal paths — the filter is a computation optimization, not a causality change

## Non-Goals

- Modifying `matches_binding()` — it operates at the goal-entity binding level, not commodity relevance
- Changing `CognitiveProfile` or `ExecutionBudget` parameters
- Addressing non-commodity goals (Sleep, Combat, etc.) — they have different candidate profiles
- Per-agent filter tuning via CognitiveProfile — future work if needed
- Modifying the strategic/tactical decomposition pipeline
- Modifying the tactical candidate filter (location-based scoping is orthogonal)

## FOUNDATIONS Alignment

| Principle | How This Spec Satisfies It |
|-----------|----------------------------|
| FND-20 (Bounded Reasoning) | Agents prune irrelevant search branches — this is how bounded actors reason. A human trying to get water wouldn't consider trading for swords or picking up waste. The filter models attentional focus within resource-bounded practical reasoning. |
| FND-12 (Compress Computation Not Causality) | Pruning candidates that provably cannot satisfy the goal commodity is computation optimization. The world still permits any action; the agent's planner simply doesn't waste search budget on irrelevant branches. No causal path is removed from the world model. |
| FND-22 (Agent Diversity) | The filter uses the goal's commodity, which already varies per agent situation. Future specs could add per-agent filter aggressiveness via `CognitiveProfile` if needed. |
| FND-3 (Concrete State Over Abstract Scores) | Filtering uses concrete commodity kinds and belief-state commodity resolution (item_lot_commodity, resource_source.commodity), not abstract relevance scores. |
| FND-14 (Belief-Only Planning) | The filter reads commodity kinds from `PlanningState` / belief views, never from authoritative world state. |
| FND-26 (Systems Through State) | The filter reads existing state (commodity profiles, facility resource sources, trade payloads) without cross-system coupling. |
| FND-28 (No Backward Compat) | No compatibility shims for unfiltered candidate paths. The filter applies unconditionally when a goal has a target commodity. |

## Deliverables

### D1: Goal-commodity extraction method

Add a method to `GoalKindPlannerExt` in `goal_model.rs`:

```rust
/// Returns the primary commodity this goal is trying to acquire, produce, or use
/// as a prerequisite. Returns `None` for goals with no commodity focus (Sleep,
/// Combat, etc.), which bypasses commodity-relevance filtering.
fn target_commodity(&self) -> Option<CommodityKind>;
```

Implementation follows the established pattern in `strategic.rs:225-231` and `goal_model.rs:694-699`:

| GoalKind | target_commodity |
|----------|-----------------|
| AcquireCommodity { commodity, .. } | Some(commodity) |
| ConsumeOwnedCommodity { commodity } | Some(commodity) |
| RestockCommodity { commodity } | Some(commodity) |
| SellCommodity { commodity } | Some(commodity) |
| MoveCargo { commodity, .. } | Some(commodity) |
| TreatWounds { .. } | Some(Medicine) |
| ProduceCommodity { recipe_id } | recipe primary output commodity |
| FreeCarryCapacity | Some(Waste) |
| All others | None |

### D2: Commodity-relevance candidate filter

Add a filter function in `search/candidates.rs` that runs **after** the existing candidate pipeline (affordance generation + binding filter + blocked/place filters) and **before** the tactical candidate filter:

```rust
/// Retains only candidates whose targets are relevant to the goal's target
/// commodity. Travel candidates always pass (location scoping is handled by
/// the tactical filter). Candidates with no commodity association (planner-only
/// synthetic candidates) always pass.
fn apply_commodity_relevance_filter(
    candidates: &mut Vec<SearchCandidate>,
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
);
```

Filter rules per `PlannerOpKind`:

| OpKind | Filter logic | Commodity resolution |
|--------|-------------|---------------------|
| Travel | Always pass | N/A — location scoping is tactical filter's job |
| MoveCargo | Pass if target entity's commodity kind == goal commodity | `state.item_lot_commodity(target)` |
| Trade | Pass if trade payload's commodity == goal commodity | Payload's `commodity` field |
| QueueForFacilityUse | Pass if facility's resource source produces goal commodity | `state.resource_source(target).commodity` |
| Harvest | Pass if resource source yields goal commodity | `state.resource_source(target).commodity` |
| Craft | Pass if recipe has goal commodity as input or output | Recipe lookup via payload's `recipe_id` |
| Heal | Always pass (terminal op for TreatWounds) | N/A |
| AskWitness | Always pass (epistemic ops serve goal indirectly) | N/A |
| All others | Always pass (non-acquisition ops are rare in ACQUIRE_OPS context) | N/A |

**Bypass condition**: If `goal.key.kind.target_commodity()` returns `None`, skip the filter entirely. Non-commodity goals are unaffected.

**Integration point**: Call `apply_commodity_relevance_filter` at the end of `search_candidates()` (candidates.rs), after the existing blocked-facility / binding / place-blocker filters but before returning the filtered vec. The tactical filter in `search/mod.rs` runs later during the expansion loop.

### D3: Decision trace integration

Add a new `RootCandidateFilterReason` variant:

```rust
CommodityIrrelevant {
    candidate_commodity: Option<CommodityKind>,
    goal_commodity: CommodityKind,
}
```

Record filtered candidates in the existing `root_candidates` trace sink so decision traces show which candidates were pruned and why. This supports FND-29 (Debuggability).

### D4: Golden test rewrite

After D1-D3 land, the 6 active `_budgets_exhaust` tests should fail (plans now found within budget). Rewrite all 12 tests in `golden_budget_exhaustion_snapshots.rs`:

**Phase 1 tests** (currently assert `BudgetExhausted`) — convert to regression guards:
- Assert `search_plan` returns `Found` (not `BudgetExhausted`)
- Assert expansion count is below a reasonable threshold (e.g., < budget limit)
- Assert the returned plan contains commodity-relevant actions
- Keep the exact same snapshot setup (same beliefs, same entities, same cognitive profiles)

**Phase 2 tests** (currently `#[ignore]`) — un-ignore and verify:
- Remove `#[ignore = "..."]` attribute
- Assert `Found` with action chain execution
- Verify the goal's postcondition (thirst/hunger decreases, wounds treated)

**End state**: 12 active tests, 0 ignored tests. The tests serve as regression guards proving these specific scenarios no longer budget-exhaust.

**Test 3 (merchant_vara_apple_at_dusty_trail)**: This test may remain `BudgetExhausted` even after the commodity filter, because Apples are at Eldergrove Forest (2 hops away) and the agent doesn't know about Eldergrove. The commodity filter reduces irrelevant candidates, but if the strategic planner can't route to an unknown location, the plan is genuinely infeasible. If this test still budget-exhausts after D1-D3, convert it to assert the correct failure mode (either `BudgetExhausted` with significantly fewer candidates, or `FrontierExhausted`) and document why the scenario is infeasible under belief constraints.

## Section H — Causal Hooks Declaration

### H.1 — Motivating consequence gap

With S88-S90 completed, the two-phase strategic/tactical pipeline scopes search by location but not by commodity relevance. At-destination candidate generation still enumerates all affordances for all known entities matching the goal's op kinds. In realistic entity densities (12-16 entities, 7-16 Waste items), this produces 1400-5700 candidates that overwhelm the 224-300 node expansion budget.

The gap is between the strategic planner's knowledge of which commodity to acquire (it already knows — see `strategic.rs:225-231`) and the tactical search's failure to use that knowledge for candidate pruning. The information exists in the system; it simply isn't consulted during candidate generation.

### H.2 — Entities, relations, records introduced

None. No new components, entities, or relations. The filter reads existing belief-state data (`item_lot_commodity`, `resource_source`, trade payloads).

One new trait method `target_commodity()` on `GoalKindPlannerExt` (derived computation, not stored state).
One new `RootCandidateFilterReason` variant for decision traces (diagnostic only).

### H.3 — Actions or world processes that mutate them

None. All changes are planner-internal. No world state is mutated.

### H.4 — Information produced, travel, observability

Diagnostic only: filtered candidates appear in decision traces with `CommodityIrrelevant` reason. This information does not enter world state or agent beliefs.

### H.5 — Conserved quantities

None affected. The filter does not create, destroy, or transfer any items or commodities.

### H.6 — Scarce capacities, contention

None introduced. The filter operates entirely within the planner's search budget.

### H.7 — Partial failures, aftermath

The filter may cause some previously-explored search branches to be pruned. If the commodity resolution is incorrect (e.g., a facility's resource source commodity doesn't match beliefs), the candidate is incorrectly pruned and the plan becomes harder to find. Mitigation: the filter only prunes candidates where commodity kind can be positively resolved to a non-matching value; unknown/unresolvable commodity kinds pass the filter (conservative default).

### H.8 — Information-path analysis

The commodity-relevance filter reads commodity information from the agent's belief state:
1. `item_lot_commodity(entity)` — what commodity an item is, per the agent's beliefs
2. `resource_source(entity).commodity` — what a facility produces, per the agent's beliefs
3. Trade payload commodity — what commodity a trade involves, per the planner's payload synthesis

All information paths go through existing belief views (`SpatialBeliefView`, `FacilityBeliefView`), which are populated by the perception pipeline (Principle 7 compliance). No omniscient queries.

### H.9 — Positive-feedback analysis

No amplifying loops. The filter is a one-shot pruning step per candidate set. It does not create state that feeds back into itself.

### H.10 — Stored state vs. derived read-model list

| Item | Classification |
|------|---------------|
| `target_commodity()` return value | Derived (computed from GoalKind variant) |
| Commodity-relevance filter result | Derived (applied per candidate, not stored) |
| `CommodityIrrelevant` trace entry | Diagnostic (decision trace, not authoritative state) |

No new authoritative stored state.

## Verification

1. `cargo test -p worldwake-ai golden_budget_exhaustion` — all 12 tests pass, 0 ignored
2. `cargo test --workspace` — no regressions
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean
4. Candidate counts in rewritten tests should show significant reduction from S93 baselines
5. Decision traces for affected goals should show `CommodityIrrelevant` filter entries
