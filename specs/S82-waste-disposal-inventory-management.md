# S82: Waste Disposal and Inventory Management

## Summary

Define a `drop_item` action that allows agents to remove items from inventory by placing them on the ground at their current location. Add a `FreeCarryCapacity` goal kind that the planner activates when an agent's carry capacity is near-full with low-value items (specifically Waste). This closes the FND-11 violation where waste accumulation has no physical dampener — agents currently have no mechanism to shed unwanted inventory.

## Phase

Phase 7: Consequence Carriers (Adjunct — Simulation Remediation)

## Status

Draft

## Crates

- `worldwake-core` (new GoalKind variant)
- `worldwake-systems` (new `drop_item` action definition and handler)
- `worldwake-ai` (goal dispatch declaration, candidate generation, planner hypothetical)

## Dependencies

- E01 (ECS, items) — completed
- E04 (action framework) — completed
- E06 (GOAP planner) — completed

## Design Goals

- Agents can voluntarily shed items to free carry capacity
- The action follows existing Transport domain patterns (`put_down` in `transport_actions.rs`)
- Dropped items persist on the ground at the agent's location (FND-04, FND-10)
- The action has preconditions, duration, cost, and occupancy (FND-08)
- Goal generation is belief-informed: agents only consider disposal when they believe they are carrying waste and capacity is strained
- Waste accumulation's positive feedback loop (production → waste → full inventory → can't produce → still have waste) gains a physical dampener (FND-11)

## Non-Goals

- Destroying items outright — items always persist on the ground after dropping
- Waste decay, composting, or environmental cleanup systems — deferred
- Complex inventory prioritization (keep valuable items, drop cheap ones) — the initial goal targets waste specifically
- Container mechanics (dropping items into containers, bins, middens) — deferred

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-01 (Emergence) | Waste piles emerge from agent production + disposal behavior, creating discoverable aftermath |
| FND-04 (Persistent Identity) | Dropped items retain identity and persist on the ground — nothing vanishes |
| FND-05 (Carriers of Consequence) | Waste piles on the ground create downstream effects: visual evidence, obstruction of space, potential scavenging |
| FND-08 (Preconditions/Duration/Cost) | Drop action requires holding the item, takes non-zero time, occupies attention |
| FND-10 (Aftermath) | Dropping leaves aftermath: item on ground, freed capacity, changed inventory state |
| FND-11 (Physical Dampener) | Disposal is the dampener for waste accumulation — agents can shed waste when capacity is strained |
| FND-14 (World ≠ Belief) | Candidate generation uses believed inventory state, not authoritative world state |
| FND-26 (Systems Interact Through State) | The needs system (production → waste) and planning system (goal generation) interact through inventory state, not direct calls |

## Deliverables

### 1. GoalKind Variant

In `crates/worldwake-core/src/goal.rs`:

```rust
/// Agent wants to free carry capacity by dropping low-value items.
FreeCarryCapacity,
```

No parameters — the goal is satisfied when the agent has dropped at least one item and regained capacity. The planner selects which item to drop during candidate generation based on beliefs about carried items.

### 2. Drop Item Action Definition

In `crates/worldwake-systems/src/transport_actions.rs`, register alongside existing `put_down`:

```rust
ActionDef {
    id: drop_item_id,
    name: "drop_item".to_string(),
    domain: ActionDomain::Transport,
    actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorHasControl],
    targets: vec![TargetSpec::EntityDirectlyPossessedByActorAnyOf {
        kinds: [EntityKind::ItemLot, EntityKind::UniqueItem],
    }],
    preconditions: vec![
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetDirectlyPossessedByActor(0),
    ],
    reservation_requirements: Vec::new(),
    duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
    body_cost_per_tick: BodyCostPerTick::zero(),
    attention_cost: Permille::new_unchecked(50),
    interruptibility: Interruptibility::FreelyInterruptible,
    commit_conditions: vec![
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetDirectlyPossessedByActor(0),
    ],
    visibility: VisibilitySpec::ParticipantsOnly,
    causal_event_tags: BTreeSet::from([
        EventTag::WorldMutation,
        EventTag::Inventory,
        EventTag::Transfer,
    ]),
    payload: ActionPayload::None,
    handler: drop_item_handler,
}
```

Duration is 2 ticks (non-instant per FND-08) with low attention cost (`50‰`). Reuses existing `TargetSpec::EntityDirectlyPossessedByActorAnyOf` and the same precondition pattern as `put_down`.

### 3. Drop Item Handler

In `crates/worldwake-systems/src/transport_actions.rs`:

- **start_drop_item**: Validate actor possesses target. Return `ActionState::Empty`.
- **tick_drop_item**: Return `ActionProgress::Continue` (no per-tick logic).
- **commit_drop_item**: `txn.clear_possessor(target)` + `txn.set_ground_location(target, actor_place)` + `txn.add_target(target)`. Identical to `commit_put_down` — the action is semantically equivalent but distinguished in the planner as a disposal-motivated action vs. a placement action.
- **abort_drop_item**: No-op.

Note: The commit function pattern matches the existing `commit_put_down` (lines 575-592 of `transport_actions.rs`). The handler can share tick and abort functions with the existing `tick_transport`/`abort_transport`.

### 4. GoalDispatchDeclaration for FreeCarryCapacity

In `crates/worldwake-ai/src/goal_dispatch_decl.rs`:

```rust
const FREE_CARRY_OPS: &[PlannerOpKind] = &[PlannerOpKind::DropItem];

const FREE_CARRY_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::DropItem];

const DECL_FREE_CARRY_CAPACITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "FreeCarryCapacity",
    provenance_family: None,
    relevant_ops: FREE_CARRY_OPS,
    invalidation_strategy: InvalidationStrategy::Never,
    feasibility_strategy: FeasibilityStrategy::AlwaysLikely,
    family_policy: GoalFamilyPolicy::standard_singleton(),
    progress_barrier_ops: FREE_CARRY_BARRIER,
};
```

`AlwaysLikely` feasibility: if the candidate generation decided to emit the goal, it's always feasible to plan (agent is holding the item).

### 5. PlannerOpKind::DropItem

In `crates/worldwake-ai/src/planner_ops.rs`:

```rust
DropItem,
```

Mapped from `(ActionDomain::Transport, "drop_item")`.

### 6. Planner Hypothetical State Effect

In `crates/worldwake-ai/src/goal_model.rs`, add a `apply_planner_step` arm for `PlannerOpKind::DropItem`:

After the drop step, the planner's hypothetical state removes the item from the agent's possession and places it on the ground. This allows the planner to chain: drop waste → now has capacity → pick up food.

### 7. Candidate Generation

In `crates/worldwake-ai/src/candidate_generation.rs`, add `emit_disposal_candidates()`:

```
fn emit_disposal_candidates(candidates, diagnostics, ctx):
    // 1. Check agent's believed carry capacity
    carry_capacity = ctx.view.carry_capacity(ctx.agent)
    current_load = ctx.view.load_of_entity(ctx.agent)
    if current_load < carry_capacity * disposal_threshold:
        return  // Not strained enough to bother

    // 2. Find waste items in believed inventory
    for (entity, state) in ctx.view.known_entity_beliefs(ctx.agent):
        if state.believed_kind != Some(EntityKind::ItemLot):
            continue
        if state.commodity_kind != Some(CommodityKind::Waste):
            continue
        if state.direct_possessor != Some(ctx.agent):
            continue

        emit_candidate(GoalKind::FreeCarryCapacity, OpportunityAnchor::Entity(entity), ...)
```

The `disposal_threshold` is a profile-driven parameter (see Deliverable 8).

### 8. DisposalProfile Component

In `crates/worldwake-core/src/`:

```rust
/// Per-agent parameters for waste disposal behavior.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisposalProfile {
    /// Fraction of carry capacity that must be used before disposal candidates
    /// are generated. 800‰ means "consider dropping when 80%+ full."
    pub capacity_strain_threshold: Permille,
}

impl Default for DisposalProfile {
    fn default() -> Self {
        Self {
            capacity_strain_threshold: Permille::new_unchecked(800),
        }
    }
}

impl Component for DisposalProfile {}
```

Universal profile (all agents can decide to drop items). Registered on `EntityKind::Agent`. Added to `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs` with `unwrap_or_default()` in `spawn_agent()`.

## Section H: Causal Hooks (FND-01)

### H1. Information-Path Analysis

- **Trigger**: Agent observes its own inventory (belief claims about possessed items with `CommodityKind::Waste`).
- **Path**: Authoritative inventory → perception → belief claims → candidate generation reads believed inventory → FreeCarryCapacity goal emitted.
- No global information needed. Agent only considers items it believes it possesses.

### H2. Positive-Feedback Analysis

- **Loop**: Production creates waste → waste fills capacity → agent drops waste → capacity freed → agent produces more → more waste.
- This is a weak positive loop: production rate is limited by recipe duration, raw material availability, and need pressure.

### H3. Concrete Dampeners

- **Physical dampener for waste loop**: Dropping waste takes time (2 ticks) and occupies attention. Agent cannot produce while dropping. Recipe duration and raw material depletion further limit production rate.
- **Dampener for waste pile growth**: Waste items on the ground are concrete world state subject to future systems (decay, cleanup, environmental effects). In the current system, waste piles accumulate but do not amplify further production.

### H4. Stored State vs. Derived

- **Stored**: `DisposalProfile` (per-agent), item possession (existing), item ground location (existing).
- **Derived**: Carry capacity strain ratio (computed from believed inventory vs. capacity). Never stored.

## SystemFn Integration

No new SystemFn required. The `drop_item` action is registered in the existing action framework during `register_transport_actions()`. Candidate generation integrates into the existing `generate_candidates()` pipeline via a new `emit_disposal_candidates()` call.

## Component Registration

In `crates/worldwake-core/src/component_schema.rs`:

- Register `DisposalProfile` on `EntityKind::Agent` (universal, with `Default`).

## Cross-System Interactions

- **Needs system → Disposal** (via state): Production actions create `CommodityKind::Waste` items in inventory. When inventory is strained, the planner generates `FreeCarryCapacity` goals.
- **Disposal → Perception** (via state): Dropped items on the ground become observable by other agents via existing perception.
- **Planner → Drop action** (via action framework): Standard GOAP plan → action execution path.

## Profile-Driven Parameters

All tunable parameters live in `DisposalProfile`:

| Parameter | Type | Default | Purpose |
|-----------|------|---------|---------|
| `capacity_strain_threshold` | `Permille` | 800 | Minimum capacity usage before disposal candidates are generated |
