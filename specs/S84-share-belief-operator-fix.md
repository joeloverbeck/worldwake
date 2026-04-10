# S84: ShareBelief Operator Fix

## Summary

Investigate and fix why `ShareBelief` goals consistently frontier-exhaust at depth 0 with 1 expansion despite agents being co-located. S69 (Goal Dispatch Consolidation) correctly registered all three ShareBelief variants with `PlannerOpKind::Tell` operators, yet the simulation shows 14 of 20 failed plans for agents are frontier-exhausted ShareBelief goals. This spec traces the confirmed root cause — a mismatch between the planning snapshot's entity indexing and the agent's belief state about co-located listeners — and delivers the fix plus golden test coverage.

## Phase

Phase 7: Consequence Carriers (Adjunct — Simulation Remediation)

## Status

Draft

## Crates

- `worldwake-ai` (search candidate matching, goal dispatch, planning snapshot)
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
| FND-08 (Preconditions/Duration/Cost) | Tell action has preconditions (actor alive, co-location, both alive), duration (2 ticks), and occupancy |
| FND-14 (World State != Belief State) | The confirmed root cause involves the distinction between authoritative co-location and the agent's belief about the listener's effective place. The planner operates on belief state only; if the agent lacks a belief about the listener's location, the search snapshot cannot index the listener at the actor's place even though the listener entity is included |
| FND-20 (Resource-Bounded Reasoning) | Budget should not be wasted on infeasible ShareBelief goals; early pruning when no listener exists |
| FND-29 (Debuggability) | The fix must leave clear traces of why ShareBelief succeeded or failed |

## Deliverables

### 1. Root Cause Investigation

The `tell` action (in `crates/worldwake-systems/src/tell_actions.rs`, lines 38-77) has these constraints and preconditions:

```
actor_constraints: [Constraint::ActorAlive]
TargetSpec::EntityAtActorPlace { kind: EntityKind::Agent }
Precondition::ActorAlive
Precondition::TargetExists(0)
Precondition::TargetAtActorPlace(0)
Precondition::TargetKind { target_index: 0, kind: EntityKind::Agent }
Precondition::TargetAlive(0)
```

The search system (`crates/worldwake-ai/src/search/mod.rs`) generates candidates at depth 0 by matching `relevant_ops` (which is `[PlannerOpKind::Tell]`) against available action definitions. The candidates must pass:

1. **Action def lookup**: `PlannerOpKind::Tell` maps to `(ActionDomain::Social, "tell")` via `classify_action_def` in `planner_ops.rs`.
2. **Target resolution**: The search calls `get_affordances_for_defs` (`affordance_query.rs:60`), which enumerates entities matching `TargetSpec::EntityAtActorPlace { kind: EntityKind::Agent }` by querying `view.entities_at(place)` on the planning snapshot.
3. **Precondition check**: All preconditions must pass against the hypothetical state.

The root cause audit originally considered several possibilities. Current code and ticket outcomes now confirm the snapshot-indexing path below as the live production contradiction:

- **Confirmed**: The agent's belief view does not report the listener's `effective_place` — The planning snapshot includes the listener entity unconditionally (via `evidence_entities`, `planning_snapshot.rs:1108`), but `build_snapshot_places` (`planning_snapshot.rs:789-792`) only indexes an entity at a place if `view.effective_place(*entity) == Some(place)`. If the agent has no belief about the listener's location (despite physical co-location), the listener will not appear in `snapshot.entities_at(actor_place)`, causing the affordance query to find zero targets.
- **b) Target resolution in the search does not correctly enumerate co-located agents for the `Tell` operator** — The affordance query may fail for a reason unrelated to the snapshot's entity index (e.g., precondition evaluation rejects valid targets).
- **c) The listener target is not populated correctly in the `GoalKind::ShareBelief { listener, .. }` variant, or the search uses the wrong entity as the target** — The candidate generation (`candidate_generation.rs:1135`) uses `social_listeners_at()` which queries `view.entities_at(place)` on the belief view. If this function correctly finds listeners but the search cannot use them, the issue is in the handoff from candidate generation to search.
- **a) [Disproven] The planning snapshot filters out co-located agents** — Codebase analysis shows this is not the case: `SnapshotEntityFilter::includes(EntityKind::Agent, alive)` always returns `true` for living agents (`planning_snapshot.rs:93-95`), and the listener is unconditionally added via `evidence_entities`. However, the listener's placement in the snapshot's `entities_at` index depends on belief-layer `effective_place` (see hypothesis d).

**Diagnostic steps for investigation**:
1. Instrument `build_snapshot_places` to log whether the listener entity appears in `entities_at(actor_place)` for ShareBelief goals
2. Check whether `view.effective_place(listener)` returns `Some(actor_place)` or `None` during snapshot construction
3. If `effective_place` returns `None`, trace why the agent lacks a belief about the listener's location despite being co-located — likely a perception gap
4. If `effective_place` is correct, check whether `get_affordances_for_defs` returns any affordances for the tell action with the listener as target

### 2. Fix: Ensure Listener Appears in Planning Snapshot's Place Index

If root cause (d) is confirmed: the agent's belief view lacks `effective_place` for the listener, so the snapshot cannot index the listener at the actor's place.

**Option A (preferred)**: In `build_snapshot_places` (`planning_snapshot.rs:789-792`), for evidence entities whose `effective_place` is `None`, fall back to checking whether the entity was observed at any included place. This preserves the belief-only planning invariant (FND-14) while handling the case where the agent knows about an entity but lacks explicit location beliefs.

**Option B**: Ensure the perception system generates `effective_place` beliefs for co-located agents. This is a broader fix that may require changes to the perception pipeline.

The fix must not violate FND-14 — agents must never read authoritative world state. Any fallback must derive from the agent's existing belief state.

### 3. Fix: Early Pruning When No Listener Available

The feasibility system (`feasibility.rs:19-20`) is explicitly documented as reordering-only: "Used to reorder candidates within the same `GoalPriorityClass` — never to exclude goals from search." The `check_colocated_or_dead` function (`feasibility.rs:240-254`) returns `Option<FeasibilityHint>` where `Option::None` (Rust's `None`, not a `FeasibilityHint` variant) means "neither co-located nor dead — no opinion."

Since the feasibility system is not a gate, early pruning requires a separate mechanism. Two approaches:

**Option A (recommended)**: Add a pre-search validation step in the planning pipeline (after feasibility scoring, before `search_plan`) that checks whether the goal's `relevant_ops` require specific target kinds and whether the snapshot contains any matching targets at the actor's place. If zero targets exist, skip the search and record a clear rejection:

```rust
// In the planning pipeline, before calling search_plan:
if snapshot_has_no_matching_targets(goal, &snapshot) {
    record_rejection("no matching targets in snapshot for goal's relevant ops");
    return PlanSearchOutcome::FrontierExhausted { expansions_used: 0 };
}
```

**Option B**: Accept the 1-expansion cost and rely on improved diagnostics (Deliverable 5) to explain why the search exhausted. This avoids adding a new mechanism but wastes budget on clearly infeasible goals.

### 4. Golden Test: ShareBelief Succeeds Under Co-Location

In `crates/worldwake-ai/tests/`:

**Setup**: Two agents at the same location. Agent A has a belief that Agent B does not have. Agent A has a ShareBelief goal targeting Agent B. Both agents have `PerceptionProfile` so they can observe each other (required for `effective_place` beliefs).

**Assertion**: Within a bounded tick count, Agent A plans and executes a `tell` action targeting Agent B. After the tell commits, Agent B's belief store contains the shared belief.

### 5. Diagnostic Enhancement

Live reassessment showed the planner already owns the intended frontier-exhaustion explanation surface through root omission tracing rather than extra counters on `PlanAttemptTrace`.

The current planner-boundary diagnostic carrier is:
- `PlanAttemptTrace.expansion_summaries`
- root expansion `SearchExpansionSummary.root_omissions`
- typed `RootOperatorOmissionReason` values rendered by `decision_trace.rs`

This already explains why a relevant root operator never surfaced, including missing action defs, missing affordance/synthesis paths, unsupported goal/operator pairings, and target-derivation failure.

Note: `CandidateGenerationDiagnostics` (`candidate_generation.rs:159-168`) still separately tracks candidate-generation-stage omissions. Search-stage omission diagnostics remain owned by the root omission trace path rather than by a second per-attempt counter/reason carrier.

## Section H: Causal Hooks (FND-01)

### H1. Information-Path Analysis

- **Trigger**: Agent has a belief it wants to share (alarm, testimony, gossip).
- **Path**: Agent's belief store → ShareBelief goal generated → planner searches for Tell operator → tell action fires → listener's belief store updated.
- The information path is fully local: both agents must be co-located.
- **Belief-layer dependency**: The planner can only find the Tell affordance if the agent has a belief about the listener's location (effective_place). This belief must be acquired through perception — the agent must have observed the listener at their shared location.

### H2. Positive-Feedback Analysis

- No new loops. Telling a belief does not amplify the original event.

### H3. Concrete Dampeners

- N/A.

### H4. Stored State vs. Derived

- **Stored**: Agent beliefs, ShareBelief goal target (listener EntityId), listener's believed effective_place.
- **Derived**: Co-location check (computed from beliefs about effective places), snapshot place index (derived from belief-layer effective_place queries).

## SystemFn Integration

No new SystemFn. The fix is within the existing search and/or snapshot filtering pipeline.

## Component Registration

No new components.

## Cross-System Interactions

- **Perception → Tell** (via belief state): Agent perceives events and co-located entities → forms beliefs including effective_place for nearby agents → ShareBelief goal generated → Tell action shares the belief.
- **Snapshot place indexing → Tell search** (potential root cause): The planning snapshot includes evidence entities unconditionally but only indexes them at a place if the belief view reports their `effective_place`. When the agent lacks this belief for the listener, the search's affordance query finds zero targets despite the listener being in the snapshot's entity set.
