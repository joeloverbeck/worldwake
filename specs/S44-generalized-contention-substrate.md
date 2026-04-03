# S44: Generalized Contention Substrate

## Summary

Extend the facility queue's grant/queue/expiry pattern into a reusable contention substrate that any exclusive affordance can use. Today only workstation-class facilities (`FacilityUseQueue`) resolve multi-agent contention through inspectable world state. All other exclusive affordances — item pickup, corpse looting, patient treatment, witness questioning, bounty claiming, storage access — resolve by engine tick order with no world-visible arbitration artifact. FOUNDATIONS II.8 and II.9 demand that contested affordances resolve through explicit world processes: "reservation, queue, grant, lock, contested race, or some other concrete world process."

The fix generalizes the `FacilityUseQueue` pattern into a domain-agnostic `ContentionQueue` that can be attached to any entity where exclusive access matters, with explicit grant/wait/expiry semantics visible to all co-located agents.

## Source

Derived from the ChatGPT architecture review (`brainstorming/improvements-to-ai-architecture.md`, Issues #3 and #9, Improvements C and Feature 9) validated against the actual codebase. Confirmed: `FacilityUseQueue` is the only contention mechanism. `ReservationRecord` in `relations.rs` provides time-windowed reservations but is not used for general affordance contention.

## Phase

Phase 5: Architectural Substrates

## Crates

- `worldwake-core` (new `ContentionQueue`, `ContentionPolicy`, `ContentionGrant`, `ContentionError` types; replace existing `FacilityUseQueue`, `ExclusiveFacilityPolicy`, `FacilityQueueIntents`, `FacilityQueueDispositionProfile`)
- `worldwake-sim` (contention-aware action validation; affordance generation considers queue state)
- `worldwake-systems` (migrate facility queue system to use generalized substrate; add contention to loot, transport, and care actions)
- `worldwake-ai` (planner awareness of contention; affordance filtering based on queue position)

## Dependencies

- S42 (completed, archived) is independent.
- S43 (completed, archived) is independent.
- Benefits from being implemented before S45 (social artifacts may need contention for claim competition).

## FOUNDATIONS Alignment

- **Principle 8, Every Action Has Preconditions, Duration, Cost, and Occupancy**: "Whenever multiple actors can lawfully attempt the same scarce or exclusive affordance, the resolution mechanism must also be explicit: reservation, queue, grant, lock, contested race, or some other concrete world process. Planner intent is not silent control."
- **Principle 9, Scheduling, Simultaneity, and Tie-Breaking Are Part of the World Model**: "For every contested or concurrent case, the world must define a lawful resolution path: ordering rule, initiative rule, arbitration artifact, simultaneous resolution window, or event-queue semantics."
- **Principle 21, Intentions Are Revisable Commitments**: "Intent is not entitlement. A plan reserves nothing unless the world contains an explicit reservation, queue position, contract, assignment, or other claim artifact."
- **Canonical Scenario E, Competing Claimants**: "Multiple agents perceive the same scarce resource... Access is resolved through an explicit race, reservation, queue, grant, lock, or other concrete world mechanism... Any resulting line, grant, blocker, or reservation is inspectable world state."

## Design Goals

1. **Generalize, don't duplicate**: Extract the `FacilityUseQueue` pattern into a reusable `ContentionQueue` that works for any entity. The existing facility queue becomes a consumer of this substrate, not a parallel system.
2. **Queue state is world state**: Grants, queue positions, and wait times are inspectable by co-located agents via perception. Other agents can see "someone is already looting that corpse."
3. **Grant ≠ reservation**: A grant authorizes the current exclusive action. It does not reserve the resource for future use. When the grant expires or the action completes, the next waiter is promoted.
4. **Failure is replanning, not blocking**: An agent that finds a queue occupied should receive a structured signal that feeds back into the AI pipeline (via blocked intent or affordance filtering), not silently fail.
5. **Phased rollout**: Start with the highest-value contention domains (corpse looting, item pickup, healing) and extend incrementally. Don't try to cover every affordance in one spec.
6. **Races and queues through one substrate**: Both queue-based contention (multiple waiters, ordered promotion) and race-based contention (first-to-start wins, no waiting) are expressed through `ContentionPolicy` configuration, not separate mechanisms.

## Deliverables

### 1. `ContentionQueue` component (`worldwake-core`)

Generalize `FacilityUseQueue` into a domain-agnostic contention queue:

```rust
/// Authoritative contention state for an entity requiring exclusive access.
/// Attached to any entity where multiple agents may compete for use.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionQueue {
    pub next_ordinal: u32,
    pub waiting: BTreeMap<u32, ContentionWaiter>,
    pub granted: Option<ContentionGrant>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionWaiter {
    pub actor: EntityId,
    pub intended_action: ActionDefId,
    pub queued_at: Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionGrant {
    pub actor: EntityId,
    pub intended_action: ActionDefId,
    pub granted_at: Tick,
    pub expires_at: Tick,
}

/// Typed queue-state errors for contention operations.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContentionError {
    DuplicateActor(EntityId),
    OrdinalOverflow,
    QueueFull,
}
```

This has the same method surface as `FacilityUseQueue` (enqueue, position_of, has_actor, remove_actor, promote_head, clear_grant, grant_expired) but is registered as a general-purpose component, not limited to workstation entities.

**Component registration constraint**: `|kind| kind == EntityKind::Agent || kind == EntityKind::Facility` — covering Phase 1 targets (corpses and patients are Agent entities with `DeadAt` or wounds; workstations are Facility entities). Extend to additional entity kinds as future phases require.

### 2. `ContentionPolicy` component (`worldwake-core`)

```rust
/// Per-entity policy governing exclusive-access contention.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionPolicy {
    /// How many ticks the current grantee may hold exclusive access.
    pub grant_hold_ticks: NonZeroU32,
    /// Whether the queue auto-promotes the head when the current grant
    /// expires, or requires explicit action to claim.
    pub auto_promote: bool,
    /// Maximum number of waiters. If reached, new entrants are rejected
    /// with `ContentionError::QueueFull`.
    /// `None` means unlimited.
    /// `Some(0)` means race mode: grant-or-reject with no waiting.
    pub max_waiters: Option<u8>,
}

impl Component for ContentionPolicy {}
```

**Component registration constraint**: Same as `ContentionQueue` — `|kind| kind == EntityKind::Agent || kind == EntityKind::Facility`.

### 3. Replace `FacilityUseQueue` with `ContentionQueue` (P28 mandate)

Per Principle 28 (No Backward Compatibility), remove `FacilityUseQueue`, `ExclusiveFacilityPolicy`, `QueuedFacilityUse`, `GrantedFacilityUse`, and `FacilityQueueError` entirely. Use `ContentionQueue` + `ContentionPolicy` directly. All existing facility queue call sites are updated to the generalized types.

Migration of `ExclusiveFacilityPolicy` → `ContentionPolicy`:
- Existing facilities get `ContentionPolicy { grant_hold_ticks: <existing value>, auto_promote: true, max_waiters: None }` — preserving current behavior (unlimited waiters, auto-promotion on grant expiry).

### 4. Migrate `FacilityQueueIntents` → `ContentionIntents`

The existing per-agent `FacilityQueueIntents` component (`crates/worldwake-core/src/intention.rs:35-38`) tracks `BTreeMap<EntityId, QueuedFacilityIntent>` — which facilities the agent is queued for. Generalize to:

```rust
/// Per-agent tracking of entities the agent is contending for.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionIntents {
    /// Map from contended entity to the agent's queued intent.
    pub intents: BTreeMap<EntityId, QueuedContentionIntent>,
}

impl Component for ContentionIntents {}
```

All existing `FacilityQueueIntents` consumers are updated. The component name changes but the pattern is preserved.

### 5. Migrate `FacilityQueueDispositionProfile` → `ContentionDispositionProfile`

The existing per-agent `FacilityQueueDispositionProfile` (`crates/worldwake-core/src/facility_queue.rs:17-21`) provides `queue_patience_ticks: Option<NonZeroU32>` — how long an agent tolerates waiting in a queue. Generalize to:

```rust
/// Per-agent tolerance for waiting in contention queues.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentionDispositionProfile {
    /// Maximum ticks the agent will wait in a queue before abandoning.
    /// `None` means infinite patience.
    pub queue_patience_ticks: Option<NonZeroU32>,
}

impl Component for ContentionDispositionProfile {}
```

Existing agents with `FacilityQueueDispositionProfile` receive `ContentionDispositionProfile` with the same field values.

### 6. Contention domains — Phase 1 targets

Attach `ContentionQueue` + `ContentionPolicy` to these entity classes:

| Entity | Exclusive Affordance | Current Resolution | New Resolution | Policy |
|--------|---------------------|-------------------|----------------|--------|
| Corpse (dead Agent) | Loot | Tick order (first action to start wins) | Queue/grant — one looter at a time, others wait or replan | `auto_promote: true, max_waiters: None` |
| Corpse (dead Agent) | Bury | Tick order | Queue/grant — shared with loot queue | `auto_promote: true, max_waiters: None` |
| Unique items on ground | Pick up | Tick order (invisible) | Race — first to start gets grant; others get structured rejection via inspectable grant state | `auto_promote: false, max_waiters: Some(0)` |
| Patients (wounded agents) | Heal (`ActionDomain::Care`) | Tick order | Queue/grant — one healer per patient at a time | `auto_promote: true, max_waiters: None` |

Phase 2 targets (future specs, not this one):
- Bounty claims (S45 social artifact contention)
- Storage/container access
- Witness time (asking questions)

### 7. Contention-aware action validation (`worldwake-sim`)

Extend `action_validation.rs` to check contention state for queued domains:

- Before starting a loot/bury/heal action on a contention-managed entity, check if the actor holds the grant or can be enqueued.
- If the entity has a `ContentionQueue` and the actor is not the grantee and the queue is not full, enqueue the actor and return a structured "queued, not started" result.
- If the queue is full (including `max_waiters: Some(0)` race mode where no waiting is allowed), return a structured "contention_rejected" result.

### 8. Contention-aware affordance generation (`worldwake-sim`/`worldwake-ai`)

`get_affordances()` should annotate affordances on contention-managed entities with queue state via a new field on the `Affordance` struct:

```rust
pub struct Affordance {
    pub def_id: ActionDefId,
    pub actor: EntityId,
    pub bound_targets: Vec<EntityId>,
    pub payload_override: Option<ActionPayload>,
    pub explanation: Option<String>,
    pub contention_status: ContentionStatus,  // NEW
}

pub enum ContentionStatus {
    /// No contention queue — affordance available as normal.
    Unmanaged,
    /// Actor already holds the grant — proceed.
    Granted,
    /// Actor is in the queue at this position.
    Queued { position: u32 },
    /// Queue exists but actor is not in it — joining required.
    Available,
    /// Queue is full — cannot join.
    Full,
}
```

The planner uses this to avoid planning for contention-full targets (feasibility hint: Unlikely when Full).

### 9. Contention system tick (`worldwake-systems`)

A `contention_system()` that runs once per tick (after action domain systems, before Perception):

1. For every entity with a `ContentionQueue` + `ContentionPolicy`:
   - If the current grant has expired, clear it.
   - If `auto_promote` and no current grant, promote head of queue.
   - Prune waiters whose actor is dead, departed (different place), or no longer intending the queued action.

This reuses the same logic currently in `facility_queue.rs` but generalized. The existing `SystemId::FacilityQueue` slot is renamed or repurposed as `SystemId::Contention`.

### 10. Perception integration

Co-located agents can perceive contention state:

- "Someone is looting that corpse" → observable via `ContentionGrant` on the corpse entity.
- "Three people are waiting for the forge" → observable via `ContentionQueue.waiting` length.

This feeds into `BelievedActivity` or a new `BelievedContentionState` on the agent's entity belief. Agents planning to use a contested entity can factor observed queue length into feasibility and ranking.

Note: Contention beliefs are subject to standard perception staleness. An agent may perceive an empty queue, travel toward the entity, and arrive to find multiple agents waiting. Stale contention beliefs are corrected through the same freshness and re-observation mechanisms as all other beliefs.

### 11. Golden tests

**Scenario A: Corpse loot contention**
- Two agents at same place, corpse appears. Both want to loot.
- First to act gets the grant. Second is queued (or replans to something else).
- After first completes looting, second is promoted and loots remaining items.
- Prove both agents' queue state is visible world state.

**Scenario B: Contention with departure**
- Agent queued for a facility departs (travels away).
- Queue prunes the departed agent.
- Next waiter is promoted.

**Scenario C: Full queue rejection**
- Entity with `max_waiters: Some(1)`. Two agents try to join.
- First is queued; second receives structured rejection.
- Second replans to an alternative.

All scenarios with deterministic replay companions.

## Section H — FOUNDATIONS Analysis

### H.1 Information-path analysis

Contention state (who holds the grant, who is waiting) is authoritative world state on the entity. Co-located agents perceive it through the standard perception system (same mechanism as `BelievedActivity`). No global queries — agents must be at the same place to observe the queue. Remote agents learn about contention only through lawful testimony chains.

Contention beliefs are subject to standard perception staleness: an agent may observe an empty queue, travel to the entity, and find the queue occupied upon arrival. Correction occurs through re-observation at the entity's location, not through remote updates. This is the same freshness model used for all entity beliefs (`observed_tick` in `BelievedEntityState`).

### H.2 Positive-feedback analysis

**No amplifying loops introduced.** Contention queues are passive — they respond to agent actions, they don't generate new ones. A longer queue makes the resource *less* attractive (agents see the wait), which is self-dampening.

Potential indirect loop: popular resource → long queue → agents avoid it → queue shrinks → becomes popular again. This is a healthy oscillation dampened by travel time, agent diversity, and alternative targets.

### H.3 Concrete dampeners

| Concern | Dampener |
|---------|----------|
| Queue grows without bound | `max_waiters` cap (policy-driven, per-entity) |
| Grant holder never releases | `grant_hold_ticks` expiry (time-bounded) |
| Dead/departed agents block queue | Per-tick pruning in `contention_system()` |
| Queue oscillation | Travel time to reach alternatives + agent diversity in patience |

### H.4 Stored state vs. derived read-model list

| Item | Classification |
|------|---------------|
| `ContentionQueue` on entity | **Stored authoritative state** — persists in save/load |
| `ContentionPolicy` on entity | **Stored authoritative state** — persists in save/load |
| `ContentionIntents` on agent | **Stored authoritative state** — persists in save/load |
| `ContentionDispositionProfile` on agent | **Stored authoritative state** — persists in save/load |
| `ContentionStatus` in affordance annotation | **Derived** — computed per-query from queue state |
| `BelievedContentionState` in agent beliefs | **Derived belief** — perceived snapshot, may be stale |

## Cross-System Interactions (Principle 12)

All interaction through state-mediated reads and writes:

- **Action validation** reads `ContentionQueue` + `ContentionPolicy` → determines if actor can start.
- **Contention system** reads queue state + world state (actor alive? co-located?) → writes queue mutations (prune, promote).
- **Perception system** reads `ContentionQueue` → writes to agent `AgentBeliefStore` (if contention state is observable).
- **AI candidate generation** reads affordance annotations → filters or adjusts feasibility.
- **AI failure handling** reads "contention_rejected" structured result → creates appropriate blocked intent.

No system directly invokes another system's privileged behavior.

## Tick System Execution Order

`ContentionSystem` (renamed from `FacilityQueue`) runs after domain action systems (so actions can complete and release grants) and before Perception (so contention state is observable in the same tick):

```
Needs → Production → Trade → Combat → BanditCamp → Contention → Politics → Perception → Patrol
```

The existing `SystemId::FacilityQueue` slot becomes `SystemId::Contention`.

## Migration Path

1. Add `ContentionQueue`, `ContentionPolicy`, `ContentionGrant`, `ContentionWaiter`, `ContentionError` to `worldwake-core`.
2. Implement queue operations (same method surface as `FacilityUseQueue`).
3. Add `ContentionIntents` and `ContentionDispositionProfile` to `worldwake-core`.
4. Replace `FacilityUseQueue` → `ContentionQueue`, `ExclusiveFacilityPolicy` → `ContentionPolicy`, `FacilityQueueIntents` → `ContentionIntents`, `FacilityQueueDispositionProfile` → `ContentionDispositionProfile` (full removal per P28).
5. Rename `SystemId::FacilityQueue` → `SystemId::Contention`.
6. Generalize `contention_system()` in `worldwake-systems`.
7. Attach `ContentionQueue` + `ContentionPolicy` to corpse entities and patient entities.
8. Add contention checks to loot, bury, and heal action validation.
9. Extend affordance generation with `ContentionStatus` field on `Affordance`.
10. Add perception of contention state.
11. Write golden tests.
12. Bump `SAVE_FORMAT_VERSION`.

## Verification

- `cargo test --workspace` passes — existing facility queue behavior unchanged (through generalized substrate).
- Golden test A proves two-agent corpse contention resolves through visible queue state.
- Golden test B proves departed-agent pruning.
- Golden test C proves full-queue rejection and replanning.
- Save/load round-trip preserves `ContentionQueue`, `ContentionPolicy`, `ContentionIntents`, and `ContentionDispositionProfile`.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
