# S142CONEVEINS-003: Emit `ContentionResolved` from `promote_ready_head` alongside `QueueGrantPromoted`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` facility-queue grant emission extended
**Deps**: archive/tickets/S142CONEVEINS-001.md (provides `EventTag::ContentionResolved`, `ContentionEventPayload`, `EventPayload.contention_event_payload`, `WorldTxn::set_contention_event_payload`, and `build_contention_event_payload` helper)

## Problem

The facility-queue substrate (`crates/worldwake-systems/src/facility_queue.rs`) emits `EventTag::QueueGrantPromoted` at the head-promotion point (`:381` inside `commit_queue_update`) via the existing `QueueUpdateEffects.extra_tag` mechanism. This event records *that* a grant was issued but not *who else was contending*, *in what arrival order*, or *what rule resolved it*. Per FND-9 (scheduling and tie-breaking are part of the world model) and FND-29A (causal history is queryable), the resolution must produce a queryable artifact carrying the full claimant list and the rule that fired. This ticket extends the facility-queue grant path to emit `ContentionResolved` alongside `QueueGrantPromoted` at the same emission point, sharing the existing `extra_tag` plumbing.

## Assumption Reassessment (2026-05-10)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `promote_ready_head` is at `crates/worldwake-systems/src/facility_queue.rs:327`. Its current emission of `EventTag::QueueGrantPromoted` rides through `commit_queue_update` (`:486`) via `QueueUpdateEffects.extra_tag` (the field-shape `extra_tag: Option<EventTag>` at `:16`). The actual `extra_tag: Some(EventTag::QueueGrantPromoted)` write is at `:381`. The function calls `queue.promote_head(tick, policy.grant_hold_ticks)` to mutate the queue, then constructs `QueueUpdateEffects` and calls `commit_queue_update`. The pre-mutation snapshot must be captured BEFORE `promote_head` is called.
2. The 8 existing inline tests in `facility_queue.rs` (test boundary at `:574`) that assert `QueueGrantPromoted` emission are: `expired_grant_is_cleared_and_emits_event:938`, `expired_grant_auto_promotes_head_when_enabled:968`, `structurally_invalid_head_is_pruned_and_emits_event:1000`, `missing_intended_action_head_is_pruned_and_emits_event:1026`, `ready_head_is_promoted_with_expected_expiry_and_event:1142`, `promote_ready_head_records_wait_observation_for_resource_source_facility:1160`, `dead_agent_corpse_head_is_promoted:1214`, `wounded_agent_care_head_is_promoted:1262`. Each currently asserts `events_by_tag(EventTag::QueueGrantPromoted).len() == 1` — these tests must be extended (not replaced) to also assert the `ContentionResolved` event was emitted.
3. The shared abstraction boundary under audit is the `QueueUpdateEffects → commit_queue_update` extra-tag mechanism. This ticket extends the mechanism to support emitting two correlated events at the same commit point, NOT a parallel write path. Per `docs/precision-rules.md` Rule 16, the canonical transport remains the existing single-commit flow; no alias path is introduced.
4. Per FND-26 (systems interact through state), this emission stays inside the systems crate and emits through the unchanged event-log writer surface (`WorldTxn → commit(event_log)`). No cross-system call is added.
5. The pre-mutation snapshot of `ContentionQueue.waiting` is the data source for `queue_position` derivation. After `promote_head` mutates the queue, the head waiter is removed; without the snapshot, the granted actor's pre-grant ordinal is irrecoverable. The reassessment confirmed this ordering requirement.

## Architecture Check

1. Reuses the existing `QueueUpdateEffects.extra_tag` mechanism by extending its shape (e.g., adding a parallel `extra_contention_event: Option<ContentionEventPayload>` field) rather than introducing a new write path. Per FND-26, systems write to state through one canonical surface.
2. The pre-mutation snapshot is captured locally inside `promote_ready_head` (or its caller) before `promote_head` is invoked; this avoids any cross-function ordering ambiguity. The helper from ticket 001 (`build_contention_event_payload`) accepts the snapshot and the granted-actor identity; the helper is the single source for `queue_position` derivation.
3. Per FND-28, no shim or alias path is introduced. The single emission at `:381` becomes a two-event emission at the same commit point; no separate "legacy QueueGrantPromoted-only" path coexists.

## Verification Layers

1. `EventTag::ContentionResolved` event present per facility-queue grant — event-log delta (`events_by_tag(EventTag::ContentionResolved)` returns the expected event after `promote_ready_head`)
2. Claimant ordering matches BTreeMap ordinal at the moment of grant — focused unit assertion against the emitted payload's `claimants` field
3. `winner = Some(granted_actor)` and `Granted` outcome flag on the matching claimant — focused unit assertion
4. Existing `QueueGrantPromoted` events continue to fire (regression guard) — the 8 named existing tests remain green with the `ContentionResolved` assertion added; both event tags are present after each grant
5. Single-layer ticket on the systems-emission surface; the AI lookup of `contention_event` is covered by ticket 005, and end-to-end attribution by ticket 007.

## What to Change

### 1. Extend `QueueUpdateEffects` to carry a contention-event payload

Add a new field `extra_contention_event: Option<ContentionEventPayload>` to the `QueueUpdateEffects` struct at `facility_queue.rs:16`. The payload is `Option` because not every queue-update path produces a contention resolution (e.g., expired grants, structurally invalid prunes — but those still warrant the existing `extra_tag` for `QueueGrantExpired` / `QueueHeadFailed` and may also emit a contention event with no winner; see step 3).

### 2. Snapshot the queue before `promote_head`

In `promote_ready_head` at `:327`, before the existing `queue.promote_head(tick, policy.grant_hold_ticks)` call, clone or otherwise snapshot the queue's `waiting` BTreeMap. Pass the snapshot to `build_contention_event_payload` (ticket 001's helper) along with the facility, the place (derive from the world transaction), the action (from the granted waiter's `intended_action`), `ContentionResolutionRule::ArrivalTime`, the granted actor's `EntityId`, and the current tick. Set `extra_contention_event: Some(payload)` on the `QueueUpdateEffects` constructed at `:381`.

### 3. Emit alongside the existing `extra_tag` in `commit_queue_update`

In `commit_queue_update` at `:486`, where the existing `extra_tag` write occurs (around `:506`), also write the contention-event payload with `WorldTxn::set_contention_event_payload` and commit it through the existing `txn.commit(event_log)` pattern at `:554`. Both events share the same commit boundary, ensuring deterministic ordering: `QueueGrantPromoted` first (existing behavior), `ContentionResolved` immediately after.

For other `extra_tag` paths (`QueueGrantExpired` at `:279`, `QueueHeadFailed` at `:318`), no `ContentionResolved` emission is added; those code paths represent grant failures or expirations rather than contention resolutions, and the spec's "every grant emits" contract scopes to actual grant issuance, not failure cleanup.

### 4. Extend the 8 existing inline tests

For each of the 8 named tests in Assumption Reassessment item 2, after the existing `events_by_tag(EventTag::QueueGrantPromoted).len() == 1` assertion, add a parallel assertion `events_by_tag(EventTag::ContentionResolved).len() == 1` and inspect the payload to confirm the claimant ordering, winner, and rule match the test's setup. Tests where the grant path is not exercised (`auto_promote_false_leaves_ready_head_waiting:1317`, `active_exclusive_action_blocks_promotion:1335`) should not assert `ContentionResolved` emission — verify these paths still emit zero `ContentionResolved` events.

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

1. The 8 named existing tests in `facility_queue.rs` (per Assumption Reassessment item 2) pass after extension to also assert `ContentionResolved`.
2. The 2 negative existing tests (`auto_promote_false_leaves_ready_head_waiting`, `active_exclusive_action_blocks_promotion`) confirm zero `ContentionResolved` events fire when no grant is issued.
3. New focused test: `every_facility_grant_emits_contention_event` asserts emission with 3-waiter contended queue.
4. Existing suite: `cargo test -p worldwake-systems facility_queue`.

### Invariants

1. Every successful `promote_ready_head` grant emits exactly one `ContentionResolved` event in addition to the existing `QueueGrantPromoted`.
2. Pre-mutation snapshot ordering is preserved: `queue_position` in the emitted payload reflects the BTreeMap ordinal at the moment before `promote_head` mutates the queue.
3. `winner` field matches the granted actor's `EntityId` (always `Some(_)` for this code path; queue-only shifts emit from a different path covered out-of-scope).
4. No emission from `QueueGrantExpired` or `QueueHeadFailed` paths.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/facility_queue.rs` (existing `#[cfg(test)]` block) — extend 8 named tests; add 1 new focused conformance test `every_facility_grant_emits_contention_event`; verify 2 negative tests remain accurate.

### Commands

1. `cargo test -p worldwake-systems facility_queue`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
