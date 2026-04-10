# S82WASDISINV-007: Add emit_disposal_candidates and CLI integration

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new candidate generation function, AgentDef field, spawn_agent wiring
**Deps**: S82WASDISINV-003, S82WASDISINV-004, S82WASDISINV-006

## Problem

No candidate generation logic emits `FreeCarryCapacity` goals, and the CLI scenario system cannot configure `DisposalProfile` on agents. This ticket adds both: the `emit_disposal_candidates()` function and the CLI integration (AgentDef field + spawn_agent wiring).

## Assumption Reassessment (2026-04-10)

1. `candidate_generation.rs` has 40+ `emit_*` functions. `GenerationContext` at lines 144-155 provides `ctx.view: &dyn GoalBeliefView`. `emit_candidate()` at lines 3281-3300 takes `(candidates, kind, anchor, evidence, blocked, current_tick)`.
2. `ctx.view.carry_capacity()` and `ctx.view.load_of_entity()` already used at lines 3051-3058. `ctx.view.commodity_quantity()` used at line 3047. `ctx.view.direct_possessor()` at `belief_view.rs:172`.
3. `AgentDef` at `scenario/types.rs:66-129` has 33 fields. No `disposal_profile` field exists.
4. `spawn_agent()` at `scenario/mod.rs:323-468` uses `unwrap_or_default()` pattern for universal profiles (e.g., line 343 for `metabolism_profile`).
5. `BelievedEntityState` at `belief.rs:1320-1339` has `believed_kind: Option<EntityKind>` and `last_known_inventory: BTreeMap<CommodityKind, Quantity>`. No `commodity_kind` or `direct_possessor` fields — must use belief-view accessor methods instead.

## Architecture Check

1. `emit_disposal_candidates()` follows the existing pattern exactly: check threshold, iterate beliefs, emit candidates. Uses belief-view accessors (P14 — never reads authoritative state).
2. CLI integration follows the universal profile pattern: `Option<DisposalProfile>` on `AgentDef`, `unwrap_or_default()` in `spawn_agent()`.
3. No backward-compatibility shims.

## Verification Layers

1. Candidate emitted when capacity strained and waste present -> focused unit test with mock belief view
2. No candidate emitted when capacity below threshold -> focused unit test
3. No candidate emitted when no waste in inventory -> focused unit test
4. DisposalProfile applied to spawned agents with default -> integration test via scenario loading
5. Candidate generation uses beliefs only, never authoritative state -> code review (P14)
6. Mixed-layer: candidate generation (AI layer) reads via `ctx.view: &dyn GoalBeliefView`. Shared boundary is `GoalBeliefView::disposal_profile()` with blanket forwarding through `ProfileBeliefView::disposal_profile()`.

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
    let Some(current_load) = ctx.view.load_of_entity(ctx.agent) else { return };

    // Check strain threshold
    if (current_load.0 as u32) * 1000 < (carry_capacity.0 as u32) * (threshold.value() as u32) {
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
let disposal = agent_def.disposal_profile.unwrap_or_default();
// ... set_component_disposal_profile(agent, disposal)
```

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)

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
5. `DisposalProfile::default()` applied to agents without explicit profile in scenario
6. Existing suite: `cargo test -p worldwake-ai && cargo test -p worldwake-cli`

### Invariants

1. Candidate generation never reads authoritative world state (P14)
2. All existing candidates unaffected by the new function
3. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (test module) — focused tests for emit_disposal_candidates with mock belief views at various strain/waste levels
2. `crates/worldwake-cli/src/scenario/mod.rs` (test module) — verify DisposalProfile is set on spawned agents

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-cli`
3. `cargo clippy --workspace --all-targets -- -D warnings`
