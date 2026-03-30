**Status**: ✅ COMPLETED

# E20: Travel Physiology & Need Fallbacks

## Epic Summary

Add physiological costs to travel (fatigue, thirst, bladder escalation via per-agent multipliers on `MetabolismProfile`), introduce a wilderness relief action as a dirtiness-penalized alternative to latrines, and let the GOAP planner discover fallback relief paths naturally through affordance evaluation. All agents — not just "companions" — benefit from these mechanics. No hardcoded fallback chains; the planner's cost-ordered search produces emergent behavior variety.

## Phase

Phase 4: Group Adaptation, CLI & Verification

## Crates

- `worldwake-core` (new fields on `MetabolismProfile`, new `EventTag` variants)
- `worldwake-sim` (body cost resolution for travel, action definition updates)
- `worldwake-systems` (new `relieve_wilderness` action + handler, travel body cost wiring)
- `worldwake-ai` (expand `goal_relevant_places` for `GoalKind::Relieve`)

## Dependencies

- E09 (needs & metabolism — provides `HomeostaticNeeds`, `MetabolismProfile`, `DeprivationExposure`, existing `toilet`/`wash` actions)
- E13 (decision architecture — provides GOAP planner, goal ranking, interrupt evaluation)
- E14 (perception & belief — provides witness observation, `VisibilitySpec` processing)

## FOUNDATIONS Alignment

- **Principle 1, Maximal Emergence**: No authored fallback sequences. The planner discovers relief paths through affordance evaluation. Latrine, wilderness, or deprivation accident — the outcome depends on world state, not a scripted chain.
- **Principle 3, Concrete State Over Abstract Scores**: No "social standing" score, no "embarrassment" metric. Social consequences are concrete: waste entities at locations, dirtiness on agents, witnessed events propagated as beliefs.
- **Principle 7, Locality**: Witnesses observe relief events only if co-located (`VisibilitySpec::SamePlace`). Beliefs propagate through Tell action, not global state mutation.
- **Principle 8, Preconditions/Duration/Cost/Occupancy**: Travel now has physiological cost (fatigue, thirst, bladder). Wilderness relief has duration, dirtiness penalty, and waste production. Nothing is free.
- **Principle 11, Physical Dampeners**: Needs escalate linearly from basal rates × multipliers. No amplifying loops introduced. Bladder is relieved by explicit action or involuntary accident — both produce waste and end the escalation.
- **Principle 19, Agent Symmetry**: These mechanics apply to all agents with `HomeostaticNeeds`. The engine makes no distinction between "companions" and other agents. Any agent traveling gets body costs; any agent with critical bladder can use wilderness relief.
- **Principle 20, Resource-Bounded Practical Reasoning**: Agents discover relief options through GOAP search, not hardcoded behavior trees. The search budget limits how many alternatives they explore — per-agent via `ReasoningProfile` (S42).
- **Principle 22, Agent Diversity**: Travel exertion multipliers on `MetabolismProfile` mean different agents respond differently to travel. An older merchant tires faster than a young guard. A well-hydrated agent's bladder fills slower than a dehydrated one's.
- **Principle 26, Systems Interact Through State**: Travel body costs produce need escalation → needs system processes thresholds → AI candidate generation reads needs → planner searches for relief actions. No system directly invokes another.

## Design Goals

1. **Travel has physiological cost**: Travel's `BodyCostPerTick` is no longer zero. Per-agent multipliers on `MetabolismProfile` control how much fatigue, thirst, and bladder increase per tick of travel.
2. **Wilderness relief as fallback affordance**: A new `relieve_wilderness` action allows relief at outdoor places with a dirtiness penalty. The planner naturally prefers latrines (no penalty) over wilderness (penalty).
3. **Emergent fallback through search**: No authored chain. The planner's cost-ordered search discovers the best available option. If nothing is available, the existing deprivation consequence system handles the accident.
4. **Social consequences via perception**: Witnessed events flow through the existing perception pipeline (E14). No new impression or reputation components.
5. **Backward compatible defaults**: `MetabolismProfile::default()` must include travel multipliers that produce the current zero-cost behavior (multiplier = `Permille(0)`) so existing tests pass without modification.

## Deliverables

### 1. Travel Exertion Multipliers on `MetabolismProfile`

Add three new fields to `MetabolismProfile` in `worldwake-core/src/needs.rs`:

```rust
/// Multiplier applied to basal fatigue_rate during travel.
/// Permille(0) = no travel fatigue (backward compatible default).
/// Permille(500) = travel adds 50% of basal fatigue rate per tick.
/// Permille(1000) = travel doubles basal fatigue rate per tick.
pub travel_fatigue_multiplier: Permille,

/// Multiplier applied to basal thirst_rate during travel.
pub travel_thirst_multiplier: Permille,

/// Multiplier applied to basal bladder_rate during travel.
pub travel_bladder_multiplier: Permille,
```

**Body cost computation**: The travel action's per-tick body cost for a given need is:

```
additional_cost = basal_rate * travel_multiplier / 1000
```

This is additive on top of the basal rate already applied by `needs_system`. For example, if `bladder_rate = Permille(10)` and `travel_bladder_multiplier = Permille(500)`, travel adds `10 * 500 / 1000 = 5` additional bladder permille per tick of travel, for a total of 15‰ per tick.

**Default values**: All multipliers default to `Permille(0)` for backward compatibility. Prototype world builders and golden tests should set non-zero values to exercise the feature.

### 2. Travel Body Cost Resolution

The travel action's `BodyCostPerTick` must be resolved dynamically from the actor's `MetabolismProfile`. Two approaches:

**Option A — New `BodyCostExpr` enum**: Add a variant like `ActorTravelExertion` to a body cost expression type (analogous to `DurationExpr`), resolved at action start from the actor's `MetabolismProfile`. The resolved `BodyCostPerTick` is stored in the `ActionInstance` and applied each tick by `needs_system`.

**Option B — Resolve in start handler**: The `start_travel` handler reads the actor's `MetabolismProfile`, computes the `BodyCostPerTick`, and stores it in the `ActionInstance.body_cost_override` (new field or via `ActionState`). The needs system uses the override instead of the static def value.

Either approach is acceptable. The key constraint: the body cost must be resolved from the actor's profile at action start and remain fixed for the action's duration (no per-tick re-reads of the profile).

**Files**:
- `crates/worldwake-sim/src/action_def.rs` — mechanism for dynamic body cost resolution
- `crates/worldwake-systems/src/travel_actions.rs` — wire travel action to use actor's MetabolismProfile

### 3. Wilderness Relief Action

New action `relieve_wilderness` registered in `worldwake-systems/src/needs_actions.rs`:

| Property | Value |
|----------|-------|
| Name | `"relieve_wilderness"` |
| Domain | `ActionDomain::Needs` |
| Actor constraints | `ActorAlive`, `ActorNotIncapacitated`, `ActorNotInTransit` |
| Place constraint | Actor at place with any outdoor tag: `Forest`, `Trail`, `Field`, `Farm`, or `Road` |
| Targets | None |
| Duration | `ActorMetabolism { kind: MetabolismDurationKind::Toilet }` |
| Body cost | `BodyCostPerTick::zero()` |
| Interruptibility | `InterruptibleWithPenalty` |
| Visibility | `VisibilitySpec::SamePlace` |
| Commit effects | Bladder → `Permille(0)`, dirtiness += `wilderness_relief_dirtiness_penalty`, `Waste` entity created at place |
| Event tags | `EventTag::WildernessRelief` (new) |

**New field on `MetabolismProfile`**:

```rust
/// Dirtiness penalty (additive permille) applied when relieving in the wilderness
/// instead of at a proper latrine. Per-agent diversity: fastidious agents
/// may have higher penalties (they feel dirtier), pragmatic agents lower.
pub wilderness_relief_dirtiness_penalty: Permille,
```

**Place constraint design**: The action is available at any place that has at least one "outdoor" tag. Rather than introducing an `Outdoor` meta-tag, the constraint checks for the presence of any tag in the set `{Forest, Trail, Field, Farm, Road}`. This avoids adding a meta-tag to every existing place while covering all outdoor locations in the prototype world. If future place tags are added (e.g., `Meadow`, `Mountain`), they should be added to this set if they represent outdoor locations.

**Handler implementation**: Follows the same pattern as existing `commit_toilet` in `needs_actions.rs`:

```rust
fn commit_relieve_wilderness(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let actor = instance.actor;
    let mut needs = txn.homeostatic_needs(actor)?;
    let profile = txn.metabolism_profile(actor)?;

    // Relieve bladder.
    needs.bladder = Permille(0);

    // Apply dirtiness penalty.
    needs.dirtiness = needs.dirtiness.saturating_add(profile.wilderness_relief_dirtiness_penalty);

    txn.set_homeostatic_needs(actor, needs)?;

    // Create waste at actor's location.
    let place = txn.current_place(actor)?;
    let waste = txn.create_item_lot(CommodityKind::Waste, Quantity(1))?;
    txn.set_ground_location(waste, place)?;

    Ok(CommitOutcome::default())
}
```

### 4. Planner Integration

**`PlannerOpKind` mapping** (`worldwake-ai/src/planner_ops.rs`):

```rust
(ActionDomain::Needs, "relieve_wilderness") => Some(PlannerOpKind::Relieve),
```

Both `toilet` and `relieve_wilderness` map to `PlannerOpKind::Relieve`. The planner treats them as interchangeable means to the same end.

**`goal_relevant_places` expansion** (`worldwake-ai/src/goal_model.rs`, line ~993):

Currently:
```rust
GoalKind::Relieve => places_with_place_tag(state, PlaceTag::Latrine),
```

Must expand to:
```rust
GoalKind::Relieve => {
    let mut places = places_with_place_tag(state, PlaceTag::Latrine);
    for tag in OUTDOOR_RELIEF_TAGS {
        places.extend(places_with_place_tag(state, *tag));
    }
    places.sort_unstable();
    places.dedup();
    places
}
```

Where `OUTDOOR_RELIEF_TAGS = &[PlaceTag::Forest, PlaceTag::Trail, PlaceTag::Field, PlaceTag::Farm, PlaceTag::Road]`.

**Natural preference ordering**: The planner prefers latrines because:
1. The `toilet` action has no secondary cost (dirtiness unchanged). The `relieve_wilderness` action increases dirtiness, which creates downstream pressure (a `Wash` goal will eventually be needed). The planner doesn't model this secondary cost directly, but...
2. If the agent is already at or near a latrine, the travel cost is lower.
3. The search explores shorter plans first (A* with travel-cost heuristic).

**No new goal kinds**: `GoalKind::Relieve` already covers this. No `SeekPrivacy` or `WildernessRelief` goal needed.

### 5. EventTag Additions

Add to `worldwake-core/src/event_tag.rs`:

```rust
/// Agent relieved themselves outdoors (not at a latrine).
WildernessRelief,
/// Agent's bladder reached deprivation threshold — involuntary accident.
BladderAccident,
```

These tags allow the perception system to categorize events for belief formation. Witnesses who observe a `WildernessRelief` or `BladderAccident` event can form beliefs about the agent's state and behavior.

### 6. Social Consequences via Existing Perception

No new social systems are introduced. The perception pipeline (E14) handles social consequences:

1. `relieve_wilderness` commits with `VisibilitySpec::SamePlace` → perception system resolves witnesses
2. Witnesses at the same place observe the event (subject to `observation_fidelity` probability)
3. Witnesses form beliefs about the event (event tag, actor, place, tick)
4. The Tell action (E15) allows witnesses to propagate beliefs to other agents
5. Concrete world state changes (waste at location, dirtiness on agent) are independently observable by any agent who visits the place or interacts with the actor

**What this means in practice**: If an agent relieves in the wilderness with no one around, there's no social consequence — only physical (waste, dirtiness). If others are present, they observe and may tell others. The social consequence is the belief, not a score.

## Emergent Scenario Walkthrough

**Agent traveling, bladder escalating**:

1. Agent departs village for distant market. Travel action begins.
2. `needs_system` applies basal bladder rate + travel bladder cost each tick. Bladder rises faster than when stationary.
3. Bladder crosses `thresholds.bladder.high()`. Candidate generation emits `GoalKind::Relieve`.
4. Ranking: `Relieve` has `Medium` or `High` priority. If current action is `FreelyInterruptible`, interrupt. If `InterruptibleWithPenalty` (travel), wait for `Critical`.
5. Bladder crosses `thresholds.bladder.critical()`. Priority becomes `Critical`. Interrupt evaluation returns `InterruptForReplan { trigger: CriticalSurvival }`.
6. Travel interrupted. `abort_travel` returns agent to origin place.
7. Planner searches for `GoalKind::Relieve`:
   - **If origin has latrine**: `toilet` action found → plan `[toilet]` → execute. Bladder → 0, no dirtiness penalty.
   - **If origin is outdoor (Forest, Road, etc.)**: `relieve_wilderness` found → plan `[relieve_wilderness]` → execute. Bladder → 0, dirtiness += penalty, waste created.
   - **If origin is indoor non-latrine (Inn, Hall)**: No relief action available here. Search for nearby places → plan `[travel to latrine]` or `[travel to outdoor place]`.
   - **If no reachable relief option**: Plan search exhausts → no plan → bladder continues rising → deprivation accident (existing `needs_system` consequence).
8. After relief, agent replans. May resume travel to market.

## Principle 30 — Causal Hook Declarations

### 1. New Entities, Relations, and Records

- **Modified component**: `MetabolismProfile` gains `travel_fatigue_multiplier`, `travel_thirst_multiplier`, `travel_bladder_multiplier`, and `wilderness_relief_dirtiness_penalty` fields (all `Permille`).
- **New EventTag variants**: `WildernessRelief`, `BladderAccident`.
- **Waste entity**: Already exists (`CommodityKind::Waste`). Created by `relieve_wilderness` commit handler.

No new entity kinds, relation types, or record types.

### 2. Actions and World Processes That Mutate State

- **`travel` (modified)**: Now applies per-tick body costs (fatigue, thirst, bladder) resolved from actor's `MetabolismProfile`.
- **`relieve_wilderness` (new)**: Sets bladder to 0, increases dirtiness by penalty, creates Waste entity at place.

### 3. Information Production, Travel, and Observation

- `relieve_wilderness` events emitted with `VisibilitySpec::SamePlace`. Co-located agents observe via perception system.
- `BladderAccident` events (existing deprivation consequence) — if not already tagged, should be tagged for categorization.
- Waste entity at location is observable by any agent who visits the place.
- Dirtiness on actor is observable through direct interaction or perception.
- Belief propagation via Tell action (E15).

### 4. Quantities Conserved, Transferred, Transformed, Created, or Destroyed

- **Waste**: Created by `relieve_wilderness` (source: action commit, explicit creation). One `Waste` unit per relief event. Same accounting as existing `toilet` action and deprivation accident.
- **Needs values**: Bladder decreased by relief (not a conserved quantity — consumed by the body). Dirtiness increased by penalty (not conserved — represents state change).
- No transfers, transformations, or destruction.

### 5. Scarce Capacities, Exclusive Affordances, Reservations, Queues, or Claims

None introduced. Wilderness relief has no capacity limit (outdoor places don't run out of space). Latrines may have occupancy limits if facility queuing (E10) is applied, but that's pre-existing.

### 6. Partial Failures and Aftermath States

- **Travel interrupted**: Agent returns to origin (existing `abort_travel` behavior). Need remains unrelieved — must replan.
- **Relief interrupted**: If `relieve_wilderness` is interrupted mid-action (rare — only by `CriticalSurvival` or `CriticalDanger`), bladder is NOT relieved. The action's effects are commit-only.
- **No relief option found**: Plan search exhausts → agent has no plan → bladder continues rising → deprivation accident produces waste + max dirtiness (existing behavior).

### 7. Positive Feedback Loops

None introduced. Need escalation is linear (basal rate × multiplier per tick). Relief actions terminate the escalation. There is no mechanism by which relieving increases the rate of future need escalation.

### 8. Physical Dampeners

N/A — no positive feedback loops to dampen.

The only loop-like dynamic is: travel → need escalation → interrupt travel → relieve → resume travel → need escalation again. This is naturally dampened by:
- **Travel time**: The agent moves closer to the destination each time, reducing future travel duration.
- **Need rates**: After relief, the need starts from zero and takes time to escalate again.
- **Action duration**: Relief takes time, during which no travel progress occurs.

### 9. Derived Views and Optimizations

None. All state is authoritative:
- `MetabolismProfile` fields: authoritative per-agent parameters.
- `HomeostaticNeeds` values: authoritative per-agent state.
- Waste entities: authoritative world state.
- Beliefs about events: authoritative per-agent belief state (via E14).

### 10. How Agents Can Become Wrong

- An agent may believe a latrine exists at a location (stale belief from prior visit) but find it destroyed or occupied upon arrival → plan fails → replanning occurs.
- An agent may not know about a nearby latrine (never visited, no one told them) → planner only considers known places → may use wilderness relief unnecessarily.
- Witnesses may fail to observe the event (perception fidelity check fails) → no belief formed → no social propagation.

Provenance/freshness: Belief about latrine location carries acquisition tick from E14 perception. Agent can reason about staleness.

### 11. Temporal and Spatial Resolution

- **Tick resolution**: Standard. Body costs applied per tick by `needs_system`. Relief actions have tick-denominated duration from `MetabolismProfile`.
- **Scheduling**: Needs system runs before Perception in the tick execution order. Body costs from travel are applied in the same tick as the travel action progresses. If travel is interrupted, the interrupt occurs during AI tick processing, before the next `needs_system` pass.
- **Tie-breaking**: No new simultaneous access issues. If two agents need the same latrine simultaneously, existing facility queuing handles it.

### 12. Boundary Conditions and External Drivers

None specific to this epic. Travel body costs apply uniformly regardless of route origin.

### 13. Target Patterns, Invariants, and Falsification Checks

**Invariants**:
- Need continuity: bladder is never silently reset. Every reset traces to a `toilet`, `relieve_wilderness`, or deprivation consequence.
- Off-camera continuity: travel body costs apply whether or not a human is observing.
- Conservation: every Waste entity has an explicit creation event.
- Agent symmetry: no agent type is exempt from travel body costs or fallback behavior.

**Falsification checks**:
- If an agent completes a long travel with zero bladder increase → travel body cost is broken.
- If a `relieve_wilderness` produces no Waste entity → conservation violated.
- If a `relieve_wilderness` has no dirtiness increase → penalty not applied.
- If witnesses at the same place don't observe the event → visibility or perception broken.

### 14. Save/Load and Replay Survival

All new state is on existing serializable components:
- `MetabolismProfile` (already `Serialize`/`Deserialize`): new fields are `Permille` (already serializable).
- `HomeostaticNeeds` (already serializable): no changes.
- Waste entities: standard entity creation, already handled by save/load.
- `EventTag` variants: already serializable enum.

Replay determinism: travel body costs are resolved from deterministic profile values. No new RNG consumption.

## FND-01 Section H — Future System Analysis

### H.1 Information-Path Analysis

```
MetabolismProfile.travel_*_multiplier (stored, per-agent)
  → needs_system reads profile + active action body cost
  → HomeostaticNeeds.bladder increases (stored, per-agent)
  → candidate_generation reads needs + thresholds
  → GoalKind::Relieve emitted at high threshold
  → ranking assigns priority class based on threshold band
  → evaluate_interrupt checks priority vs current action interruptibility
  → (if critical) interrupt_action aborts travel
  → search_plan explores Relieve goal
  → relieve_wilderness or toilet action started
  → commit handler: bladder → 0, dirtiness += penalty, waste created
  → event emitted with VisibilitySpec::SamePlace
  → perception_system resolves witnesses at place
  → witnesses form beliefs about event (subject to observation_fidelity)
  → Tell action (E15) enables belief propagation
```

Every step traces to a prior step. No information teleportation.

### H.2 Positive-Feedback Analysis

No amplifying loops. Need escalation is linear and terminates upon relief or accident. Travel body costs are additive, not multiplicative with other costs.

### H.3 Concrete Dampeners

N/A — no positive feedback loops identified. The travel-interrupt-relieve-resume cycle is self-limiting: travel distance decreases, need starts from zero after relief, and relief action duration prevents immediate resumption.

### H.4 Stored State vs. Derived Read-Model

| Item | Authoritative / Derived |
|------|------------------------|
| `MetabolismProfile.travel_*_multiplier` | Authoritative (per-agent component) |
| `MetabolismProfile.wilderness_relief_dirtiness_penalty` | Authoritative (per-agent component) |
| `HomeostaticNeeds.bladder` | Authoritative (per-agent component) |
| `HomeostaticNeeds.dirtiness` | Authoritative (per-agent component) |
| Waste entity at place | Authoritative (world state) |
| Bladder urgency level (low/medium/high/critical) | Derived (from needs value + threshold band) |
| Candidate goal list | Derived (from needs + beliefs + thresholds) |

## Component Registration

### Modified Components

- **`MetabolismProfile`** (already registered on `EntityKind::Agent`): Four new `Permille` fields. Default values: all `Permille(0)` for backward compatibility.

### New EventTag Variants

- `EventTag::WildernessRelief`
- `EventTag::BladderAccident`

No new component types or relation types.

## SystemFn Integration

No new system functions. All behavior flows through existing systems:

- **`needs_system`** (existing): Applies body costs from active actions, including travel's now-nonzero body cost. Handles deprivation consequences (accident) at critical threshold.
- **Perception system** (existing, E14): Processes `SamePlace` visibility for `relieve_wilderness` events.
- **Action framework** (existing): Registers and executes `relieve_wilderness` like any other needs action.
- **GOAP planner** (existing, E13): Searches for `GoalKind::Relieve` with expanded `goal_relevant_places`.

## Tests

### Unit Tests

- [ ] `MetabolismProfile` default has all travel multipliers at `Permille(0)`
- [ ] Travel body cost resolves correctly from MetabolismProfile multipliers
- [ ] `relieve_wilderness` constraint rejects indoor places (Inn, Hall, Barracks, Store)
- [ ] `relieve_wilderness` constraint accepts outdoor places (Forest, Trail, Field, Farm, Road)
- [ ] `relieve_wilderness` commit: bladder → 0, dirtiness += penalty, Waste created at place
- [ ] `relieve_wilderness` maps to `PlannerOpKind::Relieve`
- [ ] `goal_relevant_places` for `Relieve` returns both latrine and outdoor places

### Golden Tests

- [ ] **T-TravelEscalation**: Agent travels a multi-tick route with non-zero travel multipliers. Fatigue, thirst, and bladder increase faster than basal rate alone.
- [ ] **T-TravelInterrupt**: Agent with high bladder starts travel. Bladder reaches critical during travel. Travel is interrupted. Agent replans for relief.
- [ ] **T-LatrinePreferred**: Agent at village with latrine has high bladder. Planner chooses `toilet` (no dirtiness penalty), not `relieve_wilderness`.
- [ ] **T-WildernessFallback**: Agent at forest place with no latrine has high bladder. Planner chooses `relieve_wilderness`. Waste created. Dirtiness increases.
- [ ] **T-DeprivationAccident**: Agent with critical bladder, no relief option available, tolerance exceeded. Deprivation accident occurs (existing behavior). Waste created. Maximum dirtiness applied.
- [ ] **T-WitnessObservation**: Agent relieves in wilderness with another agent co-located. Co-located agent (with `PerceptionProfile`) forms belief about the event.
- [ ] **T-NoWitness**: Agent relieves in wilderness alone. No beliefs formed in other agents.
- [ ] **T-AgentDiversity**: Two agents with different `travel_bladder_multiplier` values travel the same route. Agent with higher multiplier reaches critical bladder sooner.
- [ ] **T-NeedContinuity**: After any relief (toilet, wilderness, accident), bladder is exactly `Permille(0)` and dirtiness reflects the appropriate penalty. No silent partial resets.

## Acceptance Criteria

- Travel has measurable physiological cost via per-agent multipliers
- Agents discover relief fallbacks through GOAP search, not authored sequences
- Wilderness relief is a real action with preconditions, duration, cost, and aftermath
- Material consequences (waste, dirtiness) persist in world state
- Social consequences flow through existing perception pipeline
- No agent type receives special treatment — agent symmetry preserved
- All existing tests continue to pass (backward compatible defaults)

## Spec References

- Section 1 (exemplar scenario 4: companion bodily needs)
- Section 4.4 (needs: bladder, hygiene)
- Section 7.5 (physiological/social propagation)
- Section 9.16 (need continuity)
- FOUNDATIONS.md Principles 1, 3, 7, 8, 11, 19, 20, 22, 26, 30

## Outcome

**Completion date**: 2026-03-30

**What was delivered** (across tickets E20COMBEH-001 through E20COMBEH-008):
- Travel exertion multipliers on `MetabolismProfile` (`travel_fatigue_multiplier`, `travel_thirst_multiplier`, `travel_bladder_multiplier`) with backward-compatible defaults of `Permille(0)`
- Dynamic body cost resolution in the travel action start handler
- `relieve_wilderness` action with outdoor place constraint, dirtiness penalty, waste production, `VisibilitySpec::SamePlace`, and `EventTag::WildernessRelief`
- `wilderness_relief_dirtiness_penalty` field on `MetabolismProfile`
- `EventTag::WildernessRelief` and `EventTag::BladderAccident` variants
- `OUTDOOR_RELIEF_TAGS` constant in topology for outdoor place detection
- Expanded `goal_relevant_places` for `GoalKind::Relieve` to include outdoor places
- Golden E2E tests: travel escalation (Scenario 58), critical bladder local relief (59), agent diversity (60), travel interrupt from bladder escalation (61), latrine preferred (62), wilderness fallback (63), deprivation accident (64), witness observation (65), no-witness isolation (66), need continuity across all relief paths (67a-c)
- Focused unit tests for action registration, outdoor/indoor place constraints, commit effects, visibility, and event tags

**Deviations from spec**:
- Witness/no-witness golden tests (E20COMBEH-008) use perception traces rather than `SocialObservation` belief entries, because the spec explicitly chose "No new impression or reputation components." The perception pipeline processes the SamePlace event but does not form a typed social observation for WildernessRelief.
