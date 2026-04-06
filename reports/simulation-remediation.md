# Simulation Remediation Proposals

Source report: `reports/simulation-observer-report.md`
Generated: 2026-04-06

## Proposed Golden Tests

### GT-1: Planner Falls Back to Addressable Needs When Top Need Is Unsatisfiable

**Source finding**: Finding 1 (Universal Dehydration — CRITICAL) + Finding 2 (Guard Theron Stuck — HIGH)
**Severity**: CRITICAL
**File**: `crates/worldwake-ai/tests/golden_resilience.rs`
**Setup**: Single AI agent with:
- High thirst (700+) and moderate hunger (400+)
- No Water items anywhere in the world (thirst unsatisfiable)
- Food items available at the agent's location (hunger satisfiable)
- Standard UtilityProfile with thirst_weight > hunger_weight
- DriveThresholds and MetabolismProfile configured so thirst escalates quickly
**Assertion**: Within 100 ticks, the agent should execute at least one `eat` or `sleep` action despite thirst being the highest-priority unsatisfied need. The agent must NOT remain idle for 100+ consecutive ticks. This verifies the planner falls back to the next-best addressable goal when the top goal has no viable plan.
**Rationale**: The observer run showed Guard Theron idle for 1024 ticks because thirst was unsatisfiable and the planner did not fall back to lower-priority needs. This test ensures the planner degrades gracefully under resource scarcity rather than entering pathological idle states. Aligns with FOUNDATIONS Principle 20 (Resource-Bounded Practical Reasoning) — agents should make the best decision available, not freeze when the optimal plan is impossible.

### GT-2: Merchant Chains Travel to Home Facility for Market Staffing

**Source finding**: Finding 3 (Economic Stagnation — HIGH) + Finding 4 (Failed Action Spirals — MEDIUM)
**Severity**: HIGH
**File**: `crates/worldwake-ai/tests/golden_merchant_selling.rs`
**Setup**: AI merchant agent with:
- MerchandiseProfile with `home_facility` set to Place A
- Agent starts at Place B (1-2 travel ticks from Place A)
- High enterprise_weight (800+), low need pressure (all needs < 200)
- Stock of sale commodities on the agent
- A buyer agent at Place A with Coin and needs matching the merchant's goods
**Assertion**: Within 50 ticks, the merchant should execute `travel` to Place A followed by `staff_market`. Verify the planner chains travel + staff_market as a multi-step plan rather than repeatedly failing `staff_market` at the wrong location.
**Rationale**: The observer run showed Merchant Vara failing `staff_market` 6 times (100% failure rate) because she was at Dusty Trail while her home_facility was Thornwall Village. The planner never chained travel + staff_market. This test protects the merchant economic loop. Aligns with FOUNDATIONS Principle 8 (actions have preconditions) and Principle 20 (agents reason through enabling subchains).

## Proposed Spec Changes

No spec changes proposed. The findings point to scenario gaps and planner behavior issues, not design-level gaps in existing specs. S55 (Causally Grounded Blocker Invalidation) and S56 (Context-Modulated Perception Exposure) are the active specs and are unrelated to these findings.

## Proposed Tickets

### TK-1: Add Water Resource Source to CLI Evaluation Scenario

**Source finding**: Finding 1 (Universal Dehydration — CRITICAL)
**Priority**: P0
**Crate(s)**: None (scenario file only)
**Description**: Add a water resource source to `scenarios/cli-evaluation.ron` so AI agents can satisfy thirst. The `drink` action and `ConsumeOwnedCommodity { Water }` goal path are fully implemented and tested by `golden_thirst_driven_acquisition`. The scenario simply lacks accessible water — only Kael (Human, inactive) holds Water items, and no `ResourceSourceDef` for Water exists. Add a water resource source at an appropriate place (e.g., Thornwall Village or Eldergrove Forest) and optionally seed some Water items at locations where AI agents start.
**Acceptance criteria**:
- `scenarios/cli-evaluation.ron` contains a `ResourceSourceDef` for Water
- Re-running `/simulation-observer` shows at least one AI agent executing a `drink` action
- Thirst averages drop significantly from the current 926-981 range

### TK-2: Investigate Planner Idle State When Top-Priority Need Has No Viable Plan

**Source finding**: Finding 2 (Guard Theron Stuck — HIGH)
**Priority**: P1
**Crate(s)**: `worldwake-ai`
**Description**: When the highest-priority need (e.g., thirst) has no viable plan because the required commodity doesn't exist, the planner should fall back to the next-best addressable goal rather than entering an extended idle state. Guard Theron was idle for 1024 consecutive ticks despite having addressable lower-priority needs (hunger at 367 avg, fatigue at 511 avg) and patrol duties.

Investigate the decision cycle in `agent_tick.rs` and `candidate_generation.rs`:
1. Does `generate_candidates` produce thirst-relief as the top candidate, blocking lower candidates?
2. Does `search_plan` fail for thirst-relief and then skip remaining candidates?
3. Is there a cooldown/blocking mechanism that prevents re-evaluation after a failed plan search?

The fix should ensure the planner evaluates multiple candidates and selects the best one with a viable plan, not the best one regardless of plan feasibility.

**Acceptance criteria**:
- An agent with an unsatisfiable top-priority need still takes actions for addressable lower-priority needs
- Max consecutive idle ticks for AI agents with addressable needs drops below 50
- GT-1 (proposed above) passes

### TK-3: Investigate Planner Multi-Step Plan Chaining for Location-Dependent Actions

**Source finding**: Finding 3 (Economic Stagnation — HIGH) + Finding 4 (Failed Action Spirals — MEDIUM)
**Priority**: P2
**Crate(s)**: `worldwake-ai`
**Description**: Merchant Vara's `staff_market` failed 6 times because she was at Dusty Trail while her `home_facility` is Thornwall Village. The planner generates the `staff_market` goal (enterprise_weight: 800) but doesn't chain `travel` as a prerequisite when the agent isn't at the required location.

Investigate whether:
1. The planner's `search_plan` considers travel as a prerequisite step for location-dependent actions
2. `staff_market`'s precondition (co-location with home facility) is visible to the planner's search
3. The `max_prerequisite_locations` setting in `ExecutionBudget` (set to 3 for Vara) is being respected

This may overlap with existing travel-chaining logic for other actions (e.g., `eat` at a remote food source). Check whether `staff_market` is missing the same planner integration that `eat`/`drink` have.

**Acceptance criteria**:
- A merchant at a remote location from their home facility successfully chains travel + staff_market
- `staff_market` StartFailed count drops to 0 when the merchant can reach the facility
- GT-2 (proposed above) passes

## Findings Not Requiring Remediation

| Finding | Severity | Reason |
|---------|----------|--------|
| 5. Sleep Loops | MEDIUM | Symptom of dehydration (Finding 1). Will likely resolve when water is available. Revisit after TK-1. |
| 6. Redundant Perception | LOW | Expected behavior for co-located idle agents. Not worth remediation. |
| 7. Kael Stuck | NONE | Expected — Human agent with no input. |
| 8. Social Isolation | LOW | Expected in small agent count (4 agents, 1 inactive). Early simulation shows healthy social activity. |
| 9. Impossible Knowledge | NONE | No violations detected. |
| 10. Belief Staleness | INCONCLUSIVE | Insufficient trace data. No remediation possible without belief snapshots in dump. |

## Summary

| Type | Count | Severity Breakdown |
|------|-------|--------------------|
| Golden Tests | 2 | 1 CRITICAL, 1 HIGH |
| Spec Changes | 0 | — |
| Tickets | 3 | 1 P0, 1 P1, 1 P2 |
