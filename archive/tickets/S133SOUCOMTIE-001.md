# S133SOUCOMTIE-001: Add capacity_observation_weight to PreferenceProfile

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core::experience::PreferenceProfile` schema extension and save format bump
**Deps**: S131 (archived) provides the underlying `ReliabilityRecord` substrate; no live ticket dependency.

## Problem

S133's same-commodity tiebreaker (D2/D4) needs a per-agent saturation point for the capacity factor — without `PreferenceProfile.capacity_observation_weight`, the factor cannot be normalized per-agent and FND-22 (agent diversity through concrete variation) collapses for capacity learning. Every other ticket in this chain consumes this field.

## Assumption Reassessment (2026-05-03)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PreferenceProfile` lives at `crates/worldwake-core/src/experience.rs:182` with six existing fields (`route_caution_weight`, `source_trust_weight`, `route_memory_capacity`, `source_memory_capacity`, `memory_retention_ticks`, `wait_sensitivity_weight`). Default impl at `experience.rs:197` enumerates all six. Existing focused coverage: `preference_profile_default_matches_fixture_baseline` (`experience.rs:280`) and `experience_values_roundtrip_through_bincode` (`experience.rs:335`); save-format roundtrip exercised by `crates/worldwake-sim/src/save_load.rs:1042-1049`.
2. The spec at `specs/S133-source-composite-tiebreaker.md` D3 specifies `Permille::new_unchecked(20)` as the universal default; no other spec proposes additional `PreferenceProfile` fields concurrently (verified by grep across `specs/`).
3. Shared abstraction boundary under audit: the universal-component schema at `crates/worldwake-core/src/component_schema.rs:384-405` registers `PreferenceProfile` once. Adding a field to the existing struct does not change the schema entry, so macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`, `world_txn.rs`) require no edits. The `world_txn.rs:2432` delta assertion uses `PreferenceProfile::default()` directly — adding a field with a `Default` value preserves the assertion automatically.
4. No failing golden motivates this ticket directly; D3 is data-substrate-only and consumed by tickets 002+.
5. Broad workspace verification exposed authored-input fallout beyond the drafted literal sweep: three checked-in scenarios deserialize explicit `PreferenceProfile` blocks and therefore had to author `capacity_observation_weight: 20`. This is current-ticket scope because `AgentDef.preference_profile` is the live scenario-definable surface for the universal profile. The generated profile catalog also had to be regenerated so the new field and already-live `wait_sensitivity_weight` are documented from source.

## Architecture Check

1. The universal component is the canonical home for per-agent reliability dials; adding a field to the existing struct is the only design that preserves FND-22 (concrete per-agent parameter) without introducing a parallel registry. Alternatives considered: (i) a separate `CapacityObservationProfile` component — rejected as gratuitous duplication of the existing universal-profile machinery; (ii) a hardcoded constant in the AI crate — rejected per FND-22 (per-agent variation) and the spec's explicit Profile-Driven-Parameters listing.
2. No backward-compatibility shim or alias is introduced. The new field has a meaningful `Default` (`pm(20)`), so universal seeding through `World::create_agent` (`crates/worldwake-core/src/world.rs:225`) continues unchanged.

## Verification Layers

1. Default value populated on universal seeding → focused unit test (`preference_profile_default_matches_fixture_baseline` extended) and existing `world_txn.rs:2385` `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` continues passing.
2. Field survives bincode save/load → save-format roundtrip extension at `crates/worldwake-sim/src/save_load.rs:1042-1049`.
3. SAVE_FORMAT_VERSION bump enforced → existing `save_load.rs:1013` `assert_eq!(SAVE_FORMAT_VERSION, 63)` updated to `64`; failure test at `save_load.rs:1368-1377` continues to validate version mismatch path.

## What to Change

### 1. Extend the struct and Default impl

In `crates/worldwake-core/src/experience.rs`:

```rust
pub struct PreferenceProfile {
    pub route_caution_weight: Permille,
    pub source_trust_weight: Permille,
    pub route_memory_capacity: u32,
    pub source_memory_capacity: u32,
    pub memory_retention_ticks: u64,
    pub wait_sensitivity_weight: Permille,
    pub capacity_observation_weight: Permille,
}
```

Set `capacity_observation_weight: Permille::new_unchecked(20)` in `Default::default()`. Update the doc comment per spec D3 ("permille `expected useful capacity`; sources with `last_observed_capacity = 20` contribute a maximum capacity bonus, anything above saturates").

### 2. Update test_utils sample

In `crates/worldwake-core/src/test_utils.rs:160-169`, add `capacity_observation_weight: Permille::new(20).unwrap()` to `sample_preference_profile`.

### 3. Bump save format version and extend roundtrip assertion

In `crates/worldwake-sim/src/save_load.rs`:
- Line 6: `pub const SAVE_FORMAT_VERSION: u32 = 64;`
- Line 1013: `assert_eq!(SAVE_FORMAT_VERSION, 64);`
- Extend the assertion block at lines 1042-1049 to additionally assert `capacity_observation_weight` survives roundtrip.

### 4. Update all PreferenceProfile literal and authored-input construction sites

Update explicit full `PreferenceProfile { ... }` literals that do not inherit through `..sample_preference_profile()` or `PreferenceProfile::default()`. The landed full-literal fallout is `test_utils.rs`, `crates/worldwake-systems/src/{trade_actions,travel_actions,production_actions}.rs`, `crates/worldwake-ai/src/ranking.rs`, `crates/worldwake-ai/tests/golden_planner_pathology.rs`, and `crates/worldwake-ai/tests/golden_experience_preferences.rs`.

Also update checked-in scenario files with explicit `preference_profile` blocks (`scenarios/survival-preferences.ron`, `scenarios/cli-evaluation.ron`, `scenarios/final-integration.ron`) because the live scenario schema uses `PreferenceProfile` directly rather than a defaulting wrapper for omitted inner fields.

### 5. Update preference_profile_default_matches_fixture_baseline

`crates/worldwake-core/src/experience.rs:280-298` — assert `profile.capacity_observation_weight == Permille::new(20).unwrap()`.

## Files to Touch

- `crates/worldwake-core/src/experience.rs` (modify — struct, Default, baseline test)
- `crates/worldwake-core/src/test_utils.rs` (modify — sample fixture)
- `crates/worldwake-sim/src/save_load.rs` (modify — SAVE_FORMAT_VERSION bump + roundtrip assertion)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — fixture)
- `crates/worldwake-systems/src/travel_actions.rs` (modify — fixture)
- `crates/worldwake-systems/src/production_actions.rs` (modify — fixture)
- `crates/worldwake-ai/src/ranking.rs` (modify — 7 fixtures)
- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify — fixture)
- `crates/worldwake-ai/tests/golden_experience_preferences.rs` (modify — fixture)
- `scenarios/survival-preferences.ron` (modify — explicit authored `preference_profile`)
- `scenarios/cli-evaluation.ron` (modify — explicit authored `preference_profile`)
- `scenarios/final-integration.ron` (modify — explicit authored `preference_profile`)
- `docs/profiles/all-profiles.md` (regenerated via `python3 scripts/profile_docs.py --write`)

## Out of Scope

- `SourceCompositeRank` derivation (ticket 002 consumes the new field).
- `SourceReliabilityDiscount` field changes (ticket 005).
- Any per-candidate composite ranking semantics (ticket 003).

## Acceptance Criteria

### Tests That Must Pass

1. `preference_profile_default_matches_fixture_baseline` — extended to assert the new default.
2. `experience_values_roundtrip_through_bincode` — bincode roundtrip preserves the new field.
3. `experience_components_roundtrip_through_world_storage` (`experience.rs:353`) — world-level component roundtrip remains green.
4. `crates/worldwake-sim/src/save_load.rs` save/load tests including the version assertion at `:1013` and the failure path at `:1368`.
5. `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` (`world_txn.rs:2385`) remains green because `PreferenceProfile::default()` still equals `PreferenceProfile::default()`.
6. `crates/worldwake-cli/src/bin/scenario_coverage.rs` tests remain green after authored scenario blocks include the new field.
7. Existing suite: `cargo test --workspace`.

### Invariants

1. `World::create_agent` continues to seed every universal profile with `Default::default()` exactly once (FND-22A: learning state lives on per-agent components).
2. Save format version is bumped exactly once for this ticket; sibling tickets in the chain do not require additional bumps because no other ticket modifies a serialized component.
3. No backward-compat shim: the old `PreferenceProfile` shape is replaced everywhere in-tree (FND-28).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/experience.rs::preference_profile_default_matches_fixture_baseline` — assert new default.
2. `crates/worldwake-sim/src/save_load.rs` save/load roundtrip — assert `capacity_observation_weight` survives.
3. `crates/worldwake-cli/src/bin/scenario_coverage.rs` existing tests — prove checked-in scenarios with explicit `preference_profile` blocks still load.
4. No new test file needed; coverage is data-only.

### Commands

1. `cargo test -p worldwake-core experience::tests` (focused).
2. `cargo test -p worldwake-sim save_load` (focused save format).
3. `cargo test -p worldwake-core --lib world_txn::tests::create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through -- --exact` (universal seeding).
4. `cargo test -p worldwake-cli --bin scenario-coverage` (authored scenario profile load fallout).
5. `cargo test --workspace` (full regression).

Merge note: Ticket 001 bumps SAVE_FORMAT_VERSION 63→64; tickets 002–006 deliberately do NOT bump it (no other persisted component is modified).

## Outcome

Completed on 2026-05-03.

- Added `PreferenceProfile.capacity_observation_weight` with default `Permille::new_unchecked(20)`.
- Updated representative fixtures, explicit profile literals, authored scenario `preference_profile` blocks, and save/load roundtrip coverage.
- Bumped `SAVE_FORMAT_VERSION` from 63 to 64 for the persisted component shape.
- Regenerated `docs/profiles/all-profiles.md`; the generator also caught older profile-doc drift unrelated to this field (for example `AgendaProfile` and newer `CognitiveProfile` fields).

## Deviations

- The drafted 17-site literal sweep included several `..sample_preference_profile()` / default-spread sites that did not need edits once the sample/default changed.
- Broad verification exposed additional live authored-input fallout in three checked-in scenarios and generated profile docs; those were brought into scope because `AgentDef.preference_profile` is the scenario-definable profile surface.

## Verification Result

- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-core experience::tests`.
- Passed `cargo test -p worldwake-sim save_load`.
- Passed `cargo test -p worldwake-core --lib world_txn::tests::create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through -- --exact`.
- Passed `python3 scripts/profile_docs.py --write` (completed with 15 pre-existing documentation-gap warnings from the profile generator).
- Initial `cargo test --workspace` failed in `worldwake-cli --bin scenario-coverage` because explicit scenario `PreferenceProfile` blocks lacked `capacity_observation_weight`; after scenario updates, passed `cargo test -p worldwake-cli --bin scenario-coverage`.
- Passed final `cargo test --workspace`.
