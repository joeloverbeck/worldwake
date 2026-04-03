# S44GENCONSUB-004: Generalize contention_system()

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — contention system logic in worldwake-systems
**Deps**: S44GENCONSUB-003

## Problem

The existing `facility_queue_system()` (now `contention_system()` after S44GENCONSUB-003) only handles facility-type entities. It must be generalized to process contention queues on any entity kind (Agent, Facility) and respect `ContentionPolicy` fields (`auto_promote`, `max_waiters`) that didn't exist in the old `ExclusiveFacilityPolicy`.

## Assumption Reassessment (2026-04-04)

1. After S44GENCONSUB-003, `contention_system()` exists in `crates/worldwake-systems/src/facility_queue.rs`. The system/function identity was generalized, but the file was intentionally not renamed in that removal ticket. It iterates entities with `ContentionQueue`, calling prune_invalid_waiters, expire_stale_grant, prune_structurally_invalid_heads, promote_ready_head.
2. Current system reads `ExclusiveFacilityPolicy.grant_hold_ticks` for promotion. After migration, it reads `ContentionPolicy.grant_hold_ticks`.
3. New logic needed: respect `auto_promote` flag (only promote when true), respect `max_waiters` cap (already enforced at enqueue time but system should validate), prune by ContentionDispositionProfile patience.
4. System runs at `SystemId::Contention` slot in canonical order: after BanditCamp, before Politics. Confirmed.
5. Current prune logic checks: actor alive, actor at same place, action def still valid. These checks generalize without change.

## Architecture Check

1. Generalizing the system function to read `ContentionPolicy` instead of `ExclusiveFacilityPolicy` is a clean extension — same pattern, more fields.
2. No backward-compatibility shims — the old system function no longer exists after S44GENCONSUB-003.

## Verification Layers

1. Grant expiry clears grant → focused unit test with mock world state
2. Auto-promote=true promotes head → focused unit test
3. Auto-promote=false does NOT promote head → focused unit test
4. Dead/departed actor pruning → focused unit test
5. Patience-exceeded waiter pruning → focused unit test
6. Single-system ticket — verification is system-level unit tests.

## What to Change

### 1. Generalize contention_system() logic

In `crates/worldwake-systems/src/facility_queue.rs`:
- Iterate ALL entities with `ContentionQueue` (not just facilities)
- Read `ContentionPolicy` for each entity
- `expire_stale_grant()`: check `grant_expired(current_tick)`, clear if expired
- `auto_promote`: only call `promote_ready_head()` when `policy.auto_promote == true`
- `prune_invalid_waiters()`: remove waiters whose actor is dead (`DeadAt`), departed (different place), or whose `ContentionIntents` no longer lists this entity
- `prune_patience_exceeded()`: new — check each waiter's agent `ContentionDispositionProfile.queue_patience_ticks` against `current_tick - queued_at`. Remove if exceeded.

### 2. Add unit tests

Test all branches: auto_promote true/false, patience exceeded, dead actor pruning, departed actor pruning, race-mode behavior (max_waiters=0).

## Files to Touch

- `crates/worldwake-systems/src/facility_queue.rs` (modify — generalize logic)

## Out of Scope

- Attaching ContentionQueue to new entity types (S44GENCONSUB-007)
- Action validation integration (S44GENCONSUB-006)
- Perception of contention state (S44GENCONSUB-008)

## Acceptance Criteria

### Tests That Must Pass

1. Expired grant is cleared and head promoted (when auto_promote=true)
2. Expired grant is cleared but head NOT promoted (when auto_promote=false)
3. Dead actor pruned from waiting queue
4. Departed actor pruned from waiting queue
5. Patience-exceeded waiter pruned
6. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. System only reads/writes contention state — no cross-system privileged calls (P26)
2. Pruning never removes the current grantee (only expired grants are cleared)
3. Promotion respects BTreeMap ordering (deterministic first-in-first-out)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/facility_queue.rs` (inline tests) — generalized system behavior with ContentionPolicy fields

### Commands

1. `cargo test -p worldwake-systems contention`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
