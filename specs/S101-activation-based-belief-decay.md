# S101: Activation-Based Belief Decay

## Summary

Replace the hard-capacity entity eviction system in `AgentBeliefStore` with ACT-R base-level activation decay. Currently, agents have a fixed `entity_memory_capacity` (default 12) with tiered eviction that arbitrarily forgets items before infrastructure. This produces brittle failure modes where agents "forget" survival-critical items at their feet because their memory slots are filled with places and facilities. The replacement computes memory activation as a power-law decay over observation history — recently and frequently observed entities persist, neglected ones fade gradually, and no hard cap exists. Memory size emerges from the agent's actual experience patterns.

Additionally, replace the `entity_claim_capacity` hard cap with confidence-threshold pruning, and add need-gated salience so item-kind entities resist decay when survival needs are critical.

## Phase

Core infrastructure (belief system overhaul)

## Status

Draft

## Crates

- `worldwake-core` (BelievedEntityState, PerceptionProfile, HomeostaticNeeds, activation computation, pruning logic)
- `worldwake-systems` (call site updates in perception.rs, epistemic_actions.rs, tell_actions.rs)
- `worldwake-ai` (golden tests)

## Dependencies

None — this replaces existing infrastructure with no new system dependencies.

## Problem Statement

### Evidence

Observer run on `scenarios/cli-evaluation.ron` (seed 7777, 600 ticks) produced the following end-state belief summaries:

| Agent | Capacity | Known Entities | Items Known | Behavior |
|-------|----------|---------------|-------------|----------|
| Kael | 16 (custom) | 16 | 7 | Healthy |
| Guard Theron | 16 (custom) | 16 | 7 | Healthy |
| Merchant Vara | 12 (default) | 12 | 3 | Struggling |
| Forager Lina | 12 (default) | 12 | **0** | Starving, 217 consecutive idle ticks |

Forager Lina visited 3 places and observed 5 facilities: `4 agents + 3 places + 5 facilities = 12 tier-1 entities = full capacity`. Every item observation was immediately evicted (tier 0). At tick 248 she arrived at Dusty Trail with `pick_up (3 targets)` — she could see items. After visiting Thornwall Village (tick 279, gaining 3 more tier-1 entities), she returned to find zero `pick_up` affordances. Items were permanently invisible despite being physically present.

### Architectural violations

- **FND-11**: The hard cap is a numeric clamp acting as a dampener, not a physical world process
- **FND-3**: The capacity number is an abstract parameter with no concrete-state basis for its value
- **FND-16**: The binary in-window/out-window retention check does not support gradual decay — entities are fully remembered then instantly forgotten
- **FOUNDATIONS preamble**: Raising the cap from 12 to 16 would be "a localized fix that avoids the real problem"

## Design Goals

- Memory retention emerges from observation patterns (frequency, recency), not from an authored capacity number
- Graceful degradation: memories fade gradually instead of vanishing at a hard boundary
- Survival-critical items resist decay during crises through need-gated salience
- Per-agent diversity through concrete profile parameters (threshold, buffer size, salience)
- All computation uses integer arithmetic (Permille, isqrt) — no floats, fully deterministic

## Non-Goals

- Variable decay exponent — d=0.5 is the cognitive science standard and is fixed. Diversity comes from threshold, buffer size, and salience parameters
- Commodity-specific salience mapping — determining which items satisfy which needs would couple the belief system to the consumption system. All items get a coarse boost during crises (FND-26 compliant)
- Episodic memory or autobiographical recall — this spec covers entity retention only, not narrative memory
- Forgetting curve visualization or debug tooling — deferred

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | Memory capacity emerges from observation patterns, not from an authored number |
| FND-2 (No Ungrounded Triggers) | Decay is a function of concrete tick history, not a drama lever |
| FND-3 (Concrete State) | Activation is derived on-demand from stored tick buffer, never stored as authoritative state. FND-22A explicitly permits agent-local derived summaries |
| FND-7 (Locality) | No change to information acquisition — perception, reports, tell still require co-location or physical carriers |
| FND-11 (Physical Dampeners) | Removes numeric cap. Decay is bounded by physical world: re-observation requires co-location, which requires travel, which takes time |
| FND-14 (World State vs Belief State) | Activation is private cognitive state. Agents cannot observe each other's memory strength |
| FND-16 (Ignorance and Uncertainty) | Beliefs decay gradually. Frequently reinforced beliefs persist; neglected ones fade |
| FND-22 (Agent Diversity) | Four per-agent parameters: activation threshold, buffer capacity, salience boost, salience urgency threshold |
| FND-22A (Learning as Concrete State) | Presentation tick buffer has accountable origin (observation events), scope (per-agent), and decay (power-law) |
| FND-26 (Systems Through State) | Salience reads need state from agent components — no calls into other systems |

## Data Structures

### BelievedEntityState

**Remove**: `observed_tick: Tick`

**Add**: `presentation_ticks: [Tick; 8]` + `presentation_tick_count: u8` — fixed-size ring buffer of the last N ticks when this entity's beliefs were updated (via direct observation, report, or tell). The effective capacity is `observation_buffer_capacity` from the agent's PerceptionProfile (max 8). No external crate dependency needed.

A derived accessor `last_observed_tick() -> Option<Tick>` returns the most recent entry for backward compatibility.

### PerceptionProfile

**Remove**:
- `entity_memory_capacity: u32`
- `entity_claim_capacity: u32`
- `memory_retention_ticks: u64`
- `infrastructure_retention_ticks: u64`

**Add**:
- `entity_activation_threshold: Permille` — entities and social observations pruned when activation falls below this. Default: `Permille(100)`
- `claim_confidence_threshold: Permille` — claims pruned when staleness-adjusted confidence falls below this. Default: `Permille(50)`
- `observation_buffer_capacity: u8` — ring buffer size per entity. Default: `5`. Diversity lever: 3 = short attention, 8 = strong memory
- `need_salience_boost: Permille` — activation bonus for item-kind entities when agent has critical needs. Default: `Permille(500)`
- `need_salience_urgency_threshold: Permille` — need value above which salience boost activates. Default: `Permille(500)`

### HomeostaticNeeds

**Add**: `max_value(&self) -> u16` — returns the maximum `.value()` across all five need fields (hunger, thirst, fatigue, bladder, dirtiness). Used by salience boost computation.

### Unchanged

- `observation_fidelity: Permille` — per-tick observation probability
- `confidence_policy: BeliefConfidencePolicy` — claim confidence decay parameters
- `institutional_memory_capacity: u32` — institutional belief capacity
- `consultation_speed_factor: Permille` — consultation speed modifier
- `contradiction_tolerance: Permille` — contradiction handling threshold
- All claim-level structures (`EntityBeliefClaim`, source, confidence, acquired_tick)

## Activation Computation

### Formula

For a known entity with presentation ticks `[t_0, t_1, ..., t_n]` at current tick `T`:

```
activation(T) = Σ floor(1000 / sqrt(max(1, T - t_j)))
```

ACT-R base-level activation with decay parameter d=0.5, scaled to Permille. The implementation uses scaled integer square root (`u64::isqrt()`, stable since Rust 1.84) to compute the floored square-root result deterministically without floats.

For social observations with a single `observed_tick`, this simplifies to:

```
activation(T) = floor(1000 / sqrt(max(1, T - observed_tick)))
```

### Reference values (single observation)

| Age (ticks since observation) | Activation contribution |
|-------------------------------|------------------------|
| 1 (just seen) | 1000 |
| 4 | 500 |
| 16 | 250 |
| 48 | 144 |
| 100 | 100 (= default threshold) |
| 400 | 50 |

### Reference values (5 observations at ages 5, 15, 25, 35, 45)

`447 + 258 + 200 + 169 + 149 = 1223` — well above any reasonable threshold. Frequently-visited locations persist indefinitely.

### Ring buffer update

When new information about an entity arrives (via `record_entity_snapshot_claims`), push the current tick onto the entity's ring buffer. If the buffer is at capacity, evict the oldest entry (FIFO).

## Pruning Logic

### Entity pruning (`prune_decayed_beliefs`)

Replaces `enforce_capacity`. Called at the same sites (perception.rs:224, perception.rs:510, epistemic_actions.rs:355, tell_actions.rs:624, tell_actions.rs:657).

```
fn prune_decayed_beliefs(&mut self, profile: &PerceptionProfile, current_tick: Tick, agent_needs: &HomeostaticNeeds) {
    // 1. Prune social observations below activation threshold
    self.social_observations.retain(|observation| {
        let age = current_tick.0.saturating_sub(observation.observed_tick.0).max(1);
        floor(1000 / sqrt(age)) >= profile.entity_activation_threshold.value()
    })

    // 2. Prune claims below confidence threshold
    for each entity in entity_claims:
        claims.retain(|claim|
            effective_claim_confidence(claim, current_tick, &profile.confidence_policy)
                >= profile.claim_confidence_threshold.value()
        )
    entity_claims.retain(|_, claims| !claims.is_empty())

    // 3. Prune entities below activation threshold
    known_entities.retain(|entity, state| {
        let base = compute_activation(current_tick, &state.presentation_ticks, state.presentation_tick_count);
        let boost = salience_boost(agent_needs, state, profile);
        base + boost >= profile.entity_activation_threshold.value()
    })

    // 4. Remove orphaned claims for pruned entities
    entity_claims.retain(|entity, _| known_entities.contains_key(entity))
}
```

### Need-gated salience

```
fn salience_boost(needs: &HomeostaticNeeds, state: &BelievedEntityState, profile: &PerceptionProfile) -> u16 {
    if state.believed_kind != Some(EntityKind::ItemLot) {
        return 0;
    }
    let max_need = needs.max_value();
    if max_need < profile.need_salience_urgency_threshold.value() {
        return 0;
    }
    (max_need as u32 * profile.need_salience_boost.value() as u32 / 1000) as u16
}
```

Graduated boost: at max_need=500 with default boost 500, bonus is 250. At max_need=1000, bonus is 500.

### Claim confidence threshold

Replaces `enforce_entity_claim_capacity` hard truncation. Claims already have staleness-adjusted confidence via `effective_claim_confidence`. Instead of sorting and truncating at a fixed count, simply retain claims above the confidence threshold. The existing confidence policy parameters control decay rate.

### Social observations

Social observations use the same activation formula as entity beliefs, applied to their single `observed_tick`. This unifies both pruning paths under one model: `floor(1000 / sqrt(age)) >= entity_activation_threshold`. No structural changes to `SocialObservation` are needed. The `within_retention_window` helper function is removed.

This is architecturally consistent: all memory (entity beliefs and social observations) decays via the same power-law function, controlled by the same per-agent threshold (FND-22 diversity). Agents with lower thresholds retain social observations longer; agents with higher thresholds forget faster.

## FND-01 Section H Analysis

### Information-path analysis

No new information paths. Activation consumes the same perception events as the current system. `record_entity_snapshot_claims` now pushes to a ring buffer instead of overwriting a single `observed_tick`. Information still arrives through FND-7-compliant local perception and physical carriers.

### Positive-feedback analysis

**Loop**: Observation -> high activation -> entity retained -> more opportunities to observe -> higher activation. This is intentional (memory reinforcement) and self-limiting: agents can only observe entities at their current location, and travel takes time. The physical world (co-location requirement, travel duration, perception fidelity) is the dampener.

No other amplifying loops introduced.

### Concrete dampeners

The observation->activation reinforcement loop is dampened by:
- **Co-location requirement**: Agent must be at the entity's location to observe it (FND-7)
- **Travel duration**: Moving between locations takes ticks, during which unobserved entities decay (FND-8)
- **Perception fidelity**: `observation_fidelity` < 1000 means some observation attempts fail (existing mechanism)
- **Ring buffer capacity**: Only the last N observations contribute to activation, preventing unbounded accumulation from a single long stay

### Stored state vs. derived read-model list

| Item | Classification |
|------|---------------|
| `presentation_ticks` ring buffer | **Stored state** — authoritative record of observation events |
| `presentation_tick_count` | **Stored state** — tracks ring buffer fill level |
| Activation value | **Derived** — computed on-demand from presentation_ticks + current tick |
| Salience boost | **Derived** — computed on-demand from agent needs + entity kind |
| Effective claim confidence | **Derived** (existing) — computed from claim fields + staleness |
| `last_observed_tick()` | **Derived** — accessor into presentation_ticks buffer |

## SystemFn Integration

No new system functions. The pruning logic runs inline within existing call sites:
- `perception::process_witness_event` (1 call site, line 224)
- `perception::apply_direct_local_observation_batch` (1 call site, line 510)
- `epistemic_actions::process_ask_witness_action` (1 call site, line 355)
- `tell_actions::process_tell_action` (2 call sites, lines 624, 657)

The function signature changes from `enforce_capacity(&mut self, profile: &PerceptionProfile, current_tick: Tick)` to `prune_decayed_beliefs(&mut self, profile: &PerceptionProfile, current_tick: Tick, agent_needs: &HomeostaticNeeds)`. Each call site must read `HomeostaticNeeds` from the agent's components (via `txn.component_homeostatic_needs(agent_id)` or equivalent world read) and pass it to the new function.

## Component Registration

### Modified components

- `PerceptionProfile` — fields changed (4 removed, 5 added). Remains universal, `Default` impl updated. Scenario `AgentDef` updated to reflect new fields. RON deserialization handles new fields with defaults for backward compatibility during scenario migration.

### No new components

Activation state lives inside `BelievedEntityState` (the ring buffer), not as a separate ECS component.

## Testing Strategy

### Golden tests (worldwake-ai)

| Test | Setup | Assert |
|------|-------|--------|
| `golden_activation_decay_prunes_stale_entities` | 1 place, 1 agent, 5 items. Agent observes then travels away. 200 ticks. | Items pruned from beliefs after ~100 ticks without re-observation |
| `golden_frequently_observed_entities_persist` | 1 place, 1 agent, 3 items. Agent stays. 500 ticks. | Items remain in beliefs entire duration due to continuous re-observation |
| `golden_need_salience_prevents_item_decay` | 2 places. Agent observes items at A, travels to B, stays. hunger=750. 200 ticks. | Items from A persist longer than baseline due to salience boost |
| `golden_no_capacity_wall_with_many_places` | Lina reproduction: 3+ places, 5+ facilities, ground items. 300 ticks. | Agent retains item knowledge AND infrastructure. pick_up affordances generated. Direct regression guard. |
| `golden_claim_confidence_threshold_prunes_stale_claims` | Agent receives multiple tell reports. Time passes. 300 ticks. | Low-confidence stale claims pruned. Fresh claims persist. No hard count limit. |

### Unit tests (worldwake-core)

- `test_activation_computation_single_observation` — formula correctness at known age values
- `test_activation_computation_multiple_observations` — ring buffer accumulation
- `test_ring_buffer_evicts_oldest_on_overflow` — FIFO behavior at capacity
- `test_salience_boost_scales_with_need_urgency` — graduated boost formula
- `test_salience_boost_zero_below_threshold` — no boost at low need values
- `test_prune_decayed_beliefs_removes_below_threshold` — pruning correctness
- `test_last_observed_tick_accessor` — backward compatibility
- `test_social_observation_activation_pruning` — social observations pruned by activation threshold
- `test_homeostatic_needs_max_value` — max_value helper returns highest need field

## Migration Notes

### Scenario files

Existing scenarios that specify `perception_profile` with `entity_memory_capacity`, `entity_claim_capacity`, `memory_retention_ticks`, or `infrastructure_retention_ticks` must be updated to the new fields. Per CLAUDE.md critical invariant "No backward compatibility layers" — old fields are removed, not shimmed.

### Existing tests

Tests that construct `PerceptionProfile` directly will need updating. Tests that use `PerceptionProfile::default()` will work with the new defaults. Tests that assert specific belief store sizes based on capacity limits must be rewritten to assert activation-based behavior.

### Removed functions

- `enforce_capacity` — replaced by `prune_decayed_beliefs`
- `enforce_entity_claim_capacity` — replaced by confidence-threshold pruning inline in `prune_decayed_beliefs`
- `entity_eviction_tier` — no longer needed (no tiered eviction)
- `within_retention_window` — no longer needed (activation replaces time-window checks)
