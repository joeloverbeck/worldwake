# S56: Context-Modulated Perception Exposure

## Summary

Replace static `observation_fidelity` with context-modulated perception that accounts for agent state (fatigue, action occupancy) and environmental properties (concealment, place visibility). Currently every observation is a flat probability roll against a fixed per-agent `Permille`. This spec makes observation probability a function of concrete local conditions, creating natural information asymmetry from physical circumstances.

## Phase

Phase 6: Architectural Substrates II

## Status

Draft

## Crates

- `worldwake-core` (exposure types, place/entity concealment components)
- `worldwake-systems` (perception system modulation)

## Dependencies

- E14 (perception & belief system) — completed
- S44 (scenario profile completeness) — completed

## Design Goals

- Observation fidelity is modulated by agent state (fatigue reduces attentiveness, active combat reduces awareness of non-combat events)
- Places and entities can have concealment properties (forest hides better than market square)
- Modulation is multiplicative on the base `observation_fidelity` — the per-agent trait still matters
- No new systems — modulation happens inside the existing perception system tick

## Non-Goals

- Full salience model (attention allocation, interest-based filtering) — deferred
- Topology-based range modifiers (observation across places) — deferred
- Active concealment actions (hiding, disguise) — deferred
- Line-of-sight or spatial geometry — the world is a place graph, not continuous space

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P2 (No Ungrounded Triggers) | Observation modulation comes from concrete state (fatigue, place, action), not drama dials |
| P7 (Locality) | Concealment is a property of the place/entity, observable locally |
| P15 (Knowledge Travels Physically) | Harder-to-observe things are harder to know about — creates natural information asymmetry |
| P16 (Ignorance) | Fatigued or occupied agents miss more, creating ignorance from physical causes |
| P22 (Agent Diversity) | Per-agent base fidelity + per-agent fatigue + per-place concealment = diverse observation outcomes |

## Deliverables

### New Types

```rust
pub struct ObservationContext {
    pub base_fidelity: Permille,          // From PerceptionProfile
    pub fatigue_penalty: Permille,        // Derived from HomeostaticNeeds.fatigue
    pub occupancy_penalty: Permille,      // Derived from active action domain
    pub place_concealment: Permille,      // From PlaceVisibilityProfile
    pub entity_concealment: Permille,     // From target entity (if applicable)
}

impl ObservationContext {
    pub fn effective_fidelity(&self) -> Permille {
        // Multiplicative: base * (1000 - fatigue_penalty) / 1000 * (1000 - occupancy_penalty) / 1000 * (1000 - concealment) / 1000
        // Clamped to [0, 1000]
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

In `observe_passive_local_entities` and `process_witness_event`:

```rust
// Before: flat roll
let observed = rng.gen_permille() <= profile.observation_fidelity;

// After: context-modulated roll
let context = ObservationContext {
    base_fidelity: profile.observation_fidelity,
    fatigue_penalty: fatigue_observation_penalty(agent_fatigue),
    occupancy_penalty: occupancy_observation_penalty(active_action_domain),
    place_concealment: place_visibility.map_or(Permille::ZERO, |p| p.base_concealment),
    entity_concealment: Permille::ZERO,  // Extension point for future hiding
};
let observed = rng.gen_permille() <= context.effective_fidelity();
```

### Fatigue Penalty Function

```rust
fn fatigue_observation_penalty(fatigue: Permille) -> Permille {
    // No penalty below 500‰ fatigue
    // Linear ramp: 0‰ penalty at 500‰ fatigue → 300‰ penalty at 1000‰ fatigue
    if fatigue.value() <= 500 { Permille::ZERO }
    else { Permille::new((fatigue.value() - 500) * 300 / 500) }
}
```

### Occupancy Penalty Function

```rust
fn occupancy_observation_penalty(domain: Option<ActionDomain>) -> Permille {
    match domain {
        Some(ActionDomain::Combat) => Permille::new(400),   // Combat heavily occupies attention
        Some(ActionDomain::Production) => Permille::new(200), // Crafting moderately occupies
        Some(ActionDomain::Travel) => Permille::new(100),    // Travel slightly occupies
        _ => Permille::ZERO,
    }
}
```

### Scenario Integration

`PlaceVisibilityProfile` added to place definitions in scenario files:
- Village Square: `base_concealment: 0` (open, everything visible)
- Forest Path: `base_concealment: 400` (dense, hard to observe)
- Market: `base_concealment: 100` (crowded but open)
- Bandit Camp: `base_concealment: 300` (hidden but not completely)

## Cross-System Interactions

- **Needs system** writes `HomeostaticNeeds.fatigue` → read by perception for penalty calculation
- **Scheduler** provides active action domain → read by perception for occupancy penalty
- **Topology** provides place concealment → read by perception per observation
- **AI planner** is unaffected — reads derived `BelievedEntityState` as before

## Profile-Driven Parameters

- `PlaceVisibilityProfile` is per-place (not per-agent)
- Fatigue and occupancy penalty curves are system functions, not profile parameters (they apply universally)
- `PerceptionProfile.observation_fidelity` remains the per-agent base

## Component Registration

- `PlaceVisibilityProfile` on `EntityKind::Place`

## Section H — Causal Hooks

1. **Information path**: Concealment and fatigue are locally observable properties. No global state queried.
2. **Positive feedback**: Low perception → missed events → surprised by threats → combat → fatigue → even lower perception. Dampened by: combat is finite (wounds, death, flee), fatigue recovers with rest.
3. **Dampeners**: Rest recovers fatigue. Combat ends. Place concealment is fixed (no runaway). Occupancy penalty is bounded per domain.
4. **Stored vs derived**: `PlaceVisibilityProfile` is stored. `ObservationContext` and `effective_fidelity()` are derived at query time.
