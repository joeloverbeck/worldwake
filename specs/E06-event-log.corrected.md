# E06: Event Log, Mutation Journal & Causal Provenance

## Epic Summary
Implement the append-only event log and the mutation-journaling layer that makes causal completeness enforceable instead of aspirational.

Without a journaled mutation path, “every persistent state change has exactly one cause” is just a slogan. E06 is where that promise becomes mechanically checkable.

## Phase
Phase 1: World Legality

## Crate
`worldwake-sim`

## Dependencies
- E05 (legal world mutations now exist)

## Why this revision exists
The original version described `EventRecord` well enough at the data level, but it left a hole in the enforcement model: if arbitrary code can mutate the world directly, `verify_completeness` cannot reliably prove anything.

Phase 1 needs a world-mutation journal:
- every persistent write goes through it
- the journal captures before / after deltas
- committing the journal emits exactly one event record
- direct out-of-band mutation is impossible in normal code

## Deliverables

### CauseRef
Replace ambiguous `Option<EventId>` semantics with an explicit cause reference.

`CauseRef`:
- `Event(EventId)`
- `SystemTick(Tick)`
- `Bootstrap`
- `ExternalInput(u64)` (or an equivalent stable input id)

Rule:
- every event has exactly one direct cause reference
- root causes are explicit, not encoded as `None`

### EventTag
Provide an ordered tag enum covering at minimum:
- `WorldMutation`
- `Inventory`
- `Transfer`
- `Reservation`
- `ActionStarted`
- `ActionCommitted`
- `ActionAborted`
- `Travel`
- `Trade`
- `Crime`
- `Combat`
- `Political`
- `Control`
- `System`

### VisibilitySpec
Use graph-friendly visibility semantics instead of vague Euclidean notions.

Recommended cases:
- `ParticipantsOnly`
- `SamePlace`
- `AdjacentPlaces { max_hops: u8 }`
- `PublicRecord`
- `Hidden`

Store both:
- intended visibility class
- resolved witness ids at event creation time

### WitnessData
`WitnessData`:
- `direct_witnesses: BTreeSet<EntityId>`
- `potential_witnesses: BTreeSet<EntityId>`

These sets must be stored in deterministic order.

### Component / Relation Delta Types
Do not record anonymous `Any` snapshots.

Provide serializable delta enums:
- `EntityDelta`
  - `Created`
  - `Archived`
- `ComponentDelta`
  - `Set { entity, component_kind, before, after }`
  - `Removed { entity, component_kind, before }`
- `RelationDelta`
  - `Added { relation_kind, ... }`
  - `Removed { relation_kind, ... }`
- `QuantityDelta`
  - `Changed { entity, commodity, before, after }`
- `ReservationDelta`
  - `Created`
  - `Released`

`component_kind` and relation kinds must be typed enums, not strings.

### StateDelta
Wrap the concrete delta families in a single ordered event payload enum:
- `StateDelta::Entity(EntityDelta)`
- `StateDelta::Component(ComponentDelta)`
- `StateDelta::Relation(RelationDelta)`
- `StateDelta::Quantity(QuantityDelta)`
- `StateDelta::Reservation(ReservationDelta)`

Rules:
- event records preserve delta order as committed by the transaction
- delta payloads must be serializable and comparable for deterministic hashing

### EventRecord
`EventRecord`:
- `event_id: EventId`
- `tick: Tick`
- `cause: CauseRef`
- `actor_id: Option<EntityId>`
- `target_ids: Vec<EntityId>`
- `place_id: Option<EntityId>`
- `state_deltas: Vec<StateDelta>`
- `visibility: VisibilitySpec`
- `witness_data: WitnessData`
- `tags: BTreeSet<EventTag>`

Rules:
- `event_id` is monotonic and gapless
- `target_ids` are stored in stable sorted order where ordering is not semantically meaningful
- `state_deltas` preserve mutation order within the event

### EventLog
`EventLog`:
- append-only `Vec<EventRecord>`
- `next_id: EventId`
- ordered secondary indices:
  - by tick
  - by actor
  - by place
  - by tag

Provide:
- `emit(record) -> EventId`
- `get(id) -> Option<&EventRecord>`
- `events_at_tick(tick) -> &[EventId]` or equivalent
- `events_by_actor(actor) -> Vec<EventId>`
- `events_by_place(place) -> Vec<EventId>`

### Mutation Journal / World Transaction
Introduce `WorldTxn` (or equivalent):
- opened with event metadata and direct cause
- exposes only journaled mutation helpers
- records before / after deltas as mutations occur
- commits by:
  1. finalizing deltas
  2. appending one `EventRecord`
  3. returning the new `EventId`

Rules:
- all persistent world writes in normal simulation code go through `WorldTxn`
- no public API may mutate authoritative state without either:
  - emitting an event, or
  - being explicitly marked as transient / cache-only

### Cause Chain Traversal
Provide:
- `trace_cause_chain(event_id) -> Vec<EventId>`
- `get_effects(event_id) -> Vec<EventId>`
- `causal_depth(event_id) -> u32`

Requirements:
- traversal is deterministic
- roots are explicit `CauseRef` cases
- reverse effect lookup is indexed, not a full scan every time

### Completeness Verification
Provide:
- `verify_completeness(world, event_log) -> Result<()>`

Implementation expectation:
- verify that every authoritative mutation path is journaled
- verify that each persisted delta batch belongs to exactly one event
- fail if a test-only bypass writes state without a matching event

This is enforceable because E03/E05 narrowed the mutation surface first.

## Invariants Enforced
- Spec 9.3: every persistent mutation has exactly one direct cause
- Spec 9.5: conserved quantity deltas remain visible and auditable
- Spec 3.10: causal depth can be measured directly

## Tests
- [ ] T07: every persistent state delta is traceable to an event and cause chain
- [ ] Event ids are sequential and gapless
- [ ] Cause-chain traversal reaches an explicit root cause
- [ ] Reverse effect lookup returns downstream events deterministically
- [ ] Event log is append-only
- [ ] Secondary indices round-trip across save/load
- [ ] State deltas correctly capture before / after values
- [ ] Completeness verification catches a deliberate out-of-band mutation in a test harness
- [ ] Event log hash is stable for identical event sequences

## Acceptance Criteria
- no persistent simulation mutation can occur without journaling
- event records are serializable, hashable, and indexed
- cause chains are traversable in both directions
- causal completeness is mechanically testable, not hand-waved

## Spec References
- Section 5.6 (event model)
- Section 9.3 (causal completeness)
- Section 3.10 (measure emergence via causal chain depth)
- Section 9.5 (conservation auditability)
