# S44SCEPROCOM-002: AgentDef + spawn_agent — universal and already-defaulted profiles

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AgentDef extended with 10 fields, spawn_agent rewritten for universal profiles
**Deps**: S44SCEPROCOM-001

## Problem

7 universal profiles (`PerceptionProfile`, `TellProfile`, `ReasoningProfile`, `EpistemicDispositionProfile`, `IntentionDispositionProfile`, `CommunicationProfile`, `PreferenceProfile`) are not applied to scenario-spawned agents. Additionally, 3 already-defaulted profiles (`DriveThresholds`, `MetabolismProfile`, `CarryCapacity`) are applied but not overridable via RON. This means agents silently lack core capabilities (perception, social transmission, reasoning) and all agents share identical urgency thresholds and metabolism — undermining Principle 22 (Agent Diversity).

## Assumption Reassessment (2026-04-03)

1. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:52-66` currently has 5 optional profile fields: `needs`, `combat_profile`, `utility_profile`, `merchandise_profile`, `trade_disposition`. Confirmed.
2. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:259-314` currently makes 9 `set_component_*` calls. Universal profiles have zero calls. Confirmed.
3. After ticket 001, all 7 universal profiles have `Default` impls, enabling `unwrap_or_default()`.
4. `DriveThresholds` at `drives.rs:58` — `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Has Default impl. Currently set unconditionally at `mod.rs:277`.
5. `MetabolismProfile` at `needs.rs:72` — has Default impl. Currently set unconditionally at `mod.rs:278`.
6. `CarryCapacity` at `production.rs:67` — `pub struct CarryCapacity(pub LoadUnits)`. Currently set unconditionally at `mod.rs:279` via `DEFAULT_AGENT_CARRY_CAPACITY`.
7. All profile types derive `Deserialize` — RON deserialization works out of the box via `#[serde(default)]` `Option<T>` fields.
8. Import additions needed in `types.rs`: `PerceptionProfile`, `TellProfile`, `ReasoningProfile`, `EpistemicDispositionProfile`, `IntentionDispositionProfile`, `CommunicationProfile`, `PreferenceProfile`, `DriveThresholds`, `MetabolismProfile`, `CarryCapacity`. Live code has all of them re-exported from `worldwake_core`, so no deep module-path imports are required. Correction applied: keep the import surface at the root re-exports. Why safe: this is a factual import-shape confirmation, not an architecture change.
9. Ticket says the only code fallout is `types.rs` plus the main spawn path. Live code also has multiple local `AgentDef` test constructors across `crates/worldwake-cli/src/scenario/mod.rs`, `display.rs`, and the CLI handler test modules that must widen with the schema. Correction applied: treat all direct `AgentDef` constructor fallout inside `worldwake-cli` as owned by this ticket. Why safe: this is direct compile-surface fallout from the widened scenario schema.
10. The `with_component_schema_entries!` macro expansion sites (`delta.rs`, `world_txn.rs`, `component_tables.rs`, `world.rs`) are unrelated to this CLI-only ticket — no component-registration fallout applies here.

## Architecture Check

1. Universal profiles use `unwrap_or_default()` — always present, RON can override. This is the same pattern as `HomeostaticNeeds` (already in AgentDef). Clean, consistent, no special cases.
2. Already-defaulted profiles switch from unconditional `Default::default()` to `unwrap_or_default()` — same end result when RON omits the field, but now overridable. No behavior change for existing scenarios.
3. No backwards-compatibility shims. Existing RON files continue to work because all new fields are `#[serde(default)]`.

## Verification Layers

1. Universal profiles always present after spawn -> focused unit test: spawn agent without RON overrides, verify all 7 profiles exist with default values
2. RON override works -> focused unit test: spawn agent with explicit PerceptionProfile in RON, verify non-default values
3. Already-defaulted profiles overridable -> focused unit test: spawn agent with explicit DriveThresholds, verify non-default values
4. Existing scenarios still load -> `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit`
5. Single-crate change (worldwake-cli) — no cross-system verification needed, but local scenario spawn tests and the bundled CLI scenario load path both need proof because this widens the RON schema and spawn contract together

## What to Change

### 1. Add 10 fields to AgentDef

In `crates/worldwake-cli/src/scenario/types.rs`, add to `AgentDef`:

**Universal profiles:**
```rust
#[serde(default)]
pub perception_profile: Option<PerceptionProfile>,
#[serde(default)]
pub tell_profile: Option<TellProfile>,
#[serde(default)]
pub reasoning_profile: Option<ReasoningProfile>,
#[serde(default)]
pub epistemic_disposition: Option<EpistemicDispositionProfile>,
#[serde(default)]
pub intention_disposition: Option<IntentionDispositionProfile>,
#[serde(default)]
pub communication_profile: Option<CommunicationProfile>,
#[serde(default)]
pub preference_profile: Option<PreferenceProfile>,
```

**Already-defaulted profiles (now overridable):**
```rust
#[serde(default)]
pub drive_thresholds: Option<DriveThresholds>,
#[serde(default)]
pub metabolism_profile: Option<MetabolismProfile>,
#[serde(default)]
pub carry_capacity: Option<CarryCapacity>,
```

Add necessary imports from `worldwake_core`.

### 2. Update spawn_agent() for universal profiles

In `crates/worldwake-cli/src/scenario/mod.rs`, after the existing `set_component_carry_capacity` call, add:

```rust
// Universal profiles — every agent gets these
let perception = agent_def.perception_profile.unwrap_or_default();
txn.set_component_perception_profile(agent_id, perception)?;

let tell = agent_def.tell_profile.unwrap_or_default();
txn.set_component_tell_profile(agent_id, tell)?;

let reasoning = agent_def.reasoning_profile.unwrap_or_default();
txn.set_component_reasoning_profile(agent_id, reasoning)?;

let epistemic = agent_def.epistemic_disposition.unwrap_or_default();
txn.set_component_epistemic_disposition_profile(agent_id, epistemic)?;

let intention = agent_def.intention_disposition.unwrap_or_default();
txn.set_component_intention_disposition_profile(agent_id, intention)?;

let communication = agent_def.communication_profile.unwrap_or_default();
txn.set_component_communication_profile(agent_id, communication)?;

let preference = agent_def.preference_profile.unwrap_or_default();
txn.set_component_preference_profile(agent_id, preference)?;
```

### 3. Update spawn_agent() for already-defaulted profiles

Change the existing unconditional lines:

```rust
// Before:
txn.set_component_drive_thresholds(agent_id, DriveThresholds::default())?;
txn.set_component_metabolism_profile(agent_id, MetabolismProfile::default())?;
txn.set_component_carry_capacity(agent_id, DEFAULT_AGENT_CARRY_CAPACITY)?;

// After:
let thresholds = agent_def.drive_thresholds.unwrap_or_default();
txn.set_component_drive_thresholds(agent_id, thresholds)?;

let metabolism = agent_def.metabolism_profile.unwrap_or_default();
txn.set_component_metabolism_profile(agent_id, metabolism)?;

let carry = agent_def.carry_capacity.unwrap_or(DEFAULT_AGENT_CARRY_CAPACITY);
txn.set_component_carry_capacity(agent_id, carry)?;
```

### 4. Update types.rs tests

Update `test_agent_def_default_optional_fields` to verify all new fields default to `None`. Update `test_scenario_def_deserialize_full` to include at least one new profile in the RON string and verify it deserializes correctly.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify) — add 10 fields, add imports, update tests
- `crates/worldwake-cli/src/scenario/mod.rs` (modify) — add 7 universal profile setters, change 3 already-defaulted to override-or-default, update local `AgentDef` test construction fallout
- `crates/worldwake-cli/src/display.rs` (modify) — widen local `AgentDef` test constructors for the new optional fields
- `crates/worldwake-cli/src/handlers/actions.rs` (modify) — widen local `AgentDef` test constructors for the new optional fields
- `crates/worldwake-cli/src/handlers/control.rs` (modify) — widen local `AgentDef` test constructors for the new optional fields
- `crates/worldwake-cli/src/handlers/events.rs` (modify) — widen local `AgentDef` test constructors for the new optional fields
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify) — widen local `AgentDef` test constructors for the new optional fields
- `crates/worldwake-cli/src/handlers/tick.rs` (modify) — widen local `AgentDef` test constructors for the new optional fields
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify) — widen local `AgentDef` test constructors for the new optional fields

## Out of Scope

- Role-specific profiles (ticket 003)
- PatrolRouteDef (ticket 003)
- Runtime enforcement / expect() conversion (ticket 004)
- Documentation updates (ticket 005)
- Scenario RON updates (separate, after all tickets)

## Acceptance Criteria

### Tests That Must Pass

1. Agent spawned without any profile overrides in RON has all 7 universal profiles with default values
2. Agent spawned with explicit `perception_profile` in RON has non-default PerceptionProfile values
3. Agent spawned with explicit `drive_thresholds` in RON has non-default DriveThresholds values
4. `test_agent_def_default_optional_fields` — all new fields default to None
5. Existing scenario loads: `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit`
6. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. All new AgentDef fields are `#[serde(default)]` `Option<T>` — existing RON files work unchanged
2. Universal profiles are unconditionally present on all scenario-spawned agents
3. Already-defaulted profiles preserve existing default values when RON omits them

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs` — update `test_agent_def_default_optional_fields` for 10 new None fields
2. `crates/worldwake-cli/src/scenario/types.rs` — update `test_scenario_def_deserialize_full` with at least one new profile
3. `crates/worldwake-cli/src/scenario/mod.rs` — add spawn test verifying universal profiles are present after spawn
4. Existing CLI unit tests that construct `AgentDef` directly — widen those test-only constructors so the crate still compiles against the expanded schema

### Commands

1. `cargo test -p worldwake-cli scenario::types::tests::test_agent_def_default_optional_fields`
2. `cargo test -p worldwake-cli scenario::types::tests::test_scenario_def_deserialize_full`
3. `cargo test -p worldwake-cli scenario::tests::test_spawn_agents_receive_default_universal_profiles`
4. `cargo test -p worldwake-cli scenario::tests::test_spawn_agent_with_profile_overrides`
5. `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit`
6. `cargo test -p worldwake-cli`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`

## Outcome

- **Completed**: 2026-04-03
- Extended `AgentDef` with the 7 universal profile fields plus overridable `drive_thresholds`, `metabolism_profile`, and `carry_capacity`.
- Updated `spawn_agent()` so scenario-spawned agents now always receive the universal profiles via `unwrap_or_default()`, while the already-defaulted profiles now use scenario override-or-default behavior.
- Added focused scenario proofs for default universal-profile application and explicit profile overrides.
- Deviation from the initial ticket shape: widening `AgentDef` also required updating direct `AgentDef` constructors in other `worldwake-cli` test modules, not just `scenario/types.rs` and `scenario/mod.rs`.
- Verification:
  - `cargo test -p worldwake-cli scenario::types::tests::test_agent_def_default_optional_fields`
  - `cargo test -p worldwake-cli scenario::types::tests::test_scenario_def_deserialize_full`
  - `cargo test -p worldwake-cli scenario::tests::test_spawn_agents_receive_default_universal_profiles`
  - `cargo test -p worldwake-cli scenario::tests::test_spawn_agent_with_profile_overrides`
  - `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit`
  - `cargo test -p worldwake-cli`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
