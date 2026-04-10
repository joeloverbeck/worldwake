**Status**: COMPLETED

# Simulation Remediation Proposals

Source report: `reports/simulation-observer-report.md`
Generated: 2026-04-09

## Proposed Golden Tests

### GT-1: Multi-Agent Convergence at Barren Location Does Not Cause Prolonged Behavioral Collapse
**Source finding**: Finding 2 (Action Loops — MEDIUM), Finding 3 (Stuck Agents — CRITICAL)
**Severity**: CRITICAL
**File**: `crates/worldwake-ai/tests/golden_simulation_gaps.rs`
**Setup**: 3+ agents at a resource-barren place (no food/water sources). Remote places hold food and water. Agents start with seeded beliefs about the remote resource locations. Run for 600+ ticks.
**Assertion**: No agent enters a sleep+relieve-only loop for more than 200 consecutive ticks. At least one agent commits a `travel` action toward a resource-bearing location within 300 ticks.
**Rationale**: Existing `golden_max_idle_under_remote_resource_scarcity` (Scenario 127) tests a single agent with pre-seeded remote beliefs and asserts <100 idle ticks. But the observer report shows the multi-agent case is qualitatively different: with 4 agents, all collapse into sleep+relieve by tick 500 for 900+ ticks. The multi-agent scenario may expose candidate explosion or contention effects not visible in the single-agent test. This test protects the invariant that agents with beliefs about remote resources should attempt travel rather than loop indefinitely.
**Existing coverage**: Partially overlaps with Scenario 127 (`golden_max_idle_under_remote_resource_scarcity`) but that test uses 1 agent with pre-seeded beliefs and runs only 300 ticks. The observer report failure occurs at scale (4 agents, 1440 ticks) where the planner budget-exhausts under higher candidate counts.

### GT-2: Agent Death from Unaddressed Needs Triggers Within Expected Timeframe
**Source finding**: Finding 3 (Stuck Agents — CRITICAL), Finding 6 (Unaddressed Needs — CRITICAL)
**Severity**: CRITICAL
**File**: `crates/worldwake-ai/tests/golden_simulation_gaps.rs`
**Setup**: One agent at a location with no eat/drink affordances and no beliefs about remote resource locations. Run until the agent dies or 600 ticks elapse.
**Assertion**: (1) The agent dies within an expected tick range (not too early — basic survival actions should delay death, not too late — needs should escalate). (2) A `DeadAt` component is set with an explicit cause traceable to the unmet need. (3) Post-death, no planning or action attempts occur for the dead agent.
**Rationale**: Guard Theron died around tick 420 but the death mechanism wasn't explicitly logged. This test ensures death from starvation/dehydration is an explicit, traceable world event (FND-04 — Persistent Identity; FND-10 — Outcomes Are Granular and Leave Aftermath). The post-death silence is already implicitly tested but should be explicitly asserted.
**Existing coverage**: `golden_supply_chain.rs` asserts `!h.agent_is_dead(...)` to confirm agents stay alive in supply chain scenarios. No existing test explicitly verifies the death-from-unmet-needs path or its traceability.

### GT-3: Affordances Include Eat/Drink When Resource Sources Exist at Agent Location
**Source finding**: Finding 6 (Unaddressed Needs — CRITICAL)
**Severity**: CRITICAL
**File**: `crates/worldwake-ai/tests/golden_simulation_gaps.rs`
**Setup**: One agent at a location with a Water resource source. A second agent at a location with an Apple resource source and a workstation (e.g., OrchardRow).
**Assertion**: The affordance set generated for each agent at tick 0 includes `drink` (for the water agent) and `eat` (for the apple agent). If a resource source exists at an agent's location, the corresponding consumption affordance must be generated.
**Rationale**: The observer report shows Vara at Thornwall Village (which has a Water source) lacked `drink` in her affordances, and Theron at Dusty Trail lacked both `eat` and `drink`. If a resource source exists at a location, the affordance system should produce consumption candidates. This may be a bug in `get_affordances` or a missing precondition in the affordance query. This is the highest-priority test because it's the root gate preventing agents from addressing their needs.
**Existing coverage**: `golden_ai_decisions.rs` `golden_fallback_to_addressable_need_when_top_need_unsatisfiable` tests fallback when thirst is unsatisfiable, but it gives the agent owned food — it doesn't test affordance generation from resource sources at the location.

## Proposed Spec Changes

### SC-1: Exploration Drive / Geographic Knowledge Acquisition
**Source finding**: Finding 8 (Belief Staleness — MEDIUM), Finding 10 (Economic Stagnation — CRITICAL)
**Spec**: New spec needed — no existing spec covers exploration behavior
**Section**: N/A (new spec)
**Change**: Design an exploration/curiosity system where agents with unsatisfied needs and no known path to satisfaction develop an exploration pressure that generates travel goals to unvisited locations. This should be profile-driven (per-agent curiosity weight) and belief-gated (agents only explore places they've heard about or that are adjacent to known places in the topology).
**FOUNDATIONS alignment**:
- FND-01 (Maximal Emergence): Exploration should emerge from unmet needs + limited geographic beliefs, not from a scripted "explore" trigger
- FND-14 (World State Is Not Belief State): Agents should reason about their own ignorance — "I don't know where water is" should motivate information-seeking behavior
- FND-15 (Knowledge Is Acquired Locally): Geographic knowledge should propagate through travel, testimony, and observation
- FND-20 (Resource-Bounded Practical Reasoning): Exploration goals compete with other goals through the normal priority/planning pipeline
- FND-22 (Agent Diversity): Curiosity/exploration weight varies per agent — some agents are homebodies, others are natural explorers

### SC-2: Plan Search Budget Scaling or Complexity Reduction for AcquireCommodity
**Source finding**: Finding 5 (Sustained Critical Needs — CRITICAL), Finding 10 (Economic Stagnation — CRITICAL)
**Spec**: This may fit as a revision to the AI planning spec (check existing planning/GOAP specs) or as a new focused spec
**Section**: Search budget and candidate pruning
**Change**: Address the AcquireCommodity budget exhaustion at 224-300 expansions with 1500-2900 candidates. Options: (a) profile-driven search budget that scales with plan complexity, (b) hierarchical plan decomposition to reduce branching, (c) belief-informed candidate pruning that filters out plans involving unknown locations. The observer report shows agents know about resources but can't plan to reach them within the expansion budget — the search space branches too widely across locations and methods.
**FOUNDATIONS alignment**:
- FND-20 (Resource-Bounded Practical Reasoning): Agents reason under bounded computation — but the bound should be tunable per agent via profiles, not a hardcoded magic number
- FND-02 (No Ungrounded Triggers): The budget should derive from agent cognitive profile, not be an arbitrary system constant

## Proposed Tickets

### TK-1: Investigate Missing Drink/Eat Affordances at Locations with Resource Sources
**Source finding**: Finding 6 (Unaddressed Needs — CRITICAL)
**Priority**: P0
**Crate(s)**: `worldwake-systems` (affordance generation), possibly `worldwake-core` (resource source → affordance mapping)
**Description**: The observer report shows Merchant Vara at Thornwall Village (which has a Water resource source) has no `drink` affordance, and Guard Theron at Dusty Trail has no `eat` or `drink` affordances. Investigate `get_affordances` / `affordance_query.rs` to determine why consumption affordances are not generated when resource sources exist at the agent's location. This may be: (a) a missing affordance rule for "resource source at location → consumption affordance", (b) a precondition requiring the agent to own the commodity before generating eat/drink, or (c) a missing intermediate step (harvest → own → eat) that the affordance system doesn't compose.
**Dependencies**: None — this is the root cause investigation
**Acceptance criteria**: (1) Identified the exact code path that filters out eat/drink affordances when a resource source is present at the location. (2) If it's a bug, fixed so affordances are generated. (3) GT-3 passes after the fix.

### TK-2: Investigate AcquireCommodity Plan Search Budget Exhaustion
**Source finding**: Finding 5 (Sustained Critical Needs — CRITICAL), Finding 10 (Economic Stagnation — CRITICAL)
**Priority**: P0
**Crate(s)**: `worldwake-ai` (planner, search.rs)
**Description**: AcquireCommodity consistently generates 1500-2900 candidates and budget-exhausts at 112-300 expansions. The plan chains likely involve: locate resource source → plan travel → plan harvest/craft → plan consume, with each step branching across multiple locations and methods. Investigate: (a) what the actual plan graph looks like for a Water acquisition from Thornwall Village, (b) where the branching explosion occurs, (c) whether candidate pruning using belief state (agent only knows 1-2 places) could reduce the search space, (d) whether the expansion budget profile parameter needs tuning or the search strategy needs restructuring.
**Dependencies**: None — can proceed in parallel with TK-1
**Acceptance criteria**: (1) Documented the AcquireCommodity plan graph shape and branching hotspots. (2) Identified a concrete approach to make the plan search succeed within budget. (3) At least one agent in the cli-evaluation scenario successfully plans and executes an AcquireCommodity chain.

### TK-3: Add Death-Cause Logging to Mortality System
**Source finding**: Finding 3 (Stuck Agents — CRITICAL)
**Priority**: P2
**Crate(s)**: `worldwake-systems` (needs/mortality)
**Description**: Guard Theron died around tick 420 but the observer report notes "death mechanism isn't explicitly logged (which need killed him, at what tick)." The `DeadAt` component records the tick but not the cause. Add a `DeathCause` component or field (e.g., `Starvation`, `Dehydration`, `Combat`, `Other`) that is set alongside `DeadAt` when an agent dies, and emit an event to the event log with the cause.
**Dependencies**: None
**Acceptance criteria**: (1) `DeadAt` is accompanied by a cause. (2) The event log contains a death event with cause, tick, and agent. (3) Observer dump can report death cause.

### TK-4: Perception System Change-Detection to Reduce Redundant Observations
**Source finding**: Finding 1 (Redundant Perception — MEDIUM)
**Priority**: P3
**Crate(s)**: `worldwake-systems` (perception)
**Description**: Every agent repeatedly observes the same entities — Kael observed itself 141 times. The perception system fires on a tick-aligned schedule regardless of whether the observed entity's state has changed. Consider: (a) a state-generation counter on entities that perception checks before creating a new observation event, (b) suppressing self-observation (agents don't need to "perceive" themselves), or (c) extending the perception interval when the observed entity hasn't changed.
**Dependencies**: None
**Acceptance criteria**: (1) Self-observation frequency is reduced or eliminated. (2) Repeated observations of unchanged entities are suppressed or batched. (3) Existing golden tests still pass (perception is not under-firing).

### TK-5: Periodic Affordance Snapshots in Observer Dump
**Source finding**: Trace Quality — "Affordances are only shown at tick 0"
**Priority**: P2
**Crate(s)**: `worldwake-cli` (observer binary)
**Description**: The observer binary currently captures affordances only at tick 0. Agents that travel have different affordances at their current location, making tick-0 data misleading for late-game analysis. Emit affordance snapshots every 200 ticks (configurable) so the observer report can track how an agent's action space evolves over time.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-29 (Debuggability) — "Why did this agent not do X?" requires knowing what affordances were available when the need arose, not just at simulation start.
**Acceptance criteria**: (1) Observer dump includes affordance snapshots at configurable intervals. (2) Remediation analysis can reference mid-simulation affordances.

### TK-6: Belief Acquisition Timeline in Observer Dump
**Source finding**: Trace Quality — "Belief summary is end-state only"
**Priority**: P2
**Crate(s)**: `worldwake-cli` (observer binary), possibly `worldwake-core` (belief event emission)
**Description**: The belief summary shows only end-state beliefs. A belief trajectory over time would reveal when agents learned about resources, places, or other agents — critical for diagnosing belief staleness (smell 8) and economic stagnation (smell 10). Add timestamped belief acquisition/update events to the dump, or at minimum belief snapshots at the same interval as affordance snapshots.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-29 (Debuggability), FND-15 (Knowledge Is Acquired Locally — tracing the knowledge acquisition path requires knowing when beliefs were acquired).
**Acceptance criteria**: (1) Observer dump includes belief acquisition timeline or periodic snapshots. (2) Staleness analysis can identify when an agent's geographic knowledge stopped growing.

### TK-7: Entity State-Change Counts Per Observation in Perception Trace
**Source finding**: Trace Quality — "Perception trace doesn't include what entity state was observed" + "No entity state-change tracking"
**Priority**: P3
**Crate(s)**: `worldwake-systems` (perception), `worldwake-cli` (observer binary)
**Description**: The perception system creates observation events but doesn't record what changed (or whether anything changed) on the observed entity. Adding a state-generation counter or change-flag to observation events would let the observer distinguish meaningful perceptions from redundant ones, making smell 1 (Redundant Perception) assessable with HIGH confidence instead of MEDIUM.
**Dependencies**: Complements TK-4 (perception change-detection). TK-7 provides the measurement; TK-4 uses it for optimization.
**FOUNDATIONS alignment**: FND-29 (Debuggability). Also supports TK-4 — if we can measure redundancy precisely, we can tune the system.
**Acceptance criteria**: (1) Observation events include a flag or counter indicating whether the observed entity's state changed since last observation. (2) Observer dump reports change-vs-unchanged observation ratios.

### TK-8: Remove Failed Plan Attempt Truncation in Observer Dump
**Source finding**: Trace Quality — "Section 7 shows 'first 20 of N' failed plan attempts — truncation may hide important late-game failures"
**Priority**: P3
**Crate(s)**: `worldwake-cli` (observer binary)
**Description**: Section 7 truncates failed plan attempts to the first 20. Late-game failures (after resource exhaustion) may reveal different planning bottlenecks than early-game failures. Either remove the truncation or add a summary of late-game failures (last 20) alongside the early ones.
**Dependencies**: None
**FOUNDATIONS alignment**: FND-29 (Debuggability) — truncated data hides causal chains.
**Acceptance criteria**: (1) Observer dump shows all failed plan attempts, or at minimum first-20 + last-20 with a count of omitted entries.

## Findings Deferred or Not Requiring Independent Remediation

| Finding | Severity | Reason for Deferral |
|---------|----------|---------------------|
| Finding 2: Action Loops | MEDIUM | Downstream symptom of Finding 5 + Finding 10 (resource starvation cascade). Once agents can plan AcquireCommodity (TK-2) and have correct affordances (TK-1), the sleep+relieve loop should break naturally. Covered by GT-1. |
| Finding 3: Stuck Agents (Kael, Vara short stucks) | MEDIUM | Short 27-34 tick idle periods are likely normal inter-action gaps. Only Theron's 1019-tick stuck is pathological, and that's caused by death (deferred to TK-1 + TK-3). |
| Finding 4: Failed Action Spirals | LOW | Localized early-game failures (staff_market, tell) that don't repeat. Not true spirals. Monitor after TK-1/TK-2 fixes. |
| Finding 7: Impossible Knowledge | NONE | No evidence of violation. No remediation needed. |
| Finding 9: Social Isolation | LOW | Partial social behavior exists. Absence of trade is downstream of economic stagnation (Finding 10). Once production/trade chains work (TK-2), social economic interaction should emerge. |

## Summary

| Type | Count | Severity Breakdown |
|------|-------|--------------------|
| Golden Tests | 3 | 3 CRITICAL |
| Spec Changes | 2 | 1 CRITICAL (SC-2), 1 MEDIUM (SC-1) |
| Tickets | 8 | 2 P0, 1 P2 behavioral, 1 P3 behavioral, 3 P2 trace-quality, 1 P3 trace-quality |
| Deferred | 5 | 1 MEDIUM, 2 LOW, 1 NONE, 1 partial-MEDIUM |

### Root Cause Chain

```
TK-1 (Missing affordances)  ──┐
                               ├──> Finding 6 (Unaddressed Needs)
                               │         │
TK-2 (Budget exhaustion)  ────┤         ├──> Finding 5 (Sustained Critical Needs)
                               │         │         │
                               │         │         ├──> Finding 3 (Theron death / stuck agents)
                               │         │         │         │
                               │         │         │         └──> TK-3 (Death cause logging)
                               │         │         │
                               │         │         └──> Finding 2 (Action loops)
                               │         │
                               └─────────┴──> Finding 10 (Economic stagnation)
                                                    │
SC-1 (Exploration drive)  ──> Finding 8 (Belief staleness / geographic trap)
                                    │
                                    └──> Finding 9 (Social isolation — no trade)

Finding 1 (Redundant perception) ──> TK-4 (independent, low priority)
Finding 4 (Failed action spirals) ──> deferred (monitor)

Trace Quality (FND-29):
  TK-5 (affordance snapshots) ──> improves future smell 6/8 diagnosis
  TK-6 (belief timeline) ──> improves future smell 8/10 diagnosis
  TK-7 (state-change counts) ──> improves future smell 1 diagnosis, feeds TK-4
  TK-8 (truncation removal) ──> improves future smell 5/10 diagnosis
```

**Implementation order**:
1. **First (parallel)**: TK-1 + TK-2 — these are the two root causes. TK-1 investigates why affordances are missing; TK-2 investigates why plan search exhausts. Both are independent investigations.
2. **After TK-1/TK-2**: GT-1, GT-2, GT-3 — write golden tests once the root causes are understood (tests may need to encode the fix).
3. **After golden tests pass**: SC-1 (exploration spec) — this is a design-level enhancement that addresses the geographic trap. It should be specced after the mechanical bugs are fixed so the spec doesn't conflate bug fixes with new features.
4. **After SC-1**: SC-2 (budget scaling spec) — may be informed by TK-2 findings. If TK-2 reveals the budget just needs a profile parameter tweak, SC-2 may be unnecessary. If it reveals a structural search problem, SC-2 becomes the design solution.
5. **Independent, low priority**: TK-3 (death cause logging), TK-4 (perception redundancy).
6. **Wave 3 (independent, low priority)**: TK-5 through TK-8 (trace-quality improvements). These are independent of behavioral fixes and can proceed in parallel. They improve future observer runs and align with FND-29.

### FOUNDATIONS Alignment

| Principle | Status | Notes |
|-----------|--------|-------|
| FND-01 (Maximal Emergence) | VIOLATED | Economic stagnation means emergence never bootstraps. Agents converge and stagnate rather than producing emergent supply chains, trade, or social behavior. |
| FND-02 (No Ungrounded Triggers) | OK | No evidence of ungrounded triggers. Plan search budget may be a magic number (see SC-2) but it's a computational bound, not a drama lever. |
| FND-06 (World Runs Without Observers) | VIOLATED | The world "runs" but degenerates into stasis by tick 500. The simulation advances but produces no meaningful change for 62% of its duration. |
| FND-07 (Locality of Motion) | OK | Agents only act on co-located or perceived information. |
| FND-08 (Preconditions, Duration, Cost) | OK | Actions have proper preconditions. The issue is that the planner can't find paths through them, not that they're missing. |
| FND-10 (Outcomes Leave Aftermath) | PARTIAL | Death occurs but cause is not explicitly logged (TK-3). |
| FND-14 (World State Is Not Belief State) | OK | Agents plan from beliefs. The problem is beliefs are too limited (Finding 8), not that they bypass belief state. |
| FND-15 (Knowledge Acquired Locally) | OK but INCOMPLETE | Knowledge acquisition works but there's no mechanism for agents to seek knowledge about unknown places (SC-1). |
| FND-20 (Resource-Bounded Practical Reasoning) | VIOLATED | The resource bounds (search budget) are so tight that agents can't reason about multi-step plans at all. The principle says agents should reason under bounded resources — but the bound should still permit basic survival planning. |
| FND-22 (Agent Diversity) | NOT TESTED | All 4 agents collapsed into identical behavior. Diversity cannot manifest when all agents are trapped in the same survival failure mode. |
| FND-29 (Debuggability) | PARTIAL — addressed by TK-3, TK-5, TK-6, TK-7, TK-8 | Decision traces and action traces are excellent. Missing: death cause logging (TK-3), affordance snapshots over time (TK-5), belief acquisition timeline (TK-6), perception state-change tracking (TK-7), untruncated failed plan attempts (TK-8). |

None of the proposed remediations introduce new FOUNDATIONS violations. GT-1/GT-2/GT-3 enforce existing principles. SC-1 extends the system to better serve FND-01/FND-15/FND-20. SC-2 tunes computational bounds to serve FND-20. TK-1/TK-2 fix bugs that prevent existing principles from functioning. TK-3/TK-4 improve debuggability (FND-29). TK-5 through TK-8 systematically address the FND-29 gaps identified in the observer report's Trace Quality Assessment — affordance snapshots (TK-5), belief timeline (TK-6), perception state-change tracking (TK-7), and untruncated plan failure data (TK-8).

## Outcome

- Completion date: 2026-04-09
- What changed: Archived this remediation report from `reports/` to `archive/reports/` because it is now exploited.
- Deviations from original plan: None.
- Verification results: Archival metadata added, file moved to `archive/reports/`, and source path removed from `reports/`.
