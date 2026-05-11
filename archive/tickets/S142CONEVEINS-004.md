# S142CONEVEINS-004: Emit `ContentionResolved` from `grant_or_signal_full` slot grants

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` resource-extraction grant emission added
**Deps**: archive/tickets/S142CONEVEINS-001.md (provides `EventTag::ContentionResolved`, `ContentionEventPayload`, `EventPayload.contention_event_payload`, `WorldTxn::set_contention_event_payload`, and `build_contention_event_payload` helper)

## Problem

The resource-extraction grant flow at `crates/worldwake-systems/src/production_actions.rs::grant_or_signal_full` (`:484`) is the second contention-resolution substrate in the codebase after the facility-queue grant flow. Before this ticket, this path issued a slot grant by setting `queue.granted = Some(ContentionGrant { ... })` at `:528` but emitted no event recording the contention. Per the spec's S142 multi-substrate hook coverage requirement, the headline scenario "three agents converge on a single-slot orchard" runs through this path, not the facility-queue path. Without emission here, the spec's "every grant emits" contract was incomplete and the orchard scenario produced no `ContentionResolved` event. This ticket adds emission to the slot-grant decision point, sharing the helper from ticket 001.

## Assumption Reassessment (2026-05-10)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `grant_or_signal_full` is at `crates/worldwake-systems/src/production_actions.rs:484`. The slot-grant decision uses `queues.queues.iter().position(...)` at `:504` to find an available `ContentionQueue` slot, then sets `queue.granted = Some(ContentionGrant { ... })` at `:528` if the slot is free. Emission must happen at the granted-slot point AFTER the slot is identified but BEFORE the mutation, so the pre-mutation snapshot can be derived from the chosen slot's `waiting` BTreeMap.
2. The 3 existing inline tests in `production_actions.rs` (test boundary at `:1243`) that exercise extraction-slot grants are: `harvest_start_grants_extraction_slot_and_releases_on_commit:2108`, `harvest_single_slot_blocks_second_actor_and_abort_releases_slot:2176`, `harvest_second_start_failure_preserves_source_until_winner_commit:2280`. The site at `:1608` (inside `grant_facility_use` test helper) is test-only and represents a different test-fixture grant path; do not modify that.
3. The shared abstraction boundary under audit is the `WorldTxn → commit(event_log)` writer surface used by adjacent extraction code. Adding emission here uses the same `txn.add_event(...)` style write the surrounding code uses; per FND-26, systems write through one canonical surface.
4. `ResourceExtractionQueues` (`crates/worldwake-core/src/contention.rs:27`) is `pub struct { pub queues: Vec<ContentionQueue> }`. Each slot is itself a `ContentionQueue` carrying its own `waiting` BTreeMap, `granted` `Option<ContentionGrant>`, and other state. The chosen slot's `waiting` is the snapshot source.
5. The action context at the grant point includes the actor (parameter), the workstation entity (parameter), and the action def (parameter). The `AffordanceKey { facility: workstation, action: def_id }` composes from these. The place is derived from the world transaction's effective-place lookup on the workstation (or threaded in directly if available).
6. Per `docs/precision-rules.md` Rule 9 (stale-request and start-failure boundaries): this emission is at the authoritative-grant boundary, NOT the AI-recovery boundary. Ticket 005 covers the AI-side blocker reconciliation boundary.

## Architecture Check

1. Adds emission at the existing grant-decision point rather than introducing a parallel write path. Per FND-26, systems remain decoupled and emit through the canonical event-log writer.
2. The pre-mutation snapshot is captured at the chosen-slot point before `queue.granted` is set; the helper from ticket 001 (`build_contention_event_payload`) consumes the snapshot. The same snapshot-before-mutate discipline as ticket 003 — both substrates use the same helper.
3. Per FND-28, no alias or shim path. The single emission point becomes the canonical resource-extraction contention-resolution emission. No "legacy no-event" path coexists.
4. Negative case (slot full, no grant issued — the function's `Err(...)` path at the `extraction_slots_full` branch around `:480`): no `ContentionResolved` is emitted because no grant was issued. This matches the facility-queue path's symmetry: emission tracks grants, not failures.

## Verification Layers

1. `EventTag::ContentionResolved` event present per resource-extraction grant — event-log delta
2. Claimant ordering matches the chosen slot's BTreeMap ordinal at moment of grant — focused unit assertion against the emitted payload
3. `winner = Some(granted_actor)`; `Granted` outcome flag on the matching claimant — focused unit assertion
4. Negative case: when no slot is granted (full slots, signal-full Err), zero `ContentionResolved` events emitted
5. Single-layer ticket on the systems-emission surface; AI lookup is in ticket 005, end-to-end attribution in ticket 007

## What to Change

### 1. Capture pre-mutation snapshot of the chosen slot

In `grant_or_signal_full` at `:484`, after the slot-position decision (`queues.queues.iter().position(...)` at `:504` and surrounding logic) and BEFORE `queue.granted = Some(ContentionGrant { ... })` at `:528`, take a snapshot of the chosen slot's `waiting` BTreeMap. The chosen slot is identified by the `position` index returned from the iterator search.

### 2. Build and emit the contention event payload

Call `worldwake_core::contention_event::build_contention_event_payload` (ticket 001) with:
- `queue_snapshot`: the cloned `waiting`-bearing `ContentionQueue` from step 1
- `facility`: the workstation entity (parameter)
- `place`: the workstation's effective place (derive from the world transaction)
- `action`: the granted action def (parameter)
- `rule`: `ContentionResolutionRule::ArrivalTime`
- `granted_actor`: `Some(actor)` (the parameter)
- `tick`: `txn.tick()`

Stage the resulting `ContentionEventPayload` in the world transaction with `WorldTxn::set_contention_event_payload`. The setter adds `EventTag::ContentionResolved`; the `WorldTxn → commit(event_log)` pipeline remains canonical.

### 3. Emit only on successful grant

If `queue.granted` is already set to a non-actor or the slot search returns `None` (all slots full), the function returns `Err(PreconditionFailed("extraction_slots_full"))`. No `ContentionResolved` event is emitted in those branches. The actor-already-holds-grant branch (re-grab) similarly emits nothing — re-grabbing an existing grant is not a new contention resolution.

### 4. Extend the 3 existing inline tests

For each of the 3 named tests in Assumption Reassessment item 2, add assertions for `ContentionResolved` emission:
- `harvest_start_grants_extraction_slot_and_releases_on_commit:2108`: assert one `ContentionResolved` on grant; verify claimant ordering reflects the test setup.
- `harvest_single_slot_blocks_second_actor_and_abort_releases_slot:2176`: assert exactly one `ContentionResolved` (on the first agent's grant); the second agent's blocked attempt emits zero.
- `harvest_second_start_failure_preserves_source_until_winner_commit:2280`: assert `ContentionResolved` count tracks actual grants, not failed attempts.

### 5. New focused conformance test `every_extraction_grant_emits_contention_event`

Add a focused test exercising a contended extraction slot (2+ waiters on the same slot), invoke `grant_or_signal_full`, and assert: (a) exactly one `ContentionResolved` per grant, (b) all waiters present in `claimants`, (c) the granted actor's `outcome: Granted`, (d) other claimants' `outcome: QueuedBehind`, (e) `winner` matches the granted actor, (f) `contested_affordance.facility` matches the workstation, (g) `contested_affordance.action` matches the action def.

## Files to Touch

- `crates/worldwake-systems/src/production_actions.rs` (modify)

## Out of Scope

- Facility-queue emission via `promote_ready_head` (ticket 003)
- AI lookup of `contention_event` (ticket 005)
- Observer rendering (ticket 006)
- End-to-end goldens (ticket 007)
- Emission from full-slot failure paths — explicit non-goal; emission tracks grants, not denials

## Acceptance Criteria

### Tests That Must Pass

1. The 3 named existing tests in `production_actions.rs` (per Assumption Reassessment item 2) pass after extension to assert `ContentionResolved`.
2. New focused test: `every_extraction_grant_emits_contention_event` asserts emission with 2+ contended waiters.
3. Existing suite: `cargo test -p worldwake-systems production_actions`.

### Invariants

1. Every successful `grant_or_signal_full` slot grant emits exactly one `ContentionResolved` event.
2. The pre-mutation snapshot is taken before `queue.granted` is set; `queue_position` in the payload reflects the BTreeMap ordinal at that moment.
3. Slot-full and re-grab paths emit zero `ContentionResolved` events.
4. `contested_affordance.facility` is the workstation entity (not the slot index); `contested_affordance.action` is the granted action def.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production_actions.rs` (existing `#[cfg(test)]` block) — extend 3 named tests; add 1 new focused conformance test.

### Commands

1. `cargo test -p worldwake-systems production_actions`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-11.

- `grant_or_signal_full` now snapshots the chosen extraction slot before mutating `queue.granted`, builds a `ContentionEventPayload` with `ContentionResolutionRule::ArrivalTime`, and stages it through `WorldTxn::set_contention_event_payload` on successful new slot grants.
- The actor-already-granted and slot-full failure paths remain non-emitting.
- The three existing harvest-slot tests now assert resource-extraction `ContentionResolved` emission/count behavior, and `every_extraction_grant_emits_contention_event` proves the contended-waiter payload shape.

## Deviations

- The existing free-slot start tests assert an event with an empty claimant snapshot because no actor was waiting before the free grab. The new contended-waiter test covers the claimant ordering and `Granted`/`QueuedBehind` payload contract.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib production_actions::tests::every_extraction_grant_emits_contention_event -- --exact`
- Passed `cargo test -p worldwake-systems production_actions`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
