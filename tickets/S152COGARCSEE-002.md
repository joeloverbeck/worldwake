# S152COGARCSEE-002: CognitiveArchetypeComponent registration and bootstrap seeding

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new universal ECS component, save format bump
**Deps**: S152COGARCSEE-001

## Problem

The assigned archetype must persist as per-agent state so the observer (ticket 006) and diagnostics (ticket 007) can read it and so it survives save/load (FND-22A, FND-29). S152 registers `CognitiveArchetypeComponent` as a universal component on `EntityKind::Agent`, defaulting to `Methodical`. The bootstrap `create_agent` path that seeds other universal profiles must also seed this component.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `World::create_agent` (`crates/worldwake-core/src/world.rs:185`) already seeds universal profiles — e.g. `world.insert_component_cognitive_profile(entity, CognitiveProfile::default())` at `world.rs:204`. The new component must be seeded in the same path. The component registration macro is `with_component_schema_entries!` at `component_schema.rs:3`; existing agent profiles register with the `|kind| kind == EntityKind::Agent` filter.
2. `crates/worldwake-core/src/world_txn.rs` carries a `create_agent` delta assertion test — `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` (`world_txn.rs:2409`). Adding a seeded component changes the per-agent component delta count; this test must be updated. (`world_txn.rs` lives in `worldwake-core`, not `worldwake-sim`.)
3. Mixed-layer boundary under audit: the ECS component-schema registration surface (`component_schema.rs`) plus the macro expansion sites that consume bare type names — `delta.rs`, `world.rs`, `component_tables.rs` (per `tickets/README.md` check #13). Each must `use crate::CognitiveArchetype`/`CognitiveArchetypeComponent` in scope.
4. (Cumulative arithmetic / save format) `SAVE_FORMAT_VERSION = 93` (`crates/worldwake-sim/src/save_load.rs:7`). Registering a new component adds a typed component table to the serialized `World`; under bincode (positional, non-self-describing) this breaks the format. This ticket bumps `93 → 94`. Confirm during implementation that the load path routes `94 => load_current_format` and that pre-94 saves are not silently accepted.
5. (Mismatch + correction) The spec's Component Registration section says `Default` returns `CognitiveArchetype::Methodical`; verify `CognitiveArchetypeComponent` wraps `CognitiveArchetype` and its `Default` yields `Methodical`. Confirm whether `profile_docs.py` (`scripts/`) enumerates non-`*Profile` components — if it regenerates a doc that includes this component, add the generated file + regen command to Files to Touch; the reassessment found no profile-field change (so the profile-doc generator is likely unaffected), but the component-doc case must be checked before finalizing.

## Architecture Check

1. Following the universal-profile precedent (seed default in `create_agent`, override at scenario spawn) keeps the component symmetric with `CognitiveProfile` and avoids a special-case bootstrap path.
2. No backwards-compatibility shim: the save bump replaces the prior format outright (FND-28); no dual-format reader is retained beyond the standard version dispatch.

## Verification Layers

1. Component is registered and queryable on agents -> focused unit test inserting/getting `CognitiveArchetypeComponent`.
2. Bootstrap seeds the default -> `create_agent` test asserts the component is present with `Methodical` (authoritative world state).
3. Save/load round-trip preserves the component -> save_load round-trip test (event-log/world-state serialization surface).
4. Single new authoritative component; no decision/action-trace layer involved at this stage — runtime population is ticket 005's contract.

## What to Change

### 1. Define `CognitiveArchetypeComponent`

In `crates/worldwake-core/src/cognitive_archetype.rs` (extends ticket 001): `pub struct CognitiveArchetypeComponent { pub archetype: CognitiveArchetype }`, `impl Component`, `impl Default` returning `Methodical`. Re-export from `lib.rs`.

### 2. Register in component schema

Add the `with_component_schema_entries!` entry in `component_schema.rs` with insert/get accessors and the `EntityKind::Agent` filter. Ensure `delta.rs`, `world.rs`, and `component_tables.rs` import the type (macro expansion sites).

### 3. Seed in bootstrap `create_agent`

In `world.rs:create_agent`, insert `CognitiveArchetypeComponent::default()` alongside the existing universal-profile seeding.

### 4. Update `world_txn.rs` delta assertion

Update `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` (`world_txn.rs:2409`) for the new per-agent component delta count.

### 5. Save format bump

Bump `SAVE_FORMAT_VERSION 93 → 94` in `crates/worldwake-sim/src/save_load.rs` and add a save/load round-trip test covering the new component.

## Files to Touch

- `crates/worldwake-core/src/cognitive_archetype.rs` (modify — add component)
- `crates/worldwake-core/src/lib.rs` (modify — re-export component)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify — macro import)
- `crates/worldwake-core/src/world.rs` (modify — registration import + `create_agent` seeding)
- `crates/worldwake-core/src/component_tables.rs` (modify — macro import)
- `crates/worldwake-core/src/world_txn.rs` (modify — delta assertion test)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` bump + round-trip test)
- `Likely:` generated component doc if `profile_docs.py` enumerates it (`grep CognitiveArchetypeComponent scripts/profile_docs.py`; regenerate with the script's documented command if affected)

## Out of Scope

- Setting the component to a *resolved* archetype at scenario spawn (ticket 005 — bootstrap seeds the default only).
- The `PersonalityAssigned` event (ticket 003).
- Observer/diagnostics reads (tickets 006/007).

## Acceptance Criteria

### Tests That Must Pass

1. `CognitiveArchetypeComponent` inserts and retrieves on an agent entity.
2. `create_agent` seeds `CognitiveArchetypeComponent` with `Methodical`; the delta assertion test passes with the updated count.
3. Save/load round-trip preserves a non-default archetype value.
4. Existing suite: `cargo test -p worldwake-core && cargo test -p worldwake-sim`

### Invariants

1. Every agent has exactly one `CognitiveArchetypeComponent` (universal).
2. `SAVE_FORMAT_VERSION` strictly increases; pre-bump saves are rejected, not silently coerced (FND-12 world meaning preserved across the boundary).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_archetype.rs` (`#[cfg(test)]`) — component insert/get + default.
2. `crates/worldwake-core/src/world_txn.rs` — updated `create_agent` delta assertion.
3. `crates/worldwake-sim/src/save_load.rs` (`#[cfg(test)]`) — archetype-component round-trip.

### Commands

1. `cargo test -p worldwake-core create_agent`
2. `cargo test -p worldwake-sim save_load`
3. `./scripts/verify.sh`

Merge note: Ticket 002 bumps SAVE_FORMAT_VERSION 93→94; ticket 003 bumps 94→95 (EventPayload field). The two bumps form a cascade — see the spec decomposition's Merge-Order Constraints; 002 must land before 003.
