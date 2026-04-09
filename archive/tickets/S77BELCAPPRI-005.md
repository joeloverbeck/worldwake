# S77BELCAPPRI-005: Reconcile tell listener memory capacity with tiered entity eviction

**Status**: COMPLETED
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
6. Reassessment found a production root cause, not just a stale test: `record_entity_snapshot_claims()` in `crates/worldwake-core/src/belief.rs:81-104` re-derived summaries from claims but only restored `believed_kind` from prior summary. Freshly internalized tell snapshots therefore lost `believed_kind`, making the new subject transient while older directly observed Agents stayed infrastructure-tier. Correction applied: preserve `snapshot.believed_kind` after re-derivation when no prior summary exists.

## Architecture Check

1. Fixing the core claim-projection boundary is cleaner than weakening tell tests around a real metadata-loss bug. The listener path should not discard `believed_kind` just because the snapshot was internalized through claims rather than direct `update_entity`.
2. No backward-compatibility shims should be introduced; the fix should either align listener behavior with the tiered eviction contract or update the stale same-domain proof to the new lawful behavior.

## Verification Layers

1. Listener tell commit preserves the lawful post-capacity entity set -> focused `tell_actions` runtime test
2. Fresh claim-projected summaries preserve snapshot `believed_kind` without prior summary -> focused `worldwake-core` unit test
3. Same-domain tell regression is resolved -> focused `cargo test -p worldwake-systems -- tell_commit_enforces_listener_memory_capacity`
4. Single subsystem interaction ticket: no golden or planner surface is required unless reassessment proves the tell path now changes planner-visible behavior

## What to Change

### 1. Reassess the live listener-capacity contract

Trace the listener internalization path used by `commit_tell_and_finalize_event()` and confirm exactly which beliefs are present before and after `enforce_capacity()` runs. Determine whether the current production result is correct under `S77BELCAPPRI-003`'s tiered Agent preservation rules.

### 2. Fix the metadata-loss boundary and tell proof

Preserve `snapshot.believed_kind` when `record_entity_snapshot_claims()` rebuilds a summary without a prior entity state, then keep the tell regression proof honest by asserting the transferred subject retains `believed_kind: Some(EntityKind::Agent)` and wins listener memory capacity over the older Agent.

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-core/src/belief.rs` (modify)

## Out of Scope

- Reverting `S77BELCAPPRI-003`'s entity-tier eviction policy
- Broad perception-path changes from `S77BELCAPPRI-004`
- Golden/E2E scenario expansion unless reassessment proves tell-memory behavior changed a planner-visible contract

## Acceptance Criteria

### Tests That Must Pass

1. Focused: `tell_commit_enforces_listener_memory_capacity` reflects the lawful post-tiering result
2. Focused: `record_entity_snapshot_claims_preserves_snapshot_believed_kind_without_prior_summary` proves fresh claim-projected summaries keep snapshot kind metadata
3. Existing suite: `cargo test -p worldwake-systems -- tell_commit_enforces_listener_memory_capacity`

### Invariants

1. Listener belief-capacity outcomes after tell commit are consistent with the live entity-tier eviction contract
2. Claim-projected summaries do not silently discard `believed_kind` when there is no prior summary

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — `tell_commit_enforces_listener_memory_capacity` — prove the lawful listener post-capacity entity set under tiered eviction
2. `crates/worldwake-core/src/belief.rs` — `record_entity_snapshot_claims_preserves_snapshot_believed_kind_without_prior_summary` — prove fresh claim-projected summaries keep transferred kind metadata

### Commands

1. `cargo test -p worldwake-core -- record_entity_snapshot_claims_preserves_snapshot_believed_kind_without_prior_summary`
2. `cargo test -p worldwake-systems -- tell_commit_enforces_listener_memory_capacity`
3. `cargo test -p worldwake-core`
4. `cargo test -p worldwake-systems`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

- Fixed a production metadata-loss bug in `crates/worldwake-core/src/belief.rs`: after `record_entity_snapshot_claims()` rebuilds a summary from claims, it now preserves `snapshot.believed_kind` when there was no prior summary to copy from.
- Updated `tell_commit_enforces_listener_memory_capacity` in `crates/worldwake-systems/src/tell_actions.rs` to prove the transferred subject retains `believed_kind: Some(EntityKind::Agent)` and survives listener memory pressure under the live tiered eviction contract.
- Added a focused core proof covering the previously missing no-prior-summary path.
- Deviation from original plan: reassessment proved the failing tell regression was caused by production metadata loss in `record_entity_snapshot_claims()`, not just stale tell test expectations, so the final fix touched both `worldwake-core` and `worldwake-systems`.

## Verification Result

- Passed `cargo test -p worldwake-core -- record_entity_snapshot_claims_preserves_snapshot_believed_kind_without_prior_summary`
- Passed `cargo test -p worldwake-systems -- tell_commit_enforces_listener_memory_capacity`
- Passed `cargo test -p worldwake-core`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p worldwake-systems` still fails outside the ticket's owned tell/listener boundary in `scheduler_driven_care_actions_apply_effects_and_preserve_conservation` (`crates/worldwake-systems/tests/e09_needs_integration.rs:122`) with `RequestedAffordanceUnavailable` for a care action request. The focused tell regression is fixed, and the remaining failure is in a separate care-action integration path.
