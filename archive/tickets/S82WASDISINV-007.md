# S82WASDISINV-007: Add emit_disposal_candidates and CLI integration

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new candidate generation function, AgentDef field, spawn_agent wiring
**Deps**: S82WASDISINV-003, S82WASDISINV-004, S82WASDISINV-006

## Problem

No candidate generation logic emits `FreeCarryCapacity` goals, and the CLI scenario system still cannot author explicit `DisposalProfile` overrides on agents. This ticket adds both: the `emit_disposal_candidates()` function and the scenario-facing `AgentDef` / `spawn_agent()` override path for disposal-profile configuration.

## Assumption Reassessment (2026-04-10)

1. `candidate_generation.rs` has 40+ `emit_*` functions. `GenerationContext` at lines 144-155 provides `ctx.view: &dyn GoalBeliefView`. `emit_candidate()` at lines 3281-3300 takes `(candidates, kind, anchor, evidence, blocked, current_tick)`.
2. `ctx.view.carry_capacity()` is already used in `candidate_generation.rs`, but `ctx.view.load_of_entity(agent)` is not a lawful carried-load surface for agents: the runtime belief view forwards it to `worldwake_core::load_of_entity()`, which returns `LoadUnits(0)` for non-item entities. Candidate-generation threshold checks must therefore derive carried load from `ctx.view.commodity_quantity(agent, kind)` plus `worldwake_core::load_per_unit(kind)`. `ctx.view.direct_possessor()` at `belief_view.rs:172`.
3. `AgentDef` at `scenario/types.rs:66-129` has 33 fields. No `disposal_profile` field exists.
4. `spawn_agent()` at `scenario/mod.rs:323-468` does not yet apply any scenario-authored disposal override, but core bootstrap already seeds `DisposalProfile::default()` universally in `World::create_agent()` at `world.rs:156-166`. The CLI slice therefore owns explicit override wiring, not default seeding.
5. `BelievedEntityState` at `belief.rs:1320-1339` has `believed_kind: Option<EntityKind>` and `last_known_inventory: BTreeMap<CommodityKind, Quantity>`. No `commodity_kind` or `direct_possessor` fields — must use belief-view accessor methods instead.

## Architecture Check

1. `emit_disposal_candidates()` follows the existing pattern exactly: derive carried-load strain from belief-view inventory, compare against threshold, iterate beliefs, emit candidates. Uses belief-view accessors only (P14 — never reads authoritative state).
2. CLI integration follows the universal profile pattern for authorable overrides: add `Option<DisposalProfile>` on `AgentDef`, and when present, write it through `spawn_agent()`. Default disposal behavior already comes from core agent bootstrap and must not be duplicated as a second canonical path.
3. No backward-compatibility shims.

## Verification Layers

1. Candidate emitted when capacity strained and waste present -> focused unit test with mock belief view
2. No candidate emitted when capacity below threshold -> focused unit test
3. No candidate emitted when no waste in inventory -> focused unit test
4. Scenario-authored `DisposalProfile` override applied to spawned agents, while default bootstrap remains intact when absent -> integration test via scenario loading
5. Candidate generation uses beliefs only, never authoritative state -> code review (P14)
6. Mixed-layer: candidate generation (AI layer) reads via `ctx.view: &dyn GoalBeliefView`. Shared boundary is `GoalBeliefView::disposal_profile()` with blanket forwarding through `ProfileBeliefView::disposal_profile()`, and scenario bootstrap must preserve the core-default / CLI-override contract rather than create a competing authoring path.

## What to Change

### 1. emit_disposal_candidates function

In `crates/worldwake-ai/src/candidate_generation.rs`:

```rust
fn emit_disposal_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let profile = ctx.view.disposal_profile(ctx.agent);
    let threshold = profile
        .map_or(Permille::new_unchecked(800), |p| p.capacity_strain_threshold);

    let Some(carry_capacity) = ctx.view.carry_capacity(ctx.agent) else { return };
    let current_load = CommodityKind::ALL
        .iter()
        .copied()
        .map(|kind| ctx.view.commodity_quantity(ctx.agent, kind).0 * worldwake_core::load_per_unit(kind).0)
        .sum::<u32>();

    // Check strain threshold
    if current_load * 1000 < (carry_capacity.0 as u32) * (threshold.value() as u32) {
        return;
    }

    // Check if agent believes it possesses any Waste
    if ctx.view.commodity_quantity(ctx.agent, CommodityKind::Waste) == Quantity(0) {
        return;
    }

    // Emit candidate for each waste item the agent believes it directly possesses
    for (entity, state) in ctx.view.known_entity_beliefs(ctx.agent) {
        if state.believed_kind != Some(EntityKind::ItemLot) { continue; }
        if !state.last_known_inventory.contains_key(&CommodityKind::Waste) { continue; }
        if ctx.view.direct_possessor(entity) != Some(ctx.agent) { continue; }

        emit_candidate(
            candidates,
            GoalKind::FreeCarryCapacity,
            OpportunityAnchor::Entity(entity),
            Evidence::from_entity(entity),
            ctx.blocked,
            ctx.current_tick,
        );
    }
}
```

### 2. Wire into generate_candidates pipeline

Call `emit_disposal_candidates(candidates, diagnostics, ctx)` from the main `generate_candidates()` function.

### 3. AgentDef field

In `crates/worldwake-cli/src/scenario/types.rs`, add to `AgentDef`:

```rust
pub disposal_profile: Option<DisposalProfile>,
```

### 4. spawn_agent wiring

In `crates/worldwake-cli/src/scenario/mod.rs`, in `spawn_agent()`:

```rust
if let Some(profile) = agent_def.disposal_profile {
    txn.set_component_disposal_profile(agent, profile)?;
}
```

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)
- `crates/worldwake-cli/src/display.rs` (test fixture fallout from `AgentDef` field addition)
- `crates/worldwake-cli/src/handlers/actions.rs` (test fixture fallout from `AgentDef` field addition)
- `crates/worldwake-cli/src/handlers/control.rs` (test fixture fallout from `AgentDef` field addition)
- `crates/worldwake-cli/src/handlers/events.rs` (test fixture fallout from `AgentDef` field addition)
- `crates/worldwake-cli/src/handlers/inspect.rs` (test fixture fallout from `AgentDef` field addition)
- `crates/worldwake-cli/src/handlers/tick.rs` (test fixture fallout from `AgentDef` field addition)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (test fixture fallout from `AgentDef` field addition)

## Out of Scope

- Waste decay, composting, environmental cleanup
- Complex inventory prioritization (keep valuable, drop cheap)
- Container mechanics (dropping into containers/bins)
- Golden E2E tests (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `emit_disposal_candidates` emits `FreeCarryCapacity` goal when load ≥ 80% and waste present
2. `emit_disposal_candidates` emits nothing when load < threshold
3. `emit_disposal_candidates` emits nothing when no waste in believed inventory
4. `emit_disposal_candidates` only emits for items the agent believes it directly possesses
5. Scenario-authored `DisposalProfile` overrides land on spawned agents, and agents without an explicit override still retain the core default
6. Existing suite: `cargo test -p worldwake-ai && cargo test -p worldwake-cli`

### Invariants

1. Candidate generation never reads authoritative world state and derives carried load from believed inventory rather than `load_of_entity(agent)` (P14)
2. All existing candidates unaffected by the new function
3. CLI disposal-profile authoring preserves a single canonical default path: core bootstrap seeds defaults, scenario loading only overrides when explicitly asked
4. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (test module) — focused tests for emit_disposal_candidates with mock belief views at various strain/waste levels
2. `crates/worldwake-cli/src/scenario/mod.rs` (test module) — verify scenario-authored DisposalProfile override is set on spawned agents while the default remains intact when absent

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-cli`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-10.

- Added `emit_disposal_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` and wired it into `generate_candidates_with_travel_horizon()`, making `FreeCarryCapacity` candidate generation live.
- Candidate generation now derives carried-load strain from believed commodity inventory via `commodity_quantity()` plus `load_per_unit()` instead of using `load_of_entity(agent)`, and only emits disposal candidates for directly possessed believed waste lots.
- Added focused candidate-generation tests covering emission at threshold, suppression below threshold, suppression without believed waste inventory, and direct-possession filtering.
- Added scenario-facing `disposal_profile: Option<DisposalProfile>` to `AgentDef` and applied explicit overrides in `spawn_agent()` without duplicating core default seeding.
- Added scenario tests proving explicit disposal-profile overrides land and that agents without an override retain the core default profile.
- Updated CLI test fixtures that manually construct `AgentDef` values so they match the new schema field.

## Deviations

- Reassessment narrowed the CLI slice: core bootstrap already seeds `DisposalProfile::default()` for every new agent, so this ticket did not add another default-seeding path in `spawn_agent()`. The landed CLI work is explicit scenario override authoring only.

## Verification Result

- Passed `cargo test -p worldwake-ai free_carry_capacity_candidate_`
- Passed `cargo test -p worldwake-cli disposal_profile`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
