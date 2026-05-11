# S142CONEVEINS-003: Emit `ContentionResolved` from `promote_ready_head` alongside `QueueGrantPromoted`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` facility-queue grant emission extended
**Deps**: archive/tickets/S142CONEVEINS-001.md (provides `EventTag::ContentionResolved`, `ContentionEventPayload`, `EventPayload.contention_event_payload`, `WorldTxn::set_contention_event_payload`, and `build_contention_event_payload` helper)

## Problem

The facility-queue substrate (`crates/worldwake-systems/src/facility_queue.rs`) emits `EventTag::QueueGrantPromoted` at the head-promotion point (`promote_ready_head` through `commit_queue_update`) via the existing `QueueUpdateEffects.extra_tag` mechanism. This event records *that* a grant was issued but not *who else was contending*, *in what arrival order*, or *what rule resolved it*. Per FND-9 (scheduling and tie-breaking are part of the world model) and FND-29A (causal history is queryable), the resolution must produce a queryable artifact carrying the full claimant list and the rule that fired. This ticket extends the facility-queue grant path to attach a typed `ContentionResolved` payload/tag to the same committed event as `QueueGrantPromoted`, sharing the existing `QueueUpdateEffects -> commit_queue_update -> WorldTxn` plumbing.

## Assumption Reassessment (2026-05-10)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `promote_ready_head` in `crates/worldwake-systems/src/facility_queue.rs` emits `EventTag::QueueGrantPromoted` through `commit_queue_update` via `QueueUpdateEffects.extra_tag`. The function calls `queue.promote_head(tick, policy.grant_hold_ticks)` to mutate the queue, then constructs `QueueUpdateEffects` and calls `commit_queue_update`. The pre-mutation snapshot must be captured BEFORE `promote_head` is called.
2. Live reassessment corrected the drafted test inventory: not every named historical test exercises a successful grant. The grant-emission tests extended by this ticket are `expired_grant_auto_promotes_head_when_enabled`, `ready_head_is_promoted_with_expected_expiry_and_event`, `promote_ready_head_records_wait_observation_for_resource_source_facility`, `dead_agent_corpse_head_is_promoted`, and `wounded_agent_care_head_is_promoted`, plus the new `every_facility_grant_emits_contention_event`. Non-grant paths (`expired_grant_is_cleared_and_emits_event`, `structurally_invalid_head_is_pruned_and_emits_event`, `missing_intended_action_head_is_pruned_and_emits_event`, `auto_promote_false_leaves_ready_head_waiting`, and `active_exclusive_action_blocks_promotion`) assert zero `ContentionResolved` events where applicable.
3. The shared abstraction boundary under audit is the `QueueUpdateEffects -> commit_queue_update` mechanism. Live `EventRecord`s can carry multiple tags plus typed payloads, so this ticket extends the mechanism to attach a `ContentionEventPayload` to the same committed event as `QueueGrantPromoted`, NOT a parallel write path. The canonical transport remains the existing single-commit flow; no alias path is introduced.
4. Per FND-26 (systems interact through state), this emission stays inside the systems crate and emits through the unchanged event-log writer surface (`WorldTxn → commit(event_log)`). No cross-system call is added.
5. The pre-mutation snapshot of `ContentionQueue.waiting` is the data source for `queue_position` derivation. After `promote_head` mutates the queue, the head waiter is removed; without the snapshot, the granted actor's pre-grant ordinal is irrecoverable. The reassessment confirmed this ordering requirement.

## Architecture Check

1. Reuses the existing `QueueUpdateEffects` mechanism by extending its shape with `extra_contention_event: Option<ContentionEventPayload>` rather than introducing a new write path. Per FND-26, systems write to state through one canonical surface.
2. The pre-mutation snapshot is captured locally inside `promote_ready_head` (or its caller) before `promote_head` is invoked; this avoids any cross-function ordering ambiguity. The helper from ticket 001 (`build_contention_event_payload`) accepts the snapshot and the granted-actor identity; the helper is the single source for `queue_position` derivation.
3. Per FND-28, no shim or alias path is introduced. The successful grant commit now carries both `EventTag::QueueGrantPromoted` and `EventTag::ContentionResolved` plus the typed contention payload; no separate "legacy QueueGrantPromoted-only" successful-grant path coexists.

## Verification Layers

1. `EventTag::ContentionResolved` event present per facility-queue grant — event-log delta (`events_by_tag(EventTag::ContentionResolved)` returns the expected event after `promote_ready_head`)
2. Claimant ordering matches BTreeMap ordinal at the moment of grant — focused unit assertion against the emitted payload's `claimants` field
3. `winner = Some(granted_actor)` and `Granted` outcome flag on the matching claimant — focused unit assertion
4. Existing `QueueGrantPromoted` events continue to fire (regression guard) — existing grant tests remain green with the `ContentionResolved` assertion added; both event tags are present on each grant event
5. Single-layer ticket on the systems-emission surface; the AI lookup of `contention_event` is covered by ticket 005, and end-to-end attribution by ticket 007.

## What to Change

### 1. Extend `QueueUpdateEffects` to carry a contention-event payload

Add a new field `extra_contention_event: Option<ContentionEventPayload>` to the `QueueUpdateEffects` struct in `facility_queue.rs`. The payload is `Option` because not every queue-update path produces a contention resolution. Expired grants and structurally invalid prunes keep their existing `extra_tag` records but do not emit `ContentionResolved` because no new grant is issued.

### 2. Snapshot the queue before `promote_head`

In `promote_ready_head`, before the existing `queue.promote_head(tick, policy.grant_hold_ticks)` call, clone or otherwise snapshot the queue's `waiting` BTreeMap. Pass the snapshot to `build_contention_event_payload` (ticket 001's helper) along with the facility, the place, the action (from the granted waiter's `intended_action`), `ContentionResolutionRule::ArrivalTime`, the granted actor's `EntityId`, and the current tick. Set `extra_contention_event: Some(payload)` on the `QueueUpdateEffects` constructed for the successful grant.

### 3. Emit alongside the existing `extra_tag` in `commit_queue_update`

In `commit_queue_update`, where the existing `extra_tag` write occurs, also write the contention-event payload with `WorldTxn::set_contention_event_payload` and commit it through the existing `txn.commit(event_log)` pattern. The resulting single `EventRecord` is indexed by both `EventTag::QueueGrantPromoted` and `EventTag::ContentionResolved`, keeping deterministic single-commit ordering without a second event-log append.

For other `extra_tag` paths (`QueueGrantExpired`, `QueueHeadFailed`), no `ContentionResolved` emission is added; those code paths represent grant failures or expirations rather than contention resolutions, and the spec's "every grant emits" contract scopes to actual grant issuance, not failure cleanup.

### 4. Extend existing grant tests

For each existing successful-grant test in Assumption Reassessment item 2, after the existing `events_by_tag(EventTag::QueueGrantPromoted).len() == 1` assertion, add a parallel assertion `events_by_tag(EventTag::ContentionResolved).len() == 1` and inspect the payload to confirm the claimant ordering, winner, and rule match the test's setup. Tests where the grant path is not exercised should assert zero `ContentionResolved` events where they retain an event log.

### 5. New focused conformance test `every_facility_grant_emits_contention_event`

Inside the existing `#[cfg(test)]` block, add a focused test that exercises a contended queue (3+ waiters), invokes `promote_ready_head`, and asserts: (a) one `ContentionResolved` event emitted, (b) all 3 claimants present in `claimants` (truncation not exercised), (c) the head waiter has `outcome: Granted`, (d) the trailing waiters have `outcome: QueuedBehind`, (e) `winner` matches the head's actor, (f) `at_tick` matches the grant tick.

## Files to Touch

- `crates/worldwake-systems/src/facility_queue.rs` (modify)

## Out of Scope

- Resource-extraction emission via `grant_or_signal_full` (ticket 004)
- AI lookup of `contention_event` (ticket 005)
- Observer rendering of contention events (ticket 006)
- End-to-end goldens on `survival-contested.ron` and other scenarios (ticket 007)
- Emission from `QueueGrantExpired` / `QueueHeadFailed` paths — these are grant-failure paths, not contention resolutions; explicit non-goal

## Acceptance Criteria

### Tests That Must Pass

1. Existing successful-grant tests in `facility_queue.rs` (per Assumption Reassessment item 2) pass after extension to also assert `ContentionResolved`.
2. The 2 negative existing tests (`auto_promote_false_leaves_ready_head_waiting`, `active_exclusive_action_blocks_promotion`) confirm zero `ContentionResolved` events fire when no grant is issued.
3. New focused test: `every_facility_grant_emits_contention_event` asserts emission with 3-waiter contended queue.
4. Existing suite: `cargo test -p worldwake-systems facility_queue`.

### Invariants

1. Every successful `promote_ready_head` grant emits exactly one event indexed by `ContentionResolved` in addition to the existing `QueueGrantPromoted` tag.
2. Pre-mutation snapshot ordering is preserved: `queue_position` in the emitted payload reflects the BTreeMap ordinal at the moment before `promote_head` mutates the queue.
3. `winner` field matches the granted actor's `EntityId` (always `Some(_)` for this code path; queue-only shifts emit from a different path covered out-of-scope).
4. No emission from `QueueGrantExpired` or `QueueHeadFailed` paths.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/facility_queue.rs` (existing `#[cfg(test)]` block) — extend existing successful-grant tests; add 1 new focused conformance test `every_facility_grant_emits_contention_event`; verify negative tests remain accurate.

### Commands

1. `cargo test -p worldwake-systems facility_queue`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-11.

- Extended `QueueUpdateEffects` with `extra_contention_event: Option<ContentionEventPayload>`.
- Captured a pre-mutation `ContentionQueue` snapshot in `promote_ready_head`, built the payload with `build_contention_event_payload`, and attached it through `WorldTxn::set_contention_event_payload` during the existing `commit_queue_update` transaction.
- Preserved the existing `QueueGrantPromoted` tag on successful grants; the same committed event is now also indexed by `ContentionResolved` and carries the typed payload.
- Added payload assertions to successful grant tests and zero-emission assertions for non-grant paths. Added `every_facility_grant_emits_contention_event` for a three-waiter queue.

## Deviations

- Live event-log semantics use one `EventRecord` with multiple tags and typed payloads, not two separately appended event records. The landed implementation therefore attaches `ContentionResolved` to the same committed event as `QueueGrantPromoted`.
- The drafted "8 existing tests" list included non-grant paths. Those tests now assert no `ContentionResolved` emission where appropriate instead of pretending a grant occurred.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib facility_queue -- --list`
- Passed `cargo test -p worldwake-systems --lib facility_queue`
- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-systems facility_queue`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `git diff --check`
