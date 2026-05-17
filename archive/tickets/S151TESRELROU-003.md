# S151TESRELROU-003: Universal profiles + component registration + bootstrap + scenario integration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — two new universal ECS components, `World::create_agent()` bootstrap seeding, AgentDef integration, profile-doc regeneration
**Deps**: archive/tickets/S151TESRELROU-001.md

## Problem

S151 needs two universal per-agent profiles (`TestimonyTrustProfile`, `RoutePreferenceProfile`) registered on `EntityKind::Agent`. These configure the derived `trust()` and `preference()` views on the runtime stores from ticket 002. Per `docs/spec-drafting-rules.md` Section 5, universal components must be wired through `component_schema.rs`, `World::create_agent()`, `AgentDef`, `spawn_agent()`, and the generated profile doc.

## Assumption Reassessment (2026-05-17)

1. `World::create_agent()` at `crates/worldwake-core/src/world.rs:184-260` seeds 20+ universal profiles with `::default()` (line 203: `insert_component_cognitive_profile`; similar calls for `metabolism_profile`, `tell_profile`, `perception_profile`, `exploration_profile`, `disposal_profile`, `preference_profile`, `risk_weight_profile`, etc.). New universal components MUST be seeded here.
2. `crates/worldwake-core/src/component_schema.rs` uses the `with_component_schema_entries!` macro at line 1-5. `MetabolismProfile` registration at lines 1382-1406 is the canonical 13-method entry template — predicate `|kind| kind == EntityKind::Agent`, strategy `txn_simple_set`.
3. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:572-654` has 31 fields, all `Option<_>` with explicit construction. 18 construction sites workspace-wide use no spread syntax — adding 2 new `Option<_>` fields with `#[serde(default)]` is **load-bearing per the spread-syntax rule** (18 > 15 → Medium effort baseline; combined with the registration + seeding + doc regen, this ticket lands at Large).
4. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:617-656` cluster uses the universal `.unwrap_or_default()` pattern (e.g., line 617: `txn.set_component_metabolism_profile(agent_id, agent_def.metabolism_profile.unwrap_or_default())?`).
5. `scripts/profile_docs.py` regenerates `docs/profiles/all-profiles.md` from Rust Profile structs (parses `crates/worldwake-cli/src/scenario/mod.rs` per the script header). Run `python3 scripts/profile_docs.py --write` after adding the new profiles.
6. `crates/worldwake-sim/src/world_txn.rs` does NOT exist in the codebase — no delta assertion to update (verified per Step 2 spot-check (e)).
7. Spec D5+D6+D11 at `archive/specs/S151-testimony-reliability-and-route-preferences.md:181-353`. The `trust()` and `preference()` impl bodies (deferred from ticket 002) land here alongside the profile types so the formula and parameters are reviewed together.

## Architecture Check

1. Per FND-22 + FND-22A: universal per-agent profile components enable concrete agent variation (per-agent topic weights for "officialist"/"gullible"/"empiricist" archetypes) without forcing a global parameter.
2. Per the "New Component on EntityKind::Agent" pattern: both components are core-resident (`crates/worldwake-core/src/`), classified universal with `Default` impls, with `AgentDef` `Option<_>` + `#[serde(default)]` so existing scenarios deserialize unchanged.
3. Per FND-26: `trust()` and `preference()` are derived computations over agent-local state — no cross-system command path. Profile defaults are the calibration substrate, not authoritative truth.
4. `#[serde(default)]` on AgentDef fields means existing `scenarios/*.ron` files do not break (grep confirmed 0 references to the new field names today).
5. Bootstrap seeding in `World::create_agent()` keeps the universal-component invariant intact: every known agent has both profiles present after creation, eliminating the need for `Option<&Component>` lookups in consumer code.

## Verified Layers

1. Component registration correctness → component-schema roundtrip test (insert, get, remove, has) per the `metabolism_profile_component_roundtrip_on_agent` pattern at `crates/worldwake-core/src/world.rs:5728`.
2. `create_agent` seeds defaults → extend `create_agent_attaches_belief_store_perception_profile_and_tell_profile` test at `world.rs:1334` to assert both new components are present with `Default` values.
3. AgentDef deserialization with omitted fields → unit test asserting RON `(agents: [(name: "x", location: "y", control: Human)])` deserializes with both new profile fields as `None`.
4. Scenario spawn applies profiles → unit test loading a scenario with explicit `testimony_trust_profile: Some(...)` and asserting the spawned agent has the expected profile component.

## Landed Changes

1. Added `TestimonyTrustProfile` and `RoutePreferenceProfile` as documented `worldwake-core` profile types with S151 defaults and focused default tests.
2. Added derived `TestimonyReliabilityEntry::trust(&TestimonyTrustProfile, TopicScope)` and `RoutePreferenceEntry::preference(&RoutePreferenceProfile, Tick)` helpers. Both return neutral `Permille(500)` below their profile minimums, derive signed evidence around neutral once enough observations exist, and clamp to `[0, 1000]`.
3. Registered both profiles in `component_schema.rs`, added the required macro-expansion imports in `delta.rs`, `world.rs`, and `component_tables.rs`, and updated component inventory/sample tests.
4. Seeded both universal profiles from `World::create_agent()` and updated the transaction delta expectation for agent creation.
5. Extended `AgentDef` with optional `testimony_trust_profile` and `route_preference_profile` fields, updated all explicit `AgentDef` literals, and wired `spawn_agent()` to apply authored values or defaults.
6. Extended scenario tests for omitted fields, default universal profile seeding, and authored S151 profile overrides.
7. Regenerated `docs/profiles/all-profiles.md` so both profiles appear in the universal-profile reference.

## Landed Files

- `crates/worldwake-core/src/testimony_trust_profile.rs` (new)
- `crates/worldwake-core/src/route_preference_profile.rs` (new)
- `crates/worldwake-core/src/testimony_reliability.rs` (modify — `trust()` impl)
- `crates/worldwake-core/src/route_preference.rs` (modify — `preference()` impl)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — two new entries)
- `crates/worldwake-core/src/component_tables.rs` (modify — macro import)
- `crates/worldwake-core/src/delta.rs` (modify — macro import, component inventory, samples)
- `crates/worldwake-core/src/world.rs` (modify — bootstrap seeding in `create_agent`)
- `crates/worldwake-core/src/world_txn.rs` (modify — create-agent delta expectation)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — two new AgentDef fields)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — two new set_component_* calls)
- `crates/worldwake-cli/src/bin/scenario_coverage.rs` (modify — authored-field accounting)
- `crates/worldwake-cli/src/display.rs` (modify — AgentDef construction sites)
- `crates/worldwake-cli/src/handlers/actions.rs` (modify — AgentDef construction sites)
- `crates/worldwake-cli/src/handlers/events.rs` (modify — AgentDef construction sites)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify — AgentDef construction sites)
- `crates/worldwake-cli/src/handlers/control.rs` (modify — AgentDef construction sites)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify — AgentDef construction sites)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify — AgentDef construction sites)
- `crates/worldwake-cli/src/scenario/lints.rs` (modify — AgentDef construction helper)
- `docs/profiles/all-profiles.md` (regenerated via `python3 scripts/profile_docs.py --write`)

## Out of Scope

- `GoalBeliefView` accessor methods — ticket 004
- Consumer reads in ranking / candidate emission / travel cost — tickets 007, 008
- Observation hook that writes to the runtime stores — ticket 006
- `SAVE_FORMAT_VERSION` bump — ticket 010

## Acceptance Result

### Passed Tests

1. Passed `TestimonyTrustProfile::default()` field-by-field coverage for the S151 defaults.
2. Passed `RoutePreferenceProfile::default()` field-by-field coverage for the S151 defaults.
3. Passed component roundtrip coverage for both registered components.
4. Passed `create_agent` bootstrap coverage proving both default components are seeded.
5. Passed AgentDef omitted-field coverage proving both new optional fields deserialize as `None`.
6. Passed derived-view coverage for neutral, positive, negative, clamped, and decayed `Permille` results.
7. Passed the existing workspace suite with `cargo test --workspace --quiet`.

### Invariants

1. Verified every agent created via `World::create_agent()` has both new universal components with `Default` values.
2. Verified derived trust/preference values clamp to valid `Permille` values.
3. Verified existing scenarios continue to deserialize and spawn unchanged when they omit the new profile fields.
4. Verified `docs/profiles/all-profiles.md` lists both new profiles after `python3 scripts/profile_docs.py --write`.

## Test Plan Result

### Added/Modified Tests

1. Added `crates/worldwake-core/src/testimony_trust_profile.rs#[cfg(test)]` default coverage.
2. Added `crates/worldwake-core/src/route_preference_profile.rs#[cfg(test)]` default coverage.
3. Added `crates/worldwake-core/src/testimony_reliability.rs#[cfg(test)]` trust formula coverage.
4. Added `crates/worldwake-core/src/route_preference.rs#[cfg(test)]` preference formula and decay coverage.
5. Extended `crates/worldwake-core/src/world.rs#[cfg(test)]` create-agent bootstrap coverage.
6. Added component roundtrip tests for both new components.
7. Extended `crates/worldwake-cli/src/scenario/types.rs#[cfg(test)]` omitted-field coverage.
8. Extended `crates/worldwake-cli/src/scenario/mod.rs#[cfg(test)]` default and authored-profile spawn coverage.

### Commands Run

1. Passed `cargo test -p worldwake-core testimony_trust_profile`.
2. Passed `cargo test -p worldwake-core route_preference_profile`.
3. Passed `cargo test -p worldwake-core trust_`.
4. Passed `cargo test -p worldwake-core preference_`.
5. Passed `cargo test -p worldwake-core create_agent`.
6. Passed `cargo test -p worldwake-core`.
7. Passed `cargo test -p worldwake-cli scenario`.
8. Passed `python3 scripts/profile_docs.py --write`; it regenerated `docs/profiles/all-profiles.md` and reported only pre-existing documentation gaps outside the S151 types.
9. Passed `cargo clippy --workspace --all-targets -- -D warnings`.
10. Passed `cargo test --workspace --quiet` after the final source edit.

## Outcome

Completed on 2026-05-17.

- Added `TestimonyTrustProfile` and `RoutePreferenceProfile` as universal agent profile components with documented defaults, component-schema entries, macro-expansion imports, crate-root exports, `World::create_agent()` seeding, transaction delta coverage, and profile-doc regeneration.
- Extended `AgentDef` and `spawn_agent()` so authored scenarios can override both profiles while omitted fields stay absent/defaulted.
- Added derived `trust()` and `preference()` views on the runtime-store entries; both stay neutral until their minimum-observation thresholds, then derive clamped `Permille` values from concrete observations/traversals. Route preference decays back to neutral after the configured observation window.
- Updated explicit `AgentDef` literals and scenario-coverage field accounting for the new authored fields.

## Deviations

- The route-preference decay implementation treats one day as 1440 ticks, matching the observer CLI's existing day comment. No separate global day constant existed on the live branch.
- The final broad proof used `cargo test --workspace --quiet` plus `cargo clippy --workspace --all-targets -- -D warnings` rather than `./scripts/verify.sh`; this covered the executable workspace test gate and CI-matching all-target clippy after the final source edit.

## Verification Result

- Passed `cargo test -p worldwake-core testimony_trust_profile`.
- Passed `cargo test -p worldwake-core route_preference_profile`.
- Passed `cargo test -p worldwake-core trust_`.
- Passed `cargo test -p worldwake-core preference_`.
- Passed `cargo test -p worldwake-core create_agent`.
- Passed `cargo test -p worldwake-core`.
- Passed `cargo test -p worldwake-cli scenario`.
- Passed `python3 scripts/profile_docs.py --write`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `cargo test --workspace --quiet`.
