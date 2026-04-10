# S82WASDISINV-008: Golden E2E test for waste disposal cycle

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S82WASDISINV-005, S82WASDISINV-007

## Problem

No golden test exercises the complete waste disposal cycle: produce waste → capacity strained → FreeCarryCapacity goal emitted → drop_item action executed → capacity freed → agent can resume production. Without this test, regressions in the disposal pipeline would go undetected.

## Assumption Reassessment (2026-04-10)

1. Golden E2E tests live in `crates/worldwake-ai/tests/`. They use `PerceptionProfile` on agents that need to observe post-production output (per CLAUDE.md: "Golden production tests require PerceptionProfile").
2. The waste disposal cycle requires: a production recipe that outputs `CommodityKind::Waste`, an agent with `CarryCapacity` low enough to become strained, and `DisposalProfile` with appropriate threshold.
3. `verify_conservation` exists and must pass after `drop_item` commits.
4. Scenario files in `scenarios/` use RON format with `AgentDef` structs.
5. This is a golden-driven ticket. The live `GoalKind` under test is `FreeCarryCapacity`. The operator surface is `PlannerOpKind::DropItem`. The candidate generation surface is `emit_disposal_candidates`.

## Architecture Check

1. Standard golden E2E test pattern: set up scenario with agents, run simulation for N ticks, assert emergent behavior via decision traces and world state.
2. No production code changes — test-only ticket.
3. No backward-compatibility shims.

## Verification Layers

1. FreeCarryCapacity goal emitted when capacity strained -> decision trace shows goal in candidate list
2. DropItem plan found and executed -> action trace shows drop_item action start + commit
3. Item appears on ground at agent's location -> authoritative world state assertion
4. Agent's carry load decreases after drop -> authoritative world state assertion
5. Conservation invariant holds -> `verify_conservation` call
6. Multi-layer ticket: candidate generation (decision trace) + action lifecycle (action trace) + authoritative outcome (world state)

## What to Change

### 1. Golden test scenario

Create a scenario with:
- One agent with a production recipe that outputs Waste (e.g., crafting that produces waste as byproduct)
- Low `CarryCapacity` (e.g., `CarryCapacity(LoadUnits(5))`) so waste accumulation strains capacity quickly
- `DisposalProfile` with default threshold (800 permille)
- `PerceptionProfile` so the agent can observe its own inventory
- Necessary raw materials or resources for production

### 2. Golden test assertions

In `crates/worldwake-ai/tests/`:

1. Run simulation until the agent has produced waste and capacity is strained
2. Assert `FreeCarryCapacity` goal appears in decision trace
3. Assert `drop_item` action appears in action trace
4. Assert waste item is on ground at agent's place (not in inventory)
5. Assert agent's load decreased
6. Assert `verify_conservation` passes
7. Optionally: assert agent resumes production after dropping waste (capacity freed)

### 3. Falsification variant

Add a variant test: agent with `capacity_strain_threshold: Permille(1000)` (always strained) should attempt disposal every cycle if holding waste.

## Files to Touch

- `crates/worldwake-ai/tests/golden_waste_disposal.rs` (new)
- `scenarios/waste-disposal-test.ron` (new — or inline scenario in test)

## Out of Scope

- Waste decay or cleanup systems
- Multi-agent waste observation scenarios
- Performance benchmarks

## Acceptance Criteria

### Tests That Must Pass

1. Agent produces waste, becomes capacity-strained, and drops waste via `drop_item` action
2. Dropped waste item exists on ground at agent's location
3. Agent's carry load is reduced after dropping
4. `verify_conservation` passes throughout simulation
5. FreeCarryCapacity goal appears in decision trace when capacity strained
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Item identity preserved across drop (same EntityId)
2. Item count conserved (no items created or destroyed)
3. Agent plans from beliefs only, never authoritative state (P14)
4. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_waste_disposal.rs` — golden E2E: produce waste → capacity strain → FreeCarryCapacity → drop_item → item on ground → capacity freed
2. `crates/worldwake-ai/tests/golden_waste_disposal.rs` — falsification: always-strained agent disposes every cycle

### Commands

1. `cargo test -p worldwake-ai golden_waste_disposal`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
