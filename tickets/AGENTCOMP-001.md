# AGENTCOMP-001: Enforce required component completeness for Agent entities

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — component_schema, World::create_agent, verification, scenario spawner
**Deps**: None (all referenced code exists)

## Problem

AI agents in the default scenario (`scenarios/default.ron`) are completely inert — they never eat, drink, travel, pick up items, or take any action despite rising needs. After 200 ticks, exactly 3 system events per tick (needs decay, perception, system marker), zero agent-initiated actions.

Root cause: the scenario spawner (`crates/worldwake-cli/src/scenario/mod.rs`) does not set `CarryCapacity` on agents. Without this component, the `pick_up` affordance validator returns `false` (`transport_actions.rs:360-362`), so agents can never acquire items, and therefore can never satisfy needs-driven goals. The GOAP planner generates candidates, searches for plans, finds no valid action chains, and the agent idles.

The golden test harness (`crates/worldwake-ai/tests/golden_harness/mod.rs`) sets all required components including `CarryCapacity(LoadUnits(50))`, `WoundList`, `BlockedIntentMemory`, `CombatProfile`, `KnownRecipes`, and `UtilityProfile` — but this setup recipe is duplicated and has drifted from the scenario spawner. The underlying architectural gap is that `World::create_agent()` only attaches 5 of the ~15 components an agent needs to function, and nothing enforces that callers supply the rest.

This is a Principle 17 (Agent Symmetry) violation: an agent's functional capability silently depends on which code path created it. It is also a Principle 27 (Debuggability) violation: a missing component produces no error, no log, no diagnostic — the agent simply does nothing.

## Assumption Reassessment (2026-03-15)

1. **`World::create_agent()` currently sets**: `Name`, `AgentData`, `AgentBeliefStore`, `PerceptionProfile`, `TellProfile` — confirmed at `crates/worldwake-core/src/world.rs:143-157`.
2. **Scenario spawner `spawn_agent()` additionally sets**: `HomeostaticNeeds`, `DeprivationExposure`, `DriveThresholds`, `MetabolismProfile`, and optionally `CombatProfile`, `UtilityProfile`, `MerchandiseProfile`, `TradeDispositionProfile` — confirmed at `crates/worldwake-cli/src/scenario/mod.rs:258-310`.
3. **Golden harness `seed_agent_with_recipes()` additionally sets**: `HomeostaticNeeds`, `DeprivationExposure`, `DriveThresholds`, `MetabolismProfile`, `UtilityProfile`, `CombatProfile`, `WoundList`, `BlockedIntentMemory`, `CarryCapacity(LoadUnits(50))`, `KnownRecipes` — confirmed at `crates/worldwake-ai/tests/golden_harness/mod.rs:228-258`.
4. **`component_schema` declares allowed-kind filters** (e.g., `|kind| kind == EntityKind::Agent`) but does not distinguish required from optional components — confirmed at `crates/worldwake-core/src/component_schema.rs`.
5. **`verification.rs` validates event-log structure** but has no entity-completeness checks — confirmed.

Mismatch: The scenario spawner is missing `CarryCapacity`, `WoundList`, `BlockedIntentMemory`, `CombatProfile` (default), `UtilityProfile` (default), and `KnownRecipes` relative to what an agent needs to function. The fix must be structural, not a patch to the spawner.

## Architecture Check

1. **Why this approach over patching `spawn_agent`**: Adding the missing components to the scenario spawner would fix the immediate bug but leaves the architectural gap open — any future agent-creation code path (new scenario formats, save/load migration, procedural generation, editor tooling) would need to independently remember the same recipe. The component schema is the right place to declare what a valid agent looks like, and `create_agent` is the right place to enforce it. This follows the existing pattern where `create_agent` already sets several defaults.

2. **Why required-component schema over a validation-only approach**: A `verify_agent_completeness()` function catches the problem after the fact but still allows malformed agents to exist temporarily. By making `create_agent` produce a complete agent by default, the invalid state can never arise. The verification function serves as defense-in-depth for edge cases (manual entity creation, deserialization).

3. **No backwards-compatibility shims**: `create_agent` will set more defaults. Callers that currently set these components after `create_agent` will harmlessly overwrite the defaults. No aliases or wrappers needed.

## What to Change

### 1. Extend `World::create_agent()` to set all required agent defaults

In `crates/worldwake-core/src/world.rs`, add the missing required components to the `create_agent` closure:

```rust
// Currently sets: Name, AgentData, AgentBeliefStore, PerceptionProfile, TellProfile
// Add:
world.insert_component_homeostatic_needs(entity, HomeostaticNeeds::default())?;
world.insert_component_deprivation_exposure(entity, DeprivationExposure::default())?;
world.insert_component_drive_thresholds(entity, DriveThresholds::default())?;
world.insert_component_metabolism_profile(entity, MetabolismProfile::default())?;
world.insert_component_carry_capacity(entity, CarryCapacity(LoadUnits(20)))?;
world.insert_component_wound_list(entity, WoundList::default())?;
world.insert_component_blocked_intent_memory(entity, BlockedIntentMemory::default())?;
world.insert_component_utility_profile(entity, UtilityProfile::default())?;
world.insert_component_combat_profile(entity, CombatProfile::default())?;
```

The default `CarryCapacity(LoadUnits(20))` is a conservative value — enough to carry a few items but not unlimited. Scenarios and tests override as needed.

### 2. Add a required-components declaration to `component_schema`

Introduce a `required_agent_components()` function (or a const array) in `crates/worldwake-core/src/component_schema.rs` that returns the list of `ComponentKind` values every `Agent` entity must have. This is the single source of truth for what constitutes a complete agent.

```rust
pub const REQUIRED_AGENT_COMPONENTS: &[ComponentKind] = &[
    ComponentKind::Name,
    ComponentKind::AgentData,
    ComponentKind::AgentBeliefStore,
    ComponentKind::PerceptionProfile,
    ComponentKind::TellProfile,
    ComponentKind::HomeostaticNeeds,
    ComponentKind::DeprivationExposure,
    ComponentKind::DriveThresholds,
    ComponentKind::MetabolismProfile,
    ComponentKind::CarryCapacity,
    ComponentKind::WoundList,
    ComponentKind::BlockedIntentMemory,
    ComponentKind::UtilityProfile,
    ComponentKind::CombatProfile,
];
```

### 3. Add `verify_agent_completeness()` to the verification module

In `crates/worldwake-core/src/verification.rs`, add:

```rust
pub fn verify_agent_completeness(world: &World) -> Result<(), Vec<AgentCompletenessError>> {
    // For each live Agent entity, check that all REQUIRED_AGENT_COMPONENTS are present.
    // Return errors listing entity + missing components.
}
```

This uses the schema from step 2. It should be called:
- At the end of `spawn_scenario()` in the CLI crate (fail-fast on scenario load)
- In integration tests after world setup
- Optionally in `WorldTxn::commit()` for newly created Agent entities (debug-only or always, depending on performance)

### 4. Simplify the scenario spawner

In `crates/worldwake-cli/src/scenario/mod.rs`, remove the explicit `set_component_homeostatic_needs`, `set_component_deprivation_exposure`, `set_component_drive_thresholds`, `set_component_metabolism_profile` calls from `spawn_agent()` since `create_agent` now handles defaults. Keep only the override logic (e.g., `if let Some(needs) = agent_def.needs { txn.set_component_homeostatic_needs(...) }`).

### 5. Simplify the golden test harness

In `crates/worldwake-ai/tests/golden_harness/mod.rs` and the unit test `Harness` in `agent_tick.rs`, remove component-setup lines that are now handled by `create_agent`. Keep only overrides (e.g., setting specific hunger values, custom carry capacity).

### 6. Update existing `create_agent` tests

The tests `create_agent_produces_correct_entity`, `create_agent_components_queryable`, and `create_agent_attaches_belief_store_perception_profile_and_tell_profile` in `crates/worldwake-core/src/world.rs` must be updated to verify all newly-added default components.

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify — expand `create_agent`)
- `crates/worldwake-core/src/component_schema.rs` (modify — add `REQUIRED_AGENT_COMPONENTS`)
- `crates/worldwake-core/src/verification.rs` (modify — add `verify_agent_completeness`)
- `crates/worldwake-core/src/lib.rs` (modify — re-export new verification function and schema constant)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — simplify `spawn_agent`)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — simplify `seed_agent_with_recipes`)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — simplify test `Harness::new`)

## Out of Scope

- Adding `KnownRecipes` to required components — recipe knowledge is a world-state acquisition (agents learn recipes), not a structural requirement. Agents without recipe knowledge simply can't craft, which is a valid state.
- Per-entity-kind required-component declarations beyond Agent (e.g., Facility, ItemLot) — valuable future work but not needed for this fix.
- Scenario RON schema changes — the existing override pattern (optional fields that override defaults) is correct and does not need modification.
- Changing the `CombatProfile::default()` or `CarryCapacity` default values for gameplay balance — this ticket uses conservative defaults; balancing is separate work.

## Acceptance Criteria

### Tests That Must Pass

1. `World::create_agent()` produces an entity with all `REQUIRED_AGENT_COMPONENTS` present.
2. `verify_agent_completeness()` returns `Ok(())` for a world where all agents were created via `create_agent`.
3. `verify_agent_completeness()` returns errors listing missing components for a manually constructed agent that lacks required components.
4. The default scenario (`scenarios/default.ron`) loads and AI agents take actions within 100 ticks (pick up items, eat, travel, etc.).
5. All existing golden tests in `crates/worldwake-ai/tests/` continue to pass.
6. All existing unit tests across the workspace continue to pass.
7. `spawn_scenario()` calls `verify_agent_completeness()` and would fail if any agent is incomplete.

### Invariants

1. Every live Agent entity in authoritative world state has all `REQUIRED_AGENT_COMPONENTS` — enforced by `create_agent` defaults and verified by `verify_agent_completeness`.
2. `create_agent` is the sole factory for Agent entities — no code path creates an Agent without going through this function.
3. The required-component list in `component_schema` is the single source of truth — `create_agent`, `verify_agent_completeness`, and documentation all derive from it.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs::create_agent_attaches_all_required_components` — verifies every `REQUIRED_AGENT_COMPONENTS` entry is present after `create_agent`.
2. `crates/worldwake-core/src/verification.rs::verify_agent_completeness_passes_for_complete_agent` — verifies `Ok` for a properly created agent.
3. `crates/worldwake-core/src/verification.rs::verify_agent_completeness_detects_missing_components` — verifies error listing for an agent with a required component manually removed.
4. `crates/worldwake-core/src/component_schema.rs::required_agent_components_subset_of_allowed` — verifies every required component is in the allowed set for `EntityKind::Agent`.
5. `crates/worldwake-cli/src/scenario/mod.rs::test_scenario_agents_pass_completeness_check` — verifies `verify_agent_completeness` passes after `spawn_scenario`.
6. Integration test: `scenarios/default.ron` AI agents produce non-system events within 100 ticks.

### Commands

1. `cargo test -p worldwake-core create_agent` — targeted create_agent tests
2. `cargo test -p worldwake-core verify_agent` — targeted verification tests
3. `cargo test -p worldwake-cli scenario` — targeted scenario tests
4. `cargo test --workspace` — full regression
5. `cargo clippy --workspace` — lint
