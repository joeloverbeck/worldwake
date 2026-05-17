# S151TESRELROU-003: Universal profiles + component registration + bootstrap + scenario integration

**Status**: PENDING
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
7. Spec D5+D6+D11 at `specs/S151-testimony-reliability-and-route-preferences.md:181-353`. The `trust()` and `preference()` impl bodies (deferred from ticket 002) land here alongside the profile types so the formula and parameters are reviewed together.

## Architecture Check

1. Per FND-22 + FND-22A: universal per-agent profile components enable concrete agent variation (per-agent topic weights for "officialist"/"gullible"/"empiricist" archetypes) without forcing a global parameter.
2. Per the "New Component on EntityKind::Agent" pattern: both components are core-resident (`crates/worldwake-core/src/`), classified universal with `Default` impls, with `AgentDef` `Option<_>` + `#[serde(default)]` so existing scenarios deserialize unchanged.
3. Per FND-26: `trust()` and `preference()` are derived computations over agent-local state — no cross-system command path. Profile defaults are the calibration substrate, not authoritative truth.
4. `#[serde(default)]` on AgentDef fields means existing `scenarios/*.ron` files do not break (grep confirmed 0 references to the new field names today).
5. Bootstrap seeding in `World::create_agent()` keeps the universal-component invariant intact: every known agent has both profiles present after creation, eliminating the need for `Option<&Component>` lookups in consumer code.

## Verification Layers

1. Component registration correctness → component-schema roundtrip test (insert, get, remove, has) per the `metabolism_profile_component_roundtrip_on_agent` pattern at `crates/worldwake-core/src/world.rs:5728`.
2. `create_agent` seeds defaults → extend `create_agent_attaches_belief_store_perception_profile_and_tell_profile` test at `world.rs:1334` to assert both new components are present with `Default` values.
3. AgentDef deserialization with omitted fields → unit test asserting RON `(agents: [(name: "x", location: "y", control: Human)])` deserializes with both new profile fields as `None`.
4. Scenario spawn applies profiles → unit test loading a scenario with explicit `testimony_trust_profile: Some(...)` and asserting the spawned agent has the expected profile component.

## What to Change

### 1. Add `crates/worldwake-core/src/testimony_trust_profile.rs` (new)

```rust
use serde::{Deserialize, Serialize};
use crate::numerics::Permille;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestimonyTrustProfile {
    pub confirmation_weight: Permille,
    pub refutation_penalty: Permille,
    pub stale_decay_per_tick: Permille,
    pub contradicted_penalty: Permille,
    pub minimum_observations: u8,
    pub trust_threshold: Permille,
    pub topic_weight_route_hazard: Permille,
    pub topic_weight_resource_availability: Permille,
    pub topic_weight_office_holder: Permille,
    pub topic_weight_accusation_credibility: Permille,
    pub topic_weight_bounty_validity: Permille,
    pub topic_weight_price_level: Permille,
    pub topic_weight_entity_whereabouts: Permille,
    pub topic_weight_general_fact: Permille,
}

impl Default for TestimonyTrustProfile {
    fn default() -> Self {
        Self {
            confirmation_weight: Permille::new(250),
            refutation_penalty: Permille::new(400),
            stale_decay_per_tick: Permille::new(1),
            contradicted_penalty: Permille::new(350),
            minimum_observations: 2,
            trust_threshold: Permille::new(400),
            topic_weight_route_hazard: Permille::new(500),
            topic_weight_resource_availability: Permille::new(500),
            topic_weight_office_holder: Permille::new(500),
            topic_weight_accusation_credibility: Permille::new(500),
            topic_weight_bounty_validity: Permille::new(500),
            topic_weight_price_level: Permille::new(500),
            topic_weight_entity_whereabouts: Permille::new(500),
            topic_weight_general_fact: Permille::new(500),
        }
    }
}
```

### 2. Add `crates/worldwake-core/src/route_preference_profile.rs` (new)

```rust
use serde::{Deserialize, Serialize};
use crate::numerics::Permille;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutePreferenceProfile {
    pub safe_traversal_weight: Permille,
    pub dangerous_traversal_penalty: Permille,
    pub days_to_decay_observations: u32,
    pub minimum_traversals: u8,
}

impl Default for RoutePreferenceProfile {
    fn default() -> Self {
        Self {
            safe_traversal_weight: Permille::new(200),
            dangerous_traversal_penalty: Permille::new(600),
            days_to_decay_observations: 30,
            minimum_traversals: 2,
        }
    }
}
```

### 3. Add derived-view impls (deferred from ticket 002)

In `crates/worldwake-core/src/testimony_reliability.rs`, add:

```rust
impl TestimonyReliabilityEntry {
    pub fn trust(&self, profile: &TestimonyTrustProfile, topic: TopicScope) -> Permille {
        let total = self.direct_confirmations + self.direct_refutations + self.stale_claims + self.contradicted_claims;
        if u32::from(profile.minimum_observations) > total { return Permille::new(500); /* neutral */ }
        // Trust = confirmations·confirmation_weight − refutations·refutation_penalty − stale·stale_decay − contradicted·contradicted_penalty
        // Apply topic_weight_<topic> as multiplicative modifier; clamp to [0, 1000].
        // Concrete formula determined during implementation; document the closed form in code comments.
        todo!()
    }
}
```

In `crates/worldwake-core/src/route_preference.rs`, add:

```rust
impl RoutePreferenceEntry {
    pub fn preference(&self, profile: &RoutePreferenceProfile, current_tick: Tick) -> Permille {
        let total = self.safe_traversals + self.dangerous_traversals;
        if u32::from(profile.minimum_traversals) > total { return Permille::new(500); /* neutral */ }
        // Apply days_to_decay_observations to decay (current_tick - last_*_tick) toward neutral
        // preference = safe·safe_traversal_weight − dangerous·dangerous_traversal_penalty, clamped [0, 1000]
        todo!()
    }
}
```

### 4. Register components in `component_schema.rs`

Add two new entries in `with_component_schema_entries!` following the `MetabolismProfile` precedent at lines 1382-1406:

```rust
{
    testimony_trust_profiles,
    TestimonyTrustProfile,
    insert_testimony_trust_profile,
    /* ... 12 more standard method names ... */
    "TestimonyTrustProfile",
    |kind| kind == EntityKind::Agent,
    TestimonyTrustProfile,
    crate::TestimonyTrustProfile,
    set_component_testimony_trust_profile,
    clear_component_testimony_trust_profile,
    txn_simple_set
},
{
    route_preference_profiles,
    RoutePreferenceProfile,
    insert_route_preference_profile,
    /* ... */
    "RoutePreferenceProfile",
    |kind| kind == EntityKind::Agent,
    RoutePreferenceProfile,
    crate::RoutePreferenceProfile,
    set_component_route_preference_profile,
    clear_component_route_preference_profile,
    txn_simple_set
},
```

Per `tickets/README.md` check #13, verify that all macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) import the new types — the macro generates code using bare type names that must be in scope at each expansion site.

### 5. Seed defaults in `World::create_agent()` (`crates/worldwake-core/src/world.rs:184-260`)

Add two `insert_component_*` calls alongside the existing 20+ universal-profile seeds (e.g., immediately after `insert_component_cognitive_profile` at line 203):

```rust
world.insert_component_testimony_trust_profile(entity, TestimonyTrustProfile::default())?;
world.insert_component_route_preference_profile(entity, RoutePreferenceProfile::default())?;
```

### 6. Extend `AgentDef` (`crates/worldwake-cli/src/scenario/types.rs:572-654`)

Add two `Option<_>` fields with `#[serde(default)]` to the trailing universal-profile cluster (after `substitute_preferences`):

```rust
#[serde(default)]
pub testimony_trust_profile: Option<TestimonyTrustProfile>,
#[serde(default)]
pub route_preference_profile: Option<RoutePreferenceProfile>,
```

Update all 18 explicit construction sites to add `testimony_trust_profile: None, route_preference_profile: None,`:

- `crates/worldwake-cli/src/display.rs:833, 877`
- `crates/worldwake-cli/src/handlers/inspect.rs:677, 727`
- `crates/worldwake-cli/src/handlers/control.rs:167, 211, 458, 502`
- `crates/worldwake-cli/src/handlers/world_overview.rs:231, 275` (plus any sibling sites in the same file)
- Remaining sites in `crates/worldwake-cli/src/handlers/*` and `display.rs` (full enumeration via `rg -n '^\s*AgentDef\s*\{$' crates/`)

### 7. Apply profiles in `spawn_agent()` (`crates/worldwake-cli/src/scenario/mod.rs:617-656`)

```rust
txn.set_component_testimony_trust_profile(agent_id, agent_def.testimony_trust_profile.unwrap_or_default())?;
txn.set_component_route_preference_profile(agent_id, agent_def.route_preference_profile.unwrap_or_default())?;
```

### 8. Re-export from `crates/worldwake-core/src/lib.rs`

```rust
pub mod testimony_trust_profile;
pub mod route_preference_profile;
pub use testimony_trust_profile::TestimonyTrustProfile;
pub use route_preference_profile::RoutePreferenceProfile;
```

### 9. Regenerate `docs/profiles/all-profiles.md`

```sh
python3 scripts/profile_docs.py --write
```

Commit the regenerated `docs/profiles/all-profiles.md`.

## Files to Touch

- `crates/worldwake-core/src/testimony_trust_profile.rs` (new)
- `crates/worldwake-core/src/route_preference_profile.rs` (new)
- `crates/worldwake-core/src/testimony_reliability.rs` (modify — `trust()` impl)
- `crates/worldwake-core/src/route_preference.rs` (modify — `preference()` impl)
- `crates/worldwake-core/src/lib.rs` (modify — re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — two new entries)
- `crates/worldwake-core/src/world.rs` (modify — bootstrap seeding in `create_agent`)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — two new AgentDef fields)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — two new set_component_* calls)
- `crates/worldwake-cli/src/display.rs` (modify — AgentDef construction sites)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify — AgentDef construction sites)
- `crates/worldwake-cli/src/handlers/control.rs` (modify — AgentDef construction sites)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify — AgentDef construction sites)
- Any other `crates/worldwake-cli/src/handlers/*.rs` with AgentDef construction (enumerate via `rg`)
- `docs/profiles/all-profiles.md` (regenerate via `python3 scripts/profile_docs.py --write`)

## Out of Scope

- `GoalBeliefView` accessor methods — ticket 004
- Consumer reads in ranking / candidate emission / travel cost — tickets 007, 008
- Observation hook that writes to the runtime stores — ticket 006
- `SAVE_FORMAT_VERSION` bump — ticket 010

## Acceptance Criteria

### Tests That Must Pass

1. `TestimonyTrustProfile::default()` matches the spec-documented defaults (confirmation_weight=250, refutation_penalty=400, etc.).
2. `RoutePreferenceProfile::default()` matches spec defaults (safe_traversal_weight=200, etc.).
3. Component roundtrip (insert/get/remove/has) works for both new components per the existing `metabolism_profile_component_roundtrip_on_agent` pattern.
4. `create_agent` seeds both new components on every new agent.
5. AgentDef deserialization with omitted profile fields produces `None` (RON: `(agents: [(name: "x", location: "y", control: Human)])` works).
6. `entry.trust(&profile, topic)` and `entry.preference(&profile, current_tick)` produce valid `Permille` values in `[0, 1000]` for any input combination.
7. Existing suite: `cargo test --workspace`.

### Invariants

1. Every agent created via `World::create_agent()` has both new universal components with `Default` values (per FND-22A's universal-component contract).
2. `Permille` values never exceed 1000 — formulas clamp to `[0, 1000]`.
3. Existing scenarios continue to deserialize and spawn unchanged (zero behavior delta for agents that don't set the new profiles).
4. `docs/profiles/all-profiles.md` lists both new profiles after regeneration (verified by re-running `python3 scripts/profile_docs.py --write` and checking diff is empty).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/testimony_trust_profile.rs#[cfg(test)]` — `Default` field-by-field assertion.
2. `crates/worldwake-core/src/route_preference_profile.rs#[cfg(test)]` — `Default` assertion.
3. `crates/worldwake-core/src/testimony_reliability.rs#[cfg(test)]` — extend ticket 002's tests with `trust()` formula coverage.
4. `crates/worldwake-core/src/route_preference.rs#[cfg(test)]` — extend with `preference()` formula + decay coverage.
5. `crates/worldwake-core/src/world.rs#[cfg(test)]` — extend `create_agent_attaches_belief_store_perception_profile_and_tell_profile` at line 1334 to include the two new components.
6. Component roundtrip tests for both new components (mirror `metabolism_profile_component_roundtrip_on_agent` at `world.rs:5728`).

### Commands

1. `cargo test -p worldwake-core testimony_trust_profile route_preference_profile`
2. `cargo test -p worldwake-core create_agent`
3. `cargo test -p worldwake-cli scenario`
4. `python3 scripts/profile_docs.py --write` (verify diff)
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `./scripts/verify.sh`
