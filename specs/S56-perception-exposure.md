# S56: Context-Modulated Perception Exposure

## Summary

Replace static `observation_fidelity` with context-modulated perception that accounts for agent state (fatigue, action occupancy) and environmental properties (concealment, place visibility). Currently every observation is a flat probability roll against a fixed per-agent `Permille`. This spec makes observation probability a function of concrete local conditions, creating natural information asymmetry from physical circumstances.

## Phase

Phase 6: Architectural Substrates II

## Status

Draft

## Crates

- `worldwake-core` (exposure types, place/entity concealment components, `Permille::ZERO` constant)
- `worldwake-sim` (`ActionDef.attention_cost` field)
- `worldwake-systems` (perception system modulation)

## Dependencies

- E14 (perception & belief system) — completed
- S44 (scenario profile completeness) — completed

## Design Goals

- Observation fidelity is modulated by agent state (fatigue reduces attentiveness, active combat reduces awareness of non-combat events)
- Places and entities can have concealment properties (forest hides better than market square)
- Modulation is multiplicative on the base `observation_fidelity` — the per-agent trait still matters
- No new systems — modulation happens inside the existing perception system tick
- Action attention cost is declared per-action on `ActionDef`, not hardcoded per-domain (P8 alignment: actions declare their own occupancy costs)

## Non-Goals

- Full salience model (attention allocation, interest-based filtering) — deferred
- Topology-based range modifiers (observation across places) — deferred
- Active concealment actions (hiding, disguise) — deferred
- Line-of-sight or spatial geometry — the world is a place graph, not continuous space

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P2 (No Ungrounded Triggers) | Observation modulation comes from concrete state (fatigue, place, action attention cost), not drama dials. Attention cost is per-action, not a central designer constant. |
| P7 (Locality) | Concealment is a property of the place/entity, observable locally |
| P8 (Action Occupancy) | Each action declares its `attention_cost` alongside duration, body cost, and other occupancy — attention demand is an action property |
| P11 (Feedback Dampening) | Fatigue → worse perception → missed threats → combat → more fatigue loop is dampened by: combat is finite (wounds, death, flee), fatigue recovers with rest, place concealment is static |
| P15 (Knowledge Travels Physically) | Harder-to-observe things are harder to know about — creates natural information asymmetry |
| P16 (Ignorance) | Fatigued or occupied agents miss more, creating ignorance from physical causes |
| P22 (Agent Diversity) | Per-agent base fidelity + per-agent fatigue + per-place concealment + per-action attention cost = diverse observation outcomes |

## Deliverables

### Permille::ZERO Constant

Add to `Permille` in `crates/worldwake-core/src/numerics.rs`:

```rust
impl Permille {
    pub const ZERO: Permille = Permille(0);
}
```

### ActionDef.attention_cost Field

Add to `ActionDef` in `crates/worldwake-sim/src/action_def.rs`:

```rust
pub struct ActionDef {
    // ... existing fields ...
    pub attention_cost: Permille,  // How much this action occupies perceptual bandwidth (0 = none, 1000 = total)
}
```

Each action handler registration sets `attention_cost` at definition time. Guideline values at launch:
- Combat actions: ~400‰ (combat heavily occupies attention)
- Production actions: ~200‰ (crafting moderately occupies)
- Travel actions: ~100‰ (travel slightly occupies)
- Needs/social/epistemic/generic actions: 0‰ (no perceptual penalty)

These values are per-action, not per-domain — a stealthy ambush action could have lower attention cost than a direct melee, even though both are `ActionDomain::Combat`. New actions declare their own cost at registration without updating any central function.

### New Types

In `crates/worldwake-core`:

```rust
pub struct ObservationContext {
    pub base_fidelity: Permille,          // From PerceptionProfile
    pub fatigue_penalty: Permille,        // Derived from HomeostaticNeeds.fatigue
    pub occupancy_penalty: Permille,      // From active action's ActionDef.attention_cost
    pub place_concealment: Permille,      // From PlaceVisibilityProfile
    pub entity_concealment: Permille,     // From target entity (if applicable)
}

impl ObservationContext {
    pub fn effective_fidelity(&self) -> Permille {
        // Multiplicative: base * (1000 - fatigue_penalty) / 1000 * (1000 - occupancy_penalty) / 1000 * (1000 - concealment) / 1000
        // Clamped to [0, 1000]
        let mut f = u32::from(self.base_fidelity.value());
        f = f * (1000 - u32::from(self.fatigue_penalty.value())) / 1000;
        f = f * (1000 - u32::from(self.occupancy_penalty.value())) / 1000;
        let concealment = u32::from(self.place_concealment.value())
            .max(u32::from(self.entity_concealment.value()));
        f = f * (1000 - concealment) / 1000;
        Permille::new_unchecked(f.min(1000) as u16)
    }
}
```

### New Component

```rust
pub struct PlaceVisibilityProfile {
    pub base_concealment: Permille,  // How hard it is to observe things here (0 = open square, 500 = dense forest)
}
```

### Perception Modulation

#### Signature Changes

`observe_passive_local_entities` and `process_witness_event` must receive active action context. In `crates/worldwake-systems/src/perception.rs`:

```rust
fn observe_passive_local_entities(
    world: &World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    rng: &mut worldwake_sim::DeterministicRng,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,  // NEW
    action_defs: &BTreeMap<ActionDefId, ActionDef>,               // NEW
    updated_stores: &mut BTreeMap<EntityId, AgentBeliefStore>,
) -> BTreeMap<EntityId, DirectLocalObservationBatch>
```

Similarly for `process_witness_event`:

```rust
fn process_witness_event(
    world: &World,
    event_log: &mut EventLog,
    rng: &mut worldwake_sim::DeterministicRng,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,  // NEW
    action_defs: &BTreeMap<ActionDefId, ActionDef>,               // NEW
    updated_stores: &mut BTreeMap<EntityId, AgentBeliefStore>,
    // ... remaining existing params ...
)
```

The parent `perception_system` already destructures `active_actions` and `action_defs` from `SystemExecutionContext` (`perception.rs:40-41`) — thread them to these callees.

#### Observation Check Integration

In `observe_passive_local_entities`, replace the flat fidelity pass-through:

```rust
// Before (current code, perception.rs:239):
// profile.observation_fidelity.value() passed to collect_direct_local_observation_batch

// After: build context and pass effective fidelity
let fatigue_penalty = fatigue_observation_penalty(
    world.get_component_homeostatic_needs(agent)
        .map_or(Permille::ZERO, |n| n.fatigue)
);
let occupancy_penalty = active_attention_cost(agent, active_actions, action_defs);
let place_concealment = world.get_component_place_visibility_profile(place)
    .map_or(Permille::ZERO, |p| p.base_concealment);

let context = ObservationContext {
    base_fidelity: profile.observation_fidelity,
    fatigue_penalty,
    occupancy_penalty,
    place_concealment,
    entity_concealment: Permille::ZERO,  // Extension point for future hiding
};

// Pass to existing passes_observation_check:
passes_observation_check(context.effective_fidelity().value(), rng)
```

Same pattern in `process_witness_event` (`perception.rs:125`).

### Fatigue Penalty Function

```rust
fn fatigue_observation_penalty(fatigue: Permille) -> Permille {
    // No penalty below 500‰ fatigue
    // Linear ramp: 0‰ penalty at 500‰ fatigue → 300‰ penalty at 1000‰ fatigue
    if fatigue.value() <= 500 {
        Permille::ZERO
    } else {
        Permille::new_unchecked((fatigue.value() - 500) * 300 / 500)
    }
}
```

### Active Attention Cost Helper

```rust
fn active_attention_cost(
    agent: EntityId,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    action_defs: &BTreeMap<ActionDefId, ActionDef>,
) -> Permille {
    // Find agent's active action and return its declared attention_cost
    for instance in active_actions.values() {
        if instance.actor == agent {
            if let Some(def) = action_defs.get(&instance.def_id) {
                return def.attention_cost;
            }
        }
    }
    Permille::ZERO
}
```

### Scenario Integration

`PlaceVisibilityProfile` added to place definitions in scenario files:

Add optional field to `PlaceDef` in `crates/worldwake-cli/src/scenario/types.rs`:

```rust
pub struct PlaceDef {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<PlaceTag>,
    #[serde(default)]
    pub visibility_profile: Option<PlaceVisibilityProfile>,
}
```

In `build_topology()` (`crates/worldwake-cli/src/scenario/mod.rs`), after `topology.add_place(place_id, ...)`, set the component via WorldTxn if present:

```rust
if let Some(vis) = &place_def.visibility_profile {
    txn.set_component_place_visibility_profile(place_id, vis.clone())?;
}
```

Example scenario values:
- Village Square: `base_concealment: 0` (open, everything visible)
- Forest Path: `base_concealment: 400` (dense, hard to observe)
- Market: `base_concealment: 100` (crowded but open)
- Bandit Camp: `base_concealment: 300` (hidden but not completely)

## Cross-System Interactions

- **Needs system** writes `HomeostaticNeeds.fatigue` → read by perception for fatigue penalty calculation (state-mediated, P26)
- **Action framework** provides `ActionDef.attention_cost` via action definitions → read by perception for occupancy penalty (state-mediated, P26)
- **Scheduler** provides active action instances → read by perception to look up the agent's current action
- **Topology** provides place concealment via `PlaceVisibilityProfile` → read by perception per observation
- **AI planner** is unaffected — reads derived `BelievedEntityState` as before

## Profile-Driven Parameters

- `PlaceVisibilityProfile` is per-place (not per-agent)
- `ActionDef.attention_cost` is per-action (not per-domain or per-agent) — grounded in the action's nature (P2, P8)
- Fatigue penalty curve is a system function (applies universally): threshold at 500‰ fatigue, linear ramp to 300‰ max penalty. This is derived from fatigue (concrete agent state), not a drama dial.
- `PerceptionProfile.observation_fidelity` remains the per-agent base

## Component Registration

- `PlaceVisibilityProfile` registered on `EntityKind::Place` in `component_schema.rs`

## SystemFn Integration

No new system function. Modulation is integrated into the existing `perception_system` (`crates/worldwake-systems/src/perception.rs:35`). The perception system already receives `SystemExecutionContext` which includes `active_actions` and `action_defs`.

## Golden Tests

### Concealment Reduces Observation Rate

An agent with `observation_fidelity: 800` in a place with `base_concealment: 400` and zero fatigue should have `effective_fidelity = 800 * 600/1000 = 480`. Over a deterministic sequence of observation rolls, this agent should observe significantly fewer entities than the same agent in an open place (`base_concealment: 0`, effective = 800).

### Fatigue Reduces Observation Rate

An agent at 800‰ fatigue gets `fatigue_penalty = (800-500)*300/500 = 180`. With base fidelity 1000 and no other modifiers: `effective = 1000 * 820/1000 = 820`. At 1000‰ fatigue: `effective = 1000 * 700/1000 = 700`. Verify the graduated reduction.

### Attention Cost From Active Action

An agent performing a combat action (`attention_cost: 400`) with base fidelity 1000 in an open place: `effective = 1000 * 600/1000 = 600`. An agent performing a travel action (`attention_cost: 100`): `effective = 1000 * 900/1000 = 900`. Verify different actions yield different observation rates.

### Multiplicative Stacking

An agent with fidelity 800, fatigue 700 (penalty 120), combat action (attention_cost 400), in forest (concealment 400):
`800 * 880/1000 = 704 → 704 * 600/1000 = 422 → 422 * 600/1000 = 253`. Verify the compounding reduction.

## Section H — Causal Hooks

1. **Motivation**: Currently all agents observe with the same flat probability regardless of their physical state or environment. This prevents natural information asymmetry from emerging — a fatigued, distracted agent in dense forest observes as well as a rested, idle agent in an open square. Existing systems cannot produce state-dependent observation variation.

2. **Entities and relations introduced**: `PlaceVisibilityProfile` (component on Place entities, stored), `ObservationContext` (transient derived struct, not stored). `ActionDef.attention_cost` field (extends existing type). `Permille::ZERO` constant (extends existing type).

3. **Actions and processes**: No new actions. Modulation is applied within existing `observe_passive_local_entities` and `process_witness_event` functions during the perception system tick.

4. **Information path**: Concealment and fatigue are locally observable properties. No global state queried. The perception system reads `HomeostaticNeeds` (agent's own state), `PlaceVisibilityProfile` (agent's current place), and `ActionDef.attention_cost` (agent's active action). All data is local to the observing agent's context.

5. **Quantities conserved**: N/A — no conserved quantities introduced.

6. **Scarce capacities**: N/A — no new exclusive affordances or contention mechanisms.

7. **Partial failures**: Observation is already pass/fail probabilistic. The modulation shifts the probability, not the outcome granularity. A "nearly passed" check has no residual effect — this is consistent with the existing perception model.

8. **Positive feedback**: Low perception → missed events → surprised by threats → combat → fatigue → even lower perception. This is a genuine amplifying loop.

9. **Dampeners**: Rest recovers fatigue (finite duration). Combat ends through wounds, death, or fleeing (finite process). Place concealment is static — no runaway. Attention cost is bounded by the action's declared value and the action's finite duration. The multiplicative model means each penalty factor cannot reduce fidelity below zero.

10. **Agent learning**: N/A — no learning, habit, or preference updates introduced by this spec.

11. **How agents can be wrong**: Agents cannot be wrong about their own fatigue (authoritative state on self). Place concealment is an environmental property that silently modulates observation probability — agents do not perceive concealment levels, they simply observe more or less. This is consistent with P16 (ignorance from physical causes).

12. **Lifecycle states**: `PlaceVisibilityProfile` has no lifecycle — it is a static property of a place. `ObservationContext` is transient, constructed and consumed within a single perception tick.

13. **Temporal resolution**: Concealment, fatigue, and attention cost are sampled at every observation roll during the perception system tick. The perception system runs once per tick per the existing schedule. No simultaneity or tie-breaking concerns — observation rolls are per-agent, per-entity, independent.

14. **Boundary conditions**: N/A — no external drivers or off-map interfaces.

15. **Derived views**: `ObservationContext` and `effective_fidelity()` are derived at query time from authoritative state (`PerceptionProfile`, `HomeostaticNeeds`, `PlaceVisibilityProfile`, `ActionDef`). Never stored.

16. **Causal records**: `PerceptionTraceEvent` already records observation pass/fail per entity. The modulated effective fidelity should be included in trace output so debugging can distinguish "missed because low base fidelity" from "missed because fatigued in concealed location."

17. **Validation patterns**: See Golden Tests section. Key invariants: (a) effective fidelity ≤ base fidelity (penalties can only reduce), (b) zero base fidelity → zero effective fidelity regardless of other factors, (c) all penalties at zero → effective equals base, (d) multiplicative stacking produces expected numerical results.

18. **Save/load**: `PlaceVisibilityProfile` is a stored component — survives save/load via standard component serialization. `ObservationContext` is transient — not saved. `ActionDef.attention_cost` is part of action definitions (code-defined, not save state). `Permille::ZERO` is a constant.
