# E13DECARC-007: Candidate goal generation from beliefs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — AI-layer logic
**Deps**: E13DECARC-004, E13DECARC-005, E13DECARC-006

## Problem

Agents need to generate candidate goals grounded in concrete current beliefs. A candidate may only be emitted when evidence exists that the goal is relevant AND at least one concrete path class could pursue it. This is the "no wish-list" rule — only real, achievable goals.

## Assumption Reassessment (2026-03-11)

1. `GoalKind`, `GoalKey`, `GoalPriorityClass`, `GroundedGoal` defined in E13DECARC-004.
2. Extended `BeliefView` available from E13DECARC-005 with all 38 methods.
3. Pressure derivation available from E13DECARC-006.
4. `UtilityProfile` available from E13DECARC-002.
5. `BlockedIntentMemory` available from E13DECARC-003.
6. `HomeostaticNeeds`, `DriveThresholds`, `MerchandiseProfile`, `DemandMemory`, `KnownRecipes` all exist in `worldwake-core`.

## Architecture Check

1. Candidate generation is a pure read-model pass — it reads beliefs and emits a `Vec<GroundedGoal>`.
2. No static universal wish list. Each candidate must trace to concrete believed evidence.
3. Enterprise goals are grounded in stock, demand, and reachable paths — not abstract ambition.
4. Candidates blocked by `BlockedIntentMemory` are suppressed before ranking.

## What to Change

### 1. Implement candidate generation in `worldwake-ai/src/candidate_generation.rs`

Main entry point:

```rust
pub fn generate_candidates(
    view: &dyn BeliefView,
    agent: EntityId,
    utility: &UtilityProfile,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
) -> Vec<GroundedGoal>
```

### 2. Implement per-goal-kind emission rules

Each `GoalKind` variant has specific emission criteria as specified in the spec. Key rules:

- **ConsumeOwnedCommodity**: drive >= low band AND agent controls matching consumable
- **AcquireCommodity(SelfConsume)**: drive >= low band AND agent lacks commodity AND at least one acquisition path exists (off-site stock, seller, source, recipe, lootable corpse)
- **Sleep/Relieve/Wash**: drive >= low band AND reachable affordance/place exists
- **ReduceDanger**: danger > 0 AND concrete reduction path exists (flee, heal, attack)
- **Heal**: target alive and wounded AND healing path exists
- **ProduceCommodity**: recipe known AND outputs serve concrete purpose AND inputs/workstation reachable
- **SellCommodity**: agent controls commodity AND sell path exists
- **RestockCommodity**: agent has MerchandiseProfile including commodity AND stock insufficient AND replenishment path exists
- **MoveCargo**: agent can control lot AND destination differs from current location
- **LootCorpse**: corpse reachable AND believed to have possessions
- **BuryCorpse**: burial affordances exist AND corpse + site concrete

### 3. Implement acquisition preference order

For acquisition/restock/production chains, prefer:
1. on-hand stock
2. fetch controlled off-site stock
3. move controlled cargo
4. buy
5. harvest
6. craft
7. loot

This preference is expressed in the `evidence_entities`/`evidence_places` fields and affects plan search ordering, not emission filtering.

### 4. Suppress blocked candidates

Filter out candidates whose `GoalKey` matches a still-valid `BlockedIntent`.

### 5. Filter dead agents

Return empty vec immediately for dead agents.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — was empty stub)

## Out of Scope

- Priority class assignment and motive scoring — E13DECARC-008
- Plan search — E13DECARC-012
- Plan selection — E13DECARC-012
- Any Phase 3+ goal kinds
- Opportunity signal calculation (used in scoring, not emission)

## Acceptance Criteria

### Tests That Must Pass

1. Dead agent generates zero candidates
2. Agent with hunger >= low band AND owned food emits `ConsumeOwnedCommodity`
3. Agent with hunger >= low band AND no food AND reachable seller emits `AcquireCommodity(SelfConsume)`
4. Agent with hunger below low band does NOT emit any hunger-related candidates
5. Agent with no known path to food AND `BlockedIntent(NoKnownPath)` does NOT emit food acquisition
6. `ReduceDanger` emitted only when derived danger > 0
7. `RestockCommodity` emitted only when agent has `MerchandiseProfile` and stock is insufficient
8. `ProduceCommodity` emitted only when recipe known, outputs serve concrete purpose, and inputs available
9. `LootCorpse` emitted only when corpse at reachable place with possessions
10. `BuryCorpse` NOT emitted when burial affordances don't exist
11. Blocked candidates are suppressed
12. No candidates reference Phase 3+ goals
13. Existing suite: `cargo test --workspace`

### Invariants

1. Every emitted candidate has at least one concrete evidence entity or place
2. No static wish list — all candidates traced to current beliefs
3. Candidates are filtered by `BlockedIntentMemory` before return
4. Dead agents produce empty candidate lists
5. No `HashMap`/`HashSet` in candidate collection

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — comprehensive unit tests using mock `BeliefView` for each goal kind emission/suppression rule

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
