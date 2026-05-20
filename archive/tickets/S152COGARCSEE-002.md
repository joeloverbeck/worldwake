# S152COGARCSEE-002: CognitiveArchetypeComponent registration and bootstrap seeding

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new universal ECS component, save format bump
**Deps**: archive/tickets/S152COGARCSEE-001.md

## Problem

The assigned archetype must persist as per-agent state so the observer (ticket 006) and diagnostics (ticket 007) can read it and so it survives save/load (FND-22A, FND-29). S152 registers `CognitiveArchetypeComponent` as a universal component on `EntityKind::Agent`, defaulting to `Methodical`. The bootstrap `create_agent` path that seeds other universal profiles must also seed this component.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `World::create_agent` (`crates/worldwake-core/src/world.rs:185`) already seeds universal profiles — e.g. `world.insert_component_cognitive_profile(entity, CognitiveProfile::default())` at `world.rs:204`. The new component must be seeded in the same path. The component registration macro is `with_component_schema_entries!` at `component_schema.rs:3`; existing agent profiles register with the `|kind| kind == EntityKind::Agent` filter.
2. `crates/worldwake-core/src/world_txn.rs` carries a `create_agent` delta assertion test — `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` (`world_txn.rs:2409`). Adding a seeded component changes the per-agent component delta count; this test must be updated. (`world_txn.rs` lives in `worldwake-core`, not `worldwake-sim`.)
3. Mixed-layer boundary under audit: the ECS component-schema registration surface (`component_schema.rs`) plus the macro expansion sites that consume bare type names — `delta.rs`, `world.rs`, `component_tables.rs` (per `tickets/README.md` check #13). Each must `use crate::CognitiveArchetype`/`CognitiveArchetypeComponent` in scope.
4. (Cumulative arithmetic / save format) `SAVE_FORMAT_VERSION = 93` (`crates/worldwake-sim/src/save_load.rs:7`). Registering a new component adds a typed component table to the serialized `World`; under bincode (positional, non-self-describing) this breaks the format. This ticket bumps `93 → 94`. Confirm during implementation that the load path routes `94 => load_current_format` and that pre-94 saves are not silently accepted.
5. (Mismatch + correction) The spec's Component Registration section says `Default` returns `CognitiveArchetype::Methodical`; the implementation proves `CognitiveArchetypeComponent` wraps `CognitiveArchetype` and its `Default` yields `Methodical`. `rg -n 'CognitiveArchetypeComponent' scripts/profile_docs.py` returned zero matches, so no generated profile/component docs were affected.

## Architecture Check

1. Following the universal-profile precedent (seed default in `create_agent`, override at scenario spawn) keeps the component symmetric with `CognitiveProfile` and avoids a special-case bootstrap path.
2. No backwards-compatibility shim: the save bump replaces the prior format outright (FND-28); no dual-format reader is retained beyond the standard version dispatch.

## Verified Layers

1. Component is registered and queryable on agents -> passed `cargo test -p worldwake-core --lib cognitive_archetype_component`.
2. Bootstrap seeds the default -> passed `cargo test -p worldwake-core --lib create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through`.
3. Save/load round-trip preserves the component -> passed `cargo test -p worldwake-sim --lib save_load`.
4. Single added authoritative component; no decision/action-trace layer is involved at this stage — runtime population remains ticket 005's contract.

## Landed Changes

### 1. Defined `CognitiveArchetypeComponent`

`crates/worldwake-core/src/cognitive_archetype.rs` now defines `pub struct CognitiveArchetypeComponent { pub archetype: CognitiveArchetype }`, implements `Component`, and defaults to `Methodical`. `lib.rs` re-exports it.

### 2. Registered in component schema

`component_schema.rs` now registers `CognitiveArchetypeComponent` with insert/get/set/clear/query/count accessors and the `EntityKind::Agent` filter. Macro expansion sites in `delta.rs`, `world.rs`, and `component_tables.rs` import and sample the type.

### 3. Seeded in bootstrap `create_agent`

`world.rs:create_agent` now inserts `CognitiveArchetypeComponent::default()` alongside the existing universal-profile seeding.

### 4. Updated `world_txn.rs` delta assertion

`create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` now expects the `CognitiveArchetypeComponent` delta.

### 5. Bumped save format

`SAVE_FORMAT_VERSION` is now `94` in `crates/worldwake-sim/src/save_load.rs`; save/load tests preserve a non-default `Skeptical` archetype component and reject pre-S152 version `93`.

## Landed Files

- `crates/worldwake-core/src/cognitive_archetype.rs` (added component and focused tests)
- `crates/worldwake-core/src/lib.rs` (re-exported component)
- `crates/worldwake-core/src/component_schema.rs` (registered component)
- `crates/worldwake-core/src/delta.rs` (macro import, component sample, `ComponentKind::ALL` assertion)
- `crates/worldwake-core/src/world.rs` (macro import and `create_agent` seeding)
- `crates/worldwake-core/src/component_tables.rs` (macro import and round-trip sample)
- `crates/worldwake-core/src/world_txn.rs` (delta assertion)
- `crates/worldwake-sim/src/save_load.rs` (`SAVE_FORMAT_VERSION` bump, pre-version rejection, non-default round-trip assertion)
- No generated component/profile docs changed; `scripts/profile_docs.py` does not enumerate `CognitiveArchetypeComponent`.

## Out of Scope

- Setting the component to a *resolved* archetype at scenario spawn (ticket 005 — bootstrap seeds the default only).
- The `PersonalityAssigned` event (ticket 003).
- Observer/diagnostics reads (tickets 006/007).

## Acceptance Result

### Tests Passed

1. `CognitiveArchetypeComponent` inserts and retrieves on an agent entity.
2. `create_agent` seeds `CognitiveArchetypeComponent` with `Methodical`; the delta assertion test passes with the updated count.
3. Save/load round-trip preserves a non-default archetype value.
4. `cargo test -p worldwake-core`, `cargo test -p worldwake-sim`, and `./scripts/verify.sh` passed.

### Invariants

1. Every `World::create_agent` agent is seeded with one `CognitiveArchetypeComponent`; scenario-spawn runtime population remains ticket 005.
2. `SAVE_FORMAT_VERSION` strictly increased to `94`; pre-bump version `93` saves are rejected, not silently coerced (FND-12 world meaning preserved across the boundary).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/cognitive_archetype.rs` (`#[cfg(test)]`) — component insert/get + default.
2. `crates/worldwake-core/src/world_txn.rs` — updated `create_agent` delta assertion.
3. `crates/worldwake-sim/src/save_load.rs` (`#[cfg(test)]`) — archetype-component round-trip.

### Verified Commands

1. Passed `cargo test -p worldwake-core --lib cognitive_archetype_component`
2. Passed `cargo test -p worldwake-core --lib create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through`
3. Passed `cargo test -p worldwake-sim --lib save_load`
4. Passed `cargo test -p worldwake-core`
5. Passed `cargo test -p worldwake-sim`
6. Passed `./scripts/verify.sh`

Merge note: Ticket 002 bumps SAVE_FORMAT_VERSION 93→94; ticket 003 bumps 94→95 (EventPayload field). The two bumps form a cascade — see the spec decomposition's Merge-Order Constraints; 002 must land before 003.

## Outcome

Completed on 2026-05-20.

- Added the persisted `CognitiveArchetypeComponent` over `CognitiveArchetype`, defaulting to `Methodical`.
- Registered the component as an agent-only authoritative ECS component and updated macro expansion imports, sample values, and hardcoded component inventories.
- Seeded the component in `World::create_agent`, so the bootstrap universal-agent path now carries the default archetype state.
- Bumped `SAVE_FORMAT_VERSION` from `93` to `94`, rejected version `93` saves, and proved a non-default `Skeptical` archetype component survives save/load.

## Deviations

- The drafted generated-doc check found no generated profile/component-doc surface for `CognitiveArchetypeComponent`; no generated docs were changed.
- Runtime scenario assignment, `PersonalityAssigned` emission, observer rendering, and diagnostics remain out of scope for later S152 tickets.

## Verification Result

- Passed `cargo test -p worldwake-core --lib cognitive_archetype_component`
- Passed `cargo test -p worldwake-core --lib create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through`
- Passed `cargo test -p worldwake-sim --lib save_load`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-sim`
- Passed `./scripts/verify.sh`
- Passed zero-match generated-doc check: `rg -n 'CognitiveArchetypeComponent' scripts/profile_docs.py`
