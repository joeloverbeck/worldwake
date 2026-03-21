**Status**: PENDING

# Prerequisite-Aware Emergent Chain Goldens

## Summary

S12 (planner prerequisite-aware search) delivered `prerequisite_places()` for `TreatWounds` and `ProduceCommodity`, enabling multi-hop plans where agents travel to remote locations to acquire prerequisites before executing terminal goals. Two golden tests shipped with S12: remote medicine procurement for care, and remote recipe-input procurement for production.

Two high-value emergent chains enabled by S12 remain untested:

1. **Supply chain restock-via-craft**: A merchant generates `RestockCommodity` from enterprise demand, but the commodity requires crafting (not just harvesting). The planner must guide the merchant toward remote recipe inputs via `prerequisite_places()`, then return to craft. This combines trade enterprise signals with S12's prerequisite-aware production planning. The existing `golden_supply_chain.rs` tests cover harvest-based restock (travel→harvest→return) but not craft-based restock. `RestockCommodity` currently returns empty from `prerequisite_places()`, preventing this chain.

2. **Stale prerequisite belief → discovery → replan**: An agent's `prerequisite_places()` returns a location from stale beliefs. The agent travels there, discovers the resource is depleted (perception corrects belief on arrival), fails to acquire the prerequisite, records a `BlockedIntent`, and replans toward an alternative source. All architectural pieces exist (perception system, belief stores, failure handling, blocked intent) but no golden test exercises this path through S12's code path.

## Discovered Via

Gap analysis of golden E2E coverage after S12 implementation. The `golden_supply_chain.rs` file has 4 active tests (harvest-based merchant restock + consumer co-located trade, each with replay) and 2 ignored tests (full combined chain blocked on S10 bilateral trade negotiation). None exercise craft-based restock with prerequisite-aware planning. The stale-belief-through-prerequisite-places path is untested despite all infrastructure being in place since E14 (perception) and S12.

## Foundation Alignment

- **Principle 1** (Maximal Emergence Through Local Causality): The craft-based supply chain emerges from enterprise demand signal → production planning → prerequisite-aware travel → craft execution, without any orchestrator code or pre-authored decomposition. No system "knows" about the supply chain; it arises from state-mediated interaction between trade enterprise, production, and S12's spatial guidance.
- **Principle 5** (Simulate Carriers of Consequence): Firewood physically travels from a remote resource source to a mill workstation to become bread. The commodity is a concrete entity with location, ownership, and conservation — not a score increment.
- **Principle 7** (Locality of Motion, Interaction, and Communication): In the stale-belief scenario, the agent acts on believed resource locations, not global truth. Belief correction happens only through direct observation on arrival. Information travels physically through perception, not through omniscient queries.
- **Principle 14** (Ignorance, Uncertainty, and Contradiction Are First-Class): The agent holds a stale belief and correctly acts on it. This is not a bug — it is the intended behavior. The simulation models agents who can be wrong and must discover their errors through physical interaction with the world.
- **Principle 15** (Surprise Comes From Violated Expectation): The agent discovers resource depletion through violated belief — it expected firewood at Location A and found none. This mismatch between belief and observation drives replanning.
- **Principle 19** (Intentions Are Revisable Commitments): After discovery, the agent revises its plan rather than stubbornly retrying. The `BlockedIntent` machinery prevents re-attempting the failed location within the blocking period.
- **Principle 24** (Systems Interact Through State, Not Through Each Other): Trade enterprise drives production planning through `DemandMemory` and `MerchandiseProfile` components, not through direct system calls. The planner reads these as belief state; the production system reads recipe inputs as world state.

## Phase

Phase 3: Information & Politics — parallel with S12 (planner enhancement, no phase dependency beyond completed E13/E14)

## Crates

- `worldwake-ai` (goal_model.rs for code change, golden tests)

## Dependencies

- S12 (planner prerequisite-aware search) — completed
- E10 (production & transport) — completed
- E11 (trade & economy) — completed
- E14 (perception & belief) — completed

## Code Change: Extend `prerequisite_places()` for `RestockCommodity`

**File**: `crates/worldwake-ai/src/goal_model.rs`

### Root Cause

`RestockCommodity` includes `PlannerOpKind::Craft` in its allowed ops (`RESTOCK_OPS`), so the planner's search CAN chain restock to craft operations. However, `prerequisite_places()` returns empty `Vec::new()` for `RestockCommodity` (approximately line 760). This means the planner receives no spatial guidance toward remote recipe inputs during restock — the A* heuristic cannot find the craft-via-remote-inputs plan within budget.

### Solution

When `RestockCommodity { commodity }` targets a commodity that has a matching recipe in `RecipeRegistry`, delegate to the same prerequisite discovery logic used by `ProduceCommodity::prerequisite_places()`:

1. Look up recipes whose output matches the restock commodity
2. For each recipe input the actor lacks in their hypothetical state, find places with:
   - Loose commodity lots (ground items)
   - Sellers advertising that commodity
   - Resource sources for that commodity
3. Cap by travel distance via `PlanningBudget::max_prerequisite_locations`

This is ~15 lines of code, directly extending S12's established pattern. The delegation can reuse the existing helper logic from the `ProduceCommodity` arm.

**Backward compatibility**: None needed (Principle 26). `RestockCommodity` previously returned empty prerequisite places — adding places is strictly additive. Existing restock-via-harvest tests are unaffected because harvest-based restock never needed prerequisite spatial guidance (the resource source IS the goal-relevant place).

## Golden Test 1: Merchant Restocks via Prerequisite-Aware Craft

**Test name**: `golden_merchant_restocks_via_prerequisite_aware_craft`
**File**: `crates/worldwake-ai/tests/golden_supply_chain.rs`

### Setup

- Merchant at General Store with `MerchandiseProfile` advertising `Bread`
- `DemandMemory` with observed demand for bread (`WantedToBuyButSellerOutOfStock`) — triggers `RestockCommodity{Bread}`
- Sated merchant (minimal metabolism to suppress survival pressure)
- High `enterprise_weight` in `UtilityProfile` (e.g., `pm(900)`)
- Mill workstation at Village Square with `WorkstationTag::Mill`
- `BakeBread` recipe registered: input=`Firewood`, output=`Bread`, workstation=`Mill`
- Firewood available ONLY at Orchard Farm via `ResourceSource` — no firewood at Village Square or General Store
- Merchant has `PerceptionProfile` and seeded beliefs about the Orchard Farm resource source and the Mill workstation
- Merchant has `KnownRecipes` including the `BakeBread` recipe

### Expected Emergent Chain

1. Enterprise logic: `restock_gap()` signals demand for bread → candidate generation emits `RestockCommodity{Bread}`
2. Ranking: high `enterprise_weight` ranks `RestockCommodity` above any low-urgency survival goal
3. Plan search: `prerequisite_places()` returns Orchard Farm (firewood source from beliefs) → combined places guide A* heuristic toward Orchard Farm first, then back to Mill
4. Plan composed: `Travel(OrchardFarm) → Harvest(firewood) or PickUp(firewood) → Travel(VillageSquare) → Craft(bread at Mill)`
5. Merchant executes full chain autonomously
6. Bread now in merchant's inventory at General Store

### Verification (Assertion Hierarchy)

1. **Authoritative world state**: Merchant owns ≥1 bread at General Store location; firewood quantity at Orchard Farm decreased
2. **Action traces**: Full lifecycle — Travel started/committed to Orchard Farm, Harvest or PickUp committed, Travel back to Village Square, Craft committed at Mill
3. **Decision traces**: `RestockCommodity{Bread}` selected in first 20 ticks; `prerequisite_places_count > 0` in at least one `SearchExpansionSummary` during the plan search
4. **Conservation**: `verify_live_lot_conservation()` and `verify_authoritative_conservation()` pass at every tick

### Deterministic Replay Companion

**Test name**: `golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically`

Same seed produces identical `(hash_world, hash_event_log)` pairs across two runs.

## Golden Test 2: Stale Prerequisite Belief Discovery and Replan

**Test name**: `golden_stale_prerequisite_belief_discovery_replan`
**File**: `crates/worldwake-ai/tests/golden_supply_chain.rs`

### Setup

- Agent **Bob** with `PerceptionProfile` (direct observation capable, reasonable `observation_fidelity`)
- Mill workstation at Village Square with `WorkstationTag::Mill`
- `BakeBread` recipe registered: input=`Firewood`, output=`Bread`, workstation=`Mill`
- **Two firewood sources**: Orchard Farm (primary, closer) and Forest Path (secondary, farther)
- Agent **Alice** at Orchard Farm — depletes ALL firewood in early ticks via scripted harvest actions
- Bob's initial position and belief seeding:
  - Bob starts at or near Orchard Farm at tick 0, observes firewood (perception creates belief)
  - Bob also observes Forest Path firewood source (has belief about both locations)
  - Bob moves to Village Square (or is repositioned) before Alice depletes Orchard Farm
  - Bob's belief about Orchard Farm firewood becomes **stale** — belief says firewood present, world says depleted
- Bob's hunger escalates via `MetabolismProfile` → `ProduceCommodity{Bread}` generated

### Expected Emergent Chain

1. Bob generates `ProduceCommodity{Bread}` from hunger pressure
2. `prerequisite_places()` returns Orchard Farm (closer, from stale belief about firewood there) and possibly Forest Path
3. A* heuristic guides toward Orchard Farm → Plan: `Travel(OrchardFarm) → PickUp(firewood) → Travel(VillageSquare) → Craft(bread)`
4. Bob travels to Orchard Farm
5. On arrival, perception system runs → Bob directly observes: no firewood at Orchard Farm (belief corrected)
6. Bob attempts Harvest or PickUp → `StartFailed` (no resource available) → `BlockedIntent` recorded for Orchard Farm acquisition
7. Replan: `prerequisite_places()` recalculates with corrected beliefs → returns Forest Path (Orchard Farm excluded or down-ranked)
8. Bob travels to Forest Path → picks up or harvests firewood → returns to Village Square → crafts bread at Mill → eats

### Verification (Assertion Hierarchy)

1. **Authoritative world state**: Bob has bread (or has eaten it, reducing hunger); firewood at Forest Path consumed; firewood at Orchard Farm remains at 0
2. **Action traces**: Two distinct travel sequences — first to Orchard Farm (wasted trip), then to Forest Path (successful acquisition)
3. **Decision traces**: First plan search shows `prerequisite_places` including Orchard Farm; second plan search (after replan) shows `prerequisite_places` including Forest Path but not Orchard Farm as primary
4. **BlockedIntent**: Bob's `BlockedIntentMemory` contains an entry related to the failed Orchard Farm acquisition
5. **Conservation**: `verify_live_lot_conservation()` passes at every tick

### Deterministic Replay Companion

**Test name**: `golden_stale_prerequisite_belief_discovery_replan_replays_deterministically`

Same seed produces identical `(hash_world, hash_event_log)` pairs across two runs.

## FND-01 Section H Analysis

### Information-Path Analysis

**Test 1 (craft-based restock)**:
- Enterprise demand enters merchant via `DemandMemory` (observed at market)
- Recipe knowledge enters via `KnownRecipes` component (seeded)
- Firewood location enters via `AgentBeliefStore` (seeded belief about Orchard Farm resource source)
- `prerequisite_places()` reads from `PlanningState` (belief-derived) — never queries world state directly
- All information has a traceable path: market observation → memory → candidate generation → planning → action

**Test 2 (stale belief)**:
- Firewood location at Orchard Farm enters Bob's beliefs via direct observation at tick 0 (perception system)
- Forest Path firewood enters Bob's beliefs via direct observation at tick 0
- Alice depletes Orchard Farm firewood — Bob is NOT present, so Bob's belief is NOT updated (stale)
- On arrival at Orchard Farm, perception system runs and Bob directly observes depletion → belief corrected
- Corrected belief feeds into replanned `prerequisite_places()` → Forest Path now primary
- Full Principle 7 compliance: information travels through perception, not omniscient query

### Positive-Feedback Analysis

No amplifying loops in either scenario. Supply chain restock is a one-shot enterprise response to observed demand. Stale belief correction is a convergent (self-correcting) process — the agent moves toward truth through observation.

### Concrete Dampeners

N/A — no positive feedback loops identified.

### Stored State vs Derived

**Stored (authoritative)**:
- `AgentBeliefStore` — believed entity states including resource source locations
- `DemandMemory` — trade demand observations driving enterprise signals
- `MerchandiseProfile` — merchant sales configuration
- `BlockedIntentMemory` — failed acquisition records with TTL expiry
- `KnownRecipes` — recipe knowledge per agent
- `ResourceSource` — authoritative resource availability at workstations

**Derived (transient computation)**:
- `prerequisite_places()` — computed per search node from `PlanningState`, never stored
- `restock_gap()` — derived from `DemandMemory` + current inventory
- `combined_relevant_places()` — union of goal-relevant + prerequisite places per search node
- A* heuristic scores — computed per expansion, never persisted

## Implementation Checklist

- [ ] **S18-001**: Extend `prerequisite_places()` for `RestockCommodity` in `goal_model.rs`. When the restock commodity has a matching recipe, delegate to `ProduceCommodity`-style prerequisite discovery for missing recipe inputs. Unit test: `RestockCommodity` with a recipe commodity returns non-empty prerequisite places including the input source location.
- [ ] **S18-002**: Golden test `golden_merchant_restocks_via_prerequisite_aware_craft` + deterministic replay companion in `golden_supply_chain.rs`. Proves the full enterprise restock → prerequisite-aware craft chain.
- [ ] **S18-003**: Golden test `golden_stale_prerequisite_belief_discovery_replan` + deterministic replay companion in `golden_supply_chain.rs`. Proves stale belief → wasted trip → perception correction → replan → successful alternative acquisition.

## Cross-References

- `archive/specs/S12-planner-prerequisite-aware-search.md` — parent spec establishing `prerequisite_places()` infrastructure
- `specs/S10-bilateral-trade-negotiation.md` — blocks the full combined supply chain (restock→trade→consumption); this spec's Test 1 exercises the restock-via-craft segment independently
- `crates/worldwake-ai/tests/golden_supply_chain.rs` — existing harvest-based restock and consumer trade tests; new tests extend this file
- `docs/golden-e2e-coverage.md` — coverage tracking for golden E2E suites
