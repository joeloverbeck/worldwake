# EXCFACACCQUE-002 — `queue_for_facility_use` Action Definition + Handler

**Status**: COMPLETED

**Spec sections**: §3
**Draft ref**: `specs/DRAFT-exclusive-facility-access-queues.md`
**Crates**: `worldwake-systems`, `worldwake-sim`

## Summary

Add a new action that represents an agent physically joining the queue at an exclusive facility. This is the concrete act of taking one's place in line — not an abstract scheduling concept.

## Reassessed Assumptions

- Action registration belongs in `crates/worldwake-systems/src/action_registry.rs`, not `worldwake-sim/src/action_def_registry.rs`.
- The queue action should fit the existing `ActionHandler` + `ActionDefRegistry` pattern already used by trade, transport, combat, and production actions.
- The current handler callbacks do not receive the `ActionDefRegistry`, so authoritative validation of a payload-carried `ActionDefId` cannot live purely inside `start()`. This ticket therefore needs a small action-engine extension: an authoritative payload-validation hook that runs during `start_action()` with access to the real registry and world.
- The facility itself is already the bound action target. Duplicating that entity in `ActionPayload` would create redundant state and another mismatch surface, so the payload should carry only the intended exclusive `ActionDefId`.
- The current `EventTag` taxonomy is intentionally broad and stable. A one-off `QueueJoined` tag would be too specific for the existing indexing model; queue joins should reuse `ActionCommitted` plus a causal world-state tag instead of expanding the enum.
- A dedicated `FacilityQueue` action domain is not currently justified. `ActionDomain` only encodes broad categories with engine-level meaning, and this action is currently part of the exclusive production access path, so it should remain in `Production` unless later planner/runtime behavior needs a new domain.
- This ticket should define the action and authoritative validation only. Automatic affordance expansion for specific intended actions belongs with candidate-generation work in `EXCFACACCQUE-008`.

## Deliverables

### 1. Action definition

Register a `queue_for_facility_use` action with:
- **Duration**: `DurationExpr::Fixed(NonZeroU32::MIN)` (1 tick)
- **Interruptibility**: `FreelyInterruptible`
- **Visibility**: `VisibilitySpec::SamePlace`
- **Body cost**: non-zero but minimal
- **Domain**: `ActionDomain::Production`

### 2. Preconditions

- `Constraint::ActorAlive`
- `Constraint::ActorNotInTransit`
- `Precondition::TargetExists(0)`
- `Precondition::TargetAtActorPlace(0)`
- `Precondition::TargetKind { target_index: 0, kind: EntityKind::Facility }`
- Authoritative start-gate validation: target facility has `ExclusiveFacilityPolicy`
- Authoritative start-gate validation: actor is not already queued or granted at this facility
- Authoritative start-gate validation: payload `intended_action` identifies a registered exclusive facility operation for the same facility/workstation type
- The action target remains the facility entity; the `ActionPayload` carries only the intended exclusive `ActionDefId`

### 3. Start-gate validation + handler implementation

- Extend the action engine with an authoritative payload-validation hook invoked from `start_action()` before handler `start()`
- **start**: assume start-gate payload validation already passed; no additional setup
- **tick**: no-op (1-tick action)
- **commit**: append actor to `FacilityUseQueue.waiting` via `enqueue()`
- **abort**: no side effects (actor was not yet added to queue)

### 4. Registration

Register the action def and handler in a new `facility_queue_actions.rs` module in `worldwake-systems`, following the pattern in `trade_actions.rs` / `transport_actions.rs`. The action is generic: one definition, parameterized by the intended exclusive operation carried in the payload. Wire it into the existing `register_all_actions()` path in `crates/worldwake-systems/src/action_registry.rs`.

### 5. ActionPayload variant

Add a `QueueForFacilityUse { intended_action: ActionDefId }` variant to `ActionPayload` in `worldwake-sim/src/action_payload.rs`, plus a typed accessor. Because automatic affordance enumeration is out of scope for this ticket, add a payload-override validator so manually requested queue actions can still be validated by the existing affordance-matching pipeline.

## Files to Touch

- `crates/worldwake-systems/src/facility_queue_actions.rs` — **new file**, action def + handler
- `crates/worldwake-systems/src/action_registry.rs` — register the new action
- `crates/worldwake-systems/src/lib.rs` — add `pub mod facility_queue_actions;` and re-export the registration helper
- `crates/worldwake-sim/src/action_payload.rs` — add `QueueForFacilityUse` variant
- `crates/worldwake-sim/src/action_handler.rs` — add authoritative payload-validation hook support
- `crates/worldwake-sim/src/start_gate.rs` — invoke authoritative payload validation during action start
- `crates/worldwake-sim/src/affordance_query.rs` — no behavior change required, but the new action must work with the existing payload-override validation path

## Explicitly Not Required

- No new `ActionDomain` variant
- No new `EventTag` variant
- No duplicate `facility` field inside the queue payload
- No automatic queue-action affordance generation yet

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
- Unit test: precondition rejects if target lacks `ExclusiveFacilityPolicy`
- Unit test: precondition rejects if actor is already queued at the same facility
- Unit test: precondition rejects if actor already has a grant at the same facility
- Unit test: precondition rejects if actor is not colocated with facility
- Unit test: payload validation rejects a non-exclusive or mismatched intended `ActionDefId`
- Unit test: payload validation accepts a matching exclusive intended `ActionDefId`
- Unit test: aborting the action does not modify queue state
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- Action duration is exactly 1 tick
- Queue ordering is deterministic (BTreeMap ordinals)
- An actor can hold at most one queue entry per facility after commit
- The action does not start or grant anything — it only appends to the queue
- The committed event remains discoverable through the existing `ActionCommitted` + causal tag flow; no dedicated queue-only event taxonomy is introduced

## Outcome

- Completed: 2026-03-13
- What actually changed:
  - Added `ActionPayload::QueueForFacilityUse { intended_action }` plus typed accessors and serialization coverage.
  - Added a new authoritative payload-validation hook to the action engine and invoked it from `start_action()` before handler start.
  - Implemented `queue_for_facility_use` in `worldwake-systems` as a generic production-domain action that validates the intended exclusive facility operation and enqueues the actor on commit.
  - Registered the action in the systems action registry and integrated the new action family into AI planner-op classification and failure handling so the workspace registry remains coherent.
  - Added direct `WorldTxn` setters for `ExclusiveFacilityPolicy` and `FacilityUseQueue` to support clean component mutation without ad hoc workarounds.
- Deviations from original plan:
  - No new `ActionDomain` or `EventTag` was added; the existing taxonomy was the cleaner architecture.
  - The ticket originally assumed the action could be implemented entirely as a normal handler addition, but the current engine required an explicit authoritative payload-validation seam first.
  - AI integration turned out to be required immediately because the planner registry completeness checks assume every registered action definition belongs to a known planner-op family.
- Verification results:
  - `cargo test -p worldwake-systems facility_queue_actions -- --nocapture`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
