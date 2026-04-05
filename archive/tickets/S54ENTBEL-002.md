# S54ENTBEL-002: Perception claim emission migration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — perception/report intake refactored to emit claims, working-memory derivation, Tell acceptance
**Deps**: S54ENTBEL-001

## Problem

Claim types and derivation exist (from 001) but the perception/report intake lane still writes directly to `known_entities`. This ticket migrates passive perception, witnessed event intake, and Tell acceptance to emit claims and derive summaries. After this ticket, `known_entities` is derived from `entity_claims` for that lane. Other explicit belief-refresh paths remain lawful direct writers and are deferred.

## Assumption Reassessment (2026-04-05)

1. `observe_passive_local_entities` at `crates/worldwake-systems/src/perception.rs:205-257` currently calls `apply_direct_local_observation_batch` which calls `store.update_entity(subject, snapshot)`. This writes directly to `known_entities`. Confirmed.
2. `process_witness_event` in perception — emits belief updates from witnessed events. Currently writes to `known_entities` via `update_entity`. Must be migrated to claim emission.
3. Tell acceptance in `crates/worldwake-systems/src/tell_actions.rs` (or perception) — when agent hears a Tell, it accepts beliefs from the speaker. Currently writes to `known_entities`. Must be migrated to claim emission with `source: Report/Rumor` and incremented chain_len.
4. `enforce_capacity` at `belief.rs:108` — after ticket 001, still operates on `known_entities` during the coexistence window. This ticket flips entity-memory retention over to `entity_claims` for claim-backed entities while preserving fallback retention for still-direct lanes.
5. Ticket 001 already consumed the additive shape change and bumped `SAVE_FORMAT_VERSION` to 27. This ticket no longer owns any legacy-save migration path; behavior changes apply only to current-format worlds going forward.
6. `derive_entity_summary` from ticket 001 rebuilds `known_entities` from claims. Called after all claim emission within a perception pass.
7. Live production code still has other lawful direct `update_entity(...)` writers outside this ticket’s information path, including explicit investigation, ask-witness, production aftermath, combat aftermath, travel aftermath, transport aftermath, and trade aftermath. This ticket does not remove those direct writers.

## Architecture Check

1. This is the behavioral migration for the perception/report lane only. Passive perception, witnessed event intake, and Tell acceptance are refactored to emit claims. After claim emission, `derive_entity_summary` rebuilds `known_entities`. The planner reads `known_entities` unchanged — zero planner changes.
2. Migration is atomic per perception pass: all claims emitted → all summaries derived → planner reads. No intermediate state where some entities have claims and others don't.
3. Other explicit belief-refresh actions remain outside this ticket. They can continue to use direct `known_entities` writes until a later cleanup ticket owns them.
4. No legacy-save migration is introduced here. Current-format worlds transition through normal runtime behavior only.

## Verification Layers

1. Perception emits claims instead of direct writes → focused unit test (claim list populated after observation)
2. `known_entities` derived from claims matches previous direct-write values → behavioral equivalence test
3. Tell acceptance creates claims with Report/Rumor source → focused unit test
4. Witness events create claims from event deltas → focused unit test
5. `enforce_entity_claim_capacity` replaces old `enforce_capacity` for entity beliefs → capacity test
6. Current-format behavior only: observed/witnessed/reported entities gain claims and derived summaries without direct writes → focused tests
7. All golden tests pass — behavioral equivalence confirmed across full E2E suite
8. Cross-layer: perception (systems) writes claims to belief store (core) → planner (AI) reads derived summaries (core). Verified by golden test pass.

## What to Change

### 1. Refactor observe_passive_local_entities

In `crates/worldwake-systems/src/perception.rs`:

Instead of calling `store.update_entity(subject, snapshot)`, decompose the observed `BelievedEntityState` into individual `EntityBeliefClaim` entries:
- One claim per populated aspect (Location if `last_known_place.is_some()`, Alive always, Inventory per commodity, etc.)
- Source: `PerceptionSource::DirectObservation`
- Confidence: computed from `observation_fidelity` via `BeliefConfidencePolicy.direct_observation_base`
- `acquired_tick`: current tick
- Append claims to `store.entity_claims`
- Increment `store.next_claim_id` for each claim

After all observations processed, call `derive_entity_summary` for each affected entity and update `store.known_entities`.

### 2. Refactor process_witness_event

Similar pattern: event state deltas converted to claims instead of direct BelievedEntityState writes. Source depends on how the witness information was acquired.

### 3. Refactor Tell acceptance

In Tell acceptance path:
- Speaker's shared claims become new claims on the listener with:
  - `source: PerceptionSource::Report { from: speaker, chain_len: speaker_chain_len + 1 }` (or Rumor if beyond direct chain)
  - Confidence recomputed via `report_base` or `rumor_base` with chain penalty
- Append to listener's `entity_claims`
- Re-derive affected entities in `known_entities`

### 4. Replace enforce_capacity for claim-backed entity beliefs

In `crates/worldwake-core/src/belief.rs`:
- `enforce_capacity` method: run `enforce_entity_claim_capacity` (from 001) for claim-backed entities first
- Preserve direct-write fallback eviction for entities that still have no backing claims
- Keep non-entity-belief enforcement unchanged (social_observations, etc.)

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify — major refactor)
- `crates/worldwake-systems/src/tell_actions.rs` (modify — Tell acceptance path, if claim sharing lives here)
- `crates/worldwake-core/src/belief.rs` (modify — enforce_capacity replacement)

## Out of Scope

- Claim types and derivation function — ticket 001
- Golden test for contradictory claims — ticket 003
- Planner changes (reads known_entities unchanged)
- Institutional beliefs (already claim-based, not migrated)
- Explicit contradiction detection (deferred per spec Non-Goals)

## Acceptance Criteria

### Tests That Must Pass

1. Passive observation emits claims instead of direct known_entities writes
2. `known_entities` derived from claims matches previous direct-write values for same input
3. Tell acceptance creates claims with correct Report/Rumor source and chain_len
4. Witness event processing creates claims from event deltas
5. `enforce_entity_claim_capacity` correctly replaces old entity belief eviction
6. No legacy-save migration support is added or required
7. All golden tests pass — behavioral equivalence
8. Existing suite: `cargo test --workspace`

### Invariants

1. After this ticket, passive perception, witnessed event intake, and Tell acceptance no longer write `known_entities` directly — they emit claims and derive summaries
2. Claim emission is atomic per perception pass — no partial claim states visible to planner
3. `next_claim_id` monotonically increases across all claim emission paths owned by this ticket
4. Other explicit belief-refresh paths may still write `known_entities` directly until a later cleanup ticket owns them
5. No older save formats are accepted by this ticket
6. SAVE_FORMAT_VERSION remains unchanged unless another persisted-shape change is introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` — Claim emission from passive observation: claim list populated, aspects match observed state
2. `crates/worldwake-systems/src/perception.rs` — Behavioral equivalence: derive_entity_summary over emitted claims matches previous direct-write output
3. `crates/worldwake-systems/src/tell_actions.rs` — Tell acceptance claim creation with source chain
4. `crates/worldwake-core/src/belief.rs` — enforce_capacity now delegates to claim enforcement
5. `None — this ticket does not change the save boundary; current-format-only support remains unchanged`

### Commands

1. `cargo test -p worldwake-systems -- perception`
2. `cargo test -p worldwake-ai` (golden tests verify behavioral equivalence)
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-05
- What changed:
  - Migrated passive perception, witnessed event intake, and Tell acceptance onto claim-backed entity belief recording.
  - Updated `AgentBeliefStore` retention to enforce claim-backed entity handling before direct-only fallback memory handling.
  - Fixed claim staleness to age reports from `claimed_event_tick` when present.
  - Corrected snapshot claim emission so fresh observations preserve canonical summary facts without over-emitting default aspects.
- Deviations from original plan:
  - The ticket remained narrowed to the passive-perception / witnessed-event / Tell lane. Other lawful direct `update_entity(...)` writers were not migrated and remain deferred.
  - Finishing the ticket required two migration bug fixes discovered during broad verification: sparse snapshot claim emission and correct `alive=false` claim emission on death refresh.
  - The broader architectural follow-up to split cross-entity memory breadth from per-subject claim depth was not absorbed here; it is captured by `S54ENTBEL-004`.
- Verification results:
  - Focused regression coverage passed for core claim derivation and capacity handling, systems perception/Tell behavior, and AI merchant/combat goldens.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
