**Status**: PENDING

# Commodity Opportunity Valuation

## Summary
Design a shared belief-facing commodity-opportunity valuation layer for Worldwake. The goal is to let agents value commodities not only for direct use (`eat`, `drink`, `heal`, remembered demand), but also for the concrete downstream opportunities those commodities unlock through known recipes and believed reachable workstations.

This spec exists because the current trade valuation helper undervalues intermediate goods such as firewood, grain, medicine precursors, and future substitute inputs when their utility is indirect rather than immediate. The result is an architectural split:
- AI can now emit `AcquireCommodity { purpose: RecipeInput(..) }`
- trade commit still evaluates bundles as if only directly useful goods matter

That split is not robust. A clean architecture needs one shared read-model for "what opportunities does this commodity unlock for this actor under this belief state?"

This spec is intentionally forward-looking. It deepens archived E11 trade/economy and archived E13 decision architecture, but it is not part of the active E14-E22 implementation sequence. Do not schedule implementation ahead of the current phase gates in `specs/IMPLEMENTATION-ORDER.md`.

## Why This Exists
Current valuation in `worldwake-sim/src/trade_valuation.rs` compares bundle outcomes using:
- direct survival relief from consumables
- wound treatment value from medicine
- remembered local demand
- coin holdings
- visible local alternative supply

That design was correct for the initial E11 scope, but it is now too narrow for the architecture that the codebase is growing into.

Concrete failures of the current shape:
- a hungry baker who knows `Bake Bread` does not value firewood unless firewood is itself directly consumable or remembered-demand stock
- a merchant or crafter can mis-evaluate selling an input that unlocks an immediately available recipe
- AI ranking and trade valuation are beginning to duplicate commodity-importance logic in separate layers

This is not only a missing feature. It is a structural gap between:
- planning-side motive formation
- commit-time bilateral trade acceptance

The correct fix is not a special-case "firewood matters if hungry." The correct fix is a shared commodity-opportunity model grounded in:
- known recipes
- believed reachable workstations
- current believed holdings
- current physiological/wound state
- remembered demand
- agent-specific bounded reasoning limits

## Spec Ownership Scan
Active `specs/E14-*` through `specs/E22-*` do not define indirect commodity valuation.

Closest related spec:
- `specs/S04-merchant-selling-market-presence.md`

That spec fixes seller visibility and concrete listed-lot trade, but it explicitly keeps the valuation helper bilateral and bundle-based without specifying how indirect commodity utility should be represented. It is adjacent, not sufficient.

## Phase
Phase 4+: Economy Deepening, Step 14

## Crates
- `worldwake-core`
- `worldwake-sim`
- `worldwake-systems`
- `worldwake-ai`

## Dependencies
- S04

E14 is not strictly required for the first implementation pass because the model only uses the actor's current beliefs and locally queryable believed opportunities. The design must remain compatible with richer future belief propagation.

## Design Goals
1. Value commodities by concrete downstream consequences, not by commodity labels or hardcoded special cases.
2. Keep one shared commodity-opportunity analysis layer for AI and trade. Do not preserve duplicate AI-only and trade-only indirect valuation logic.
3. Preserve belief locality. Agents may value only recipes, workstations, alternatives, and demand they can currently believe.
4. Support multi-input and multi-step recipe chains in a bounded, deterministic way.
5. Keep valuation derived. No stored market score, recipe desirability scalar, or global commodity price table.
6. Make reasoning limits explicit and per-agent rather than hidden constants.
7. Preserve no-backward-compatibility rules: when this lands, bespoke recipe-input ranking shims should be removed in favor of the shared analysis.

## Non-Goals
- global equilibrium pricing
- omniscient market forecasts
- a generic utility engine for every system in the game
- replacing planner search with valuation logic
- scripting trade acceptance around special commodities

## Deliverables

### 1. `CommodityValuationProfile` Component
Add a new per-agent profile that bounds indirect commodity reasoning. This is separate from `UtilityProfile` because it configures reasoning limits and decay, not AI motive weights.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CommodityValuationProfile {
    recipe_opportunity_depth: NonZeroU8,
    recipe_place_horizon: u8,
    indirect_value_decay_per_step: Permille,
}

impl Component for CommodityValuationProfile {}
```

Meaning:
- `recipe_opportunity_depth`: how many recipe edges this actor reasons through when valuing a commodity
- `recipe_place_horizon`: how many place-graph hops this actor considers when valuing recipe availability
- `indirect_value_decay_per_step`: per-step discount applied to value propagated backward through recipe chains

These are reasoning bounds, not global rules of the world.

### 2. Extend the AI-facing valuation read surface
Add:

```rust
fn commodity_valuation_profile(&self, agent: EntityId) -> Option<CommodityValuationProfile>;
```

This should land on the narrow AI-facing goal/value read surface, not blindly on the broad mixed affordance/search trait. If the post-E14 boundary uses a dedicated goal-forming trait, extend that surface instead of defaulting to the full `BeliefView`.

### 3. Shared `commodity_opportunity` Module in `worldwake-sim`
Add a new shared module that derives concrete commodity value channels from beliefs plus recipes.

Suggested public surface:

```rust
pub struct CommodityOpportunityBreakdown {
    pub direct_survival_score: u64,
    pub treatment_score: u64,
    pub enterprise_score: u64,
    pub indirect_recipe_score: u64,
}

pub fn commodity_opportunity_score(
    actor: EntityId,
    commodity: CommodityKind,
    belief: &dyn BeliefView,
    recipes: &RecipeRegistry,
    holdings: &BTreeMap<CommodityKind, u32>,
    local_alternatives: &BTreeMap<CommodityKind, u32>,
) -> CommodityOpportunityBreakdown;
```

The exact type shape may change, but the module must remain:
- belief-facing
- recipe-aware
- deterministic
- reusable by both AI and trade valuation

### 4. Recipe Opportunity Propagation Rules
Indirect commodity value must be derived from concrete believed recipe opportunities.

Minimum compliant behavior:
- only consider recipes the actor knows
- only consider recipes whose required workstation is believed reachable within `recipe_place_horizon`
- propagate value backward from recipe outputs to inputs
- apply `indirect_value_decay_per_step` at each propagated edge
- respect multi-input requirements: an input only gains recipe value from a recipe when the remaining sibling inputs are already believed accessible or are themselves satisfied by shallower bounded opportunities
- choose the best opportunity path deterministically; do not sum every overlapping path into runaway value inflation

This is a bounded recipe-closure analysis, not an action planner.

### 5. Direct Value Channels Remain Concrete
The shared analysis must preserve current direct channels:
- direct survival relief from consumables
- wound-treatment value from treatment commodities
- remembered demand value from `DemandMemory`
- visible alternative local supply reducing marginal value

Indirect recipe value is added as another channel, not as a replacement.

### 6. Extend `evaluate_trade_bundle`
`evaluate_trade_bundle` must use the shared commodity-opportunity layer rather than a trade-only direct-use snapshot.

The clean shape is:
- either extend the helper signature to accept `&RecipeRegistry`
- or wrap it in a richer trade-valuation context that includes recipes

No AI crate dependency is permitted.

Acceptance still remains bilateral and bundle-based:
- current holdings snapshot
- receipt-only snapshot
- post-trade snapshot
- compare all three from the actor's perspective

The difference is that commodity value now includes indirect recipe opportunities.

### 7. AI Integration
`worldwake-ai` must stop owning bespoke indirect recipe-value logic once this is implemented.

Required follow-on integration:
- `AcquireCommodity { purpose: RecipeInput(..) }` ranking should use the shared commodity-opportunity layer
- `ProduceCommodity` ranking should use the same output opportunity analysis
- candidate generation may continue to identify missing inputs directly, but the "how valuable is this input?" question must be answered by the shared layer

This removes the current architectural split where AI knows a recipe input matters but trade valuation does not.

### 8. Merchant-Side Consequences
Seller-side acceptance must also use indirect opportunity value.

Examples:
- a seller should be less willing to sell the last firewood they need for an immediately reachable bread recipe
- a merchant with remembered unmet demand for bread may prefer to keep grain or firewood if those inputs unlock profitable stock generation

This must emerge from the same shared opportunity analysis, not from seller-only exceptions.

## Detailed Behavioral Rules

### Direct Survival Value
A commodity has direct survival value if its `CommodityConsumableProfile` relieves a currently pressured need.

This remains unchanged from current valuation:
- use current held quantity plus visible alternative local supply
- clamp realized relief by current need pressure

### Treatment Value
A commodity has treatment value if it has a treatment profile and current wounds make treatment concretely useful.

This remains unchanged from current valuation:
- use current held quantity plus visible alternative local supply
- clamp realized treatment value by wound burden / applicable quantity

### Enterprise Value
A commodity has enterprise value when remembered unmet demand indicates the actor can use or sell that stock meaningfully.

This remains concrete:
- only remembered local demand counts
- no hidden price curve or scarcity scalar is introduced

### Indirect Recipe Value
A commodity has indirect recipe value when:
- it is an input to a known recipe
- that recipe's workstation is believed reachable within the actor's valuation horizon
- the recipe's output has direct survival, treatment, or enterprise value
- the remaining recipe inputs are believed accessible within the actor's bounded opportunity depth

This allows firewood to matter to a hungry baker because firewood unlocks bread, and bread has concrete hunger relief.

### Multi-Input Recipes
Value propagation must account for sibling inputs rather than pretending each input independently creates the output.

Required rule:
- an input receives only its share of a recipe opportunity if the rest of the recipe is believed completable within the current bounded evaluation

This prevents a single irrelevant input from inheriting full output value when the rest of the recipe is unavailable.

### Multi-Step Chains
Indirect value may propagate through more than one recipe step, bounded by `recipe_opportunity_depth`.

Example:
- firewood -> smelt iron -> forge tool -> harvest grain -> bake bread

The actor may reason through such a chain only if:
- each step is believed reachable
- each step stays within depth limit
- each step discounts propagated value by `indirect_value_decay_per_step`

### Deterministic Tie-Breaking
When multiple recipes or paths could justify indirect value:
- iterate recipes in deterministic registry order
- compare candidate paths deterministically
- break ties by lower depth first, then stable recipe/order identity

No hash-order or floating-point comparison is allowed.

## Component Registration
Register in authoritative schema:
- `CommodityValuationProfile` on `EntityKind::Agent`

No commodity-value cache is permitted as authoritative state.

## SystemFn Integration

### `worldwake-core`
- add `CommodityValuationProfile`
- add component registration / schema wiring

### `worldwake-sim`
- add `commodity_opportunity.rs`
- extend `BeliefView` with `commodity_valuation_profile`
- rework `trade_valuation.rs` to use shared commodity-opportunity analysis
- keep all valuation logic belief-facing and deterministic

### `worldwake-systems`
- trade action commit uses the upgraded valuation helper
- no trade-action special cases for recipe inputs

### `worldwake-ai`
- replace bespoke recipe-input ranking inheritance with shared commodity-opportunity scoring
- keep candidate generation grounded in known missing inputs and real paths
- do not add compatibility wrappers preserving both old and new ranking paths

## Cross-System Interactions (Principle 12)
- E11 trade reads beliefs, remembered demand, recipes, and valuation profile to accept or reject bundles
- E13 planning reads the same beliefs, recipes, and valuation profile to rank indirect commodity goals
- E10 production defines the concrete recipes, inputs, outputs, and workstation constraints that create indirect value
- E09 needs and E12 wounds determine whether outputs are concretely useful
- future seller-listing work uses the same valuation layer when deciding whether stock should be sold or retained

All interactions remain state-mediated through:
- holdings
- known recipes
- workstation beliefs
- `DemandMemory`
- `CommodityValuationProfile`
- emitted trade / production events

No system calls another system's behavior directly.

## FND-01 Section H

### Information-Path Analysis
- indirect commodity value comes only from recipes the actor knows
- recipe reachability comes only from believed workstation/place information
- remembered demand comes only from concrete observed interactions
- no actor values an input because "the engine knows it will be useful"; the value must be traceable to known recipe structure plus believed reachable production context

### Positive-Feedback Analysis
- recipe inputs acquired cheaply -> more craftable outputs -> more survival or sale capacity -> more ability to acquire inputs
- merchants retaining enabling inputs -> more stocked outputs -> more coin -> more procurement ability

### Concrete Dampeners
- finite input stocks and finite source throughput
- workstation occupancy and queueing
- travel time to believed reachable facilities
- carry-capacity limits on moving enabling inputs
- body-cost and time cost of production steps
- demand-memory retention decay
- coin conservation
- input consumption on commit

### Stored vs Derived State
Stored authoritative state:
- `CommodityValuationProfile`
- inventories / lots / possession / ownership
- `KnownRecipes`
- `DemandMemory`
- workstation and source components
- trade / production / transport events

Derived transient read-model:
- commodity opportunity breakdowns
- propagated indirect recipe value
- bundle valuation snapshots
- "should this actor keep or trade this input?"
- "how valuable is this recipe input relative to direct self-care?"

## Invariants
- indirect commodity utility is always traceable to concrete recipe/output opportunities
- no global price table, recipe utility table, or market singleton is introduced
- no authoritative commodity-value cache exists
- AI and trade do not keep separate permanent indirect-utility logic
- all bounded reasoning limits are explicit per-agent profile state
- all iteration order remains deterministic
- all [0,1]-style fields use `Permille`

## Tests
- [ ] hungry baker values firewood positively when it closes a believed reachable bread recipe
- [ ] the same baker does not value firewood through that recipe when no reachable mill is believed available
- [ ] multi-input recipe does not grant full indirect value to one input when sibling inputs remain unavailable beyond valuation depth
- [ ] indirect value propagates through two recipe steps when within profile depth and decays by configured `Permille`
- [ ] seller refuses a bundle that would give up the last enabling input for a higher-valued local recipe opportunity
- [ ] remembered demand can still outweigh recipe retention when enterprise value is concretely higher
- [ ] AI ranking and trade valuation agree on the sign of recipe-input value for the same belief snapshot
- [ ] deterministic replay preserves trade acceptance outcomes under indirect recipe valuation
- [ ] no-needs agent without useful recipes still evaluates purely from enterprise / coin / wound channels

## Acceptance Criteria
- intermediate commodities can be valued through concrete downstream recipe opportunities
- recipe-input trade acceptance no longer diverges structurally from AI motive formation
- seller and buyer both reason about indirect utility through the same shared layer
- no hardcoded commodity exceptions are introduced
- no hidden global market score or recipe desirability cache is introduced
- all new fields use proper newtypes (`Permille`, `NonZeroU8`, `u8`)
- all authoritative iteration remains deterministic

## References
- [E11-trade-economy.md](/home/joeloverbeck/projects/worldwake/archive/specs/E11-trade-economy.md)
- [E13-decision-architecture.md](/home/joeloverbeck/projects/worldwake/archive/specs/E13-decision-architecture.md)
- [S04-merchant-selling-market-presence.md](/home/joeloverbeck/projects/worldwake/specs/S04-merchant-selling-market-presence.md)
- [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md)
- [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md)
