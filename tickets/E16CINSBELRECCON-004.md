# E16CINSBELRECCON-004: WorldTxn Record Helpers and Belief Projection

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new methods on WorldTxn in worldwake-core
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-002, E16CINSBELRECCON-003

## Problem

Mutating systems (office installation, support declaration, faction changes) need atomic helpers to create records, append/supersede entries, and project institutional claims into agent belief stores — all within a single `WorldTxn` commit. Without these, record mutation would require ad-hoc component reads/writes scattered across handlers.

## Assumption Reassessment (2026-03-21)

1. `WorldTxn` in `world_txn.rs` provides transactional mutation with staged commit. It already has helpers for entity creation, component mutation, and relation changes. No record-specific helpers exist.
2. `World` in `world.rs` will have `create_record()` from ticket -002. `WorldTxn` needs its own `create_record()` that goes through the transaction journal.
3. Projection into `AgentBeliefStore.institutional_beliefs` requires reading the agent's `PerceptionProfile.institutional_memory_capacity` to enforce capacity limits.
4. N/A — not a planner ticket.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatch.
12. N/A.

## Architecture Check

1. Centralizing record mutation in `WorldTxn` ensures atomicity (record entry + authoritative state change in one commit) and prevents scattered component reads. Follows the existing pattern of domain-specific WorldTxn helpers.
2. No backward-compatibility shims.

## Verification Layers

1. `create_record()` produces valid Record entity → focused unit test
2. `append_record_entry()` adds entry to RecordData.entries → unit test
3. `supersede_record_entry()` marks old entry superseded → unit test
4. `project_institutional_belief()` writes into agent's institutional_beliefs → unit test
5. Capacity enforcement evicts oldest when exceeding memory_capacity → unit test
6. Single-layer ticket — WorldTxn helpers only, no cross-layer coupling.

## What to Change

### 1. `WorldTxn::create_record()` in `world_txn.rs`

Allocate a `Record` entity, attach `RecordData` component with provided `RecordKind`, `home_place`, `issuer`, `consultation_ticks`, `max_entries_per_consult`. Place the record at `home_place` via existing placement relations.

### 2. `WorldTxn::append_record_entry()` in `world_txn.rs`

Read the record's `RecordData`, call `RecordData::append_entry(claim, tick)`, write the mutated component back. Return the new `RecordEntryId`.

### 3. `WorldTxn::supersede_record_entry()` in `world_txn.rs`

Read the record's `RecordData`, call `RecordData::supersede_entry(old_id, new_claim, tick)`, write back. Return the new `RecordEntryId` or error if old_id not found.

### 4. `WorldTxn::project_institutional_belief()` in `world_txn.rs`

Given: agent EntityId, `InstitutionalBeliefKey`, `BelievedInstitutionalClaim`.

- Read the agent's `AgentBeliefStore`
- Read the agent's `PerceptionProfile.institutional_memory_capacity`
- Insert the claim into `institutional_beliefs[key]`
- If total institutional belief count exceeds capacity, evict the oldest entry (by `learned_tick`) across all keys
- Write the mutated `AgentBeliefStore` back

## Files to Touch

- `crates/worldwake-core/src/world_txn.rs` (modify — add four new methods)

## Out of Scope

- `World::create_record()` (ticket -002)
- Who calls these helpers (office/faction handlers in ticket -007, consult handler in ticket -005)
- AI reading institutional beliefs (Phase B2 tickets)
- Tell integration (ticket -008)
- Perception projection (ticket -006)

## Acceptance Criteria

### Tests That Must Pass

1. `create_record()` produces entity with `EntityKind::Record` and correct `RecordData` fields
2. `create_record()` places the record at the specified home_place
3. `append_record_entry()` adds an entry with monotonically increasing `RecordEntryId`
4. `supersede_record_entry()` creates a new entry with `supersedes: Some(old_id)` and old entry is excluded from `active_entries()`
5. `supersede_record_entry()` returns error for nonexistent `old_id`
6. `project_institutional_belief()` inserts claim into agent's `institutional_beliefs`
7. `project_institutional_belief()` evicts oldest belief when capacity exceeded
8. All helpers commit atomically (no partial state if commit succeeds)
9. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Record entries are append-only within `RecordData` (no entry removal, only supersession)
2. Belief capacity is enforced by `PerceptionProfile.institutional_memory_capacity`
3. All mutations go through `WorldTxn` staged commit (no direct component mutation)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world_txn.rs` (or dedicated test module) — focused tests for each helper method, capacity enforcement, error cases

### Commands

1. `cargo test -p worldwake-core world_txn`
2. `cargo clippy --workspace && cargo test --workspace`
