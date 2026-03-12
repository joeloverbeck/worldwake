# GOLDENE2E-003: Multi-Hop Travel Plan

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: None

## Problem

All current travel is single-edge (VillageSquare ↔ OrchardFarm via shortest path). The planner's multi-step travel capability (Dijkstra pathfinding → sequential travel actions across multiple edges) is untested. An agent placed at a distant location should find a multi-hop route to reach food.

**Coverage gap filled**:
- GoalKind: `AcquireCommodity` via multi-hop travel (tests planner depth)
- Topology: BanditCamp, ForestPath, NorthCrossroads, EastFieldTrail (4 new places used)
- Cross-system chain: Needs pressure → goal generation → Dijkstra pathfinding → multi-edge sequential travel → harvest at destination

## Assumption Reassessment (2026-03-12)

1. `build_prototype_world()` creates edges connecting all 12 places (confirmed in `crates/worldwake-core/src/topology.rs`).
2. BanditCamp is connected via ForestPath → NorthCrossroads → ... → OrchardFarm through the prototype topology — exact path needs verification from topology edges.
3. The GOAP planner in `crates/worldwake-ai/src/search.rs` can chain multiple travel actions — this is the capability being tested.
4. `PrototypePlace` enum includes BanditCamp, ForestPath, NorthCrossroads, EastFieldTrail (confirmed).
5. `prototype_place_entity()` const fn can produce `EntityId` for any `PrototypePlace` variant (confirmed).

## Architecture Check

1. This test validates that the GOAP search depth is sufficient to find plans requiring 3+ sequential travel actions before reaching the goal location. This is a critical planner capability.
2. Fits naturally in `golden_ai_decisions.rs` since it tests AI planning depth.
3. No shims — uses existing topology and action framework.

## What to Change

### 1. Add harness constants for distant places

In `golden_harness/mod.rs`:
```rust
pub const BANDIT_CAMP: EntityId = prototype_place_entity(PrototypePlace::BanditCamp);
pub const FOREST_PATH: EntityId = prototype_place_entity(PrototypePlace::ForestPath);
pub const NORTH_CROSSROADS: EntityId = prototype_place_entity(PrototypePlace::NorthCrossroads);
pub const EAST_FIELD_TRAIL: EntityId = prototype_place_entity(PrototypePlace::EastFieldTrail);
```

### 2. Add harness helper: `agent_location()`

```rust
pub fn agent_location(&self, agent: EntityId) -> Option<EntityId> {
    self.world.ground_location(agent)
}
```

### 3. Write golden test: `golden_multi_hop_travel_plan`

In `golden_ai_decisions.rs`:
- Agent at BanditCamp, critically hungry, no food locally.
- Orchard Farm has apples (workstation + resource source).
- No food at any intermediate location.
- Run simulation for up to 150 ticks (multi-hop travel takes many ticks).
- Assert: agent eventually arrives at Orchard Farm (or harvests apples there).
- Assert: agent is no longer at BanditCamp after traveling.

**Expected emergent chain**: Hunger pressure → AcquireCommodity goal → planner finds path BanditCamp → [intermediate hops] → OrchardFarm → harvest action.

### 4. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P3 from Part 3 to Part 1.
- Update topology matrix: 4 new places used.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add place consts, `agent_location()`)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — add test)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Goal switching during multi-hop travel (that's GOLDENE2E-006)
- Death during travel (that's GOLDENE2E-012)
- Verifying the exact path taken (Dijkstra may choose different routes)
- Travel time optimization

## Engine Discovery Protocol

This ticket is a golden e2e test that exercises emergent behavior through the real AI loop.
If implementation reveals that the engine cannot produce the expected emergent behavior,
the following protocol applies:

1. **Diagnose**: Identify the specific engine limitation (missing candidate generation path, planner op gap, action handler deficiency, belief view gap, etc.).
2. **Do not downgrade the test**: The test scenario defines the desired emergent behavior. Do not weaken assertions or remove expected behaviors to work around engine gaps.
3. **Fix forward**: Implement the minimal, architecturally sound engine change that enables the emergent behavior. Document the change in a new "Engine Changes Made" subsection under "What to Change". Each fix must:
   - Follow existing patterns in the affected module
   - Include focused unit tests for the engine change itself
   - Not introduce compatibility shims or special-case logic
4. **Scope guard**: If the required engine change exceeds this ticket's effort rating by more than one level (e.g., a Small ticket needs a Large engine change), stop and apply the 1-3-1 rule: describe the problem, present 3 options, recommend one, and wait for user confirmation before proceeding.
5. **Document**: Record all engine discoveries and fixes in the ticket's Outcome section upon completion, regardless of whether fixes were needed.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_multi_hop_travel_plan` — agent at BanditCamp travels multiple hops to reach Orchard Farm and harvest apples
2. Agent leaves BanditCamp (location changes from starting position)
3. Agent eventually harvests or acquires apples (total apple lots increase or agent has apples)
4. Simulation completes within 150 ticks without deadlock
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: BanditCamp, ForestPath, NorthCrossroads, EastFieldTrail marked as used
6. Existing suite: `cargo test -p worldwake-ai --test golden_ai_decisions`
7. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation: apple lots never exceed initial resource source quantity
3. Determinism: same seed produces same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_multi_hop_travel_plan` — proves multi-hop travel planning

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_multi_hop_travel_plan`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
