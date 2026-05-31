# S177WATSRCQUA-003: `WaterToleranceProfile` universal per-agent component

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-core` (new module, component_schema registration, `create_agent` seeding), `worldwake-sim/belief_view` (new accessor + RuntimeBeliefView impl), `worldwake-cli/scenario` (AgentDef field + spawn_agent setter), `worldwake-sim/save_load` (SAVE_FORMAT_VERSION bump), `docs/profiles` (regeneration)
**Deps**: S177WATSRCQUA-001

## Problem

The spec's D5 deliverable adds per-agent tolerance to water quality — a hardy agent suffers less from muddy water (higher thirst-relief factor + lower dirtiness penalty) than a fragile agent. Per the FOUNDATIONS-aligned Q3 resolution from `/reassess-spec`, this lives on a new universal `WaterToleranceProfile` component (per-agent), not on `MetabolismProfile` or `CommodityConsumableProfile`. Without this profile, every agent would experience identical water-quality consequences, collapsing the FND-22 agent-diversity emergent surface that the spec's headline scenario depends on.

## Assumption Reassessment (2026-05-31)

1. The universal-profile contract per `docs/spec-drafting-rules.md` Section 5: register in `component_schema.rs`, add `Option<...>` field to `AgentDef`, set via `unwrap_or_default()` in `spawn_agent()`, `expect()` at runtime. Existing precedents: `MetabolismProfile`, `PerceptionProfile`, `CognitiveProfile`, `PreferenceProfile`, `MemoryCapacityProfile`, `UtilityProfile`, `CommunicationProfile` — 22+ profiles in `worldwake-core` seeded by `world.rs::create_agent` at lines 185-280+. `WaterToleranceProfile` is added to this seeding list.
2. `world.rs::create_agent` seeds universal profiles via `world.insert_component_*` calls. New profile is inserted at the end of the seeding block. The function returns a `Result<EntityId, WorldError>`, propagating insertion errors.
3. `world_txn.rs::create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` at `crates/worldwake-core/src/world_txn.rs:2490` asserts which component-creation deltas occur when `create_agent` runs. Adding `WaterToleranceProfile` to the seeding requires updating this test's delta count and the asserted component list. The test currently asserts a specific set of components — locate the assertion block and add the new component.
4. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:585-675` follows the `metabolism_profile: Option<MetabolismProfile>` pattern at line 640. New field `water_tolerance_profile: Option<WaterToleranceProfile>` follows the same shape with `#[serde(default)]`.
5. `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:949-1048` seeds `metabolism_profile` at lines 978-979 via `unwrap_or_default()` + `set_component_metabolism_profile`. New `WaterToleranceProfile` insertion follows the same pattern.
6. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:322+` carries per-agent accessors. The new accessor `water_tolerance_profile(agent: EntityId) -> Option<WaterToleranceProfile>` follows the existing per-profile accessor pattern (e.g., `metabolism_profile`, `perception_profile`). `RuntimeBeliefView` impl in `crates/worldwake-sim/src/per_agent_belief_view.rs` provides the backing implementation — no `impl_goal_belief_view!` macro is used in the codebase per reassessment finding I5; the impl pattern is individual impl blocks.
7. `scripts/profile_docs.py` regenerates `docs/profiles/all-profiles.md`. Adding a new profile to core requires running this script as part of the ticket.
8. Default impl: `WaterToleranceProfile::default()` provides baseline tolerance — `Clean: 1000‰ relief / 0‰ dirtiness`, `Stale: 700‰ relief / 80‰ dirtiness`, `Muddy: 450‰ relief / 200‰ dirtiness` (per spec D5). These values are spec-authored; the default is the "average agent" tolerance. Scenario-authored agents override per profile diversity.
9. `WaterToleranceProfile` stores `BTreeMap<WaterQuality, Permille>` (not `HashMap`) per the CLAUDE.md determinism invariant (`BTreeMap`/`BTreeSet` only in authoritative state).
10. The component is referenced by ticket 004 (source_composite quality factor reads tolerance), ticket 005 (Drink reads tolerance to scale relief + add dirtiness). Adding it before those tickets unblocks them.
11. Adjacent contradictions: none. This is a clean additive ticket — no behavior modified, no existing tests rewritten, only the universal-profile seeding test gets one assertion added.

## Architecture Check

1. Per the Q3 FOUNDATIONS-delegated resolution: `WaterToleranceProfile` (vs. extending `MetabolismProfile` or `CommodityConsumableProfile`) follows FND-22 (per-agent variation is the emergent surface — diversity drives the scarcity ↔ quality tradeoff), FND-5 (a dedicated carrier-of-consequence — agents' bodies' tolerance to water quality), and FND-29A (inspectability — "why this water choice" lands in one queryable component). The 35 existing profile components in core set the pattern "distinct domain → distinct profile."
2. Universal-profile contract (vs. role-specific) — every agent has a body that responds to water quality; making it universal with a sensible `Default` is the FND-26 cohesion choice and avoids `Option<&WaterToleranceProfile>` lookups at every read site.
3. `BTreeMap<WaterQuality, Permille>` (vs. paired fields per variant or a fixed array indexed by enum discriminant) follows the determinism invariant and is grep-friendly. Iteration is deterministic.

## Verification Layers

1. Component registration compiles and survives world-txn round-trip — `world_txn.rs` test update validates this.
2. Universal seeding by `create_agent` — focused test asserts the component is present immediately after `create_agent` returns.
3. Scenario authoring round-trip — focused test loads a RON with explicit `water_tolerance_profile:` overrides and confirms the override flows through `spawn_agent` to the agent's stored component.
4. Default tolerance values — focused test on `WaterToleranceProfile::default()` confirms the per-spec tier values (Clean=1000/0, Stale=700/80, Muddy=450/200).
5. Belief-view accessor returns `Some` for known agents (FND-14B self-authoritative read), `None` for unknown agents.

## What to Change

### 1. New module `crates/worldwake-core/src/water_tolerance_profile.rs`

```rust
//! Per-agent tolerance to water quality. Universal profile component.

use crate::{Component, Permille, WaterQuality};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WaterToleranceProfile {
    /// Per-quality relief multiplier applied to `CommodityConsumableProfile.thirst_relief_per_unit`.
    /// Clean = 1000‰ (neutral); Stale, Muddy < 1000‰.
    #[serde(default)]
    pub thirst_relief_factor: BTreeMap<WaterQuality, Permille>,
    /// Per-quality dirtiness penalty added to `HomeostaticNeeds::dirtiness` on Drink commit.
    /// Clean = 0‰; Stale, Muddy > 0‰.
    #[serde(default)]
    pub dirtiness_penalty: BTreeMap<WaterQuality, Permille>,
}

impl Component for WaterToleranceProfile {}

impl Default for WaterToleranceProfile {
    fn default() -> Self {
        Self {
            thirst_relief_factor: BTreeMap::from([
                (WaterQuality::Clean, Permille::new(1000).unwrap()),
                (WaterQuality::Stale, Permille::new(700).unwrap()),
                (WaterQuality::Muddy, Permille::new(450).unwrap()),
            ]),
            dirtiness_penalty: BTreeMap::from([
                (WaterQuality::Clean, Permille::new(0).unwrap()),
                (WaterQuality::Stale, Permille::new(80).unwrap()),
                (WaterQuality::Muddy, Permille::new(200).unwrap()),
            ]),
        }
    }
}

impl WaterToleranceProfile {
    #[must_use]
    pub fn thirst_relief_factor(&self, quality: WaterQuality) -> Permille {
        self.thirst_relief_factor
            .get(&quality)
            .copied()
            .unwrap_or_else(|| Permille::new(1000).unwrap())
    }

    #[must_use]
    pub fn dirtiness_penalty(&self, quality: WaterQuality) -> Permille {
        self.dirtiness_penalty
            .get(&quality)
            .copied()
            .unwrap_or_else(|| Permille::new(0).unwrap())
    }
}
```

Re-export from `crates/worldwake-core/src/lib.rs`.

### 2. Register in `component_schema.rs`

`crates/worldwake-core/src/component_schema.rs`: add `WaterToleranceProfile` to the `with_component_schema_entries!` macro with `EntityKind::Agent` filter, following the precedent of `MetabolismProfile` / `PerceptionProfile` / `CognitiveProfile`. This generates the standard accessor set (`insert_component_water_tolerance_profile`, `get_component_water_tolerance_profile`, `entities_with_water_tolerance_profile`, etc.).

Per `tickets/README.md` check #13, verify imports in `delta.rs`, `world.rs`, and `component_tables.rs` — the macro generates code using bare type names that must be in scope at each expansion site.

### 3. Seed in `world.rs::create_agent`

`crates/worldwake-core/src/world.rs:185-280+`: add an `insert_component_water_tolerance_profile(entity, WaterToleranceProfile::default())?;` line at the end of the seeding block, following the pattern of other universal profiles.

### 4. Update `world_txn.rs` delta assertion test

`crates/worldwake-core/src/world_txn.rs:2490` (`create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through`): increment the asserted component-creation count by 1, and add `WaterToleranceProfile` to the expected-components list.

### 5. Add field to `AgentDef`

`crates/worldwake-cli/src/scenario/types.rs:585-675` (after the existing `metabolism_profile: Option<MetabolismProfile>` at line 640):

```rust
#[serde(default)]
pub water_tolerance_profile: Option<WaterToleranceProfile>,
```

### 6. Insert in `spawn_agent()`

`crates/worldwake-cli/src/scenario/mod.rs:949-1048`: after the `metabolism_profile` insertion at lines 978-979, add:

```rust
let water_tolerance = agent_def.water_tolerance_profile.unwrap_or_default();
txn.set_component_water_tolerance_profile(agent_id, water_tolerance)?;
```

### 7. Add `GoalBeliefView` accessor

`crates/worldwake-sim/src/belief_view.rs`: add to the trait (next to `metabolism_profile`):

```rust
fn water_tolerance_profile(&self, agent: EntityId) -> Option<WaterToleranceProfile>;
```

`crates/worldwake-sim/src/per_agent_belief_view.rs`: add the `RuntimeBeliefView` impl (next to the existing `metabolism_profile` impl) — self-authoritative read (FND-14B: actor's own profile):

```rust
fn water_tolerance_profile(&self, agent: EntityId) -> Option<WaterToleranceProfile> {
    (agent == self.agent)
        .then(|| self.world.get_component_water_tolerance_profile(agent).cloned())
        .flatten()
}
```

Also add a stub impl in any other `GoalBeliefView` impl blocks (e.g., test fixtures) so the trait remains object-safe.

### 8. Regenerate profile docs

Run `python3 scripts/profile_docs.py` (or whatever the project's convention is — check `scripts/profile_docs.py --help` first) to regenerate `docs/profiles/all-profiles.md`. Include the regenerated doc in the ticket's diff.

### 9. Bump `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs:7`: change `112` to `113`.

## Files to Touch

- `crates/worldwake-core/src/water_tolerance_profile.rs` (new — component definition + Default impl + accessor methods + focused tests)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `WaterToleranceProfile`)
- `crates/worldwake-core/src/component_schema.rs` (modify — register `WaterToleranceProfile` on `EntityKind::Agent`)
- `crates/worldwake-core/src/world.rs` (modify — seed `WaterToleranceProfile::default()` in `create_agent`; expand existing component-list test)
- `crates/worldwake-core/src/world_txn.rs` (modify — update `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` test to expect the new component)
- `crates/worldwake-core/src/delta.rs` (modify if needed — verify `WaterToleranceProfile` import for macro expansion site per README check #13)
- `crates/worldwake-core/src/component_tables.rs` (modify if needed — same as delta.rs)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — add `water_tolerance_profile` field to `AgentDef`)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — insert in `spawn_agent()`)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add `water_tolerance_profile` accessor to trait)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — add `RuntimeBeliefView` impl)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump `SAVE_FORMAT_VERSION` 112→113)
- `docs/profiles/all-profiles.md` (modify — regenerated by `scripts/profile_docs.py`)

## Out of Scope

- Drink reading `WaterToleranceProfile` — owned by ticket 005.
- Source-rank composite reading tolerance for quality discount — owned by ticket 004.
- Authoring per-agent tolerance overrides in existing scenarios — out of scope; defaults apply universally for ticket 003 alone. Tickets 009-010 author tolerance diversity in new scenarios.
- Role-specific tolerance (e.g., "child agents are more sensitive") — out of scope; universal default + scenario overrides cover diversity sufficiently.

## Acceptance Criteria

### Tests That Must Pass

1. New: `water_tolerance_profile_default_values` in `crates/worldwake-core/src/water_tolerance_profile.rs` — confirms `Default::default()` returns Clean=1000/0, Stale=700/80, Muddy=450/200.
2. New: `water_tolerance_profile_serialization_roundtrip` — bincode roundtrip with Default and a customized profile.
3. New: `water_tolerance_profile_accessor_methods` — `thirst_relief_factor(Muddy)` returns 450, `dirtiness_penalty(Clean)` returns 0.
4. Modified: `create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through` in `crates/worldwake-core/src/world_txn.rs:2490` — expanded to expect `WaterToleranceProfile` in the agent's components.
5. New: `spawn_agent_seeds_default_water_tolerance_when_unauthored` in `crates/worldwake-cli/src/scenario/mod.rs` test module — RON without `water_tolerance_profile:` produces an agent with `WaterToleranceProfile::default()`.
6. New: `spawn_agent_applies_authored_water_tolerance_override` — RON with explicit `water_tolerance_profile: Some(…)` produces an agent with the override.
7. New: `water_tolerance_profile_belief_view_returns_self_authoritative` in `crates/worldwake-sim/src/per_agent_belief_view.rs` test module — accessor returns `Some` for the actor itself, `None` for another agent (FND-14B self-authoritative scope).
8. Existing: `cargo test --workspace` passes.

### Invariants

1. Every agent created through `world::create_agent` OR `cli::spawn_agent` carries a `WaterToleranceProfile` component immediately after creation.
2. Runtime reads of `WaterToleranceProfile` on known agents are infallible (`expect()` pattern per universal-profile contract).
3. `BTreeMap<WaterQuality, Permille>` iteration order is deterministic across all platforms (determinism invariant).
4. `SAVE_FORMAT_VERSION` is now 113.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/water_tolerance_profile.rs` (new test module) — default values, roundtrip, accessor methods.
2. `crates/worldwake-core/src/world.rs` (existing test module extension) — `create_agent_attaches_belief_store_perception_profile_and_tell_profile` (line 1353) and sibling assertions need `WaterToleranceProfile` added.
3. `crates/worldwake-core/src/world_txn.rs` (existing test extension) — update delta assertion at line 2490.
4. `crates/worldwake-cli/src/scenario/mod.rs` (test module extension) — spawn_agent with and without `water_tolerance_profile`.
5. `crates/worldwake-sim/src/per_agent_belief_view.rs` (test module extension) — belief-view accessor scope.

### Commands

1. `cargo test -p worldwake-core water_tolerance_profile` — targeted profile tests.
2. `cargo test -p worldwake-core create_agent` — bootstrap path tests.
3. `cargo test -p worldwake-cli spawn_agent` — scenario tests.
4. `cargo test -p worldwake-sim belief_view` — accessor tests.
5. `python3 scripts/profile_docs.py` — regenerate docs.
6. `./scripts/verify.sh` — full workspace.

See Merge-Order Constraints in Step 6 summary — SAVE_FORMAT_VERSION cascade includes this bump (112→113).
