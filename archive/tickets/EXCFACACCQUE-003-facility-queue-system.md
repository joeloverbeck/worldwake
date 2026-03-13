**Status**: ✅ COMPLETED

# EXCFACACCQUE-003 — `facility_queue_system` + System Manifest Registration

**Spec sections**: §4, §7, §8
**Crates**: `worldwake-systems`, `worldwake-sim`

## Summary

Complete the missing authoritative queue-maintenance layer for the already-landed exclusive-facility queue types and join action. Add `facility_queue_system` as the system that maintains stored queue state each tick: prune invalid memberships, expire stale grants, classify the queue head as promotable vs stalled vs permanently invalid, and promote the head only when the intended exclusive operation is presently startable.

This ticket is narrower than the original draft implied:
- `ActionDefId` already lives in `worldwake-core`
- `ExclusiveFacilityPolicy`, `FacilityUseQueue`, `QueuedFacilityUse`, and `GrantedFacilityUse` already exist
- `queue_for_facility_use` already exists and is registered in the action catalog
- system dispatch wiring lives in `worldwake-systems/src/lib.rs`, not in `worldwake-sim/src/system_dispatch.rs`
- tick phase ordering is controlled by `SystemManifest`; `tick_step.rs` should not change unless the manifest contract itself changes

The system must fit the current architecture cleanly and avoid introducing a second arbitration path. Queue advancement becomes authoritative world-state maintenance; exclusive operation start gating remains the responsibility of the action path in follow-up ticket `EXCFACACCQUE-004`.

## Revalidated Assumptions

1. The repo already contains the queue domain foundations from `EXCFACACCQUE-001` and `EXCFACACCQUE-002`.
2. The current scheduler still has closed system slots `Needs -> Production -> Trade -> Combat -> Perception -> Politics`; `Perception` and `Politics` are currently no-op placeholders.
3. There is not yet a dedicated permanent-capability model for "actor can never perform this intended action again." This ticket must not invent one. Permanent head failure in scope is therefore limited to structural invalidation that the current code can authoritatively prove:
   - intended action definition no longer exists
   - intended action definition is no longer an exclusive facility operation
   - facility no longer has the required workstation marker
4. Temporary non-readiness must stall the queue rather than consume a turn. In the current codebase that includes, at minimum:
   - depleted harvest stock
   - occupied craft workstation
   - transient missing craft inputs/tools
   - any other false action constraint/precondition that is not one of the structural invalidations above
5. The clean implementation path is to reuse shared action constraint/precondition evaluation for queue-head readiness rather than duplicate harvest/craft legality logic inside the new system.

## Deliverables

### 1. `facility_queue_system` function

Create `facility_queue_system(world, event_log, current_tick)` in a new module `crates/worldwake-systems/src/facility_queue.rs`.

**Responsibilities (in this order):**

1. **Prune dead/departed/deallocated actors**: For each facility with a `FacilityUseQueue`, iterate waiting entries and remove any whose actor:
   - Is dead (`DeadAt` component exists)
   - Is no longer colocated with the facility (check placement relation)
   - No longer exists in the allocator

2. **Expire stale grants**: If `granted` is `Some` and `current_tick >= expires_at`, clear the grant and emit a same-place `QueueGrantExpired` event.

3. **Prune structurally invalid queue heads** (with failure events): If the queue head's intended action is structurally invalid under the current architecture, prune it and emit a same-place `QueueHeadFailed` event. In scope for this ticket:
   - Intended action definition missing
   - Intended action definition no longer resolves to an exclusive facility operation
   - Facility lost the required `WorkstationTag`

   Explicitly out of scope for this ticket:
   - Inventing a new permanent actor-capability model
   - Distinguishing "temporarily lacks tool/input" from "will never have tool/input again" beyond the action constraints/preconditions that already exist

4. **Promote head to granted**: If `granted` is `None`, no exclusive facility action is currently running on this facility, and the queue head's intended action currently satisfies its authoritative action constraints and preconditions (excluding any future grant gate from `EXCFACACCQUE-004`), promote the queue head to `granted` with `expires_at = current_tick + policy.grant_hold_ticks`, then emit a same-place `QueueGrantPromoted` event.

**Critical**: Temporary non-readiness does NOT prune or skip the head. The queue simply stalls until the intended action becomes startable again or the actor leaves/dies.

### 2. System manifest registration

Add a `FacilityQueue` variant to `SystemId` in `crates/worldwake-sim/src/system_manifest.rs`.

**Execution order**: After action commit/abort processing and before any later information/planning phases. In the current manifest this means:
- `Needs`
- `Production`
- `Trade`
- `Combat`
- `FacilityQueue`
- `Perception`
- `Politics`

### 3. System dispatch integration

Wire `facility_queue_system` into `crates/worldwake-systems/src/lib.rs` so the dispatch table matches the expanded manifest order.

### 4. Event tags

Add to `crates/worldwake-core/src/event_tag.rs`:
- `QueueGrantExpired` — grant forfeited due to timeout
- `QueueHeadFailed` — queue head pruned for permanent impossibility
- `QueueGrantPromoted` — head promoted to granted

## Files to Touch

- `crates/worldwake-systems/src/facility_queue.rs` — **new file**, system function
- `crates/worldwake-systems/src/lib.rs` — add `pub mod facility_queue;`
- `crates/worldwake-sim/src/system_manifest.rs` — add `FacilityQueue` system ID
- `crates/worldwake-sim/src/affordance_query.rs` — expose shared authoritative constraint/precondition evaluation helpers for queue-head readiness checks
- `crates/worldwake-systems/src/facility_queue_actions.rs` — expose shared exclusive-operation classification helper if needed by the system
- `crates/worldwake-core/src/event_tag.rs` — add event tags

## Out of Scope

- The `queue_for_facility_use` action implementation itself (already landed)
- Grant requirement gate on harvest/craft (EXCFACACCQUE-004)
- Belief views or AI planning (EXCFACACCQUE-005–010)
- Conservation verification changes (queue state does not affect item conservation)
- Inventing new permanent actor incapability state or a parallel exclusive-action taxonomy
- Changing `tick_step.rs`

## Acceptance Criteria

### Tests that must pass
- Unit test: dead actor is pruned from queue
- Unit test: actor who departed the facility's place is pruned from queue
- Unit test: deallocated actor is pruned from queue
- Unit test: expired grant (`current_tick >= expires_at`) is cleared
- Unit test: queue head with structurally invalid action (workstation removed) is pruned and `QueueHeadFailed` event emitted
- Unit test: queue head with missing intended action definition is pruned and `QueueHeadFailed` event emitted
- Unit test: queue head with temporarily depleted stock is NOT pruned and is NOT promoted — queue stalls
- Unit test: queue head with temporarily blocked craft start (for example occupied workstation) is NOT pruned and is NOT promoted
- Unit test: when grant is `None`, queue is non-empty, head action is presently startable, and no exclusive action is running, head is promoted to granted
- Unit test: promoted grant has correct `expires_at = current_tick + grant_hold_ticks`
- Unit test: when an exclusive action IS running on the facility, no promotion occurs
- Unit test: `QueueGrantPromoted` event is emitted on promotion
- Unit test: `QueueGrantExpired` event is emitted on grant expiry
- Unit test: system is idempotent — running twice in the same tick produces same result
- Unit test: manifest canonical order includes `FacilityQueue` between `Combat` and `Perception`
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- At most one active grant per facility at any time
- Queue order is stable and deterministic for a fixed seed
- Pruning only occurs for structural invalidation or invalid membership, never for temporary non-readiness
- The system does not start actions — it only advances queue state
- Events are emitted with `VisibilitySpec::SamePlace`
- Shared action legality checks remain the source of truth for queue-head promotability
- System runs after action processing and before later planning-relevant phases via manifest order

## Outcome

- Completion date: 2026-03-13
- What actually changed:
  - Added `crates/worldwake-systems/src/facility_queue.rs` with authoritative queue maintenance for invalid-membership pruning, structural head failure pruning, stale-grant expiry, and readiness-based grant promotion
  - Added queue lifecycle event tags in `worldwake-core`
  - Added `FacilityQueue` to the canonical system manifest and wired the system in `worldwake-systems/src/lib.rs`
  - Reused shared action constraint/precondition evaluation from `worldwake-sim/src/affordance_query.rs` so queue-head promotability stays aligned with authoritative action legality
  - Reused the existing exclusive-operation classification helper from `facility_queue_actions`
- Deviations from original plan:
  - No `tick_step.rs` changes were needed; manifest order already controls execution order
  - No `worldwake-sim/src/system_dispatch.rs` changes were needed; dispatch integration lives in `worldwake-systems/src/lib.rs`
  - "Actor permanently incapable" pruning was intentionally narrowed to structural invalidation the current architecture can actually prove; temporary non-readiness now stalls the queue instead of inventing a new permanence model
- Verification results:
  - `cargo test -p worldwake-systems facility_queue`
  - `cargo test -p worldwake-sim system_manifest`
  - `cargo test -p worldwake-core event_tag`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
