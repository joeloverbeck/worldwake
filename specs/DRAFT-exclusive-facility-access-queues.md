**Status**: DRAFT

# Exclusive Facility Access Queues

## Summary
Design a clean replacement for scheduler-order monopolization at exclusive facilities. The current engine can prevent crashes under same-tick contention, but it still allows one agent to monopolize a finite harvest source because access order emerges from request timing and repeated replanning, not from concrete world state.

This spec fixes that by making contested facility access explicit, local, and observable:
- agents join a concrete queue for exclusive facility use
- the facility grants one concrete turn at a time
- each grant covers one real operation, not an invisible timeslice
- after completing one turn, an agent must re-enter at the back if it still wants more

The design is generic across exclusive facilities, not harvest-specific. Harvest is the motivating case, but the same architecture should later support craft stations, treatment benches, office desks, docks, gates, or any other single-user bottleneck.

This spec is forward-looking. It is not part of the active E14-E22 sequence and should not be scheduled ahead of the current phase gates in [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md).

## Why This Exists
Current exclusive-facility behavior is legal but not architecturally complete:
- workstation reservation prevents simultaneous mutation
- best-effort autonomous requests prevent same-tick contention from crashing the tick
- but no concrete world state models turn-taking among multiple colocated claimants

That leaves an undesirable outcome:
- one agent can repeatedly monopolize a finite source
- waiting order is implicit in scheduler timing
- losing agents learn only through start failures and replans, not through explicit local queue state

This violates the spirit of several foundations even if it does not immediately violate hard invariants:
- Principle 3: access order is not grounded in concrete state
- Principle 7: agents should respond to locally observable bottlenecks, not hidden scheduler order
- Principle 8: contested access should consume time and occupancy explicitly
- Principle 27: when asked "why did this agent not get the apples?", the answer should come from queue and grant state, not from incidental request ordering

## Phase
Future production/AI hardening, post-E22 scheduling only.

## Crates
- `worldwake-core`
- `worldwake-sim`
- `worldwake-systems`
- `worldwake-ai`

## Dependencies
- E10 production, transport, and route occupancy
- E13 grounded decision architecture
- E14 perception/beliefs for future richer non-local queue knowledge

E14 is not required for the first implementation pass because the queue is local concrete state at the same place, but all queue reasoning must remain belief-safe.

## Design Goals
1. Replace implicit scheduler-order access with explicit world-state turn-taking.
2. Keep contention local and observable at the facility.
3. Generalize across all exclusive facilities, not just orchard harvest.
4. Preserve deterministic one-at-a-time legality through existing reservations.
5. Make each turn correspond to one concrete operation, not an abstract fairness score.
6. Do not add compatibility shims or dual access paths.

## Non-Goals
- Do not add global fairness scoring, starvation scores, or scheduler-level round-robin logic.
- Do not guarantee egalitarian distribution of final goods across agents.
- Do not add omniscient queue discovery across places.
- Do not replace reservations; queueing complements reservations by deciding who is allowed to attempt the next exclusive action.
- Do not solve all multi-agent market allocation. This spec is about exclusive facility usage.

## Deliverables

### 1. `ExclusiveFacilityPolicy` Component
Concrete authoritative policy on facilities that require queued turn-taking.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ExclusiveFacilityPolicy {
    grant_hold_ticks: NonZeroU32,
}
```

This component belongs on `EntityKind::Facility`.

`grant_hold_ticks` is how long a granted turn remains reserved for the granted actor before forfeiture if the actor does not begin the real operation. This is facility-authored data, not a global magic constant.

### 2. `FacilityUseQueue` Component
Concrete queue state stored on the facility itself. There is **one queue per facility** — not per operation type. At most one grant is active at any time regardless of what operation the grant authorizes.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct FacilityUseQueue {
    next_ordinal: u32,
    waiting: BTreeMap<u32, QueuedFacilityUse>,
    granted: Option<GrantedFacilityUse>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct QueuedFacilityUse {
    actor: EntityId,
    intended_action: ActionDefId,
    queued_at: Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct GrantedFacilityUse {
    actor: EntityId,
    intended_action: ActionDefId,
    granted_at: Tick,
    expires_at: Tick,
}
```

Queue entries and grants reference the `ActionDefId` that the agent intends to perform (e.g. `harvest_apples`, `craft_flour`). This avoids a parallel taxonomy — every future exclusive action type is automatically supported without extending an enum.

This is stored authoritative state, not a derived cache. It answers:
- who is waiting
- in what order
- for which concrete facility use (by `ActionDefId`)
- who currently owns the next turn

**Key invariant**: A facility has at most one active grant at any time. An actor cannot hold more than one active grant for the same facility.

### 3. Queue Participation Is a Real Action
Add a generic facility-domain or production-domain action:
- `queue_for_facility_use`

Action meaning:
- "I physically take my place in line for this facility."

Properties:
- duration: `DurationExpr::Fixed(NonZeroU32::MIN)`
- interruptibility: `FreelyInterruptible`
- visibility: `VisibilitySpec::SamePlace`
- body cost: non-zero but minimal

Preconditions:
- actor alive
- actor not in transit
- target facility exists and is colocated
- target facility has `ExclusiveFacilityPolicy`
- actor is not already queued or granted for the same facility
- requested action def is a registered exclusive action for that facility type

Commit behavior:
- append the actor to the queue tail with the next ordinal

This is not "waiting" in the abstract. It is the concrete act of joining the local line.

### 4. Queue Membership Lifecycle
Queue membership is **persistent world state** on the facility component. It survives after the `queue_for_facility_use` action completes. The agent is free to do other things while waiting in the queue (eat, drink, rest, etc.).

The `facility_queue_system` prunes members who:
- **Die** — entity is dead or deallocated
- **Leave the place** — actor is no longer colocated with the facility
- **No longer exist** — entity was purged from the allocator

Voluntary withdrawal happens through replanning: if the agent decides the wait is not worth it, it simply does not act on the grant (which expires), and gets pruned on place departure. A per-agent `queue_patience_ticks: NonZeroU32` profile field can govern how long an agent is willing to wait before replanning away. This parameter lives in concrete profile state, not as a hidden constant.

### 5. Real Operations Require a Matching Grant
Exclusive facility actions such as `harvest:*` and `craft:*` must require a matching `GrantedFacilityUse`.

For example, harvest preconditions become conceptually:
- actor knows recipe
- target facility colocated and correct workstation
- source has enough stock
- facility grant exists for `(actor, matching ActionDefId)`

Starting the real operation:
- consumes the grant
- acquires the existing reservation lock
- proceeds exactly as today

The grant decides who may try next. The reservation still prevents illegal overlap once the attempt begins.

### 6. One Grant, One Concrete Operation
Each grant authorizes exactly one exclusive action instance.

Consequences:
- one harvest batch per turn
- one craft job per turn
- after commit or abort, the actor must queue again if it still wants another turn

This is the core anti-monopolization rule, and it is concrete rather than statistical.

### 7. `facility_queue_system`
Add a dedicated system function that maintains facility queues.

**Execution order**: `facility_queue_system` runs **after** action commit/abort processing and **before** AI planning input generation. This ensures grants are promoted after operations complete and before agents make new decisions.

Responsibilities:
- prune invalid queued entries if the actor died, left the place, or no longer exists (see §4)
- expire grants whose `expires_at` has passed without the actor starting the operation
- if no grant is active and no matching exclusive action is running, promote the queue head to `granted`

**Pruning policy — permanent impossibility only**: The queue head is pruned (with a queue-failure event) only for **permanent impossibility**:
- facility destroyed
- workstation tag removed from facility
- actor permanently incapable of performing the intended action

For **temporary conditions** like depleted stock, the queue system simply **does not promote** — the head waits in place. Stock regenerates via `resource_regeneration_system`; eagerly pruning for depleted stock would cause agents to repeatedly join, get ejected, and rejoin in a wasteful churn cycle. The agent may voluntarily leave the queue through replanning if waiting exceeds its `queue_patience_ticks` profile threshold.

This system does not start actions. It only advances queue state.

### 8. Queue-Failure Events
When a queued request is dropped at the head because the operation **permanently** cannot happen, emit a concrete event visible at the same place.

Examples:
- facility lost required workstation marker
- facility destroyed
- actor lost capability to perform the intended action

This gives nearby agents a local information carrier explaining why the line stopped moving.

Note: temporary stock depletion does **not** trigger a failure event — the queue simply stalls until stock recovers or the agent replans away.

### 9. Belief Queries
Extend belief-safe reads with local queue visibility:

```rust
/// Returns the actor's position in the facility's queue (0-indexed from head),
/// or None if the actor is not queued at this facility.
fn facility_queue_position(
    &self,
    facility: EntityId,
    actor: EntityId,
) -> Option<u32>;

/// Returns the current active grant at this facility, if any.
fn facility_grant(
    &self,
    facility: EntityId,
) -> Option<&GrantedFacilityUse>;
```

Since there is one queue per facility (not per operation type), the position query does not need an action discriminator. The grant query returns the full `GrantedFacilityUse` which includes the `intended_action: ActionDefId`.

Only colocated or otherwise belief-valid knowledge may surface in agent-facing planning.

### 10. AI Planning Semantics
Exclusive operations gain a real barrier:
- `QueueForFacilityUse`

Planning flow for a hungry agent at a contested orchard becomes:
- need pressure
- `AcquireCommodity { Apple }`
- `queue_for_facility_use(OrchardRow, harvest_apples)`
- **suspended on blocking barrier** (queue position > 0 and no grant)
- grant received → replan with grant available
- `harvest:harvest_apples`
- pick up
- consume

**Handling the "wait for grant" gap**: After the `QueueForFacilityUse` step, the planner treats the plan as **suspended on a blocking barrier** (queue position > 0 and no grant). The agent does not idle — the decision runtime checks each tick whether a grant exists:
- **No grant yet**: The agent is free to pursue other interruptible goals (eat, drink, rest) or simply wait. The queue membership persists regardless.
- **Grant received**: The agent's decision runtime detects the grant, marks the current plan as dirty, and triggers a replan. With the grant now available, the harvest/craft action becomes directly executable and the planner produces a plan that begins with it.

This is analogous to how travel barriers work: the planner recognizes the agent needs to be somewhere else and inserts a travel step. Here, the planner recognizes the agent needs a grant and inserts a queue step, then suspends until the grant materializes.

If another agent holds the grant or sits ahead in line, that is now explicit local world state, not invisible competition.

### 11. Candidate Generation
For exclusive facility uses:
- if actor already has a matching grant, emit the direct exclusive action
- else if actor is already queued, do not emit duplicate queue actions
- else if the facility is locally visible and the use is legal in principle, emit `queue_for_facility_use` with the appropriate `ActionDefId`

This replaces the current pattern where multiple agents repeatedly emit the same exclusive action request from the same snapshot.

### 12. Ranking
Ranking should treat:
- already granted next turn as higher-motive than merely being able to join the queue
- joining a local queue as a valid progress step, not a failure state

No abstract fairness bonus is introduced.

If a later implementation needs abandonment behavior, use per-agent profile data such as:
- `queue_patience_ticks: NonZeroU32`

That parameter must live in concrete profile state, not as a hidden constant.

### 13. No Compatibility Layer
When this spec is implemented:
- exclusive facilities stop using direct multi-agent start contention as the primary arbitration mechanism
- planner/candidate generation must route autonomous exclusive use through the queue/grant path
- do not preserve dual semantics where some exclusive uses bypass the queue while others use it

The existing best-effort autonomous request handling may remain as a generic engine safety net, but it must no longer be the primary contention-resolution architecture for exclusive facilities.

## Component Registration
Register in `component_schema.rs`:
- `ExclusiveFacilityPolicy` on `EntityKind::Facility`
- `FacilityUseQueue` on `EntityKind::Facility`

Register serialization for:
- `QueuedFacilityUse`
- `GrantedFacilityUse`

No global queue manager singleton is permitted.

## SystemFn Integration

### `worldwake-core`
- add queue/grant/policy types
- add facility queue components
- `QueuedFacilityUse` and `GrantedFacilityUse` reference `ActionDefId` (from `worldwake-sim`); if this creates a dependency issue, the `ActionDefId` type may need to live in `worldwake-core` or the queue types may live in `worldwake-sim`

### `worldwake-sim`
- extend belief views, planning snapshot, and planning state with local queue/grant queries using `ActionDefId`
- preserve deterministic queue ordering through `BTreeMap`
- register `facility_queue_system` in the system manifest with execution order: after action commit/abort, before AI input generation

### `worldwake-systems`
- add `queue_for_facility_use` action definition referencing `ActionDefId` for the intended operation
- add `facility_queue_system` with permanent-impossibility-only pruning
- update harvest and craft action definitions to require matching grants (matched by `ActionDefId`)
- emit local queue-failure events only for permanent impossibility (facility destroyed, workstation removed, actor incapable)

### `worldwake-ai`
- add `QueueForFacilityUse` planner op and semantics with blocking-barrier support
- update candidate generation to emit `queue_for_facility_use` with the appropriate `ActionDefId` when an exclusive facility is locally available
- update ranking to treat queue join as the normal access path for exclusive facilities
- update decision runtime to detect grant arrival and trigger replan
- update failure handling so queue head invalidation becomes explicit blocker state rather than repeated reservation collisions

## Cross-System Interactions (Principle 12)
- needs and enterprise goals drive demand for exclusive facility use
- queue state influences planning through local facility components
- production actions consume grants and resource stock
- failure events update local belief and blocked-intent memory
- transport remains responsible for moving produced lots after the exclusive operation completes

All interactions remain state-mediated through:
- `FacilityUseQueue`
- `GrantedFacilityUse`
- `ExclusiveFacilityPolicy`
- facility-local events
- normal reservations
- normal item/resource state

No system directly tells another system which agent should go next.

## FND-01 Section H

### Information-Path Analysis
- Queue membership is local state on the facility.
- Grants are local state on the facility.
- Agents only know the line they can physically observe or have previously learned through explicit reports in a later beliefs implementation.
- A nearby agent can answer "who is next" by observing queue state at the place, not by consulting a scheduler.
- Head-of-line failure is communicated through a same-place event, not a silent planner miss.

### Positive-Feedback Analysis
- successful use → goods acquired → agent can requeue for more
- success at a productive facility can increase pressure to stay near the facility and exploit it

### Concrete Dampeners
The following are **designed dampeners** — mechanisms this spec introduces to limit positive feedback:

1. **Primary — one-grant-one-operation + mandatory tail re-entry**: Each grant authorizes exactly one exclusive action instance. After completion, the actor must rejoin at the back of the queue. This prevents any single agent from monopolizing a facility through rapid consecutive operations.
2. **Secondary — grant expiry timer**: `grant_hold_ticks` on `ExclusiveFacilityPolicy` causes unused grants to forfeit after a facility-specific duration. This prevents dead or distracted agents from stalling the queue indefinitely.
3. **Tertiary — place-departure pruning**: Agents who leave the facility's place are automatically removed from the queue. This prevents phantom queue members from occupying slots while physically elsewhere.

Supporting natural constraints (not spec contributions, but worth noting):
- Source depletion stalls the queue naturally (stock must regenerate before further grants can execute).
- Travel time and occupancy still apply before and after queue use.

### Stored vs Derived State
Stored authoritative state:
- `ExclusiveFacilityPolicy`
- `FacilityUseQueue`
- `QueuedFacilityUse`
- `GrantedFacilityUse`
- facility-local failure events
- normal reservation, resource source, and item state

Derived transient read-model:
- queue position number for a given actor
- whether an actor may currently start an exclusive action (has matching grant)
- whether a facility is locally blocked by another agent's turn
- whether a queued request remains permanently impossible

## Invariants
- exclusive turn-taking is represented in world state, never inferred from scheduler order
- one grant maps to one concrete exclusive action instance
- a facility has at most one active grant at any time
- an actor cannot hold more than one queue entry for the same facility
- harvest/craft stock remains exactly conserved as before
- queue order is deterministic and stable for a fixed seed and event history
- all queue visibility remains local
- no backwards compatibility path preserves direct autonomous bypass of exclusive queues
- queue pruning occurs only for permanent impossibility, never for temporary stock depletion

## Tests
- [ ] four hungry agents at one orchard queue locally instead of colliding on direct harvest starts
- [ ] the first two harvest grants in the finite `Quantity(4)` orchard scenario go to two distinct queued agents before any one agent may receive a second grant
- [ ] starting a harvest without a matching grant is impossible even if the facility is otherwise valid
- [ ] actor death or departure removes queue membership automatically
- [ ] expired grants advance the queue to the next eligible actor
- [ ] queue head with temporarily depleted stock stalls the queue (does not prune) until stock regenerates or agent replans away
- [ ] queue head with permanently impossible operation (workstation removed) is pruned with a local failure event
- [ ] craft stations use the same queue/grant path without a separate fairness subsystem
- [ ] planning snapshot and replay remain deterministic under queue advancement
- [ ] best-effort autonomous input handling remains a safety net but is no longer exercised in the normal contested-harvest path
- [ ] agent with grant that replans to a different goal lets the grant expire and queue advances
- [ ] agent in queue can perform non-exclusive actions (eat, drink) without losing queue position

## Acceptance Criteria
- exclusive facility contention is resolved through queue/grant world state, not scheduler order
- harvest and craft both use the same exclusive-access architecture
- monopolization through immediate repeat starts at the same exclusive facility is eliminated
- losing agents can locally explain why they are waiting or why their turn failed
- no abstract fairness score, starvation score, or round-robin scheduler is introduced
- no compatibility layer preserves the old direct autonomous contention path as the normal mechanism
- all authoritative iteration remains deterministic
- queue entries and grants reference `ActionDefId`, not a parallel enum taxonomy
- temporary stock depletion stalls the queue; only permanent impossibility prunes

## References
- [E10-production-transport.md](/home/joeloverbeck/projects/worldwake/archive/specs/E10-production-transport.md)
- [E13-decision-architecture.md](/home/joeloverbeck/projects/worldwake/archive/specs/E13-decision-architecture.md)
- [DRAFT-merchant-selling-market-presence.md](/home/joeloverbeck/projects/worldwake/specs/DRAFT-merchant-selling-market-presence.md)
- [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md)
- [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md)
