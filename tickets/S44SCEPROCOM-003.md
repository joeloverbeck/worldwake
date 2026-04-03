# S44SCEPROCOM-003: AgentDef + spawn_agent — role-specific profiles + PatrolRouteDef

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AgentDef extended with 9 role-specific fields, PatrolRouteDef added, spawn_agent extended
**Deps**: S44SCEPROCOM-002

## Problem

9 role-specific profile components are registered on `EntityKind::Agent` but have no scenario path. Guard agents can't be given patrol routes, thief agents can't be configured with theft disposition, and justice enforcers can't be given accusation weights — all through the scenario system. This means the CLI evaluation scenario can't exercise theft, justice, patrol, pursuit, facility queuing, commodity valuation, or substitution features.

## Assumption Reassessment (2026-04-03)

1. `TheftDispositionProfile` at `crime.rs:18` — fields: `steal_duration_ticks: NonZeroU32`, `theft_motive_weight: Permille`, `witness_risk_penalty: Permille`. No Default impl. Derives `Serialize, Deserialize`. No EntityId fields. Confirmed.
2. `JusticeDispositionProfile` at `crime.rs:28` — fields: `accusation_motive_weight: Permille`, `fine_severity: Permille`. No Default. No EntityId. Confirmed.
3. `ViolationDispositionProfile` at `violation.rs:168` — fields: `investigation_duration_ticks: NonZeroU32`, `violation_memory_retention_ticks: u32`, `investigation_motive_weight: Permille`, `ownership_motive_bonus: Permille`. No Default. No EntityId. Confirmed.
4. `PatrolProfile` at `patrol.rs:17` — fields: `base_dwell_ticks: u32`, `dwell_vigilance_scale_ticks: u32`, `vigilance: Permille`, `route_adaptation_sensitivity: Permille`, `patrol_motive_weight: Permille`. No Default. No EntityId. Confirmed.
5. `PatrolRoute` at `patrol.rs:8` — fields: `assigned_places: Vec<EntityId>`, `current_index: usize`. Contains EntityId → needs PatrolRouteDef with string names. Confirmed.
6. `PursuitProfile` at `pursuit.rs:15` — fields: `min_location_confidence: Permille`, `max_pursuit_travel_ticks: NonZeroU32`. No Default. No EntityId. Confirmed.
7. `FacilityQueueDispositionProfile` at `facility_queue.rs:18` — fields: `queue_patience_ticks: Option<NonZeroU32>`. No Default. No EntityId. Confirmed.
8. `CommodityValuationProfile` at `valuation.rs:9` — fields: `recipe_opportunity_depth: NonZeroU8`, `recipe_place_horizon: u8`, `indirect_value_decay_per_step: Permille`. No Default. No EntityId. Confirmed.
9. `SubstitutePreferences` at `trade.rs:91` — fields: `preferences: BTreeMap<TradeCategory, Vec<CommodityKind>>`. No Default. No EntityId. Directly serializable via RON — no Def wrapper needed. Confirmed.
10. After ticket 002, AgentDef already has universal + already-defaulted profile fields. This ticket adds role-specific fields below those.
11. Existing `MerchandiseProfileDef` pattern at `types.rs:73-78` is the precedent for Def types with string-to-EntityId resolution.

## Architecture Check

1. Role-specific profiles use conditional `if let Some(...)` — same pattern as existing `CombatProfile`, `MerchandiseProfile`, `TradeDispositionProfile`. Consistent and proven.
2. `PatrolRouteDef` follows the `MerchandiseProfileDef` precedent — string names resolved to EntityIds during spawning. `current_index` is set to 0 (start of patrol route).
3. No backwards-compatibility shims. All new fields are `#[serde(default)]` — existing RON files work unchanged.

## Verification Layers

1. Role-specific profiles applied when present in RON -> focused test: spawn agent with TheftDispositionProfile, verify component exists
2. Role-specific profiles absent when not in RON -> focused test: spawn agent without TheftDispositionProfile, verify component absent
3. PatrolRouteDef resolves place names -> focused test: spawn agent with PatrolRouteDef referencing place names, verify PatrolRoute has correct EntityIds and current_index=0
4. PatrolRouteDef invalid name -> focused test: spawn with nonexistent place name, verify error
5. Existing scenarios still load -> `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit`

## What to Change

### 1. Add PatrolRouteDef to types.rs

In `crates/worldwake-cli/src/scenario/types.rs`, after `MerchandiseProfileDef`:

```rust
/// Scenario-specific patrol route using string place names instead of `EntityId`.
///
/// `PatrolRoute` in core contains `assigned_places: Vec<EntityId>`, which
/// cannot appear in a RON file before entities are spawned. This def uses
/// place name strings, resolved to `EntityId` during spawning.
#[derive(Clone, Debug, Deserialize)]
pub struct PatrolRouteDef {
    pub assigned_places: Vec<String>,
}
```

### 2. Add 9 role-specific fields to AgentDef

```rust
#[serde(default)]
pub theft_disposition: Option<TheftDispositionProfile>,
#[serde(default)]
pub justice_disposition: Option<JusticeDispositionProfile>,
#[serde(default)]
pub violation_disposition: Option<ViolationDispositionProfile>,
#[serde(default)]
pub patrol_profile: Option<PatrolProfile>,
#[serde(default)]
pub patrol_route: Option<PatrolRouteDef>,
#[serde(default)]
pub pursuit_profile: Option<PursuitProfile>,
#[serde(default)]
pub facility_queue_disposition: Option<FacilityQueueDispositionProfile>,
#[serde(default)]
pub commodity_valuation: Option<CommodityValuationProfile>,
#[serde(default)]
pub substitute_preferences: Option<SubstitutePreferences>,
```

Add necessary imports from `worldwake_core`.

### 3. Update spawn_agent() with conditional setters

In `crates/worldwake-cli/src/scenario/mod.rs`, after the existing role-specific profile block:

```rust
if let Some(ref profile) = agent_def.theft_disposition {
    txn.set_component_theft_disposition_profile(agent_id, profile.clone())?;
}
if let Some(ref profile) = agent_def.justice_disposition {
    txn.set_component_justice_disposition_profile(agent_id, profile.clone())?;
}
if let Some(ref profile) = agent_def.violation_disposition {
    txn.set_component_violation_disposition_profile(agent_id, profile.clone())?;
}
if let Some(ref profile) = agent_def.patrol_profile {
    txn.set_component_patrol_profile(agent_id, profile.clone())?;
}
if let Some(ref route_def) = agent_def.patrol_route {
    let assigned_places = route_def.assigned_places.iter()
        .map(|name| resolve_name(names, name, &format!("agent '{}' patrol route", agent_def.name)))
        .collect::<Result<Vec<_>, _>>()?;
    txn.set_component_patrol_route(agent_id, PatrolRoute { assigned_places, current_index: 0 })?;
}
if let Some(ref profile) = agent_def.pursuit_profile {
    txn.set_component_pursuit_profile(agent_id, profile.clone())?;
}
if let Some(ref profile) = agent_def.facility_queue_disposition {
    txn.set_component_facility_queue_disposition_profile(agent_id, profile.clone())?;
}
if let Some(ref profile) = agent_def.commodity_valuation {
    txn.set_component_commodity_valuation_profile(agent_id, profile.clone())?;
}
if let Some(ref prefs) = agent_def.substitute_preferences {
    txn.set_component_substitute_preferences(agent_id, prefs.clone())?;
}
```

### 4. Update types.rs tests

- `test_agent_def_default_optional_fields`: verify all 9 new fields default to None
- `test_scenario_def_deserialize_full`: add at least one role-specific profile and a `PatrolRouteDef` to the RON string
- Add test for PatrolRouteDef deserialization specifically

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify) — add PatrolRouteDef, 9 fields, imports, tests
- `crates/worldwake-cli/src/scenario/mod.rs` (modify) — add 9 conditional setters with PatrolRoute name resolution

## Out of Scope

- Universal profiles (ticket 002)
- Runtime enforcement / expect() (ticket 004)
- Documentation (ticket 005)
- Adding Default impls for role-specific profiles (not needed — they're conditional)
- Scenario RON updates (separate, after all tickets)

## Acceptance Criteria

### Tests That Must Pass

1. Agent spawned with `theft_disposition` in RON has TheftDispositionProfile component
2. Agent spawned without `theft_disposition` does NOT have TheftDispositionProfile
3. Agent with `patrol_route: (assigned_places: ["Place A", "Place B"])` gets PatrolRoute with correct EntityIds and `current_index: 0`
4. Agent with `patrol_route` referencing nonexistent place produces ScenarioError
5. `test_agent_def_default_optional_fields` — all 9 new fields are None
6. Existing scenario loads: `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit`
7. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. All new fields are `#[serde(default)]` `Option<T>` — existing RON files work unchanged
2. Role-specific profiles are only set when explicitly present in RON
3. PatrolRoute `current_index` is always 0 on spawn (no runtime state in scenario definition)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs` — update default-fields test, add PatrolRouteDef deser test
2. `crates/worldwake-cli/src/scenario/types.rs` — update full-deser test with role-specific profile
3. `crates/worldwake-cli/src/scenario/mod.rs` — add spawn test for role-specific profiles and PatrolRoute resolution

### Commands

1. `cargo test -p worldwake-cli -- scenario`
2. `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
