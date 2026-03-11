# E13DECARC-007: Candidate goal generation from beliefs

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None - AI-layer logic
**Deps**: E13DECARC-004, E13DECARC-005, E13DECARC-006

## Problem

Agents need a deterministic candidate-generation pass that turns current beliefs into a grounded set of possible goals. The pass must only emit candidates justified by concrete believed evidence. It must not invent wishlist goals that the current belief surface cannot support.

## Assumption Reassessment (2026-03-11)

1. `GoalKind`, `GoalKey`, and `CommodityPurpose` already exist in `worldwake-core`.
2. `GoalPriorityClass` and `GroundedGoal` already exist in `worldwake-ai`.
3. `BeliefView` already has the full 38-method Phase-2 surface in `worldwake-sim`.
4. `derive_pain_pressure()` and `derive_danger_pressure()` already exist in `worldwake-ai`.
5. `BlockedIntentMemory` already exists in `worldwake-core`.
6. `UtilityProfile` already exists in `worldwake-core`, but it is not needed for candidate emission and must stay out of this ticket's API.
7. `KnownRecipes` on `BeliefView` exposes recipe ids only. Candidate generation cannot reason about recipe outputs, inputs, tools, or workstation tags without also reading a `RecipeRegistry`.
8. `worldwake-ai/src/candidate_generation.rs` does not exist yet. This ticket must create it rather than "modify an empty stub".
9. The current belief surface does not expose enough information to ground:
   - off-site controlled stock retrieval
   - cargo-destination selection
   - concrete buyer-side sell paths
   - burial affordances or burial-site discovery

## Architecture Check

1. Candidate generation remains a pure read-model pass over `&dyn BeliefView` plus readonly registries and memory.
2. Ranking is out of scope here. This ticket grounds and suppresses candidates only.
3. Because `GroundedGoal` currently also carries ranking fields, this ticket should populate `priority_class = GoalPriorityClass::Background` and `motive_score = 0`. E13DECARC-008 will assign real ranking data.
4. The clean API for this ticket is:

```rust
pub fn generate_candidates(
    view: &dyn BeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
) -> Vec<GroundedGoal>
```

5. Do not smuggle unavailable world knowledge behind helper aliases. If a goal cannot be grounded from the present belief and registry surface, it stays out of scope for now.

## Scope Correction

### In Scope for This Ticket

Ground only the Phase-2 goals that the current codebase can justify cleanly:

- `ConsumeOwnedCommodity`
- `AcquireCommodity { purpose: SelfConsume }`
- `Sleep`
- `Relieve`
- `Wash`
- `ReduceDanger`
- `Heal`
- `ProduceCommodity`
- `RestockCommodity`
- `LootCorpse`

### Explicitly Out of Scope for This Ticket

These goal kinds stay deferred until the belief surface or surrounding systems expose enough concrete state to ground them honestly:

- `SellCommodity`
- `MoveCargo`
- `BuryCorpse`

Also out of scope:

- off-site stock retrieval preference
- cargo-moving preference
- plan-search ordering hints beyond concrete evidence capture
- priority assignment and motive scoring (E13DECARC-008)

## What to Change

### 1. Create `worldwake-ai/src/candidate_generation.rs`

Add the public `generate_candidates()` entry point and module-local helpers.

### 2. Ground supported goal kinds only when present beliefs justify them

Implement these emission rules:

- `ConsumeOwnedCommodity { commodity }`
  - emit when the relevant bodily drive is at or above its low band
  - and the agent currently controls a local lot of that exact commodity with the needed consumable effect

- `AcquireCommodity { commodity, SelfConsume }`
  - emit when the relevant bodily drive is at or above its low band
  - and the agent lacks a local controlled lot of that commodity/effect
  - and at least one local concrete path exists:
    - co-located seller for that commodity
    - co-located resource source for that commodity
    - co-located corpse with possessions containing that commodity
    - known recipe whose outputs contain that commodity and whose local workstation/input/tool requirements are currently satisfiable

- `Sleep`
  - emit when fatigue is at or above its low band
  - use the agent as concrete evidence because the current needs action is self-contained and does not require a separate target

- `Relieve`
  - emit when bladder pressure is at or above its low band
  - use the agent as concrete evidence because the current toilet action is self-contained and does not require a separate target

- `Wash`
  - emit when dirtiness is at or above its low band
  - and the agent controls local water

- `ReduceDanger`
  - emit when derived danger is above zero
  - and at least one concrete mitigation path exists now:
    - an adjacent place exists
    - the agent has medicine and a wounded local target exists
    - a current attacker exists

- `Heal { target }`
  - emit for each local alive wounded target when the actor controls medicine

- `ProduceCommodity { recipe_id }`
  - emit when the recipe is known
  - and the recipe has a locally satisfiable workstation/input/tool path
  - and at least one output serves a concrete current purpose:
    - self-consume
    - treatment
    - merchant restock backed by `MerchandiseProfile` plus matching `DemandMemory`

- `RestockCommodity { commodity }`
  - emit only when:
    - the actor has a `MerchandiseProfile` containing the commodity
    - matching `DemandMemory` shows concrete missed demand for that commodity
    - current controlled stock is absent or below observed demand
    - and at least one local replenishment path exists:
      - seller
      - source
      - recipe path
      - corpse loot

- `LootCorpse { corpse }`
  - emit when a local corpse has direct possessions

### 3. Suppress blocked candidates

Filter candidates whose `GoalKey` is still blocked at `current_tick`.

### 4. Filter dead agents

Return an empty vector immediately for dead agents.

### 5. Keep results deterministic

- use `BTreeMap` / `BTreeSet` or explicit sorting only
- no `HashMap` / `HashSet`
- deterministic iteration order for emitted candidates

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (export the new module)

## Out of Scope

- Priority class assignment and motive scoring - E13DECARC-008
- Plan search - E13DECARC-012
- Plan selection - E13DECARC-012
- Any Phase 3+ goal kinds
- Unsupported Phase 2 goals that need richer belief queries (`SellCommodity`, `MoveCargo`, `BuryCorpse`)

## Acceptance Criteria

### Tests That Must Pass

1. Dead agent generates zero candidates
2. Hunger above low band plus owned bread emits `ConsumeOwnedCommodity { Bread }`
3. Hunger above low band plus no owned bread plus local seller emits `AcquireCommodity(SelfConsume)`
4. Hunger below low band emits no hunger-driven candidates
5. A still-blocked acquisition goal is suppressed
6. `Sleep` emits when fatigue is above the low band
7. `Relieve` emits when bladder is above the low band
8. `Wash` emits only when dirtiness is above the low band and local controlled water exists
9. `ReduceDanger` emits only when derived danger is above zero and a mitigation path exists
10. `Heal` emits only for local wounded targets when the actor has medicine
11. `ProduceCommodity` emits only when a known recipe has a satisfiable local path and a concrete current purpose
12. `RestockCommodity` emits only when `MerchandiseProfile` plus `DemandMemory` show concrete need and a replenishment path exists
13. `LootCorpse` emits only when a local corpse has possessions
14. No candidate is emitted for `SellCommodity`, `MoveCargo`, or `BuryCorpse`
15. Existing suite: `cargo test -p worldwake-ai`
16. Existing suite: `cargo test --workspace`

### Invariants

1. Every emitted candidate has at least one concrete evidence entity or place
2. No static wish list - all candidates trace to current beliefs
3. Candidates are filtered by `BlockedIntentMemory` before return
4. Dead agents produce empty candidate lists
5. No `HashMap` / `HashSet` in candidate collection
6. Candidate generation does not depend on `UtilityProfile`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` - unit tests covering supported emission and suppression rules, plus explicit non-emission for deferred goal kinds

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

Outcome amended: 2026-03-11

- Completion date: 2026-03-11
- What actually changed:
  - created `crates/worldwake-ai/src/candidate_generation.rs`
  - exported `generate_candidates()` from `worldwake-ai`
  - implemented deterministic, belief-grounded candidate generation for:
    - `ConsumeOwnedCommodity`
    - `AcquireCommodity { purpose: SelfConsume }`
    - `Sleep`
    - `Relieve`
    - `Wash`
    - `ReduceDanger`
    - `Heal`
    - `ProduceCommodity`
    - `RestockCommodity`
    - `LootCorpse`
  - added focused unit coverage for supported emission and suppression rules
- Deviations from original plan:
  - removed `UtilityProfile` from this ticket's API because emission does not need ranking weights
  - added `RecipeRegistry` to the API because recipe ids alone were insufficient to ground production paths
  - explicitly deferred `SellCommodity`, `MoveCargo`, and `BuryCorpse` because the current belief surface cannot ground them cleanly without hidden world knowledge
  - a later 2026-03-11 refinement split ranking back out of `GroundedGoal`; candidate generation now returns evidence-only grounded goals and E13DECARC-008 owns `RankedGoal`
- Verification results:
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
