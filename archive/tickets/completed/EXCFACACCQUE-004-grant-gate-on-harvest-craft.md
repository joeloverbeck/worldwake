# EXCFACACCQUE-004 — Grant Requirement Gate on Harvest and Craft Starts

**Status**: Completed
**Spec sections**: `specs/DRAFT-exclusive-facility-access-queues.md` §5, §6, §13
**Crates**: `worldwake-sim`, `worldwake-systems`
**Depends on**: implemented queue/grant groundwork from `EXCFACACCQUE-001` through `EXCFACACCQUE-003`

## Reassessed Assumptions

This ticket was reassessed against the current codebase before implementation.

### What was incorrect in the original ticket

- Queue/grant infrastructure is already present in code, not just in prior tickets:
  - `crates/worldwake-core/src/facility_queue.rs`
  - `crates/worldwake-systems/src/facility_queue_actions.rs`
  - `crates/worldwake-systems/src/facility_queue.rs`
- The queue system file is `crates/worldwake-systems/src/facility_queue.rs`, not `facility_queue_system.rs`.
- `EXCFACACCQUE-003` does **not** expose a dedicated shared readiness helper yet. It currently promotes queue heads by directly evaluating action constraints and preconditions.
- Adding a raw `Precondition::FacilityGrantExists` to harvest/craft action defs would deadlock queue promotion:
  - promotion currently checks the same declarative readiness rules to decide whether the head may receive a grant
  - if grant existence becomes one of those rules, the head can never become grantable
- Existing harvest/craft tests do **not** already set up facility grants. Start-path tests will need targeted updates.

### Current architectural reality

- Declarative `actor_constraints` and `preconditions` drive:
  - affordance generation
  - authoritative start validation in `start_action`
  - queue-head readiness checks in `facility_queue`
- Handler-level authoritative validators already exist and are the right place for start-only checks that must **not** participate in affordance discovery or queue promotion.

## Architectural Decision

Do **not** introduce a declarative grant precondition for harvest/craft in this ticket.

Instead:

1. Extract a shared authoritative helper for **base exclusive-facility start readiness**:
   - actor constraints satisfied
   - normal preconditions satisfied
   - no grant requirement included
2. Reuse that helper in:
   - `worldwake-systems` queue-head promotion
   - `worldwake-sim` start validation path, indirectly or directly through existing start checks
3. Layer **matching-grant validation** on top as a start-only rule for harvest/craft handlers.
4. Consume the matching grant in `on_start`, after start validation succeeds and in the same start transaction as reservation acquisition.

This keeps one authoritative readiness model for “could this operation start if the actor held the turn?” while preserving a separate gate for “does this actor currently own the turn?”.

## Summary

Require a matching `GrantedFacilityUse` before a harvest or craft action may start at an exclusive facility. Starting the real operation consumes that grant, so one grant authorizes exactly one operation.

The implementation must preserve clean architecture:

- no parallel legality engines
- no compatibility shim that lets exclusive starts bypass grant ownership
- no precondition design that causes queue promotion to self-block

## Deliverables

### 1. Shared authoritative base-readiness helper

Add or extract one reusable authoritative helper for:

- `ActionDef`
- `actor`
- `targets`
- current authoritative world state

This helper must evaluate the same non-grant readiness rules used to decide whether an exclusive facility action is startable in principle.

Use it from `crates/worldwake-systems/src/facility_queue.rs` so queue promotion stops depending on ad hoc inline iteration over constraints/preconditions.

Preferred location:

- `crates/worldwake-sim/src/action_validation.rs`

if that keeps the source of truth near authoritative action validation.

### 2. Grant validation for harvest starts

In `crates/worldwake-systems/src/production_actions.rs`:

- require a matching grant for `(actor, def.id)` before `harvest:*` may start
- implement this as a start-only authoritative validation path, not as a declarative `Precondition`

### 3. Grant validation for craft starts

Same as harvest for `craft:*` action starts.

### 4. Consume grant on start

In `start_harvest` and `start_craft`:

- verify the facility has a matching active grant for `(actor, def.id)`
- clear that grant when the action starts
- do this in the start transaction, after generic validation has succeeded

Grant consumption must happen exactly once per successful action start.

### 5. Preserve readiness alignment

Queue-head promotion and real action start must remain aligned on base readiness:

- stock availability
- workstation tag validity
- recipe knowledge
- tool availability
- staged craft occupancy rules

The grant check is an additional start-only ownership gate layered on top of that shared readiness, not a second competing legality engine.

### 6. No direct exclusive-start bypass

After this ticket:

- authoritative harvest/craft starts at exclusive facilities must fail without a matching grant
- do not preserve an alternative successful start path that skips the grant

This ticket does **not** retarget AI candidate generation yet. Queue-first autonomous planning remains the responsibility of `EXCFACACCQUE-007` through `EXCFACACCQUE-009`.

## Files to Touch

- `crates/worldwake-sim/src/action_validation.rs` — shared authoritative base-readiness helper
- `crates/worldwake-sim/src/lib.rs` — export helper if needed
- `crates/worldwake-systems/src/facility_queue.rs` — reuse shared base-readiness helper
- `crates/worldwake-systems/src/production_actions.rs` — add grant validation and grant consumption for harvest/craft starts

## Out of Scope

- queue types or component registration
- `queue_for_facility_use` action definition
- system manifest ordering
- AI planner/candidate generation/ranking/runtime changes from `EXCFACACCQUE-005` onward
- non-exclusive actions such as eat, drink, sleep, travel, transport, trade, and combat
- belief-side queue visibility

## Acceptance Criteria

### Tests that must pass

- unit test: harvest start without a matching grant fails
- unit test: harvest start with a matching grant succeeds and consumes the grant
- unit test: craft start without a matching grant fails
- unit test: craft start with a matching grant succeeds and consumes the grant
- unit test: after grant consumption, the same actor cannot start a second harvest without re-queueing
- unit test: a grant for one `ActionDefId` does not authorize a different `ActionDefId` on the same facility
- unit test: queue-head promotion still stalls on temporary depletion rather than pruning
- unit test: queue-head promotion still stalls on occupied craft workstations rather than pruning
- unit test: queue-head promotion still prunes structurally invalid facilities
- relevant existing harvest/craft start-path tests updated to provision grants explicitly
- relevant targeted crate tests
- `cargo test --workspace`
- `cargo clippy --workspace`

### Invariants that must remain true

- every exclusive harvest/craft start requires a matching grant
- grant is consumed exactly once per successful start
- reservation locking still prevents overlap
- queue promotion uses the same base readiness model as action start
- non-exclusive actions remain unaffected
- no grant requirement is encoded in a way that prevents queue-head promotion

## Notes For Implementation Review

The current architecture is cleaner if the system distinguishes two questions explicitly:

- “Is this operation startable in principle right now?”
- “Does this actor currently own the right to start it?”

That separation is robust and extensible across future exclusive facilities. Collapsing both into a single declarative precondition at this stage would make the system less correct, not more.

## Outcome

Implemented:

- extracted shared authoritative action-def readiness validation in `worldwake-sim`
- reused that shared validation from both start gating and facility queue promotion
- added handler-level grant validation plus start-time grant consumption for harvest/craft
- scoped grant enforcement to facilities that explicitly opt into exclusivity via `ExclusiveFacilityPolicy` and `FacilityUseQueue`
- strengthened production tests to cover missing grants, wrong grants, grant consumption, and reservation interaction

Changed from original plan:

- did **not** add a new declarative precondition variant
- did **not** touch `action_semantics.rs`
- did **not** enforce grants on non-exclusive facilities, because that would have turned ordinary production sites into accidental queue-only bottlenecks and regressed unrelated AI flows
