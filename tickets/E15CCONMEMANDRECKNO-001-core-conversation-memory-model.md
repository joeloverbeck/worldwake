# E15CCONMEMANDRECKNO-001: Core Conversation Memory Model

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` belief schema, retention helpers, TellProfile fields
**Deps**: `specs/E15c-conversation-memory-and-recipient-knowledge.md`, `specs/IMPLEMENTATION-ORDER.md`

## Problem

E15c requires first-class conversation memory, but `crates/worldwake-core/src/belief.rs` currently stores only `known_entities` and `social_observations`. There is no lawful place to remember what an agent already told or heard, no retention-aware read API, and no share-equivalent snapshot type that ignores bookkeeping-only belief refreshes.

## Assumption Reassessment (2026-03-19)

1. `crates/worldwake-core/src/belief.rs` currently defines `AgentBeliefStore` with only `known_entities` and `social_observations`, and `TellProfile` with only `max_tell_candidates`, `max_relay_chain_len`, and `acceptance_fidelity`.
2. Existing focused coverage in `worldwake-core` covers generic belief retention and TellProfile serialization, but not conversation memory: `belief::tests::enforce_capacity_removes_stale_entities_and_social_observations`, `belief::tests::enforce_capacity_evicts_oldest_entities_deterministically`, `belief::tests::tell_profile_roundtrips_through_bincode`.
3. There is no current symbol for `SharedBeliefSnapshot`, `TellMemoryKey`, `ToldBeliefMemory`, `HeardBeliefMemory`, or retention-aware conversation-memory reads; this is missing architecture, not a rename.
4. The E15c spec explicitly requires retention to apply on reads as well as writes. Current `AgentBeliefStore::enforce_capacity()` only mutates storage on write-side maintenance.
5. No current test names in `cargo test -p worldwake-core -- --list` cover second social-memory lane capacity or independent eviction of told vs heard state.
6. Mismatch and correction: the current core ticket surface is broader than the old E15/E15b belief model. This ticket must establish the data model first, before AI or systems tickets attempt to use it.

## Architecture Check

1. Storing conversation memory inside `AgentBeliefStore` is cleaner than adding an AI-only resend cache because it keeps memory as explicit world state and preserves the E14 belief boundary.
2. The share-equivalent comparison must live beside `BelievedEntityState` in `worldwake-core`, not as duplicated ad hoc field comparisons in AI and systems.
3. No backwards-compatibility shim is allowed: the old schema must evolve in place.

## Verification Layers

1. `SharedBeliefSnapshot` ignores bookkeeping-only belief refreshes -> focused unit tests in `worldwake-core`
2. Retention-aware read helpers ignore expired records before cleanup writes -> focused unit tests in `worldwake-core`
3. Told and heard capacity are enforced independently and deterministically -> focused unit tests in `worldwake-core`
4. Additional runtime/action trace mapping is not applicable yet because this ticket is schema/helper-only.

## What to Change

### 1. Extend `AgentBeliefStore`

Add `told_beliefs` and `heard_beliefs` keyed by `(counterparty, subject)` plus the corresponding memory record types and dispositions.

### 2. Add shareable belief snapshot helpers

Introduce `SharedBeliefSnapshot`, `to_shared_belief_snapshot()`, share-equivalence helpers, and `RecipientKnowledgeStatus` derivation based on retention-aware told memory.

### 3. Add explicit policy and maintenance APIs

Extend `TellProfile` with `conversation_memory_capacity` and `conversation_memory_retention_ticks`, then add `record_told_belief`, `record_heard_belief`, `enforce_conversation_memory`, `told_belief_memory`, and `heard_belief_memory`.

### 4. Update component/schema roundtrips

Adjust core serialization/component registration surfaces and their tests so the new fields round-trip through world/component/delta APIs without adding shims.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify)
- `crates/worldwake-core/src/world_txn.rs` (modify)

## Out of Scope

- AI candidate generation changes
- Tell action commit semantics
- Runtime/planning view trait plumbing
- Golden social behavior coverage

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core belief::tests::shared_belief_snapshot_ignores_observed_tick_and_matches_shareable_content`
2. `cargo test -p worldwake-core belief::tests::conversation_memory_read_helpers_ignore_expired_entries_before_cleanup`
3. `cargo test -p worldwake-core belief::tests::enforce_conversation_memory_evicts_oldest_told_and_heard_entries_independently`
4. `cargo test -p worldwake-core belief::tests::tell_profile_roundtrips_through_bincode`
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Conversation memory is stored as agent-local belief state, never as a global or AI-only cache.
2. Share-equivalence depends only on shareable content, not `observed_tick`.
3. Expired tell memory must be invisible through the read API even before a later maintenance write runs.
4. Told-memory and heard-memory capacity limits remain deterministic and independent.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — add focused retention, eviction, snapshot-equivalence, and recipient-knowledge helper tests.
2. `crates/worldwake-core/src/world.rs` — update component roundtrip/default tests for the expanded `TellProfile` and `AgentBeliefStore`.
3. `crates/worldwake-core/src/world_txn.rs` — update delta/component mutation tests for the expanded schema.

### Commands

1. `cargo test -p worldwake-core belief::tests::shared_belief_snapshot_ignores_observed_tick_and_matches_shareable_content`
2. `cargo test -p worldwake-core belief::tests::enforce_conversation_memory_evicts_oldest_told_and_heard_entries_independently`
3. `cargo test -p worldwake-core`
