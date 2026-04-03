# S44GENCONSUB-004: Generalize contention_system()

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — contention system logic in worldwake-systems
**Deps**: S44GENCONSUB-003

## Problem

The existing `facility_queue_system()` (now `contention_system()` after S44GENCONSUB-003) only handles facility-type entities. It must be generalized to process contention queues on any entity kind (Agent, Facility) and respect `ContentionPolicy` fields (`auto_promote`, `max_waiters`) that didn't exist in the old `ExclusiveFacilityPolicy`.

## Assumption Reassessment (2026-04-04)

1. After S44GENCONSUB-003, `contention_system()` exists in `crates/worldwake-systems/src/facility_queue.rs`. The loop already scans every entity carrying `ContentionQueue`; the stale part is not entity iteration but the facility-specific readiness/pruning assumptions inside the helpers. Confirmed.
2. Current system reads `ContentionPolicy.grant_hold_ticks`, but it still ignores `ContentionPolicy.auto_promote` and treats promotion as unconditional whenever the head is ready. Confirmed.
3. `max_waiters` is already enforced at enqueue time by `ContentionQueue::enqueue(..., max_waiters)`. The live system does not need a second admission gate here unless it is repairing corrupted state; the real owned gap is pruning and promotion behavior after queue state already exists.
4. The live queue cleanup path only prunes dead/deallocated/departed actors. It does not currently prune waiters whose `ContentionIntents` no longer name the entity/action, and it does not enforce `ContentionDispositionProfile.queue_patience_ticks`. Confirmed.
5. System runs at `SystemId::Contention` slot in canonical order: after BanditCamp, before Politics. Confirmed.

## Architecture Check

1. The clean extension is to make queue maintenance respect generalized contention policy and per-agent queue state (`ContentionIntents`, `ContentionDispositionProfile`) without adding a second admission path.
2. No backward-compatibility shims — the old system function no longer exists after S44GENCONSUB-003.

## Verification Layers

1. Grant expiry clears grant → focused unit test with mock world state
2. Auto-promote=true promotes head → focused unit test
3. Auto-promote=false does NOT promote head → focused unit test
4. Dead/departed actor pruning still holds → focused unit test
5. Missing/mismatched `ContentionIntents` entry prunes waiter → focused unit test
6. Patience-exceeded waiter pruning → focused unit test
6. Single-system ticket — verification is system-level unit tests.

## What to Change

### 1. Generalize contention_system() logic

In `crates/worldwake-systems/src/facility_queue.rs`:
- Read `ContentionPolicy` for each entity
- `expire_stale_grant()`: check `grant_expired(current_tick)`, clear if expired
- `auto_promote`: only call `promote_ready_head()` when `policy.auto_promote == true`
- `prune_invalid_waiters()`: remove waiters whose actor is dead (`DeadAt`), departed (different place), or whose `ContentionIntents` no longer lists this entity/action
- `prune_patience_exceeded()`: new — check each waiter's agent `ContentionDispositionProfile.queue_patience_ticks` against `current_tick - queued_at`. Remove if exceeded.
- Keep the current facility-exclusive structural validity branch for workstation actions in this ticket; broader non-facility contention domains remain downstream work.

### 2. Add unit tests

Test all branches: auto_promote true/false, patience exceeded, dead actor pruning, departed actor pruning, and intent-mismatch pruning.

## Files to Touch

- `crates/worldwake-systems/src/facility_queue.rs` (modify — generalize logic)

## Out of Scope

- Attaching ContentionQueue to new entity types (S44GENCONSUB-007)
- Action validation integration (S44GENCONSUB-006)
- Perception of contention state (S44GENCONSUB-008)

## Acceptance Criteria

### Tests That Must Pass

1. Expired grant is cleared and head promoted when `auto_promote=true`
2. Expired grant is cleared but head NOT promoted (when auto_promote=false)
3. Dead actor pruned from waiting queue
4. Departed actor pruned from waiting queue
5. Waiter without matching `ContentionIntents` entry is pruned
6. Patience-exceeded waiter pruned
6. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. System only reads/writes contention state and agent-local contention components — no cross-system privileged calls (P26)
2. Pruning never removes the current grantee (only expired grants are cleared)
3. Promotion respects BTreeMap ordering (deterministic first-in-first-out)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/facility_queue.rs` (inline tests) — generalized system behavior with ContentionPolicy fields

### Commands

1. `cargo test -p worldwake-systems contention`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completed: 2026-04-04

What changed:
- Updated `crates/worldwake-systems/src/facility_queue.rs` so `contention_system()` now respects `ContentionPolicy.auto_promote`, prunes stale waiters whose `ContentionIntents` no longer match the queue entry, enforces `ContentionDispositionProfile.queue_patience_ticks`, and clears matching agent-side contention intent entries when those waiters are removed.
- Preserved the existing facility-exclusive structural validity branch for workstation actions in this ticket while generalizing the maintenance behavior onto the new contention policy and per-agent contention state.
- Added focused systems proof for auto-promote false, intent-mismatch pruning, patience expiry, expiry-driven promotion, and kept the existing dead/departed/deallocated pruning and deterministic queue behavior coverage green.

Deviations from original plan:
- The main ticket correction was architectural rather than mechanical: the loop already scanned every entity with `ContentionQueue`, and `max_waiters` was already enforced at enqueue time. The real owned gap was maintenance behavior inside the system tick, not a second admission-time gate.
- Required verification exposed two adjacent but ticket-owned proof-surface updates in `worldwake-ai`: `golden_production.rs` had to be recalibrated from an old “grant expiry before intended action” narrative to the honest local-detour-with-grant-reuse contract, and `planner_conformance.rs` needed explicit `ContentionIntents` seeding for the queue-for-facility handler path.

Verification results:
- `cargo test -p worldwake-systems auto_promote_false_leaves_ready_head_waiting`
- `cargo test -p worldwake-systems waiter_without_matching_contention_intent_is_pruned`
- `cargo test -p worldwake-systems patience_exceeded_waiter_is_pruned`
- `cargo test -p worldwake-systems expired_grant_auto_promotes_head_when_enabled`
- `cargo test -p worldwake-systems`
- `cargo test -p worldwake-ai --test golden_production golden_local_detour_reuses_existing_grant_before_harvest`
- `cargo test -p worldwake-ai --test planner_conformance conformance_queue_for_facility`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
