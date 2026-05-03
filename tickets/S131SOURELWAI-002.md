# S131SOURELWAI-002: Wait observation hooks at both grant-promotion sites

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` (facility queue + resource-extraction grant handlers gain learning hooks)
**Deps**: archive/tickets/S131SOURELWAI-001.md

## Problem

The motivating scenario for S131 (Agent A and B competing at North Orchard with `BlockingFact(ReservationConflict)` events at ticks 7, 65, 66, 408, 1085) runs through `ResourceExtractionQueues`, while the analogous facility-queue contention case (well/forge contention) runs through `ContentionQueue`. Today neither substrate produces a wait observation that the agent's `SourceReliability` can learn from. This ticket adds parallel wait-observation hooks at both grant-promotion sites so an agent who waits N ticks for a granted slot writes that wait into the source's `ReliabilityRecord.average_wait_ticks` for the (entity, commodity) key.

## Assumption Reassessment (2026-05-03)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Facility-queue substrate**: `fn promote_ready_head` lives at `crates/worldwake-systems/src/facility_queue.rs:315–367`. The head waiter is read at line 337 (`let Some(queued) = queue.waiting.values().next() else { return Ok(()); };`) BEFORE `queue.promote_head(tick, policy.grant_hold_ticks)` at line 351 removes the waiter from `waiting`. `EventTag::QueueGrantPromoted` emits at line 362 inside `commit_queue_update`. `ContentionWaiter.queued_at: Tick` is at `crates/worldwake-core/src/contention.rs:38`. Existing inline tests in `facility_queue.rs` `#[cfg(test)]` block (line 531+): `ready_head_is_promoted_with_expected_expiry_and_event:1089`, `dead_agent_corpse_head_is_promoted:1107`, `wounded_agent_care_head_is_promoted:1155`, `auto_promote_false_leaves_ready_head_waiting:1210`. **Resource-extraction substrate**: `fn grant_or_signal_full` lives at `crates/worldwake-systems/src/production_actions.rs:462–515`. The slot's head waiter is read implicitly at line 484 (`queue.waiting.values().next()`) inside the eligibility scan; the grant transition writes `queue.granted = Some(ContentionGrant {...})` at lines 500–505 immediately after `queue.remove_actor(actor)` at line 499. Existing inline tests: `harvest_start_grants_extraction_slot_and_releases_on_commit:1874`, `harvest_start_failure_records_source_intrinsic_reliability_failure:1702`, `harvest_start_picks_free_slot_for_three_concurrent_agents:3816`, `harvest_start_enqueues_third_actor_when_single_slot_is_full:3877`.
2. The `(entity, commodity)` key for `SourceReliability.sources` is `SourceKey { entity, commodity }`, defined and used in `crates/worldwake-ai/src/ranking.rs` and `crates/worldwake-systems/src/experience_recording.rs`. For facility-queue grants on workstations carrying `ResourceSource`, the commodity is `ResourceSource.commodity` (`crates/worldwake-core/src/production.rs:75–83`). For non-resource-source facility queues (e.g., a forge with no commodity association), this hook is skipped.
3. Cross-system boundary under audit: the contention/grant lifecycle (S44, archived) is the producer; the agent's `SourceReliability` is the consumer. Per FND-26, the hook reads `ContentionWaiter.queued_at` from authoritative state (the queue is the agent's own observable position) and writes to the agent's own component — no cross-system imperative call. The new write happens inside the same `WorldTxn` that commits the grant, so the observation lands atomically with the grant transition.
4. The shared abstraction boundary is the act of "this agent's queued reservation transitioned to granted". Both substrates resolve to the same wait-time semantics (`current_tick - queued_at`), but the substrates remain distinct because `ContentionQueue` and `ResourceExtractionQueues` are independent components (FND-26) with different lifecycles. The hook is duplicated rather than abstracted because the read paths differ (singular queue vs. per-slot queues) and abstracting would couple the two grant handlers.
6. Intended verification layer is focused/unit (`#[cfg(test)]` blocks of the two production modules) — the wait-observation write is a state mutation observable by reading the actor's `SourceReliability` after the grant transition. Decision-trace and golden coverage land in tickets 004 and 005 respectively.

## Architecture Check

1. Each hook reads the head waiter's `queued_at` BEFORE the mutation that removes the waiter, captures `wait_ticks` as a local, then writes the agent's updated `SourceReliability` after the existing grant commit. This preserves the existing grant's atomicity (the grant transition + wait observation are in one transaction) and avoids re-reading the queue after mutation.
2. No backwards-compatibility shim. The hook is a new write site on the existing `SourceReliability` component; no parallel learning path is introduced and no legacy observation channel persists.
3. The duplication of the hook across the two substrates is intentional per FND-26 — abstracting "any grant transition fires wait observation" through a shared helper would couple `facility_queue.rs` and `production_actions.rs` against a derived event abstraction that doesn't exist in the world model. The two write sites are the lawful authoritative producers of grant transitions; they each invoke the same `ReliabilityRecord::observe_wait` method from S131SOURELWAI-001.

## Verification Layers

1. Wait-observation correctness on facility-queue grants → focused/unit test in `facility_queue.rs` `#[cfg(test)]` block: enqueue an actor at Tick(10), promote at Tick(15), assert the actor's `SourceReliability.sources[SourceKey].average_wait_ticks == 5` and `wait_observation_count == 1`.
2. Wait-observation correctness on resource-extraction grants → focused/unit test in `production_actions.rs` `#[cfg(test)]` block: enqueue an actor at Tick(10) on a full slot, free the slot via commit at Tick(15), assert the actor's `SourceReliability.sources[SourceKey].average_wait_ticks == 5` after the next harvest start grants the slot.
3. Zero-wait skip behavior → focused/unit test: a fresh actor that grabs a free slot with no prior queue position records *no* observation (skipping zero waits avoids diluting the running mean).
4. No new authoritative state flows → existing event-log assertions on `EventTag::QueueGrantPromoted` continue to hold; new SourceReliability writes are component mutations on the actor entity (FND-26 state-mediated). No action-trace surface change.
5. Single-causal-layer ticket for the grant-substrate side; the AI consumer (composite ranking) is verified in ticket 004.

## What to Change

### 1. Facility-queue grant hook in `facility_queue.rs::promote_ready_head`

Modify the function (currently `crates/worldwake-systems/src/facility_queue.rs:315–367`) so that:

- Capture `let queued_actor = queued.actor;` and `let queued_at = queued.queued_at;` immediately after the existing `queued` borrow at line 337 (before the `head_is_ready_to_start` check at line 340).
- After the existing `commit_queue_update` call (line 355–366) that emits `EventTag::QueueGrantPromoted`, look up the (entity, commodity) for this facility:
  - Read `ResourceSource` on the facility via `world.get_component_resource_source(facility)`. If absent, skip wait observation (the facility has no commodity association).
  - Otherwise, build `SourceKey { entity: facility, commodity: source.commodity }`.
- Compute `let wait_ticks: u32 = (tick.0 - queued_at.0).try_into().unwrap_or(u32::MAX);` (saturate on overflow; in practice wait ticks fit in `u32`).
- Fetch `world.get_component_source_reliability(queued_actor).cloned().unwrap_or_default()`, call `record.observe_wait(wait_ticks)` on the entry obtained via `reliability.sources.entry(key).or_insert_with(|| ReliabilityRecord::new(tick))` (using the constructor from ticket 001), then write back via `world.set_component_source_reliability(queued_actor, reliability)`.

### 2. Resource-extraction grant hook in `production_actions.rs::grant_or_signal_full`

Modify the function (currently `crates/worldwake-systems/src/production_actions.rs:462–515`) so that:

- Inside the `if let Some(slot) = chosen_slot { ... }` branch, after the eligibility check `if queue.granted.is_none()` (line 492):
  - Before line 499 `queue.remove_actor(actor)`, read the slot's head waiter: `let head_queued_at = queue.waiting.values().next().filter(|w| w.actor == actor).map(|w| w.queued_at);`. Bind it to a local because line 499 mutates `waiting`.
- After the existing `txn.set_component_resource_extraction_queues(workstation, queues)` write at line 506, if `head_queued_at` is `Some(queued_at)`:
  - Compute `wait_ticks = txn.tick().0 - queued_at.0` (saturating cast to u32).
  - Skip the observation when `wait_ticks == 0` (an actor taking a free slot with no prior queue position should not record a zero-wait observation).
  - Read `ResourceSource` on the workstation via `txn.get_component_resource_source(workstation)`. The harvest start handler already requires the workstation to carry `ResourceSource`, so this should always succeed — if absent, skip silently.
  - Build `SourceKey { entity: workstation, commodity: source.commodity }`.
  - Fetch the actor's `SourceReliability`, call `record.observe_wait(wait_ticks)` on the entry, write back via `txn.set_component_source_reliability(actor, reliability)`.

### 3. Add focused tests

In `facility_queue.rs` `#[cfg(test)]` block (after the existing promote tests around line 1210), add:

- `promote_ready_head_records_wait_observation_for_resource_source_facility`: build a workstation carrying both `ContentionQueue` and `ResourceSource { commodity: Apple }`; enqueue an actor at Tick(10); call the system function at Tick(15); assert the actor's `SourceReliability.sources[SourceKey { entity: facility, commodity: Apple }].average_wait_ticks == 5` and `wait_observation_count == 1`.
- `promote_ready_head_skips_wait_observation_when_facility_has_no_resource_source`: build a workstation with `ContentionQueue` but no `ResourceSource`; promote a head; assert the actor's `SourceReliability` is empty (no key inserted, no observation recorded).

In `production_actions.rs` `#[cfg(test)]` block (after the existing harvest tests around line 3960), add:

- `harvest_start_records_wait_observation_when_promoted_from_queue`: two actors contend for a one-slot orchard; first actor harvests and commits at Tick(15) freeing the slot; second actor was queued at Tick(10) and gets the slot at Tick(15); assert the second actor's `SourceReliability.sources[SourceKey { entity: workstation, commodity: Apple }].average_wait_ticks == 5`.
- `harvest_start_skips_wait_observation_for_zero_wait_grant`: single actor takes a free slot with no prior queue position at Tick(20); assert no `SourceReliability` entry is inserted (zero-wait skip).

## Files to Touch

- `crates/worldwake-systems/src/facility_queue.rs` (modify) — `promote_ready_head` body + 2 new tests.
- `crates/worldwake-systems/src/production_actions.rs` (modify) — `grant_or_signal_full` body + 2 new tests.

## Out of Scope

- Capacity observation in perception — covered by ticket 003.
- Ranking-side consumption of `average_wait_ticks` — covered by ticket 004.
- Golden coverage of cross-tick wait learning — covered by ticket 005.
- New `EventTag` for wait observation — explicitly Non-Goal in the spec; the hook is a state-mediated write piggy-backing on existing grant transitions, observable via component reads in tests.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems facility_queue::tests::promote_ready_head_records_wait_observation_for_resource_source_facility`
2. `cargo test -p worldwake-systems facility_queue::tests::promote_ready_head_skips_wait_observation_when_facility_has_no_resource_source`
3. `cargo test -p worldwake-systems production_actions::tests::harvest_start_records_wait_observation_when_promoted_from_queue`
4. `cargo test -p worldwake-systems production_actions::tests::harvest_start_skips_wait_observation_for_zero_wait_grant`
5. Existing tests must continue to pass: `ready_head_is_promoted_with_expected_expiry_and_event`, `dead_agent_corpse_head_is_promoted`, `wounded_agent_care_head_is_promoted`, `harvest_start_grants_extraction_slot_and_releases_on_commit`, `harvest_start_failure_records_source_intrinsic_reliability_failure`, `harvest_start_picks_free_slot_for_three_concurrent_agents`, `harvest_start_enqueues_third_actor_when_single_slot_is_full`. None of these assert on `SourceReliability` today, so the new write should not perturb their assertions.
6. Existing suite: `cargo test --workspace`.

### Invariants

1. The wait-observation write happens after the grant transition is committed via the existing `commit_queue_update` / `txn.set_component_resource_extraction_queues` call — never before. Reordering would violate the read-before-mutate ordering for `queued_at`.
2. Zero-wait grants (no prior queue position) record no observation — the running mean in `ReliabilityRecord.average_wait_ticks` stays grounded in actual contention events, not in immediate access.
3. Facilities without a `ResourceSource` component skip wait observation — the (entity, commodity) key requires a commodity association the facility itself doesn't have.
4. The hook does not introduce new event-log entries (per spec Non-Goal "no new event tag"); the existing `EventTag::QueueGrantPromoted` is the authoritative grant record.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/facility_queue.rs` — 2 new `#[test]` fns in the existing `#[cfg(test)]` block per Section 3.
2. `crates/worldwake-systems/src/production_actions.rs` — 2 new `#[test]` fns in the existing `#[cfg(test)]` block per Section 3.

### Commands

1. `cargo test -p worldwake-systems facility_queue::tests` — narrowest verification while iterating on the facility-queue hook.
2. `cargo test -p worldwake-systems production_actions::tests` — narrowest verification while iterating on the resource-extraction hook.
3. `cargo test -p worldwake-systems` — confirms both hooks land cleanly in the same crate.
4. `cargo test --workspace` — confirms downstream test fixtures (especially in `worldwake-ai` agent_tick tests) still pass with new writes happening on `SourceReliability`.
5. `scripts/verify.sh` — full pre-PR gate.
