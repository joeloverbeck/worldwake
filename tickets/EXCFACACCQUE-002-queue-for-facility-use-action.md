# EXCFACACCQUE-002 — `queue_for_facility_use` Action Definition + Handler

**Spec sections**: §3
**Crates**: `worldwake-systems`, `worldwake-sim`

## Summary

Add a new action that represents an agent physically joining the queue at an exclusive facility. This is the concrete act of taking one's place in line — not an abstract scheduling concept.

## Deliverables

### 1. Action definition

Register a `queue_for_facility_use` action with:
- **Duration**: `DurationExpr::Fixed(NonZeroU32::MIN)` (1 tick)
- **Interruptibility**: `FreelyInterruptible`
- **Visibility**: `VisibilitySpec::SamePlace`
- **Body cost**: non-zero but minimal
- **Domain**: Production (or a new `FacilityQueue` domain if warranted — follow existing patterns)

### 2. Preconditions

- `Constraint::ActorAlive`
- `Constraint::ActorNotInTransit`
- `Precondition::TargetExists { target_index: 0 }`
- `Precondition::TargetAtActorPlace { target_index: 0 }`
- `Precondition::TargetKind { target_index: 0, kind: EntityKind::Facility }`
- Custom precondition: target facility has `ExclusiveFacilityPolicy` component
- Custom precondition: actor is not already queued or granted at this facility
- The `ActionPayload` must carry the `ActionDefId` of the intended exclusive operation

### 3. Handler implementation

- **start**: validate preconditions, no additional setup
- **tick**: no-op (1-tick action)
- **commit**: append actor to `FacilityUseQueue.waiting` via `enqueue()`, emit a same-place event
- **abort**: no side effects (actor was not yet added to queue)

### 4. Registration

Register the action def and handler in a new `facility_queue_actions.rs` module in `worldwake-systems`, following the pattern in `production_actions.rs`. The action is generic — one definition, parameterized by the `ActionDefId` of the intended operation carried in the payload.

### 5. ActionPayload variant

Add a `QueueForFacilityUse { facility: EntityId, intended_action: ActionDefId }` variant to `ActionPayload` in `worldwake-sim/src/action_payload.rs`.

## Files to Touch

- `crates/worldwake-systems/src/facility_queue_actions.rs` — **new file**, action def + handler
- `crates/worldwake-systems/src/lib.rs` — add `pub mod facility_queue_actions;`
- `crates/worldwake-sim/src/action_payload.rs` — add `QueueForFacilityUse` variant
- `crates/worldwake-sim/src/action_def_registry.rs` — register the new action def (or call from systems registration)
- `crates/worldwake-sim/src/action_domain.rs` — possibly add `FacilityQueue` domain variant
- `crates/worldwake-core/src/event_tag.rs` — add `QueueJoined` event tag

## Out of Scope

- `facility_queue_system` (EXCFACACCQUE-003)
- Grant requirement on harvest/craft (EXCFACACCQUE-004)
- AI planner ops or candidate generation (EXCFACACCQUE-007–008)
- Belief view extensions (EXCFACACCQUE-005)
- Queue pruning or grant promotion logic (EXCFACACCQUE-003)

## Acceptance Criteria

### Tests that must pass
- Unit test: `queue_for_facility_use` action can be constructed with valid preconditions
- Unit test: committing the action appends actor to `FacilityUseQueue.waiting` with correct ordinal
- Unit test: committing emits a same-place event with `QueueJoined` tag (or equivalent)
- Unit test: precondition rejects if target lacks `ExclusiveFacilityPolicy`
- Unit test: precondition rejects if actor is already queued at the same facility
- Unit test: precondition rejects if actor already has a grant at the same facility
- Unit test: precondition rejects if actor is not colocated with facility
- Unit test: aborting the action does not modify queue state
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- Action duration is exactly 1 tick
- Queue ordering is deterministic (BTreeMap ordinals)
- An actor can hold at most one queue entry per facility after commit
- The action does not start or grant anything — it only appends to the queue
- Event log records the queue-join event with proper causal linking
