# AGENTCOMP-001: Restore default-scenario agent cargo mobility by spawning `CarryCapacity`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — scenario spawning and regression tests
**Deps**: None

## Problem

AI agents in the bundled default scenario are effectively inert under needs pressure. The current CLI integration coverage only proves that the scenario loads and emits system events for a few ticks; it does not assert that AI agents can produce non-system actions in the spawned world.

The concrete blocker is narrower than originally assumed: scenario-spawned agents do not receive `CarryCapacity`, and the transport action path treats that component as mandatory for `pick_up`. Without it, agents cannot pick up food or water from the ground, so need-driven acquisition plans stall at a core affordance.

## Assumption Reassessment (2026-03-15)

1. **`World::create_agent()` is intentionally minimal.**
   It currently sets `Name`, `AgentData`, `AgentBeliefStore`, `PerceptionProfile`, and `TellProfile` in [world.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs#L143). That is appropriate for a low-level entity factory in `worldwake-core`.

2. **Scenario spawning already owns simulation-facing agent defaults.**
   `spawn_agent()` in [scenario/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/mod.rs#L258) sets `HomeostaticNeeds`, `DeprivationExposure`, `DriveThresholds`, and `MetabolismProfile`, plus optional scenario overrides such as `CombatProfile`, `UtilityProfile`, `MerchandiseProfile`, and `TradeDispositionProfile`.

3. **`CarryCapacity` is the missing component that directly blocks the reported behavior.**
   `pick_up` validation fails when the actor lacks carry capacity in [inventory.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/inventory.rs#L77), and the payload-override validator rejects `pick_up` affordances when `view.carry_capacity(actor)` is `None` in [transport_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/transport_actions.rs#L336).

4. **Several components from the original ticket are not structural requirements for a live agent.**
   `BlockedIntentMemory` and `UtilityProfile` are defaulted at read time in [agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs#L184). Missing `CombatProfile` and `WoundList` are also tolerated for non-combat and unwounded agents in multiple call sites. `KnownRecipes` is clearly situational knowledge, not a universal structural invariant.

5. **`verification.rs` is not a general runtime completeness layer today.**
   The public verification surface currently focuses on event-log integrity; world-state reconstruction helpers are test-only in [verification.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/verification.rs#L1). Adding runtime entity-completeness enforcement there would be a separate architectural feature, not a small bug fix.

6. **Changing `create_agent` in core has a critical blast radius.**
   `WorldTxn::create_agent()` is used widely across core and tests. Loading scenario-specific gameplay defaults into that constructor would couple `worldwake-core` to higher-level simulation assumptions and alter many unrelated tests.

## Architecture Check

1. **Why not broaden `World::create_agent()`**
   The core factory should create a valid agent entity, not silently attach the entire current AI/needs/combat stack. Pushing scenario/gameplay defaults downward would make the low-level constructor harder to reason about, increase churn across core tests, and reduce flexibility for future agent bootstrap contexts.

2. **Why fix the scenario bootstrap instead**
   The defect lives in a high-level spawn recipe that already owns needs/metabolism defaults. `CarryCapacity` belongs in that same recipe because cargo mobility is part of how this scenario seeds functional agents, not a universal invariant that every possible agent-like entity in core must always carry.

3. **What should remain explicit**
   Optional profiles and learned state should stay opt-in. This preserves Principle 20 agent diversity and avoids making every agent combat-ready, production-ready, or utility-tuned just because it exists.

4. **Longer-term ideal architecture**
   If agent bootstrap recipes keep repeating across scenario spawning and specialized test harnesses, the right follow-up is a shared higher-level agent bootstrap helper outside `worldwake-core`, parameterized by explicit defaults and overrides. That is larger than this ticket and should not be smuggled into a bug fix.

## What to Change

### 1. Spawn `CarryCapacity` for scenario agents

In [scenario/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/scenario/mod.rs), set a default carry capacity for every spawned agent alongside the other scenario-owned defaults.

Use a conservative default that is sufficient for baseline self-care and cargo movement in the bundled scenario. Keep the value local to scenario bootstrap unless an existing project-wide agent-default constant already exists.

### 2. Add regression coverage for spawned-agent cargo capability

In the scenario tests, add coverage that agents spawned from `ScenarioDef` receive the expected default carry capacity. This should verify the bootstrap contract directly instead of inferring it from unrelated behavior.

### 3. Add an end-to-end default-scenario behavior test

Strengthen the CLI integration suite so the bundled default scenario must produce at least one non-system event within a bounded number of ticks. This closes the exact gap that allowed the regression to land.

The assertion should be behavior-focused:
- load `scenarios/default.ron`
- tick long enough for need pressure and perception to matter
- verify that the event log contains at least one event not attributable solely to the system loop

## Files to Touch

- `tickets/AGENTCOMP-001.md` (modify — reassessed scope and assumptions)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — add default `CarryCapacity` during spawn)
- `crates/worldwake-cli/src/scenario/types.rs` (only if a scenario override is needed; otherwise leave unchanged)
- `crates/worldwake-cli/src/scenario/mod.rs` tests (modify — direct carry-capacity regression coverage)
- `crates/worldwake-cli/tests/integration.rs` (modify — default scenario behavior regression)

## Out of Scope

- Expanding `World::create_agent()` to attach needs, combat, utility, wound, blocked-intent, or recipe components.
- Adding a `REQUIRED_AGENT_COMPONENTS` declaration to `worldwake-core`.
- Adding runtime `verify_agent_completeness()` machinery to `verification.rs`.
- Declaring `KnownRecipes` or combat state mandatory for every agent.
- Refactoring all agent bootstrap code paths into a shared helper in this ticket.

## Acceptance Criteria

1. Agents spawned by `spawn_scenario()` receive a default `CarryCapacity`.
2. The bundled default scenario produces at least one non-system event within the tested tick budget.
3. The regression is covered by automated tests in the CLI/scenario layer.
4. Existing relevant tests continue to pass.
5. `cargo clippy --workspace` passes after the change.

## Test Plan

### New/Modified Tests

1. Scenario unit test verifying that spawned agents receive default `CarryCapacity`.
2. CLI integration test verifying that `scenarios/default.ron` produces at least one non-system event within a bounded number of ticks.

### Commands

1. `cargo test -p worldwake-cli scenario`
2. `cargo test -p worldwake-cli --test integration`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Updated the ticket scope before implementation. The original plan assumed a core-wide required-component architecture that the current code does not support and does not need for this bug.
- Implemented the narrow fix in scenario bootstrap: spawned agents now receive default `CarryCapacity(LoadUnits(20))` in `worldwake-cli`.
- Added direct regression coverage that scenario-spawned agents receive default carry capacity.
- Added an end-to-end CLI integration test proving the bundled default scenario produces at least one AI-authored event within 100 ticks.
- Did not change `World::create_agent()`, `component_schema`, or `verification.rs`. Those broader changes were dropped because they would push scenario-level defaults into `worldwake-core` and overstate which agent components are truly structural.
