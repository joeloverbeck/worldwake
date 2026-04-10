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
- `worldwake-sim` (new `GoalBeliefView` / `ProfileBeliefView` disposal-profile accessor path)
- `worldwake-ai` (goal dispatch declaration, dispatch key, candidate generation, planner hypothetical, ranking)

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

In `crates/worldwake-systems/src/transport_actions.rs`, register alongside existing `put_down` within `register_transport_actions()`:

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
    duration: DurationExpr::Fixed(NonZeroU32::MIN),
    body_cost_per_tick: BodyCostPerTick::zero(),
    attention_cost: Permille::new_unchecked(100),
    interruptibility: Interruptibility::InterruptibleWithPenalty,
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

The live `drop_item` contract mirrors the current `put_down` transport action: `DurationExpr::Fixed(NonZeroU32::MIN)`, `Permille::new_unchecked(100)`, and `Interruptibility::InterruptibleWithPenalty`. This keeps disposal aligned with the existing transport-domain timing/occupancy model while reusing `TargetSpec::EntityDirectlyPossessedByActorAnyOf` (fixed 2-element array `[EntityKind; 2]` per `action_semantics.rs:42`) and the same precondition pattern as `put_down`.

A new `ActionHandler` struct must be registered via `handlers.register(drop_item_id, drop_item_handler)` even though tick and abort functions are shared with the existing transport handlers.

### 3. Drop Item Handler

In `crates/worldwake-systems/src/transport_actions.rs`:

- **start_drop_item**: Validate actor possesses target. Return `ActionState::Empty`.
- **tick_drop_item**: Reuse `tick_transport` (lines 652-660) — return `ActionProgress::Continue`.
- **commit_drop_item**: `txn.clear_possessor(target)` + `txn.set_ground_location(target, actor_place)` + `txn.add_target(target)`. Identical to `commit_put_down` (lines 575-592 of `transport_actions.rs`). The action is semantically equivalent but distinguished in the planner as a disposal-motivated action vs. a placement action.
- **abort_drop_item**: Reuse `abort_transport` (lines 663-671) — no-op.

### 4. GoalDispatchKey Variant

In `crates/worldwake-ai/src/goal_dispatch_key.rs`:

- Add `FreeCarryCapacity` variant to `GoalDispatchKey` enum.
- Add `FreeCarryCapacity` to the `ALL` constant array.
- Add match arm in `from_goal_kind()`: `GoalKind::FreeCarryCapacity => GoalDispatchKey::FreeCarryCapacity`.

### 5. PlannerOpKind::DropItem

In `crates/worldwake-ai/src/planner_ops.rs`:

Add `DropItem` variant to `PlannerOpKind` enum.

In `classify_action_def()` (line 88), add a new match arm:

```rust
(ActionDomain::Transport, "drop_item") => Some(PlannerOpKind::DropItem),
```

This must be placed before the existing `(ActionDomain::Transport, "pick_up" | "put_down" | "steal")` arm to avoid being captured by a future wildcard.

The `PlannerOpSemantics` for `DropItem` reuses `PlannerTransitionKind::PutDownGroundLot` since the hypothetical state effect is identical (clear possessor, set ground location):

```rust
PlannerOpKind::DropItem => PlannerOpSemantics {
    op_kind: PlannerOpKind::DropItem,
    may_appear_mid_plan: false,
    is_materialization_barrier: true,
    transition_kind: PlannerTransitionKind::PutDownGroundLot,
},
```

### 6. GoalDispatchDeclaration for FreeCarryCapacity

In `crates/worldwake-ai/src/goal_dispatch_decl.rs`:

```rust
const FREE_CARRY_OPS: &[PlannerOpKind] = &[PlannerOpKind::DropItem];

const FREE_CARRY_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::DropItem];

const DECL_FREE_CARRY_CAPACITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "FreeCarryCapacity",
    provenance_family: None,
    relevant_ops: FREE_CARRY_OPS,
    invalidation_strategy: InvalidationStrategy::NoOpinion,
    feasibility_strategy: FeasibilityStrategy::AlwaysLikely,
    family_policy: SELF_CARE_POLICY,
    progress_barrier_ops: FREE_CARRY_BARRIER,
};
```

`AlwaysLikely` feasibility: if the candidate generation decided to emit the goal, it's always feasible to plan (agent is holding the item). `SELF_CARE_POLICY` matches the self-care nature of capacity management (suppression: Never, penalty interrupt on critical survival, reactive free interrupt).

### 7. GoalKindPlannerExt Implementation

In `crates/worldwake-ai/src/goal_model.rs`, implement all 11 `GoalKindPlannerExt` trait methods for `GoalKind::FreeCarryCapacity`:

1. **`ranked_goal_provenance_family`**: Return `None` — no drive or danger provenance.
2. **`relevant_op_kinds`**: Return `&[PlannerOpKind::DropItem]`.
3. **`relevant_observed_commodities`**: Return `Some(BTreeSet::from([CommodityKind::Waste]))` — the goal cares about waste items.
4. **`build_payload_override`**: Return `Ok(None)` — `ActionPayload::None`, no override needed.
5. **`apply_planner_step`**: For `PlannerOpKind::DropItem`, remove the target item from the agent's hypothetical possession and place it on the ground. This allows chaining: drop waste → now has capacity → pick up food.
6. **`is_progress_barrier`**: Return `true` when `step.op_kind == PlannerOpKind::DropItem`.
7. **`is_satisfied`**: Check that the agent's hypothetical load is below the `capacity_strain_threshold` from `DisposalProfile`. If no profile is available, satisfied when any item has been dropped (load decreased from initial state).
8. **`goal_relevant_places`**: Return the agent's current believed place — dropping happens in-place.
9. **`prerequisite_places`**: Return empty — no travel prerequisite (agent drops at current location).
10. **`matches_binding`**: Return `true` when `op_kind == PlannerOpKind::DropItem`.
11. **`candidate_is_available`**: Return `true` when `op_kind == PlannerOpKind::DropItem` and the agent has waste items in hypothetical possession.

### 8. Ranking Integration

In `crates/worldwake-ai/src/ranking.rs`:

**Priority class** (`priority_class()` function): `GoalKind::FreeCarryCapacity` maps to `GoalPriorityClass::Low`. Disposal is housekeeping — important but not urgent compared to survival or enterprise goals.

**Motive score** (`motive_score()` function): Compute based on capacity strain:

```
GoalKind::FreeCarryCapacity => {
    let Some(carry_cap) = context.view.carry_capacity(context.agent) else { return 0 };
    let Some(load) = context.view.load_of_entity(context.agent) else { return 0 };
    let strain = Permille::new_unchecked(
        (u32::from(load.0) * 1000 / u32::from(carry_cap.0).max(1)).min(1000) as u16
    );
    score_product(context.utility.enterprise_weight, strain)
}
```

Higher capacity strain produces higher motive score, scaled by the agent's `enterprise_weight`.

### 9. Candidate Generation

In `crates/worldwake-ai/src/candidate_generation.rs`, add `emit_disposal_candidates()`:

```
fn emit_disposal_candidates(candidates, diagnostics, ctx):
    // 1. Read disposal profile via belief view
    let profile = ctx.view.disposal_profile(ctx.agent);
    let threshold = profile.map_or(Permille::new_unchecked(800), |p| p.capacity_strain_threshold);

    // 2. Check agent's believed carry capacity
    let Some(carry_capacity) = ctx.view.carry_capacity(ctx.agent) else { return };
    let Some(current_load) = ctx.view.load_of_entity(ctx.agent) else { return };

    // 3. Check strain threshold
    if current_load.0 * 1000 < carry_capacity.0 as u32 * threshold.value() as u32 {
        return;  // Not strained enough
    }

    // 4. Find waste items in believed inventory via commodity_quantity
    //    Check if agent believes it possesses any Waste
    if ctx.view.commodity_quantity(ctx.agent, CommodityKind::Waste) == Quantity(0) {
        return;  // No believed waste
    }

    // 5. Emit candidate for each waste item the agent believes it directly possesses
    for (entity, state) in ctx.view.known_entity_beliefs(ctx.agent) {
        if state.believed_kind != Some(EntityKind::ItemLot) { continue }
        if !state.last_known_inventory.contains_key(&CommodityKind::Waste) { continue }
        if ctx.view.direct_possessor(entity) != Some(ctx.agent) { continue }

        emit_candidate(
            candidates,
            GoalKind::FreeCarryCapacity,
            OpportunityAnchor::Entity(entity),
            Evidence::from_entity(entity),
            ctx.blocked,
            ctx.current_tick,
        );
    }
```

Call `emit_disposal_candidates()` from the main `generate_candidates()` pipeline.

### 10. DisposalProfile Component

In `crates/worldwake-core/src/`:

```rust
/// Per-agent parameters for waste disposal behavior.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

Universal profile (all agents can decide to drop items). Registered on `EntityKind::Agent` in `component_schema.rs`. Added to `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs` with `unwrap_or_default()` in `spawn_agent()`.

### 11. Belief-View Accessor for DisposalProfile

In `crates/worldwake-sim/src/belief_view.rs`:

Add `disposal_profile(&self, entity: EntityId) -> Option<DisposalProfile>` to both the `ProfileBeliefView` and `GoalBeliefView` surfaces, with blanket forwarding from `GoalBeliefView` through `ProfileBeliefView`, so the AI crate can read `capacity_strain_threshold` through its live caller-facing belief-view interface during candidate generation and goal satisfaction checks.

Follow the existing pattern for profile accessors (e.g., `perception_profile`, `cognitive_profile`).

## Section H: Causal Hooks (FND-01)

### H1. Information-Path Analysis

- **Trigger**: Agent observes its own inventory (belief claims about possessed items with `CommodityKind::Waste`).
- **Path**: Authoritative inventory → perception → belief claims → candidate generation reads believed inventory → FreeCarryCapacity goal emitted.
- No global information needed. Agent only considers items it believes it possesses.

### H2. Positive-Feedback Analysis

- **Loop**: Production creates waste → waste fills capacity → agent drops waste → capacity freed → agent produces more → more waste.
- This is a weak positive loop: production rate is limited by recipe duration, raw material availability, and need pressure.

### H3. Concrete Dampeners

- **Physical dampener for waste loop**: Dropping waste takes time and occupies attention under the live transport contract (`DurationExpr::Fixed(NonZeroU32::MIN)`, `Permille::new_unchecked(100)`, `Interruptibility::InterruptibleWithPenalty`). Agent cannot produce while dropping. Recipe duration and raw material depletion further limit production rate.
- **Dampener for waste pile growth**: Waste items on the ground are concrete world state subject to future systems (decay, cleanup, environmental effects). In the current system, waste piles accumulate but do not amplify further production.

### H4. Stored State vs. Derived

- **Stored**: `DisposalProfile` (per-agent), item possession (existing), item ground location (existing).
- **Derived**: Carry capacity strain ratio (computed from believed inventory vs. capacity). Never stored.

### H5. Conservation

Items are never created or destroyed by the drop action. The `commit_drop_item` handler transfers possession: item moves from agent inventory to ground at agent's place. Total item count is conserved.

### H6. Partial Failures

If the `drop_item` action is interrupted before commit (e.g., by combat or critical need), the item remains in the agent's inventory. No partial drop state exists — the action is atomic at commit. The agent may reattempt disposal on the next planning cycle.

### H7. Target Patterns and Invariants

- **Golden test**: An agent with a production recipe that generates waste should eventually emit `FreeCarryCapacity` goals and execute `drop_item` actions when capacity is strained. Verify: (1) waste item appears on ground at agent's location, (2) agent's carry load decreases, (3) agent can resume production after dropping.
- **Invariant**: `verify_conservation` must pass after `drop_item` commits — no items created or destroyed.
- **Falsification**: An agent with `capacity_strain_threshold: 1000‰` (always strained) should attempt disposal every cycle if holding waste.

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
