# E07: Deterministic Action Model & Execution Lifecycle

## Epic Summary
Implement the action definition model, affordance query, start/commit legality checks, reservation acquisition, interrupt/abort handling, and serializable active-action state.

This epic must set up the exact execution pipeline shared by human and AI control without smuggling in player exceptions or unserializable closures.

## Phase
Phase 1: World Legality

## Crate
`worldwake-sim`

## Dependencies
- E06 (actions emit journaled events and use reservations)

## Why this revision exists
The original version had the right high-level shape, but two Phase 1 risks were still exposed:
- `ActionInstance` held a reference to `ActionDef`, which is awkward for save/load and replay
- affordance evaluation depended on a “beliefs” object before the belief system exists

Phase 1 needs:
- stable ids instead of references in persisted action state
- a world-view abstraction that can be backed by real beliefs later

## Deliverables

### ActionDefId
Introduce a stable action-definition identifier:
- `ActionDefId(u32)`

Action definitions are registered in a deterministic static order.

### Supporting Action IDs and State Types
Introduce stable identifiers and serializable action state.

- `ActionHandlerId(u32)`
- `ActionInstanceId(u64)`
- `ActionStatus = Pending | Active | Committed | Aborted | Interrupted`
- `ActionState` as a serializable enum or struct for handler-local persistent state

Rules:
- ids are monotonic / deterministic in the domains where they are assigned
- `ActionState` may store only serializable data
- handler-local transient data that cannot survive save/load is forbidden

### ActionDef
Every action definition must include all ten semantics from spec 3.7.

`ActionDef` fields:
- `id: ActionDefId`
- `name: String`
- `actor_constraints: Vec<Constraint>`
- `targets: Vec<TargetSpec>`
- `preconditions: Vec<Precondition>`
- `reservation_requirements: Vec<ReservationReq>`
- `duration: DurationExpr`
- `interruptibility: Interruptibility`
- `commit_conditions: Vec<Precondition>`
- `visibility: VisibilitySpec`
- `causal_event_tags: BTreeSet<EventTag>`
- `handler: ActionHandlerId`

Rules:
- there are no optional shortcut fields that omit one of the ten semantics
- `duration` resolves to integer ticks only
- action definitions themselves are deterministic static data

### Action Handler Registry
Provide a deterministic registry mapping `ActionHandlerId` to executable logic.

Rules:
- active actions store only `def_id` plus serializable state, never function pointers, direct handler ids, or references
- handlers mutate world state only through `WorldTxn`
- registry iteration order is stable

### Supporting Semantic Types
Define explicit serializable types for the action schema:
- `Constraint`
- `TargetSpec`
- `Precondition`
- `ReservationReq`
- `DurationExpr`
- `Interruptibility`

Rules:
- these types may contain ids, enums, and deterministic parameters only
- they may not embed closures or trait objects
- `DurationExpr` resolves to integer ticks only

### KnowledgeView / ActionContext
Introduce an abstraction used by affordance evaluation:
- `KnowledgeView` or `ActionContext`

Requirements:
- current Phase 1 implementation may back it with authoritative state
- later it must be swappable for a belief-backed view without changing the action API
- affordance queries must never depend directly on omniscient world access by design

### Affordance Result
Do not return raw `ActionDef` values.

Provide:
- `Affordance`
  - `def_id: ActionDefId`
  - `actor: EntityId`
  - `bound_targets: Vec<EntityId>`
  - `explanation: Option<String>` (optional but useful for UI/debug)
  - deterministic sort key

`get_affordances(view, actor) -> Vec<Affordance>`

Rules:
- returned affordances are sorted deterministically by:
  1. `ActionDefId`
  2. bound target ids
- human and AI code use the same affordance query

### ActionInstance
`ActionInstance`:
- `instance_id: ActionInstanceId`
- `def_id: ActionDefId`
- `actor: EntityId`
- `targets: Vec<EntityId>`
- `start_tick: Tick`
- `remaining_ticks: u32`
- `status: ActionStatus`
- `reservation_ids: Vec<ReservationId>`
- `local_state: Option<ActionState>`

Rules:
- the full struct is serializable
- executable dispatch is derived from `def_id -> ActionDef -> handler`, keeping behavior selection under a single source of truth
- no borrowed references
- no closure-captured transient state

### Start Gate
Starting an action must:
1. validate actor constraints
2. validate start preconditions against `KnowledgeView`
3. acquire reservations atomically
4. create an `ActionInstance`
5. emit an action-start event

Failure returns a precise error and emits nothing persistent unless a failure event is explicitly desired.

### Tick / Progress
Provide:
- `tick_action(instance_id, world, scheduler_ctx) -> Result<ActionProgress>`

Rules:
- remaining ticks decrement deterministically
- completed actions move to commit validation
- interrupted / aborted actions stop consuming time immediately

### Commit Validation
When `remaining_ticks == 0`:
- re-evaluate commit conditions on authoritative state
- if true: commit through `WorldTxn`
- if false:
  - abort cleanly
  - release reservations
  - emit action-aborted event
  - emit replan signal / record

### Interrupt / Abort
Provide:
- `interrupt(instance_id, reason) -> Result<()>`
- `abort(instance_id, reason) -> Result<()>`

Rules:
- interrupt obeys `Interruptibility`
- abort always succeeds
- both release reservations
- both emit auditable events

### Replan Signal
Provide a serializable failure / replan record:
- `ReplanNeeded { agent, failed_action, reason }`

It may be emitted as:
- a dedicated event payload, or
- a scheduler-side queue tied to the action-abort event

Either way, it must survive save/load if pending.

## Invariants Enforced
- Spec 9.9: no action commits unless commit conditions are true at commit time
- Spec 9.12: action legality does not branch on player status
- Spec 6.4: human control uses the same affordance and execution pipeline as AI control

## Tests
- [ ] T05: affordance query never returns an action with false start preconditions in the acting view
- [ ] T06: action aborts cleanly if commit conditions fail
- [ ] Interrupt respects `Interruptibility`
- [ ] Abort always succeeds and releases reservations
- [ ] Replan record is emitted on failure
- [ ] Affordance results are identical regardless of `ControlSource`
- [ ] Active `ActionInstance` state survives serialization round-trip
- [ ] All ten action semantics are required by the type model
- [ ] Affordance ordering is deterministic
- [ ] Action effects mutate world state only through `WorldTxn`

## Acceptance Criteria
- full action lifecycle: define -> query -> start -> tick -> commit / abort
- persisted action state is serializable and replay-safe
- preconditions are checked at both start and commit boundaries
- the exact same pipeline serves human and AI control

## Spec References
- Section 3.7 (explicit action semantics)
- Section 6.4 (human control uses same action query)
- Section 9.9 (legal action execution)
- Section 9.12 (player symmetry)
