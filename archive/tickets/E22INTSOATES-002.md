# E22INTSOATES-002: T27 — Controlled Agent Death

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E22INTSOATES-001

## Problem

No existing golden tests human-controlled agent death and control continuity. T27 verifies that when a `ControlSource::Human` agent is killed through normal combat, the world continues advancing, no inputs are processed for the dead agent, and the corpse/inventory persist as world entities.

## Assumption Reassessment (2026-03-31)

1. `DeadAt` component exists — confirmed in `crates/worldwake-core/src/combat.rs` and `component_tables.rs`.
2. `CombatProfile` component exists with wound capacity — confirmed in `crates/worldwake-core/src/combat.rs`.
3. Combat action handlers exist in `crates/worldwake-systems/src/combat.rs` — confirmed.
4. `ControllerState.controlled_entity()` API exists — confirmed in `crates/worldwake-sim/src/controller_state.rs`.
5. `InputKind::RequestAction` exists in `crates/worldwake-sim/src/input_event.rs` — confirmed.
6. Existing golden combat tests (`golden_combat.rs`) test combat mechanics but not human-controlled agent death specifically — confirmed.
7. No `Resurrection` event tag or mechanism exists in the codebase — confirmed (grepped `Resurrection`, no results).
8. T27 isolates controlled-agent death from political/patrol/trade systems. Setup uses only two agents (victim + attacker) with no offices, patrols, or trade.
9. No adjacent contradictions.
10. No mismatches.
11. T27 tick budget is ≤ 50 ticks. Attacker with high-damage profile kills low-wound-capacity victim quickly.

## Architecture Check

1. T27 uses the same combat system as all other tests — no special death handler needed. The scenario proves that the existing death handling works correctly for human-controlled agents.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Agent death → authoritative world state (`DeadAt` component present on Agent A)
2. World continuity → `Scheduler.current_tick()` increments for ≥ 10 further ticks after death
3. No post-death inputs → event-log delta (no `InputKind::RequestAction` for Agent A after death tick)
4. Corpse/inventory persistence → authoritative world state (entity and owned items exist, Principle 4)
5. Control transfer → `ControllerState.controlled_entity()` returns `None` or different entity
6. No resurrection → event-log scan (no resurrection-like events for Agent A)
7. Determinism → state hash comparison across 2 seeds

## What to Change

### 1. Add T27 scenario to `crates/worldwake-ai/tests/golden_integration.rs`

- `fn run_t27_controlled_agent_death(seed: Seed) -> (StateHash, StateHash)`:
  - Build minimal 1-place world
  - Agent A: `ControlSource::Human`, `CombatProfile` with low wound capacity
  - Attacker: `ControlSource::Ai`, high-damage `CombatProfile`, `GoalKind::EngageHostile { target: agent_a }`
  - Enable action tracing
  - Run ticks until Agent A has `DeadAt` component
  - Record death tick
  - Run ≥ 10 more ticks
  - Verify: `DeadAt` on Agent A
  - Verify: `Scheduler.current_tick()` advanced ≥ 10 ticks past death
  - Verify: no `InputKind::RequestAction` for Agent A after death tick
  - Verify: Agent A's entity still exists with inventory items
  - Verify: `ControllerState.controlled_entity()` returns `None` or different entity
  - Verify: no resurrection event in EventLog
  - Return `(hash_world, hash_event_log)`
- Two `#[test]` functions: `t27_controlled_agent_death_seed_1`, `t27_controlled_agent_death_seed_2`

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Changes to combat system or death handling
- Other E22 scenarios
- Any engine code changes

## Acceptance Criteria

### Tests That Must Pass

1. `t27_controlled_agent_death_seed_1` — agent killed, world continues, no post-death inputs
2. `t27_controlled_agent_death_seed_2` — determinism verification
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `DeadAt` component set on Agent A after lethal combat
2. World advances ≥ 10 ticks after death without panic
3. No `InputKind::RequestAction` processed for dead agent
4. Persistent identity (Principle 4): Agent A entity and inventory persist as world entities

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs::t27_controlled_agent_death_seed_1` — proves human-controlled agent death handling
2. `crates/worldwake-ai/tests/golden_integration.rs::t27_controlled_agent_death_seed_2` — determinism

### Commands

1. `cargo test -p worldwake-ai --test golden_integration -- t27`
2. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-31
- **What changed**: Added T27 scenario (`run_t27_controlled_agent_death`) and two test functions (`t27_controlled_agent_death_seed_1`, `t27_controlled_agent_death_seed_2`) to `crates/worldwake-ai/tests/golden_integration.rs`.
- **Deviations**: Inventory persistence assertion was relaxed from "items remain on corpse" to "apple conservation across corpse + attacker equals initial quantity." The loot system legitimately transfers items from corpses, so asserting frozen inventory would overfit to a non-contract. Entity persistence (Principle 4) is verified via `entity_kind`.
- **Verification**: `cargo test -p worldwake-ai --test golden_integration -- t27` (2/2 pass), `cargo test -p worldwake-ai` (36/36 pass), `cargo test --workspace` (all pass), `cargo clippy -p worldwake-ai --test golden_integration` (clean).
