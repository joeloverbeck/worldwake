# S84: ShareBelief Operator Fix

## Summary

Investigate and fix why `ShareBelief` goals consistently frontier-exhaust at depth 0 with 1 expansion despite agents being co-located. S69 (Goal Dispatch Consolidation) correctly registered all three ShareBelief variants with `PlannerOpKind::Tell` operators, yet the simulation shows 14 of 20 failed plans for agents are frontier-exhausted ShareBelief goals. This spec traces the root cause — likely a mismatch between the `tell` action's `TargetSpec` and the planner's candidate search — and delivers the fix plus golden test coverage.

## Phase

Phase 7: Consequence Carriers (Adjunct — Simulation Remediation)

## Status

Draft

## Crates

- `worldwake-ai` (search candidate matching, goal dispatch)
- `worldwake-systems` (tell action definition, if target spec adjustment needed)

## Dependencies

- S69 (Goal Dispatch Consolidation) — completed (registered ShareBelief goal dispatch)

## Design Goals

- ShareBelief goals produce valid plans when a co-located listener exists
- When no listener exists, the goal is pruned early with a clear rejection reason (not silently frontier-exhausted at depth 0)
- The fix is minimal and targeted — no architectural changes to the tell action or goal dispatch system
- Golden test proves ShareBelief planning succeeds under co-location

## Non-Goals

- Reworking the tell action semantics or payload structure
- Adding travel-to-listener planning (ShareBelief is inherently local — if no listener is co-located, the goal should be deferred, not expanded into travel)
- Modifying goal ranking or priority for ShareBelief relative to other social goals

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-01 (Emergence) | Social information propagation (telling, gossip, testimony) is a core emergence mechanism — agents sharing beliefs drives downstream decisions |
| FND-07 (Locality) | Tell requires co-location — beliefs travel through physical proximity |
| FND-08 (Preconditions/Duration/Cost) | Tell action has preconditions (co-location, both alive), duration (2 ticks), and occupancy |
| FND-20 (Resource-Bounded Reasoning) | Budget should not be wasted on infeasible ShareBelief goals; early pruning when no listener exists |
| FND-29 (Debuggability) | The fix must leave clear traces of why ShareBelief succeeded or failed |

## Deliverables

### 1. Root Cause Investigation

The `tell` action (in `crates/worldwake-systems/src/tell_actions.rs`, lines 38-76) has these preconditions:

```
TargetSpec::EntityAtActorPlace { kind: Agent }
Precondition::TargetExists(0)
Precondition::TargetAtActorPlace(0)
Precondition::TargetKind { target_index: 0, kind: EntityKind::Agent }
Precondition::TargetAlive(0)
```

The search system (`crates/worldwake-ai/src/search/mod.rs`) generates candidates at depth 0 by matching `relevant_ops` (which is `[PlannerOpKind::Tell]`) against available action definitions. The candidates must pass:

1. **Action def lookup**: `PlannerOpKind::Tell` maps to `(ActionDomain::Social, "tell")`.
2. **Target resolution**: The search must find entities matching `TargetSpec::EntityAtActorPlace { kind: Agent }` in the planning snapshot.
3. **Precondition check**: All preconditions must pass against the hypothetical state.

The likely root cause is one of:
- **a)** The planning snapshot does not include co-located agents (filtered out by S73's entity relevance filtering, which may exclude agents not relevant to the goal's `relevant_op_kinds()`).
- **b)** The target resolution in the search does not correctly enumerate co-located agents for the `Tell` operator.
- **c)** The listener target is not populated in the `GoalKind::ShareBelief { listener, .. }` variant, or the search uses the wrong entity as the target.

### 2. Fix: Ensure Co-Located Agents Appear in Planning Snapshot for Tell Goals

If root cause (a) is confirmed: S73's `relevant_op_kinds()` for ShareBelief returns `[PlannerOpKind::Tell]`, and the snapshot relevance predicate must include `EntityKind::Agent` entities at the agent's place when Tell is a relevant op.

In `crates/worldwake-ai/src/search/` (snapshot construction or candidate resolution), ensure the relevance predicate for `PlannerOpKind::Tell` includes:

```rust
PlannerOpKind::Tell => {
    // Include agents at the same place as the planning agent
    entity_kind == EntityKind::Agent && entity_place == agent_place
}
```

### 3. Fix: Early Pruning When No Listener Available

If the goal target (listener) is specified in `GoalKind::ShareBelief { listener, .. }` but the listener is not co-located at planning time, the feasibility strategy `ColocationOrDead` should return `None` (Uncertain), which currently does NOT block the search — it only deprioritizes.

Add a pre-search filter in the goal dispatch pipeline: if `ColocationOrDead` returns `None` (neither co-located nor dead), skip the search entirely and record a clear rejection reason:

```rust
FeasibilityHint::None => {
    // Listener exists but not co-located — goal is infeasible right now
    record_rejection("listener not co-located");
    return PlanSearchOutcome::FrontierExhausted { expansions_used: 0 };
}
```

This prevents wasting 1 expansion on a goal that cannot produce candidates.

### 4. Golden Test: ShareBelief Succeeds Under Co-Location

In `crates/worldwake-ai/tests/`:

**Setup**: Two agents at the same location. Agent A has a belief that Agent B does not have. Agent A has a ShareBelief goal targeting Agent B.

**Assertion**: Within a bounded tick count, Agent A plans and executes a `tell` action targeting Agent B. After the tell commits, Agent B's belief store contains the shared belief.

### 5. Diagnostic Enhancement

Extend `PlanAttemptTrace` or `CandidateGenerationDiagnostics` to record, for frontier-exhausted goals:
- Number of operators checked
- Number of target entities found in snapshot matching the operator's `TargetSpec`
- If zero targets: the reason (no co-located agents, no agents in snapshot, listener not at place)

## Section H: Causal Hooks (FND-01)

### H1. Information-Path Analysis

- **Trigger**: Agent has a belief it wants to share (alarm, testimony, gossip).
- **Path**: Agent's belief store → ShareBelief goal generated → planner searches for Tell operator → tell action fires → listener's belief store updated.
- The information path is fully local: both agents must be co-located.

### H2. Positive-Feedback Analysis

- No new loops. Telling a belief does not amplify the original event.

### H3. Concrete Dampeners

- N/A.

### H4. Stored State vs. Derived

- **Stored**: Agent beliefs, ShareBelief goal target (listener EntityId).
- **Derived**: Co-location check (computed from belief about effective places).

## SystemFn Integration

No new SystemFn. The fix is within the existing search and/or snapshot filtering pipeline.

## Component Registration

No new components.

## Cross-System Interactions

- **Perception → Tell** (via belief state): Agent perceives events → forms beliefs → ShareBelief goal generated → Tell action shares the belief.
- **S73 Snapshot filtering → Tell search** (potential conflict): S73's entity relevance filter may exclude agents from the snapshot when the goal's `relevant_op_kinds()` doesn't signal that agents are needed. The fix ensures Tell goals include co-located agents in the snapshot.
