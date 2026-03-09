# E08: Tick Loop, Deterministic Scheduler, Replay & Save/Load

**Status**: COMPLETED

## Epic Summary
Implement the scheduler, deterministic RNG service, input queue, replay recorder/replayer, canonical state hashing, and versioned save/load.

This epic is the Phase 1 gate. If this design is loose, every later system becomes harder to trust.

## Phase
Phase 1: World Legality (gate epic)

## Crate
`worldwake-sim`

## Dependencies
- E07 (serializable active actions and auditable action events)

## Why this revision exists
The original version had the right top-level list, but it still left too much room for accidental nondeterminism:
- no hard rule for system execution order
- no tie-break for inputs on the same tick
- no canonical hash format
- no insulation from random-call-order drift across subsystems

Phase 1 needs a scheduler that is deterministic by construction, not by wishful thinking.

## Deliverables

### SimulationState
Persist one authoritative simulation root:
- `world: World`
- `event_log: EventLog`
- `scheduler: Scheduler`
- `replay_state: ReplayState`
- `controller_state: ControllerState`
- `rng_state: DeterministicRng`

All fields in `SimulationState` must be serializable.

### Scheduler
`Scheduler` manages the tick loop and active action set.

Required fields:
- `current_tick: Tick`
- `active_actions: BTreeMap<ActionInstanceId, ActionInstance>`
- `system_order: Vec<SystemId>`
- deterministic queues for pending replan records and other scheduler-owned state

`SystemId` is a stable identifier for registered simulation systems. System order is defined by a fixed manifest, not dynamic registration order.

Rules:
- `system_order` is fixed and explicit
- active actions are progressed in sorted `ActionInstanceId` order
- no system iteration may depend on hash-map order

### Per-Tick Flow
Required deterministic tick sequence:

1. Drain input events scheduled for `current_tick` in `(tick, sequence_no)` order
2. Apply control-binding changes and accepted action requests
3. Progress active actions in sorted id order
4. Validate and commit completed actions in sorted id order
5. Run registered systems in fixed `system_order`
6. Emit end-of-tick marker / checkpoint data
7. Increment `current_tick`

No phase may mutate authoritative state outside journaled event-producing paths.

### Deterministic RNG Service
Wrap `ChaCha8Rng` (or another pinned deterministic algorithm) behind a scheduler-owned API.

Requirements:
- all randomness in authoritative simulation comes from this service
- no direct `thread_rng`, OS randomness, or wall-clock seeding
- full RNG internal state is serializable
- recommended: support deterministic named substreams derived from `(master_seed, tick, subsystem_id, sequence_no)` to reduce accidental coupling between unrelated systems

### Input Queue
`InputEvent`:
- `scheduled_tick: Tick`
- `sequence_no: u64`
- `kind: InputKind`

`InputKind` minimum cases:
- `RequestAction { actor, def_id, targets }`
- `CancelAction { actor, action_instance_id }`
- `SwitchControl { from: Option<EntityId>, to: Option<EntityId> }`

Rules:
- input ordering is deterministic
- multiple inputs on the same tick are resolved by `sequence_no`
- inputs request actions; they do not directly mutate world state

### Replay Recording
Record:
- initial canonical state hash
- master seed
- ordered input log
- per-tick event-log hash checkpoints
- per-tick state-hash checkpoints (at least configurable intervals)

### Replay Execution
Replay flow:
1. load the initial state snapshot
2. restore the master seed / RNG state
3. inject recorded inputs at recorded ticks
4. compare event-log and state hashes at configured checkpoints

Pass condition:
- same initial state + same seed + same input log => identical checkpoint hashes and identical final state hash

### Canonical Hashing
Provide canonical hash helpers for:
- world state
- event log
- full simulation state

Rules:
- hashes are derived from canonical serialized bytes, not `Debug` output
- canonical serialization order must be stable
- hashing must ignore transient caches if any are introduced later

### Save / Load
Provide versioned binary persistence:
- `save(path) -> Result<()>`
- `load(path) -> Result<SimulationState>`

Save file contents must include:
- full authoritative world state
- event log
- scheduler state
- active actions
- pending reservations
- control binding / controller state
- RNG internal state
- replay input queue and checkpoint metadata as needed

Rules:
- save/load is a semantic round-trip, not just “it deserializes”
- format version is stored explicitly
- old or mismatched versions fail cleanly

### State Equality / Test Utilities
Provide helpers for:
- exact final-state equality checks
- checkpoint hash comparisons
- uninterrupted-run vs save/load-continued-run comparisons

## Invariants Enforced
- Spec 9.1: simulation authority remains in the scheduler + world mutation path
- Spec 9.2: same seed and same inputs produce the same results
- Spec 9.19: save/load preserves complete authoritative state
- Spec 9.21: changing or losing the controlled agent does not halt simulation law

## Tests
- [ ] T08: replay determinism passes
- [ ] T09: save/load round-trip matches uninterrupted execution
- [ ] Scheduler tick order is deterministic
- [ ] Active actions complete on the correct ticks
- [ ] Same seed yields the same random outcomes
- [ ] Input events on the same tick are resolved by sequence number
- [ ] State hash is stable for identical states
- [ ] Event-log hash is stable for identical event sequences
- [ ] Save files include world, events, scheduler, active actions, reservations, control state, and RNG state
- [ ] Replay checkpoint hashes match at every configured checkpoint

## Phase 1 Gate
Before proceeding to Phase 2, all of the following must be green:
- [ ] T01 unique location
- [ ] T02 conservation
- [ ] T03 no negative inventory
- [ ] T04 reservation lock
- [ ] T05 precondition gate
- [ ] T06 commit validation
- [ ] T07 event provenance
- [ ] T08 replay determinism
- [ ] T09 save/load round-trip
- [ ] T13 containment acyclic
- [ ] randomized invariant tests across E01-E08

## Acceptance Criteria
- the tick loop drives all authoritative simulation
- replay matches canonical checkpoint hashes
- save/load preserves complete authoritative state
- there are no unordered or hidden nondeterministic code paths in the gate stack

## Spec References
- Section 5.1 (fixed tick simulation)
- Section 9.1 (simulation authority)
- Section 9.2 (determinism)
- Section 9.19 (save/load integrity)
- Section 9.21 (controlled-agent mortality / continuity)
- Section 12 (Phase 1 gate)

## Outcome

Completion date: 2026-03-09

What actually changed:

1. Implemented the deterministic scheduler tick loop, canonical `SimulationState` root, deterministic RNG service, replay recording/execution, canonical state hashing, and versioned save/load in `worldwake-sim`.
2. Proved T08 and T09 through focused module tests in `replay_execution.rs` and `save_load.rs`.
3. Kept Phase 1 gate coverage distributed across the owning modules and crates instead of introducing a separate umbrella gate harness.

Differences from the original plan:

1. The state-equality and continuation proofs were satisfied by `SimulationState` equality plus focused replay/save-load tests, not by a new public `test_utils` facade.
2. Phase 1 gate verification remains a workspace-level aggregation of authoritative module tests rather than a single `phase1_gate.rs` integration test file.

Verification results:

1. `cargo test -p worldwake-sim` passed.
2. `cargo test -p worldwake-core --test relation_invariants` passed.
3. `cargo test -p worldwake-core verification` passed.
4. `cargo test -p worldwake-core conservation` passed.
5. `cargo clippy --workspace --all-targets -- -D warnings` passed.
6. `cargo test --workspace` passed.
