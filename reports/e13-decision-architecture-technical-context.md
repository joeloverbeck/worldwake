# E13 Decision Architecture — Technical Context Report

Companion document for external review of `specs/E13-decision-architecture.md`. Assumes strong GOAP knowledge. Focuses on Worldwake-specific constraints, codebase realities, and gaps the spec leaves open.

Read alongside: `specs/E13-decision-architecture.md` and `docs/FOUNDATIONS.md`.

---

## Section 1: Codebase Reality Summary

### BeliefView Trait (23 methods)

Defined in `crates/worldwake-sim/src/belief_view.rs`. Every AI read must go through `&dyn BeliefView` — never `&World`.

```rust
pub trait BeliefView {
    fn is_alive(&self, entity: EntityId) -> bool;
    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind>;
    fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
    fn is_in_transit(&self, entity: EntityId) -> bool;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId>;
    fn adjacent_places(&self, place: EntityId) -> Vec<EntityId>;
    fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool;
    fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32;
    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity;
    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind>;
    fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile>;
    fn direct_container(&self, entity: EntityId) -> Option<EntityId>;
    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId>;
    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag>;
    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource>;
    fn has_production_job(&self, entity: EntityId) -> bool;
    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool;
    fn has_control(&self, entity: EntityId) -> bool;
    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool;
    fn is_dead(&self, entity: EntityId) -> bool;
    fn is_incapacitated(&self, entity: EntityId) -> bool;
    fn has_wounds(&self, entity: EntityId) -> bool;
}
```

The spec calls for 12 new extension methods:

| New Method | Overlaps With Existing | Purpose |
|---|---|---|
| `homeostatic_needs(agent) -> Option<HomeostaticNeeds>` | — | Pressure values for 5 drives |
| `drive_thresholds(agent) -> Option<DriveThresholds>` | — | Per-drive urgency bands |
| `wounds(agent) -> Vec<Wound>` | `has_wounds` (bool only) | Severity-based pain derivation |
| `visible_hostiles_for(agent) -> Vec<EntityId>` | — | Danger pressure derivation |
| `agents_selling_at(place, commodity) -> Vec<EntityId>` | — | Trade candidate generation |
| `known_recipes(agent) -> Vec<RecipeId>` | `knows_recipe` (single bool) | Production candidate enumeration |
| `matching_workstations_at(place, tag) -> Vec<EntityId>` | — | Crafting feasibility |
| `resource_sources_at(place, commodity) -> Vec<EntityId>` | — | Harvest candidate generation |
| `demand_memory(agent) -> Vec<DemandObservation>` | — | Restock motivation |
| `merchandise_profile(agent) -> Option<MerchandiseProfile>` | — | Enterprise goal gating |
| `corpse_entities_at(place) -> Vec<EntityId>` | — | Loot candidate generation |
| `in_transit_state(entity) -> Option<InTransitOnEdge>` | `is_in_transit` (bool only) | Route-aware planning |

`OmniscientBeliefView` (`crates/worldwake-sim/src/omniscient_belief_view.rs`) wraps `&World` and delegates every method to authoritative state. It is documented as a temporary Principle 10 violation until E14 provides per-agent belief stores. All 12 extensions must be implemented there too.

### Affordance System

`get_affordances()` in `crates/worldwake-sim/src/affordance_query.rs`:

```rust
pub fn get_affordances(
    view: &dyn BeliefView,
    actor: EntityId,
    registry: &ActionDefRegistry,
) -> Vec<Affordance>
```

Returns:
```rust
pub struct Affordance {
    pub def_id: ActionDefId,
    pub actor: EntityId,
    pub bound_targets: Vec<EntityId>,
    pub payload_override: Option<ActionPayload>,
    pub explanation: Option<String>,
}
```

The function does recursive target binding across all `ActionDef` entries in the registry, evaluates `Constraint` and `Precondition` checks against the `BeliefView`, and returns only physically legal actions with all targets already bound. The planner does **not** need to re-derive legal actions from scratch — it can use `get_affordances()` as its successor function.

`ActionDefRegistry` (`crates/worldwake-sim/src/action_def_registry.rs`) is a simple `Vec<ActionDef>` wrapper.

### Action Framework

**ActionDef** (16 fields, `crates/worldwake-sim/src/action_def.rs`):
```rust
pub struct ActionDef {
    pub id: ActionDefId,              // newtype around u32
    pub name: String,
    pub domain: ActionDomain,
    pub actor_constraints: Vec<Constraint>,
    pub targets: Vec<TargetSpec>,
    pub preconditions: Vec<Precondition>,
    pub reservation_requirements: Vec<ReservationReq>,
    pub duration: DurationExpr,
    pub body_cost_per_tick: BodyCostPerTick,
    pub interruptibility: Interruptibility,
    pub commit_conditions: Vec<Precondition>,
    pub visibility: VisibilitySpec,
    pub causal_event_tags: BTreeSet<EventTag>,
    pub payload: ActionPayload,
    pub handler: ActionHandlerId,
}
```

**ActionPayload** (6 variants, `crates/worldwake-sim/src/action_payload.rs`):
```rust
pub enum ActionPayload {
    None,
    Harvest(HarvestActionPayload),   // recipe_id, workstation_tag, output_commodity/quantity, tool_kinds
    Craft(CraftActionPayload),       // recipe_id, workstation_tag, inputs[], outputs[], tool_kinds
    Trade(TradeActionPayload),       // counterparty, offered/requested commodity+quantity
    Combat(CombatActionPayload),     // target, weapon
    Loot(LootActionPayload),         // target
}
```

**ActionInstance** (`crates/worldwake-sim/src/action_instance.rs`):
```rust
pub struct ActionInstance {
    pub instance_id: ActionInstanceId,
    pub def_id: ActionDefId,
    pub payload: ActionPayload,
    pub actor: EntityId,
    pub targets: Vec<EntityId>,
    pub start_tick: Tick,
    pub remaining_duration: ActionDuration,
    pub status: ActionStatus,
    pub reservation_ids: Vec<ReservationId>,
    pub local_state: Option<ActionState>,
}
```

**Interruptibility** (`crates/worldwake-sim/src/action_semantics.rs`):
```rust
pub enum Interruptibility {
    NonInterruptible,
    InterruptibleWithPenalty,
    FreelyInterruptible,
}
```

**DurationExpr** (9 variants, `crates/worldwake-sim/src/action_semantics.rs`):
```rust
pub enum DurationExpr {
    Fixed(NonZeroU32),
    TargetConsumable { target_index: u8 },
    TravelToTarget { target_index: u8 },
    ActorMetabolism { kind: MetabolismDurationKind },  // Toilet | Wash
    ActorTradeDisposition,
    Indefinite,
    CombatWeapon,
    TargetTreatment { target_index: u8, commodity: CommodityKind },
}
```

**Critical for E13**: `DurationExpr::resolve_for()` takes `(&World, EntityId, &[EntityId], &ActionPayload)` — it reads `&World` directly, not `&dyn BeliefView`. The planner cannot call this function. See Section 2 for the implications.

### InputEvent Pipeline

`crates/worldwake-sim/src/input_event.rs`:
```rust
pub enum InputKind {
    RequestAction {
        actor: EntityId,
        def_id: ActionDefId,
        targets: Vec<EntityId>,
        payload_override: Option<ActionPayload>,
    },
    CancelAction { actor: EntityId, action_instance_id: ActionInstanceId },
    SwitchControl { from: Option<EntityId>, to: Option<EntityId> },
}

pub struct InputEvent {
    pub scheduled_tick: Tick,
    pub sequence_no: u64,
    pub kind: InputKind,
}
```

This is the only interface between planner output and the simulation. The planner must produce `InputKind::RequestAction` values.

### ReplanNeeded Signal

`crates/worldwake-sim/src/replan_needed.rs`:
```rust
pub struct ReplanNeeded {
    pub agent: EntityId,
    pub failed_action_def: ActionDefId,
    pub failed_instance: ActionInstanceId,
    pub reason: AbortReason,
    pub tick: Tick,
}
```

`AbortReason` (`crates/worldwake-sim/src/action_handler.rs`):
```rust
pub enum AbortReason {
    CommitConditionFailed(String),
    Interrupted(String),
    ExternalAbort(String),
}
```

### Type Constraints

- **No floats**: All numeric values are `Permille` (0–1000 integer), `Quantity` (newtype), `LoadUnits` (newtype), or raw `u32`/`NonZeroU32`.
- **Determinism**: `BTreeMap`/`BTreeSet` only in authoritative state (never `HashMap`/`HashSet`). `ChaCha8Rng` for seeded randomness.
- **No wall-clock time**: All durations are tick counts.
- `ActionDefId` is a newtype: `ActionDefId(u32)`, generated via `action_id_type!` macro.

### Existing Components from E09–E12

**E09 — Needs** (`crates/worldwake-core/src/needs.rs`, `crates/worldwake-core/src/drives.rs`):

| Type | Fields | Notes |
|---|---|---|
| `HomeostaticNeeds` | `hunger`, `thirst`, `fatigue`, `bladder`, `dirtiness` (all `Permille`) | Authoritative body state |
| `MetabolismProfile` | 5 decay rates (`Permille`), `rest_efficiency`, 4 tolerance durations, `toilet_ticks`, `wash_ticks` (all `NonZeroU32`) | Per-agent physiology parameters |
| `DriveThresholds` | 7 `ThresholdBand` fields: `hunger`, `thirst`, `fatigue`, `bladder`, `dirtiness`, `pain`, `danger` | Per-drive urgency classification |
| `ThresholdBand` | `low`, `medium`, `high`, `critical` (private `Permille`, strict inequality enforced) | Accessed via getters |
| `DeprivationExposure` | 4 `u32` tick counters: `hunger_critical_ticks`, `thirst_critical_ticks`, `fatigue_critical_ticks`, `bladder_critical_ticks` | Cumulative critical-pressure exposure |

**E10 — Production** (`crates/worldwake-core/src/production.rs`, `crates/worldwake-sim/src/recipe_def.rs`):

| Type | Fields | Notes |
|---|---|---|
| `KnownRecipes` | `recipes: BTreeSet<RecipeId>` | Per-agent recipe knowledge |
| `RecipeDefinition` | `name`, `inputs: Vec<(CommodityKind, Quantity)>`, `outputs: Vec<(CommodityKind, Quantity)>`, `work_ticks: NonZeroU32`, `required_workstation_tag: Option<WorkstationTag>`, `required_tool_kinds: Vec<UniqueItemKind>`, `body_cost_per_tick: BodyCostPerTick` | Registry item, not a component |
| `ResourceSource` | `commodity`, `available_quantity`, `max_quantity`, `regeneration_ticks_per_unit: Option<NonZeroU32>`, `last_regeneration_tick: Option<Tick>` | Depletable stock at a place |
| `InTransitOnEdge` | `edge_id: TravelEdgeId`, `origin`, `destination`, `departure_tick`, `arrival_tick` | Marks entity in transit |

**E11 — Trade** (`crates/worldwake-core/src/trade.rs`):

| Type | Fields | Notes |
|---|---|---|
| `MerchandiseProfile` | `sale_kinds: BTreeSet<CommodityKind>`, `home_market: Option<EntityId>` | Sale intent |
| `DemandMemory` | `observations: Vec<DemandObservation>` | Unmet demand history |
| `DemandObservation` | `commodity`, `quantity`, `place`, `tick`, `counterparty: Option<EntityId>`, `reason: DemandObservationReason` | Single demand event |
| `DemandObservationReason` | `WantedToBuyButNoSeller`, `WantedToBuyButSellerOutOfStock`, `WantedToBuyButTooExpensive`, `WantedToSellButNoBuyer` | 4-variant enum |
| `TradeDispositionProfile` | `negotiation_round_ticks: NonZeroU32`, `initial_offer_bias: Permille`, `concession_rate: Permille`, `demand_memory_retention_ticks: u32` | Negotiation parameters |

**E12 — Combat** (`crates/worldwake-core/src/combat.rs`, `crates/worldwake-core/src/wounds.rs`):

| Type | Fields | Notes |
|---|---|---|
| `CombatProfile` | `wound_capacity`, `incapacitation_threshold`, `attack_skill`, `guard_skill`, `defend_bonus`, `natural_clot_resistance`, `natural_recovery_rate`, `unarmed_wound_severity`, `unarmed_bleed_rate` (all `Permille`), `unarmed_attack_ticks: NonZeroU32` | Per-agent combat parameters |
| `WoundList` | `wounds: Vec<Wound>` | Authoritative wound vector |
| `Wound` | `id: WoundId`, `body_part: BodyPart`, `cause: WoundCause`, `severity: Permille`, `inflicted_at: Tick`, `bleed_rate_per_tick: Permille` | Single wound record |
| `DeadAt` | `DeadAt(Tick)` | Death marker, tuple struct |

`BodyPart`: 6 variants (Head, Torso, LeftArm, RightArm, LeftLeg, RightLeg).
`WoundCause`: `Deprivation(DeprivationKind)` or `Combat { attacker, weapon: CombatWeaponRef }`.

---

## Section 2: Spec Tensions and Open Questions

### PlanningState is undefined

The spec says "compact PlanningState derived from BeliefView" but never defines its fields. This is the most critical underspecification. Questions:

- What state does the forward search mutate? The planner needs a lightweight, copyable snapshot to branch on. But `BeliefView` returns `Vec<Wound>`, `Vec<DemandObservation>`, etc. — heap-allocated and expensive to clone per node.
- How much of `BeliefView` goes into `PlanningState`? All 23+12 methods worth of data? Or a pruned subset relevant to the current goal?
- What is the branching factor? If `PlanningState` includes all entities at all reachable places, the state space could be enormous. If it's pruned to goal-relevant entities, who decides the pruning boundary?

### Search budget / depth limits

No guidance on:
- Maximum plan depth (steps)
- Maximum node expansions
- Time budget per planning cycle
- What happens when the budget is exhausted (return best partial plan? fall back to reactive behavior? idle?)

For a tick-driven simulation where planning runs every tick for every AI agent, this is performance-critical.

### Goal utility formula is unspecified

`score_candidate()` returns `Permille` but no formula is given. Open questions:
- How do `UtilityProfile` weights combine with pressure values? Additive weighted sum? Multiplicative? Min-of-weighted?
- Is the formula `weight * pressure` per drive, then sum? If so, how is enterprise scored when it has no corresponding pressure field in `HomeostaticNeeds`?
- How are pain and danger pressures weighted relative to homeostatic pressures? They use the same `Permille` scale but represent fundamentally different urgencies.
- Is there normalization? If one agent has all weights at 500 and another has all at 1000, do they produce meaningfully different behavior or just proportionally scaled scores?

### PlannedAction to InputEvent target ordering

The spec says target ordering must match `Affordance.bound_targets` but doesn't specify the packing convention. `PlannedAction` has 4 optional `EntityId` fields (`target_entity`, `target_place`, `target_item`, `reservation_target`) but `targets: Vec<EntityId>` in `InputKind::RequestAction` is positional. The conversion function must know which fields map to which positions, and this varies per `ActionDefId` based on its `TargetSpec` list in `ActionDef.targets`. The spec should either:
- Define the canonical ordering per action type, or
- Require the conversion function to look up the `ActionDef` and match field semantics to `TargetSpec` slots

### BlockedIntent expiry mechanism

The spec says memory "naturally expires" but by what rule?
- Fixed tick count per `BlockedIntent`? If so, what count? Configurable per agent via profile?
- Based on world state changes only (i.e., never time-expires)?
- Both (whichever comes first)?
- Does `TradeDispositionProfile.demand_memory_retention_ticks` provide a precedent? That field governs `DemandMemory` retention, suggesting a similar per-agent configurable retention window could work.

### Danger pressure derivation is vague

The spec says `derive_danger_pressure` "counts and factors believed hostiles" but doesn't specify:
- What makes an entity "hostile"? Actively attacking? Previously attacked? Different faction? The `BeliefView` extension `visible_hostiles_for()` is listed but "hostile" is not defined.
- How does entity count translate to pressure? Linear? Capped? One hostile = X Permille, two = 2X?
- Does distance matter? A hostile 3 places away vs. co-located?
- Does combat capability factor in (armed vs. unarmed, wounded vs. healthy)?
- With `OmniscientBeliefView`, "visible" means "exists anywhere" — does the implementation need to scope this to co-located or nearby entities even before E14?

### Enterprise weight grounding

When does enterprise kick in vs. survival?
- Is there a priority ordering (survival always beats enterprise)?
- Or is it purely weight-based, meaning an agent with `enterprise_weight: 900` and `hunger_weight: 100` would choose selling bread over eating when moderately hungry?
- The spec lists enterprise as applying "when the agent has concrete commerce or production affordances" — but how is the enterprise pressure value itself computed? It's not a homeostatic field and not a derived physical pressure like pain/danger.

### Multi-step plan failure handling

If step 3 of a 5-step plan fails revalidation:
- Does the agent replan from scratch (expensive but simple)?
- Does it try to salvage steps 4–5 if they're still valid?
- Does it attempt to find an alternative for step 3 only?
- The `ReplanNeeded` signal carries `AbortReason` but the spec doesn't define a failure-recovery strategy beyond "discard or revise invalid tail."

### Concurrent goal pursuit

Can an agent pursue opportunistic goals while executing a plan? Example: an agent traveling to buy food passes a lootable corpse. Can it interrupt to loot and then resume the travel plan? The spec's interrupt evaluation focuses on urgency-based interrupts (danger, critical needs) but doesn't address opportunity-based interrupts.

### DurationExpr::resolve_for takes &World, not &dyn BeliefView

This is a concrete API mismatch. `DurationExpr::resolve_for()` signature:
```rust
pub fn resolve_for(self, world: &World, actor: EntityId, targets: &[EntityId], payload: &ActionPayload) -> Result<ActionDuration, String>
```

It reads `world.get_component_metabolism_profile()`, `world.effective_place()`, `world.topology()`, etc. The planner operates on `&dyn BeliefView` and cannot call this function. Options:
1. Create a parallel `estimate_for(&dyn BeliefView, ...)` that approximates duration from beliefs
2. Extend `BeliefView` with methods that expose the data `resolve_for` needs (metabolism profile, topology edge weights)
3. Use fixed cost estimates in the planner and let revalidation handle duration surprises

The spec defers `MetabolismProfile` from `BeliefView` extensions, making option 2 incomplete. This needs a design decision.

---

## Section 3: Emergence Considerations

### Agent diversity mechanisms

`UtilityProfile` weights create personality variation, but the spec doesn't discuss:
- **Weight distributions**: How are initial weights seeded? Uniform random? Normal distribution around archetype centers? Hand-tuned per role?
- **Variance thresholds**: How much weight difference is needed to produce observably different behavior? If all agents have hunger_weight between 450–550, diversity is cosmetic.
- **DriveThresholds variation**: Per-agent threshold bands are a second diversity axis. A guard with a high danger threshold stays calm longer. But seeding strategies are unspecified.
- **Interaction between diversity axes**: `UtilityProfile` weights and `DriveThresholds` bands interact multiplicatively. Small differences in both could compound or cancel.

### Information asymmetry is absent (temporarily)

`OmniscientBeliefView` means all agents share perfect, complete information. This eliminates:
- **Trade arbitrage**: No agent can profit from knowing something others don't.
- **Rumors and misinformation**: Impossible — every agent knows truth.
- **Exploration motivation**: No reason to scout when you already know everything.
- **Surprise and ambush**: Attackers can't surprise because defenders know they're coming.
- **Herd convergence**: All agents see the same state, so with similar weights they'll all pursue the same goals.

This is documented as temporary (E14 adds per-agent beliefs), but it fundamentally constrains what emergence is possible in Phase 2 testing.

### Feedback loop inventory (spec misses one)

The spec identifies:
1. Enterprise success → more stock → more sales → more enterprise
2. Plan failure → repeated replanning → same failure (dampened by `BlockedIntent`)

Missing:
3. **Combat success → loot → stronger agent → more combat success**. An agent that wins a fight loots the corpse, potentially gaining weapons or supplies that make them more effective in future fights. The dampener is wound accumulation and finite lootable targets, but the spec doesn't analyze this loop.

4. **Resource depletion death spiral**: Multiple agents targeting the same `ResourceSource` deplete it faster → scarcity → more agents travel farther → more travel fatigue → more needs pressure → more competition for remaining sources. Dampened by regeneration_ticks_per_unit on `ResourceSource`, but only if regeneration is fast enough relative to consumption.

### Herd behavior risk

With omniscient beliefs and similar utility weights, all hungry agents target the same food source simultaneously. The affordance system filters by co-location (you can only buy from sellers at your current place), but:
- All agents at the same place with the same need will attempt the same purchase.
- Only one can succeed per tick if stock is limited.
- The losers get `BlockedIntent` (out of stock) and wait — but if stock replenishes, they all retry simultaneously again.
- There's no mechanism for agents to spread out across multiple suppliers or stagger their attempts.

### Goal interaction / contention

Agent A plans to buy bread from Merchant B. Agent C also plans to buy bread from Merchant B. If Merchant B has only one unit:
- The plan revalidation catches this reactively (one agent gets the bread, the other's precondition fails).
- But both agents spent planning budget on a plan that at most one can execute.
- There is no proactive contention avoidance (e.g., "I see another agent heading to the same seller, so I'll try a different one").
- With omniscient beliefs, agents could in principle do this — but the spec doesn't define contention-aware planning.

### Idle agent problem

If all candidate goals are blocked (via `BlockedIntent` suppression) and no new evidence arrives:
- What does the agent do? The spec doesn't define a fallback behavior.
- Options: explicit idle/wait action, wander to a new place (which might reveal new affordances), or do nothing (which could mean the agent stands motionless for hundreds of ticks).
- "Do nothing" is a valid simulation outcome (an agent with no options waits), but it should be an explicit design choice, not an accidental gap.

### Temporal planning depth

Can agents reason about multi-step sequences like "travel to town → buy food → eat"? The planner section implies yes (forward state-space search with action effects), but:
- Travel itself is a multi-tick action. Does the planner model the travel as a single plan step that takes N ticks, or does it need to plan each tick of travel?
- If travel is one step, then "travel → buy → eat" is a 3-step plan. But "harvest wheat → craft flour → craft bread → sell bread" is 4+ steps. What's the practical depth limit before search becomes intractable?
- `DurationExpr::TravelToTarget` resolves to `edge.travel_time_ticks()` — so the planner needs topology edge weights to estimate travel costs. These are accessible from `BeliefView.adjacent_places()` only as adjacency (no weight), creating another gap.

---

## Section 4: Algorithmic Design Space Constraints

### Integer arithmetic only

All heuristic and utility functions must use `Permille` (0–1000 integer arithmetic). Implications:
- Weighted sum: `(weight * pressure) / 1000` loses precision. A weight of 100 and pressure of 50 gives `(100 * 50) / 1000 = 5`. Fine-grained discrimination between low-pressure candidates may be lost.
- Division truncation means ordering could depend on evaluation order. Must define tie-breaking.
- Intermediate products can overflow `u32` if not careful: `Permille(1000) * Permille(1000) = 1,000,000` which fits in `u32` but sum of 8 such products doesn't.

### Determinism requirement

Same seed + same inputs = same decisions. The planner:
- Must not use `HashMap` or `HashSet` (non-deterministic iteration order).
- Must process agents in a deterministic order (by `EntityId` or similar).
- If the planner uses any randomness (e.g., tie-breaking), it must draw from the seeded `ChaCha8Rng`.
- Priority queues must break ties deterministically.

### Affordance system as successor function

`get_affordances()` already does the heavy lifting:
- Enumerates all legal actions with all targets bound
- Evaluates physical constraints and preconditions against `BeliefView`
- Returns `Affordance` structs with `bound_targets` and `payload_override` ready

The planner can call this as its action-generation step rather than reimplementing precondition checking. However:
- `get_affordances()` enumerates actions for the **current** belief state. In forward search, the planner needs to evaluate affordances for **hypothetical future states** (after applying earlier plan steps). This requires either:
  - A mutable `PlanningState` that implements `BeliefView` (so `get_affordances()` can query it), or
  - A separate, simpler action enumeration that works on `PlanningState` directly

### Action cost model

`DurationExpr` has 9 variants with complex resolution:
- `Fixed(N)`: trivial — just N ticks.
- `TravelToTarget`: requires Dijkstra distance from current place to target. The planner needs access to topology edge weights, which `BeliefView` doesn't currently expose.
- `ActorMetabolism { Toilet | Wash }`: reads `MetabolismProfile.toilet_ticks` / `wash_ticks`. But `MetabolismProfile` is explicitly deferred from `BeliefView` extensions.
- `ActorTradeDisposition`: reads `TradeDispositionProfile.negotiation_round_ticks`. Also not in `BeliefView`.
- `TargetConsumable`: reads `CommodityConsumableProfile.consumption_ticks_per_unit` — this is available via existing `BeliefView.item_lot_consumable_profile()`.
- `CombatWeapon`: reads weapon attack speed. Not in `BeliefView`.
- `TargetTreatment`: reads wound/medicine interaction. Not in `BeliefView`.
- `Indefinite`: combat and similar open-ended actions. The planner cannot meaningfully cost these.

At least 5 of 9 duration variants cannot be resolved through `BeliefView`. The planner needs either a cost estimation strategy or `BeliefView` extensions for cost-relevant data.

### State-space size

The prototype world has ~5 commodity types, ~3–5 places, ~5–10 agents, ~15+ action types. Per agent per tick:
- Each action type may have multiple valid target bindings (buy from seller A vs. seller B, travel to place X vs. Y).
- With 15 action types and an average of 3 target bindings each, the branching factor is ~45 per step.
- A 4-step plan search explores ~45^4 ≈ 4 million nodes without pruning.
- The spec doesn't address pruning strategies: alpha-beta, beam search, goal-directed heuristic cutoffs.

### Tick-driven execution

Plans execute one step per tick-cycle. Between steps, the world changes:
- Other agents act (buy the item you planned to buy, occupy the workstation you planned to use).
- Resource sources deplete or regenerate.
- Agents enter or leave transit.

This means plans are inherently fragile. The spec's revalidation step handles this, but doesn't discuss:
- How frequently replanning occurs in practice (every tick? only on failure?).
- Whether replanning cost dominates the tick budget for large agent counts.

### Reservation system

`ActionDef.reservation_requirements` and `ActionInstance.reservation_ids` exist. `BeliefView.reservation_conflicts(entity, range)` checks for conflicts. The planner should:
- Check reservations before planning to use a workstation or resource.
- Potentially create speculative reservations during planning to prevent contention.
- The spec doesn't discuss whether the planner should reason about reservations or just let revalidation catch conflicts reactively.

---

## Section 5: Cross-System Integration Specifics

### E09 → E13 (Needs → Decision)

| E09 Type | E13 Reads | Via |
|---|---|---|
| `HomeostaticNeeds` | 5 `Permille` pressure values | `view.homeostatic_needs(agent)` (new extension) |
| `DriveThresholds` | 7 `ThresholdBand` values (includes pain + danger bands) | `view.drive_thresholds(agent)` (new extension) |
| `MetabolismProfile` | **Not read by E13** (explicitly deferred) | — |
| `DeprivationExposure` | **Not read by E13** (explicitly deferred) | — |

The decision system compares each of the 5 homeostatic pressures against the corresponding `ThresholdBand` from `DriveThresholds`. Pain and danger are derived pressures compared against their own bands.

### E10 → E13 (Production → Decision)

| E10 Type | E13 Reads | Via |
|---|---|---|
| `KnownRecipes` | `BTreeSet<RecipeId>` — which recipes the agent can produce | `view.known_recipes(agent)` (new extension) |
| `RecipeDefinition` | Inputs, outputs, work_ticks, workstation requirements, tools | `ActionDefRegistry` (recipes are baked into harvest/craft `ActionDef` entries) |
| `ResourceSource` | `commodity`, `available_quantity` — whether a source has harvestable stock | `view.resource_sources_at(place, commodity)` then `view.resource_source(entity)` (existing) |
| `InTransitOnEdge` | `origin`, `destination`, `departure_tick`, `arrival_tick` | `view.in_transit_state(entity)` (new extension) |

Note: `RecipeDefinition` lives in `worldwake-sim`, not `worldwake-core`. The planner doesn't query recipes directly — recipe parameters are embedded in `ActionPayload::Harvest` and `ActionPayload::Craft` payloads within the `ActionDef` registry.

### E11 → E13 (Trade → Decision)

| E11 Type | E13 Reads | Via |
|---|---|---|
| `MerchandiseProfile` | `sale_kinds`, `home_market` — whether the agent is a merchant and for what | `view.merchandise_profile(agent)` (new extension) |
| `DemandMemory` | `Vec<DemandObservation>` — evidence of unmet demand for restock motivation | `view.demand_memory(agent)` (new extension) |
| `TradeDispositionProfile` | **Not read directly** — but `DurationExpr::ActorTradeDisposition` reads `negotiation_round_ticks` | Potential gap: planner needs this for cost estimation |
| `DemandObservationReason` | Consumed indirectly via `DemandObservation.reason` | — |

### E12 → E13 (Combat → Decision)

| E12 Type | E13 Reads | Via |
|---|---|---|
| `WoundList` / `Wound` | Wound severities for pain derivation; wound count for health assessment | `view.wounds(agent)` (new extension) |
| `CombatProfile` | `incapacitation_threshold` (for `is_incapacitated` check) | Existing `view.is_incapacitated(entity)` |
| `DeadAt` | Death check — skip dead agents entirely | Existing `view.is_dead(entity)` |

Note: `visible_hostiles_for()` is the new extension for danger derivation, but "hostile" is undefined at the type level — it's a query semantic that the `OmniscientBeliefView` must implement, likely by checking active combat state or faction relationships (which don't yet exist).

### E13 → Simulation

The sole output type:
```rust
InputKind::RequestAction {
    actor: EntityId,
    def_id: ActionDefId,
    targets: Vec<EntityId>,              // packed from PlannedAction's named fields
    payload_override: Option<ActionPayload>,
}
```

`PlannedAction.target_entity`, `.target_place`, `.target_item`, `.reservation_target` must be packed into `targets: Vec<EntityId>` matching the positional ordering defined by `ActionDef.targets: Vec<TargetSpec>` for the corresponding `def_id`. The `payload_override` comes from the `Affordance.payload_override` that generated the plan step.

---

## Section 6: What the Spec Gets Right (Preserve These)

1. **Belief-only planning through `&dyn BeliefView`**: All AI reads go through the trait, never `&World`. This ensures E14 (per-agent beliefs) can replace the omniscient adapter without changing any E13 logic.

2. **Grounded candidate generation**: Goals arise from concrete evidence in the belief state (owned food, visible sellers, known recipes with available inputs), not from a static universal wishlist. This prevents agents from pursuing goals they have no evidence are achievable.

3. **`BlockedIntent` as concrete replan dampener**: Failure memory tied to a specific `GoalKind` + `related_entity` + `BlockingFact` prevents the catastrophic replan loop where an agent tries the same impossible action every tick forever.

4. **Parameterized plan steps**: `PlannedAction` stores *which* seller, *which* item, *which* destination — not bare `ActionDefId`s. This enables meaningful revalidation (check that the specific seller still has stock) and proper `InputEvent` construction.

5. **Derived pain/danger pressures**: Pain from `WoundList` and danger from believed threats are computed as transient `Permille` values each tick, never stored as authoritative state. This upholds Principle 3 (concrete state over abstract scores).

6. **Clean pipeline separation**: Candidate generation → utility ranking → planning → execution. Each stage has a well-defined responsibility and input/output type. The spec resists collapsing these stages.

7. **Future-epic goals explicitly deferred**: `ClaimOffice`, `SupportClaimant`, `Escort`, `Raid`, `EstablishCamp` are listed as out of scope. This prevents scope creep and ensures Phase 2 delivers a working survival/logistics loop before attempting political/military AI.

8. **`OmniscientBeliefView` documented as temporary scaffolding**: The spec is honest about the Principle 10 violation and frames it as a known debt to be resolved in E14, not a permanent architecture choice.

9. **Agent symmetry preservation**: The spec explicitly requires the same action legality and effect pipeline for human and AI agents, with `ControlSource` only changing the input source.

10. **`ReplanNeeded` integration**: The spec connects the existing action framework's abort signals to the decision architecture's `BlockedIntent` memory, creating a clean feedback path from execution failure to planning adjustment.
