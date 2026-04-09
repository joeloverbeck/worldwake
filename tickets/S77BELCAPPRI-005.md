# S77BELCAPPRI-005: Reconcile tell listener memory capacity with tiered entity eviction

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — listener belief-capacity semantics or same-domain test setup must be corrected
**Deps**: S77BELCAPPRI-003

## Problem

After `S77BELCAPPRI-003` changed `AgentBeliefStore::enforce_capacity()` to preserve infrastructure-tier entities ahead of transient ones, the broad `worldwake-systems` suite now fails in `tell_actions::tests::tell_commit_enforces_listener_memory_capacity`. The failing test still expects a pre-tiering eviction outcome (`older_subject` fully evicted, `known_entities.len() == 1`) after a tell commit. This ticket must determine whether the tell listener path is now wrong under the new entity-tier contract or whether the test setup/assertions are stale.

## Assumption Reassessment (2026-04-09)

1. Broad verification after `S77BELCAPPRI-004` failed at `crates/worldwake-systems/src/tell_actions.rs:2534-2588` in `tell_commit_enforces_listener_memory_capacity`, specifically `assert!(listener_store.get_entity(&older_subject).is_none())`.
2. The listener-side setup seeds `older_subject` as a directly observed Agent belief and then commits a tell about another Agent subject while `entity_memory_capacity == 1` (`tell_actions.rs:2546-2578`).
3. `AgentBeliefStore::enforce_capacity()` in `crates/worldwake-core/src/belief.rs:200-216` now evicts by `(entity_eviction_tier(state), observed_tick, entity)` instead of pure age; `entity_eviction_tier()` in `belief.rs:1959-1968` treats live Agents as infrastructure-tier.
4. This is a mixed-boundary ticket: the contract under audit is the listener belief-internalization path in `tell_actions` versus the authoritative belief-capacity contract in `worldwake-core::AgentBeliefStore::enforce_capacity()`.
5. Adjacent contradiction classification: this is a separate follow-up exposed by broader verification, not in-scope for `S77BELCAPPRI-004`'s perception gate removal.

## Architecture Check

1. Reassessing the listener-memory proof against the live core eviction contract is cleaner than silently weakening broad verification or leaving the failing tell test as unexplained repo debt.
2. No backward-compatibility shims should be introduced; the fix should either align listener behavior with the tiered eviction contract or update the stale same-domain proof to the new lawful behavior.

## Verification Layers

1. Listener tell commit preserves the lawful post-capacity entity set -> focused `tell_actions` runtime test
2. Entity eviction semantics remain the same after tell-driven internalization -> focused `worldwake-core` / `worldwake-systems` proof at the strongest owned layer
3. Broad same-crate regression is resolved -> `cargo test -p worldwake-systems`
4. Single subsystem interaction ticket: no golden or planner surface is required unless reassessment proves the tell path now changes planner-visible behavior

## What to Change

### 1. Reassess the live listener-capacity contract

Trace the listener internalization path used by `commit_tell_and_finalize_event()` and confirm exactly which beliefs are present before and after `enforce_capacity()` runs. Determine whether the current production result is correct under `S77BELCAPPRI-003`'s tiered Agent preservation rules.

### 2. Fix the stale surface at the right boundary

If production behavior is correct, rewrite the failing tell test to prove the new lawful contract. If the tell path is incorrectly retaining or projecting entities, fix the production boundary instead and keep the focused proof honest about which entities should survive listener memory pressure.

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-core/src/belief.rs` (modify only if reassessment proves the production eviction contract is wrong)

## Out of Scope

- Reverting `S77BELCAPPRI-003`'s entity-tier eviction policy
- Broad perception-path changes from `S77BELCAPPRI-004`
- Golden/E2E scenario expansion unless reassessment proves tell-memory behavior changed a planner-visible contract

## Acceptance Criteria

### Tests That Must Pass

1. Focused: `tell_commit_enforces_listener_memory_capacity` reflects the lawful post-tiering result
2. Existing suite: `cargo test -p worldwake-systems -- tell_commit_enforces_listener_memory_capacity`
3. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Listener belief-capacity outcomes after tell commit are consistent with the live entity-tier eviction contract
2. The ticket proves the strongest honest boundary: stale test fixed if production is correct, production fixed if the listener path violates core eviction semantics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — `tell_commit_enforces_listener_memory_capacity` — prove the lawful listener post-capacity entity set under tiered eviction
2. `crates/worldwake-core/src/belief.rs` — only if reassessment proves a core eviction contradiction rather than stale tell test expectations

### Commands

1. `cargo test -p worldwake-systems -- tell_commit_enforces_listener_memory_capacity`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`
