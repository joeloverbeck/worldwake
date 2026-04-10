# S79RESSOUCON-001: Wire `KnownRecipes` into `AgentDef` and `spawn_agent`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario spawning (`worldwake-cli`)
**Deps**: S79 spec

## Problem

Agents spawned from RON scenarios cannot harvest because `KnownRecipes` is not wired into the scenario system. The `KnownRecipes` ECS component exists and is registered on `EntityKind::Agent` in `component_schema.rs`, but `AgentDef` has no field for it and `spawn_agent()` never calls `set_component_known_recipes()`. Agents therefore spawn with empty `KnownRecipes` and fail the `Constraint::ActorKnowsRecipe(RecipeId)` precondition on all harvest and craft actions.

## Assumption Reassessment (2026-04-09)

1. `KnownRecipes` exists at `crates/worldwake-core/src/production.rs:38-57` with `BTreeSet<RecipeId>`, derives `Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize`, implements `Component`. Registered on `EntityKind::Agent` in `component_schema.rs:958-978`. Confirmed via grep.
2. `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:67` derives `Clone, Debug, Deserialize` only (no `Default`). Has 30+ `Option<T>` profile fields but no `known_recipes` field. Confirmed via read.
3. Shared boundary: `AgentDef` struct literal construction sites. `minimal_agent()` at `scenario/mod.rs:502-534` is the canonical construction site; all other test sites use `..minimal_agent(...)` spread. Non-spread construction sites exist in `display.rs`, `handlers/inspect.rs`, `handlers/actions.rs`, `handlers/events.rs`, `handlers/world_overview.rs`, `handlers/tick.rs`, `handlers/control.rs`. Total: ~20+ struct literal sites across 9 files, all in `worldwake-cli`.
4. Ticket says `spawn_agent()` can resolve recipe names in place if `RecipeRegistry` is "already a parameter or accessible from the existing context". Live code has `RecipeRegistry::new()` constructed inside `spawn_scenario()` after `spawn_entities()` currently runs, and `spawn_agent()` has no registry access. Correction applied: thread `&RecipeRegistry` from `spawn_scenario()` into `spawn_entities()` and `spawn_agent()` before resolving `known_recipes`. Safe because this is a mechanical call-graph correction inside the ticket's owned `worldwake-cli` scenario boundary.
5. Additional live contradiction: CLI scenarios currently build an empty `RecipeRegistry`, so even correct `known_recipes` wiring would resolve no names and register no recipe-backed harvest/craft actions. Correction applied: widen this ticket to bootstrap the CLI scenario path with the canonical production recipe registry instead of `RecipeRegistry::new()`.
6. Additional live contradiction: scenario `facilities` and `resource_sources` currently spawn as separate entities, but harvest actions require one facility target carrying both `WorkstationMarker` and `ResourceSource`. Correction applied: widen this ticket to let `ResourceSourceDef` attach to a named facility, then update live scenarios that need harvest to author that explicit combined shape.

## Architecture Check

1. Follows the existing pattern for role-specific profile components: `Option<T>` field in `AgentDef`, conditional `if let Some(...)` in `spawn_agent()`, recipe names resolved to `RecipeId` via `RecipeRegistry` at spawn time. This ticket also widens the CLI bootstrap to use a canonical production recipe registry so recipe-backed actions and `known_recipes` resolve against the same source of truth.
2. Harvest requires an explicit combined authoritative target (`WorkstationMarker` + `ResourceSource`) on one facility entity. Scenario authoring must be able to express that same shape directly; otherwise the CLI path diverges from the live production contract.
3. No backward-compatibility shims. New scenario fields use `#[serde(default)]` and explicit facility attachment is additive. Existing scenarios can still deserialize, while scenarios that want lawful harvest can name and attach resource sources to the correct facility.

## Verification Layers

1. Recipe knowledge populated at spawn → authoritative world state: `world.get_component_known_recipes(agent)` returns expected `RecipeId` set after `spawn_agent()`
2. Scenario recipe registry bootstrap → runtime surface: `spawn_scenario()` produces a non-empty recipe registry and recipe-backed harvest/craft action defs
3. Scenario resource-source attachment → authoritative world state: named facility carries both `WorkstationMarker` and `ResourceSource` when authored that way
4. Harvest precondition passes at the scenario boundary → authoritative setup parity: agents with `known_recipes` can lawfully satisfy `Constraint::ActorKnowsRecipe(RecipeId)` against the spawned recipe registry

## What to Change

### 1. Add `known_recipes` field to `AgentDef`

In `crates/worldwake-cli/src/scenario/types.rs`, add to the `AgentDef` struct:

```rust
#[serde(default)]
pub known_recipes: Option<Vec<String>>,
```

Place it near the other production-related fields (after `commodity_valuation` / `substitute_preferences`).

### 2. Add recipe resolution and component wiring in `spawn_agent()`

In `crates/worldwake-cli/src/scenario/mod.rs`, within `spawn_agent()` (after the existing profile-setting block), add:

```rust
if let Some(recipe_names) = &agent_def.known_recipes {
    let mut recipe_ids = BTreeSet::new();
    for name in recipe_names {
        if let Some(id) = recipes.id_by_name(name) {
            recipe_ids.insert(id);
        }
        // Silently skip unknown recipe names — scenario may reference
        // recipes not registered in the current recipe registry.
    }
    if !recipe_ids.is_empty() {
        txn.set_component_known_recipes(agent_id, KnownRecipes::with(recipe_ids));
    }
}
```

This requires `recipes: &RecipeRegistry` to be threaded into `spawn_entities()` / `spawn_agent()` from `spawn_scenario()`.

### 3. Bootstrap the CLI scenario path with the canonical production recipe registry

Replace the empty `RecipeRegistry::new()` bootstrap in `spawn_scenario()` with the canonical production recipe registry used for scenario-driven runtime setup, so:
- `known_recipes` names resolve to live `RecipeId` values
- `build_full_action_registries()` registers harvest/craft actions for those same recipes

### 4. Let `ResourceSourceDef` attach to a named facility

Expand the scenario schema so authored facilities can be named and `resource_sources` can target a specific facility. When `facility` is provided, `spawn_scenario()` should set the `ResourceSource` component on that existing facility entity rather than creating a separate source entity.

Update the live scenario files that rely on harvestable sources so the apple source attaches to the orchard facility explicitly.

### 5. Update all `AgentDef` struct literal construction sites

Add `known_recipes: None` to:
- `minimal_agent()` in `scenario/mod.rs:502-534`
- All non-spread construction sites in `display.rs`, `handlers/inspect.rs`, `handlers/actions.rs`, `handlers/events.rs`, `handlers/world_overview.rs`, `handlers/tick.rs`, `handlers/control.rs`

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — add field to `AgentDef`)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — add recipe wiring in `spawn_agent()`, bootstrap scenario recipes, attach named resource sources to facilities, update tests and `minimal_agent()`)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — add optional facility naming/attachment fields for scenario facilities and resource sources)
- `crates/worldwake-systems/src/action_registry.rs` or another canonical production-recipe owner (modify — expose the canonical production recipe registry used by scenario bootstrap)
- `crates/worldwake-cli/src/display.rs` (modify if needed — add `known_recipes: None` only at non-spread `AgentDef` struct literals)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify if needed — add `known_recipes: None` only at non-spread `AgentDef` struct literals)
- `crates/worldwake-cli/src/handlers/actions.rs` (modify if needed — add `known_recipes: None` only at non-spread `AgentDef` struct literals)
- `crates/worldwake-cli/src/handlers/events.rs` (modify if needed — add `known_recipes: None` only at non-spread `AgentDef` struct literals)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify if needed — add `known_recipes: None` only at non-spread `AgentDef` struct literals)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify if needed — add `known_recipes: None` only at non-spread `AgentDef` struct literals)
- `crates/worldwake-cli/src/handlers/control.rs` (modify if needed — add `known_recipes: None` only at non-spread `AgentDef` struct literals)
- `scenarios/default.ron` (modify — name orchard facility / attach apple source explicitly if needed)
- `scenarios/cli-evaluation.ron` (modify — name orchard facility / attach apple source explicitly if needed)

## Out of Scope

- Planner effect modeling for harvest (ticket S79RESSOUCON-002)
- Golden E2E tests for harvest-to-consume chain (ticket S79RESSOUCON-003)
- Making `KnownRecipes` universal (it is role-specific — not all agents need recipes)
- Adding default recipe knowledge for agents at resource source locations (may be a follow-up scenario design concern)
- Changes to eat/drink or harvest action semantics

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `spawn_agent()` with `known_recipes: Some(vec!["Harvest Apples".into()])` produces an agent whose `KnownRecipes` contains the corresponding `RecipeId`
2. Unit test: `spawn_agent()` with `known_recipes: None` produces an agent with no `KnownRecipes` component (or empty)
3. Unit test: unknown recipe name in `known_recipes` is silently skipped
4. Unit test: `spawn_scenario()` bootstraps a non-empty recipe registry containing canonical production recipes
5. Unit test: named `ResourceSourceDef` attaches its `ResourceSource` to the named facility entity instead of spawning a separate source entity
6. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. `AgentDef` without `known_recipes` in RON deserializes successfully (`#[serde(default)]`)
2. Recipe name resolution uses the canonical runtime `RecipeRegistry` — no hardcoded recipe IDs in scenario agent setup
3. `KnownRecipes` classification is role-specific: conditional application, no mandatory Default
4. Scenario-authored harvest sources can be expressed as one facility carrying both workstation and resource-source state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/mod.rs` (test module) — test that `spawn_agent` correctly resolves recipe names to `RecipeId` and sets the `KnownRecipes` component
2. `crates/worldwake-cli/src/scenario/mod.rs` (test module) — test that missing recipe names are silently skipped
3. `crates/worldwake-cli/src/scenario/mod.rs` (test module) — test canonical scenario recipe-registry bootstrap
4. `crates/worldwake-cli/src/scenario/mod.rs` (test module) — test named facility/source attachment

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo build --workspace`

## Outcome

Completed on 2026-04-09.

- Added `known_recipes` to `AgentDef` and wired scenario agent spawning to resolve recipe names against the runtime recipe registry before setting `KnownRecipes`.
- Replaced the CLI scenario bootstrap's empty `RecipeRegistry` with a canonical production recipe registry so scenario-spawned simulations register harvest/craft actions and can resolve recipe names consistently.
- Expanded scenario authoring so facilities may be named and `resource_sources` may attach to an existing named facility, allowing scenarios to express the single facility shape harvest actions require (`WorkstationMarker` + `ResourceSource`).
- Updated the live default and CLI-evaluation scenarios so apple sources attach to explicit orchard facilities, and forager agents now declare `known_recipes` for `Harvest Apples`.
- Created follow-up ticket `S79RESSOUCON-004` for the still-unowned water harvest contract exposed during reassessment.

## Deviations

- Reassessment widened the ticket beyond the original `AgentDef`/`spawn_agent()` slice because the live CLI path also lacked a populated runtime recipe registry and could not author facility-attached resource sources. Landing only the original field wiring would not have satisfied the scenario harvest contract honestly.

## Verification Result

- Passed `cargo test -p worldwake-cli`
- Passed `cargo build --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
