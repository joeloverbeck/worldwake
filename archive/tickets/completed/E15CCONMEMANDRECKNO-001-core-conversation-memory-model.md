# E15CCONMEMANDRECKNO-001: Core Conversation Memory Model

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` belief schema, retention helpers, TellProfile fields
**Deps**: `specs/E15c-conversation-memory-and-recipient-knowledge.md`, `specs/IMPLEMENTATION-ORDER.md`

## Problem

E15c requires first-class conversation memory, but `crates/worldwake-core/src/belief.rs` currently stores only `known_entities` and `social_observations`. There is no lawful place to remember what an agent already told or heard, no retention-aware read API, and no share-equivalent snapshot type that ignores bookkeeping-only belief refreshes.

## Assumption Reassessment (2026-03-19)

1. `crates/worldwake-core/src/belief.rs` currently defines `AgentBeliefStore` with only `known_entities` and `social_observations`, and `TellProfile` with only `max_tell_candidates`, `max_relay_chain_len`, and `acceptance_fidelity`.
2. Existing focused coverage in `worldwake-core` covers generic belief retention and TellProfile serialization, but not conversation memory: `belief::tests::enforce_capacity_removes_stale_entities_and_social_observations`, `belief::tests::enforce_capacity_evicts_oldest_entities_deterministically`, `belief::tests::tell_profile_roundtrips_through_bincode`. Existing component roundtrip coverage already exists in `world::tests::agent_belief_store_component_roundtrip_on_agent`, `world::tests::tell_profile_component_roundtrip_on_agent`, `world_txn::tests::set_component_agent_belief_store_records_component_delta_and_updates_world_on_commit`, and `world_txn::tests::set_component_tell_profile_records_component_delta_and_updates_world_on_commit`.
3. There is no current symbol for `SharedBeliefSnapshot`, `TellMemoryKey`, `ToldBeliefMemory`, `HeardBeliefMemory`, `HeardBeliefDisposition`, or retention-aware conversation-memory reads; this is missing architecture, not a rename.
4. The E15c spec explicitly requires retention to apply on reads as well as writes. Current `AgentBeliefStore::enforce_capacity()` only mutates storage on write-side maintenance, and there is no parallel conversation-memory read surface yet.
5. `component_schema.rs`, `world.rs`, and most of `world_txn.rs` already derive component plumbing generically. The likely production edit surface is `crates/worldwake-core/src/belief.rs` plus fixture/test updates in `crates/worldwake-core/src/component_tables.rs`, `crates/worldwake-core/src/delta.rs`, `crates/worldwake-core/src/world.rs`, `crates/worldwake-core/src/world_txn.rs`, and workspace call sites that construct `TellProfile` or `AgentBeliefStore`.
6. No current test names in `cargo test -p worldwake-core -- --list` cover second conversation-memory lane capacity, independent eviction of told vs heard state, or share-equivalence that ignores bookkeeping-only refreshes.
7. Mismatch and correction: this ticket should establish the core data model and helper API first, while limiting itself to compile-through fixture updates outside `worldwake-core`. AI candidate-generation, tell commit semantics, and runtime/planning query plumbing remain follow-on tickets.

## Architecture Check

1. Storing conversation memory inside `AgentBeliefStore` is cleaner than adding an AI-only resend cache because it keeps memory as explicit world state and preserves the E14 belief boundary.
2. The share-equivalent comparison must live beside `BelievedEntityState` in `worldwake-core`, not as duplicated ad hoc field comparisons in AI and systems.
3. Extending the existing belief-store and TellProfile schema is cleaner than introducing a parallel conversation-memory component because the policy and retained artifacts remain colocated with the belief data they govern.
4. No backwards-compatibility shim is allowed: the old schema must evolve in place.

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

Adjust the existing core serialization/component roundtrip fixtures and tests so the new fields round-trip through world/component/delta APIs without adding shims. Do not rewrite generic component plumbing unless the implementation proves an actual hole.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify fixture/test samples as needed)
- `crates/worldwake-core/src/delta.rs` (modify fixture/test samples as needed)
- `crates/worldwake-core/src/lib.rs` (modify re-exports for new core belief types/helpers)
- `crates/worldwake-core/src/world.rs` (modify fixture/test samples as needed)
- `crates/worldwake-core/src/world_txn.rs` (modify fixture/test samples as needed)
- workspace test/support call sites that construct `TellProfile` or `AgentBeliefStore` directly (modify only as needed for compile-through)

## Out of Scope

- AI candidate generation changes
- Tell action commit semantics
- Runtime/planning view trait plumbing
- Golden social behavior coverage
- Refactoring generic component-schema macros without an implementation-proven need

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core belief::tests::shared_belief_snapshot_ignores_observed_tick_and_matches_shareable_content`
2. `cargo test -p worldwake-core belief::tests::conversation_memory_read_helpers_ignore_expired_entries_before_cleanup`
3. `cargo test -p worldwake-core belief::tests::enforce_conversation_memory_evicts_oldest_told_and_heard_entries_independently`
4. `cargo test -p worldwake-core belief::tests::tell_profile_roundtrips_through_bincode`
5. `cargo test -p worldwake-core world::tests::agent_belief_store_component_roundtrip_on_agent`
6. `cargo test -p worldwake-core world_txn::tests::set_component_tell_profile_records_component_delta_and_updates_world_on_commit`
7. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Conversation memory is stored as agent-local belief state, never as a global or AI-only cache.
2. Share-equivalence depends only on shareable content, not `observed_tick`.
3. Expired tell memory must be invisible through the read API even before a later maintenance write runs.
4. Told-memory and heard-memory capacity limits remain deterministic and independent.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — add focused retention, eviction, snapshot-equivalence, and recipient-knowledge helper tests.
2. `crates/worldwake-core/src/world.rs` — update component roundtrip/default tests for the expanded `TellProfile` and `AgentBeliefStore`.
3. `crates/worldwake-core/src/world_txn.rs` — update component mutation tests for the expanded schema.
4. `crates/worldwake-core/src/component_tables.rs` and `crates/worldwake-core/src/delta.rs` — update roundtrip/sample-value fixtures for the expanded schema.

### Commands

1. `cargo test -p worldwake-core belief::tests::shared_belief_snapshot_ignores_observed_tick_and_matches_shareable_content`
2. `cargo test -p worldwake-core belief::tests::conversation_memory_read_helpers_ignore_expired_entries_before_cleanup`
3. `cargo test -p worldwake-core belief::tests::enforce_conversation_memory_evicts_oldest_told_and_heard_entries_independently`
4. `cargo test -p worldwake-core world::tests::agent_belief_store_component_roundtrip_on_agent`
5. `cargo test -p worldwake-core world_txn::tests::set_component_tell_profile_records_component_delta_and_updates_world_on_commit`
6. `cargo test -p worldwake-core`

## Outcome

- Outcome amended: 2026-03-19
- Completion date: 2026-03-19
- What actually changed:
  - extended `AgentBeliefStore` with first-class `told_beliefs` and `heard_beliefs`
  - added `TellMemoryKey`, `ToldBeliefMemory`, `HeardBeliefMemory`, `HeardBeliefDisposition`, `SharedBeliefSnapshot`, and `RecipientKnowledgeStatus`
  - added share-equivalence, retention-aware read helpers, recipient-knowledge derivation, and deterministic conversation-memory eviction in `crates/worldwake-core/src/belief.rs`
  - extended `TellProfile` with conversation-memory capacity and retention policy
  - normalized conversation-memory identity so `(counterparty, subject)` lives only in `TellMemoryKey`; the stored `ToldBeliefMemory` and `HeardBeliefMemory` values now carry only payload/timing/disposition data
  - updated core roundtrip/sample fixtures plus cross-crate `TellProfile` constructors required for compile-through
- Deviations from original plan:
  - no generic component-schema refactor was needed; existing macro-driven world/component plumbing already handled the expanded schema
  - `component_schema.rs` did not need production edits
  - cross-crate AI/systems changes were limited to schema-constructor updates, not behavior changes
  - the final core model intentionally diverges from the draft spec's initial record shape by removing duplicated identity fields from the stored tell-memory values
- Verification results:
  - focused tests passed:
    - `cargo test -p worldwake-core belief::tests::shared_belief_snapshot_ignores_observed_tick_and_matches_shareable_content`
    - `cargo test -p worldwake-core belief::tests::conversation_memory_read_helpers_ignore_expired_entries_before_cleanup`
    - `cargo test -p worldwake-core belief::tests::enforce_conversation_memory_evicts_oldest_told_and_heard_entries_independently`
    - `cargo test -p worldwake-core world::tests::agent_belief_store_component_roundtrip_on_agent`
    - `cargo test -p worldwake-core world_txn::tests::set_component_tell_profile_records_component_delta_and_updates_world_on_commit`
  - broader verification passed:
    - `cargo test -p worldwake-core`
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets -- -D warnings`
