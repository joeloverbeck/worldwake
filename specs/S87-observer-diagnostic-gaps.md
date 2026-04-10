# S87: Observer Diagnostic Gaps

## Summary

Add three missing diagnostic features to the observer binary that were identified by the 2026-04-10 simulation remediation report but were not delivered by S85 (Observer Behavioral Enrichment). These features aggregate data already present in the event log and world state — no simulation logic changes are required.

1. **Belief formation/decay timeline** (TK-6): Per-agent temporal view of when beliefs were formed and lost, answering "Why did this agent not know about the Well?"
2. **StartFailed precondition name** (TK-7): Extract and display the `reason` field from `ActionTraceKind::StartFailed` events, answering "Why did staff_market fail?"
3. **Inventory capacity timeline** (TK-8): Per-agent carry capacity utilization over time in binned format, answering "When did inventory reach saturation?"

**Phase**: 7 (Adjunct — Simulation Remediation Phase 3)
**Status**: Draft
**Crates**: `worldwake-cli` (observer binary only)
**Dependencies**: None

## Design Goals

- Enable the observer to answer three specific diagnostic questions that could not be answered during the 2026-04-10 simulation analysis
- Aggregate from existing event log data and per-tick world state — no new events or simulation changes
- Match the observer's existing output style (markdown tables, 100-tick bins, per-agent sections)

## Non-Goals

- Modifying simulation logic, event emission, or action framework
- Adding new event tags or trace kinds
- Real-time observer streaming (observer runs post-hoc on completed simulation)
- Interactive belief exploration (this is a static dump)

## FOUNDATIONS Alignment

| Principle | How This Spec Satisfies It |
|-----------|----------------------------|
| FND-29 (Debuggability Is a Product Feature) | Directly addresses three identified diagnostic gaps: belief provenance, action failure diagnosis, and inventory timeline |
| FND-14 (World State Is Not Belief State) | Belief timeline shows the agent's belief layer independently from world truth, making the distinction visible |
| FND-8 (Every Action Has Preconditions) | StartFailed reasons make precondition failures transparent and inspectable |

## Section H — Causal Hooks Declaration

### H.1 — Motivating consequence gap

The observer could not answer: (a) when agents formed or lost beliefs about remote resource locations, (b) which precondition caused Merchant Vara's 5 staff_market StartFailed events, or (c) when Forager Lina's inventory reached saturation. These gaps blocked root-cause analysis during the simulation remediation.

### H.2–H.10

N/A — observer-only changes. No world state, actions, entities, quantities, feedback loops, or cross-system interactions are introduced or modified.

## Deliverables

### D1: Belief Formation/Decay Timeline (TK-6)

**File**: `crates/worldwake-cli/src/bin/observer.rs`

**Data source**: Per-tick comparison of each agent's `AgentBeliefStore.known_entities` key set. The observer already accesses world state per tick for needs/location sampling.

**New tracking state** (observer-internal, not world state):

```rust
struct BeliefTimelineEvent {
    tick: u64,
    kind: BeliefTimelineKind,
    entity: EntityId,
    detail: String,  // e.g., "Well at Thornwall Village"
}

enum BeliefTimelineKind {
    Formed,
    Lost,
}
```

**Per-tick logic**: For each living agent, read `world.get_component_agent_belief_store(agent_id)` and extract the `known_entities` key set. Compare against the previous tick's key set (stored in a `BTreeMap<EntityId, BTreeSet<EntityId>>`). New keys produce `Formed` events; missing keys produce `Lost` events. For `Formed` events, derive a human-readable detail string from the entity's `believed_kind` and `last_known_place` fields.

**Output format** (new subsection under Per-Agent Belief Summary):

```markdown
**Belief History (100-tick bins)**
| Bin | Formed | Lost | Net | Notable |
|-----|--------|------|-----|---------|
| 0-99 | 3 | 0 | +3 | +Well, +OrchardRow, +Guard |
| 100-199 | 1 | 1 | 0 | +Tavern, -Well |
| 200-299 | 0 | 2 | -2 | -OrchardRow, -Guard |
```

The "Notable" column shows display names for up to 5 events per bin, truncated with `...` if more.

### D2: StartFailed Precondition Name (TK-7)

**File**: `crates/worldwake-cli/src/bin/observer.rs`

**Data source**: `ActionTraceKind::StartFailed { reason, request, legality }` — the `reason: String` field already contains the formatted precondition failure (e.g., `"PreconditionFailed(\"ActorNotPlaced\")"`, `"ReservationUnavailable(E42)"`). The observer currently increments a counter at the StartFailed match arm but discards all fields.

**New AgentStats field**:

```rust
start_failed_details: Vec<(u64, String, String)>,  // (tick, action_name, reason)
```

**Per-tick logic**: In the existing `ActionTraceKind::StartFailed` match arm, destructure to extract `reason` and push `(current_tick, action_name, reason)` into the new vec.

**Output format** (new subsection under Action Summary):

```markdown
**Start Failure Details**
| Action | Count | Reasons |
|--------|-------|---------|
| staff_market | 5 | ActorNotPlaced x5 |
| harvest | 2 | NoResourceSource x1, QueueFull x1 |
```

Reasons are grouped per action, counted by frequency, and displayed with the top 3 reasons per action. If `legality` trace is present, append it in parentheses.

Also enhance the existing `FailedActionSpiral` anomaly to include the most common reason string, so the anomaly output directly answers "why did this action fail repeatedly?"

### D3: Inventory Capacity Timeline (TK-8)

**File**: `crates/worldwake-cli/src/bin/observer.rs`

**Data source**: Per-tick world state sampling. The observer already samples needs and location each tick.

- `world.get_component_carry_capacity(agent_id)` → `Option<&CarryCapacity>` (max capacity as `LoadUnits`)
- `world.possessions_of(agent_id)` → items carried
- Per-item load via the item's component data

**New AgentStats field**:

```rust
inventory_samples: Vec<InventorySample>,
```

```rust
struct InventorySample {
    current_load: u32,
    capacity: u32,
    item_count: u32,
}
```

**Per-tick logic**: For each living agent, compute current load (sum of possession loads) and capacity, push an `InventorySample`. For agents without `CarryCapacity`, record capacity as 0 and skip.

**Output format** (new subsection under Per-Agent Summary):

```markdown
**Inventory capacity timeline** (carry capacity: 18 LU)
| Bin | Avg Load | Max Load | Items | Utilization |
|-----|----------|----------|-------|-------------|
| 0-99 | 4 LU | 8 LU | 2 | 22% |
| 100-199 | 12 LU | 18 LU | 5 | 67% |
| 200-299 | 18 LU | 18 LU | 7 | 100% (SATURATED) |
```

Bins at >= 95% utilization for 50+ consecutive ticks trigger an `InventorySaturation` anomaly with the tick range.

## SystemFn Integration

None. Observer runs post-hoc on a completed simulation log, not as a SystemFn.

## Component Registration

None. No new components.

## Cross-System Interactions

None. Observer-only changes. The observer reads world state and event traces but does not modify them.

## Profile-Driven Parameters

None. Observer diagnostic formatting is not agent behavior — no per-agent profiles needed.
