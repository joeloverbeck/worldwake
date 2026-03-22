# E16CINSBELRECCON-004: WorldTxn Record Helpers and Belief Projection

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new methods on WorldTxn in worldwake-core
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-002, E16CINSBELRECCON-003

## Problem

Mutating systems (office installation, support declaration, faction changes) need atomic helpers to create records, append/supersede entries, and project institutional claims into agent belief stores — all within a single `WorldTxn` commit. Without these, record mutation would require ad-hoc component reads/writes scattered across handlers.

## Assumption Reassessment (2026-03-21)

1. `WorldTxn` in `crates/worldwake-core/src/world_txn.rs` already has `create_record(record: RecordData) -> Result<EntityId, WorldError>`, but it is only a thin transactional wrapper over `World::create_record`. The missing surfaces are transactional record-entry mutation and transactional institutional-belief projection.
2. `World` in `crates/worldwake-core/src/world.rs` already has `create_record(record: RecordData, tick: Tick) -> Result<EntityId, WorldError>`. Current live behavior creates an `EntityKind::Record` with `RecordData` and leaves it physically unplaced (`world::tests::create_record_produces_correct_entity` currently asserts `world.is_in_transit(id)`). That diverges from this ticket's original assumption that record creation already places the record at `home_place`.
3. `RecordData` in `crates/worldwake-core/src/institutional.rs` already owns the authoritative entry logic: `append_entry`, `supersede_entry`, `entries_newest_first`, and `active_entries`, with focused unit coverage there. `WorldTxn` should wrap those methods transactionally rather than re-implement record semantics.
4. `AgentBeliefStore` in `crates/worldwake-core/src/belief.rs` already owns entity-belief capacity enforcement via `enforce_capacity`, but it currently does not enforce `PerceptionProfile.institutional_memory_capacity` for `institutional_beliefs` at all. The clean boundary is to add institutional-belief insertion/eviction behavior on `AgentBeliefStore` and have `WorldTxn` call into it.
4. N/A — not a planner ticket.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. Mismatch + correction: the ticket previously treated `WorldTxn::create_record()` as missing and treated record placement at `home_place` as already implied. In reality `create_record()` already exists in both `World` and `WorldTxn`, while placement does not. Scope is corrected to: keep the existing helper but upgrade record creation semantics so records are actually placed at `home_place`, then add the missing transactional entry/projection helpers around the existing `RecordData` and `AgentBeliefStore` logic.
12. N/A.

## Architecture Check

1. The beneficial architecture is:
   - `RecordData` owns record-entry semantics.
   - `AgentBeliefStore` owns institutional-belief insertion and eviction semantics.
   - `World`/`WorldTxn` own authoritative placement and transactional commit semantics.
   This is cleaner than pushing record-entry and eviction policy directly into `WorldTxn`, because it keeps domain invariants attached to the data they govern and leaves `WorldTxn` as the atomic mutation boundary rather than a second semantic owner.
2. No backward-compatibility shims.

## Verification Layers

1. record creation produces a physically local record artifact at `home_place` -> focused `world` / `world_txn` unit tests plus authoritative world-state assertion
2. transactional append/supersede wrappers preserve `RecordData` append-only semantics -> focused `world_txn` unit tests plus `RecordData` component delta assertion
3. institutional-belief projection writes the expected claim under the expected key -> focused `world_txn` unit tests plus authoritative `AgentBeliefStore` assertion
4. institutional-belief capacity eviction is deterministic across keys by `learned_tick` -> focused `belief` / `world_txn` unit tests
5. single-crate ticket, but mixed surfaces inside `worldwake-core`; additional action-trace / decision-trace mapping is not applicable.

## What to Change

### 1. Upgrade record creation semantics in `world.rs` and `world_txn.rs`

Keep the existing `World::create_record()` / `WorldTxn::create_record()` API shape, but change the implementation so records are actually placed at `RecordData.home_place` at creation time instead of being left in transit. This makes records consistent with the E16c requirement that they are first-class world artifacts with locality, and it avoids creating a second "record location" concept separate from the generic placement system.

### 2. Add transactional record-entry helpers in `world_txn.rs`

Add:
- `WorldTxn::append_record_entry()`
- `WorldTxn::supersede_record_entry()`

Both helpers should:
- read the staged `RecordData`
- delegate to the existing `RecordData` methods
- write the updated component back through the existing transactional component-delta path
- return `RecordEntryId`
- surface missing/superseded-entry failures as `WorldError`, without introducing a parallel compatibility error layer

### 3. Add institutional-belief insertion/eviction logic on `AgentBeliefStore`

Add a small focused helper on `AgentBeliefStore` for recording one institutional belief under one key and enforcing `PerceptionProfile.institutional_memory_capacity` deterministically across all keys by oldest `learned_tick`. `WorldTxn` should use this helper rather than open-coding eviction logic.

### 4. Add `WorldTxn::project_institutional_belief()` in `world_txn.rs`

Given: agent EntityId, `InstitutionalBeliefKey`, `BelievedInstitutionalClaim`.

- Read the agent's `AgentBeliefStore`
- Read the agent's `PerceptionProfile.institutional_memory_capacity`
- Insert the claim through the new `AgentBeliefStore` helper
- If total institutional belief count exceeds capacity, evict deterministically by oldest `learned_tick` across all keys
- Write the mutated `AgentBeliefStore` back

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify — place records at `home_place` on creation)
- `crates/worldwake-core/src/world_txn.rs` (modify — add/upgrade transactional helpers and tests)
- `crates/worldwake-core/src/belief.rs` (modify — add institutional-belief insertion/capacity helper and tests)

## Out of Scope

- `World::create_record()` (ticket -002)
- Who calls these helpers (office/faction handlers in ticket -007, consult handler in ticket -005)
- AI reading institutional beliefs (Phase B2 tickets)
- Tell integration (ticket -008)
- Perception projection (ticket -006)

## Acceptance Criteria

### Tests That Must Pass

1. `create_record()` produces entity with `EntityKind::Record` and correct `RecordData` fields
2. `create_record()` places the record at the specified `home_place` and no longer leaves it in transit
3. `append_record_entry()` adds an entry with monotonically increasing `RecordEntryId`
4. `supersede_record_entry()` creates a new entry with `supersedes: Some(old_id)` and old entry is excluded from `active_entries()`
5. `supersede_record_entry()` returns error for nonexistent `old_id`
6. `project_institutional_belief()` inserts claim into agent's `institutional_beliefs`
7. `project_institutional_belief()` evicts oldest belief when capacity exceeded
8. institutional-belief capacity eviction is deterministic across keys when ticks tie or when one key becomes empty after eviction
9. All helpers commit atomically (no partial state if commit succeeds)
9. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Record entries are append-only within `RecordData` (no entry removal, only supersession)
2. Belief capacity is enforced by `PerceptionProfile.institutional_memory_capacity`
3. All mutations go through `WorldTxn` staged commit (no direct component mutation)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs`
   `world::tests::create_record_produces_correct_entity` — verifies the architectural change from "record exists but is unplaced" to "record is a physically local artifact at its `home_place`".
2. `crates/worldwake-core/src/belief.rs`
   `belief::tests::record_institutional_belief_enforces_capacity_deterministically` — proves oldest institutional belief eviction works across keys.
3. `crates/worldwake-core/src/belief.rs`
   `belief::tests::record_institutional_belief_breaks_ties_by_key_then_position` — locks deterministic eviction order when `learned_tick` ties.
4. `crates/worldwake-core/src/belief.rs`
   `belief::tests::record_institutional_belief_clears_all_when_capacity_is_zero` — covers the zero-capacity edge case explicitly.
5. `crates/worldwake-core/src/world_txn.rs`
   `world_txn::tests::create_record_records_typed_component_delta` — verifies transactional creation now records the `LocatedIn` relation for records.
6. `crates/worldwake-core/src/world_txn.rs`
   `world_txn::tests::append_record_entry_records_component_delta_and_updates_world_on_commit` — proves append mutation is transactional and commits correctly.
7. `crates/worldwake-core/src/world_txn.rs`
   `world_txn::tests::supersede_record_entry_records_component_delta_and_updates_world_on_commit` — proves supersession stays append-only and commits correctly.
8. `crates/worldwake-core/src/world_txn.rs`
   `world_txn::tests::supersede_record_entry_rejects_missing_entry_without_recording_partial_deltas` — proves failed supersession does not leak partial deltas.
9. `crates/worldwake-core/src/world_txn.rs`
   `world_txn::tests::project_institutional_belief_records_component_delta_and_updates_world_on_commit` — proves projection mutates `AgentBeliefStore` transactionally.
10. `crates/worldwake-core/src/world_txn.rs`
    `world_txn::tests::project_institutional_belief_evicts_oldest_claim_across_keys` — proves projection respects institutional memory capacity via the shared belief-store policy.

### Commands

1. `cargo test -p worldwake-core world::tests::create_record_produces_correct_entity -- --exact`
2. `cargo test -p worldwake-core belief::tests::record_institutional_belief_enforces_capacity_deterministically -- --exact`
3. `cargo test -p worldwake-core world_txn::tests::append_record_entry_records_component_delta_and_updates_world_on_commit -- --exact`
4. `cargo test -p worldwake-core world_txn::tests::project_institutional_belief_records_component_delta_and_updates_world_on_commit -- --exact`
5. `cargo test -p worldwake-core`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - `World::create_record` and `WorldTxn::create_record` were upgraded so records are placed at `RecordData.home_place` rather than remaining in transit.
  - `AgentBeliefStore` gained the shared institutional-belief insertion and deterministic capacity-enforcement logic.
  - `WorldTxn` gained transactional `append_record_entry`, `supersede_record_entry`, and `project_institutional_belief` helpers that delegate to the correct semantic owners and record proper component deltas.
- Deviations from original plan:
  - The original ticket assumed `create_record()` was missing; in reality it already existed and needed semantic correction, not duplication.
  - The original ticket put institutional eviction logic directly in `WorldTxn`; the shipped version moved that policy into `AgentBeliefStore` and kept `WorldTxn` as the atomic mutation boundary.
- Verification results:
  - `cargo test -p worldwake-core` passed.
  - `cargo clippy --workspace` passed.
  - `cargo test --workspace` passed.
