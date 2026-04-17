# S107: Proactive Diversification Exploration

**Status**: DRAFT

## Summary

Add a proactive exploration pathway driven by curiosity and resource-insurance pressure rather than immediate need satisfaction. Currently, agents explore only when survival needs are pressing and local resources are exhausted (S80/S102). Once a viable survival cycle is found, exploration stops entirely — agents rationally exploit the optimal corridor and never discover backup resources. This creates emergent single-source dependency: all agents converge on the same locations, ignoring reachable alternatives.

The fix: a new `DiversificationProfile` component that generates `ExploreLocation` goals when survival needs are *comfortable* and curiosity pressure has accumulated. The drive is gated by a need-slack veto (exploration utility drops to zero when any survival need exceeds a comfort threshold) and parameterized per-agent for population diversity (FND-22). Familiarity with places is tracked as concrete visit history in the belief store (FND-3), not as stored novelty scores.

## Phase

Agent reasoning enhancement (exploration lifecycle)

## Crates

- `worldwake-core` (new `DiversificationProfile` component, `PlaceVisitRecord` in belief store, `ExplorationMotivation` enum in `goal.rs`)
- `worldwake-sim` (`GoalBeliefView` accessor for `DiversificationProfile`, `RuntimeBeliefView`/`PerAgentBeliefView` impl forwarding)
- `worldwake-ai` (proactive exploration candidate emission in `candidate_generation.rs`, golden tests, `ExploreLocation` match site updates for `ExplorationMotivation`)
- `worldwake-cli` (scenario support for `DiversificationProfile`)

## Dependencies

- S80 (Exploration Drive) — reuses `ExploreLocation` goal kind and exploration frontier selection
- S102 (Frontier-Aware Exploration) — reuses BFS frontier candidate selection, must not conflict with need-driven exploration triggers

No dependency on any pending Phase 7 spec. Can be implemented independently.

## Problem Statement

### Evidence

Scenario analysis of `survival-scattered.ron` (seed 205005, 1440 ticks, 3 agents, 6 places):

| Place | Agent A Ticks | Agent B Ticks | Agent C Ticks | Resources |
|-------|--------------|--------------|--------------|-----------|
| River Crossing | 949 | 718 | 1044 | Well, WashBasin |
| Lowland Farm | 440 | 650 | 336 | FieldPlot (Grain) |
| Hilltop Camp | 14 | 14 | 16 | Well |
| Woodland Clearing | 1 | 1 | 1 | None |
| Ravine Shelter | 0 | 1 | 0 | None |
| Orchard Hollow | 0 | 0 | 0 | OrchardRow (Apples) |

All 3 agents independently converge on River Crossing + Lowland Farm. Orchard Hollow (food source, 3 hops from Woodland Clearing) is never visited despite all agents knowing "Harvest Apples". The scenario designed 6 places but only 2 are used.

This is correct behavior under S80/S102: exploration triggers only when `need_pressure >= need_activation_threshold` AND the agent is in self-care fallback. Once agents find the River Crossing corridor, their needs are satisfied and exploration drive drops below threshold.

### Architectural Gap

The missing behavior is **proactive diversification** — exploring for backup resources, novelty, or information when immediate needs are satisfied. This is well-established in agent AI literature:

- **Optimal Foraging Theory** (Charnov 1976): Marginal Value Theorem predicts agents should scout when current patch returns diminish, but the current system has no diminishing-returns signal for functional patches.
- **Intrinsic Motivation** (Oudeyer & Kaplan 2007, Schmidhuber): Curiosity as per-region learning progress. Agents should be motivated to reduce uncertainty about unvisited places.
- **Ant Colony Scouting**: Even with a known primary food source, 5-15% of foragers continue scouting. This maintains population-level awareness of alternatives.
- **RimWorld Recreation Variety**: Visit-count-based satisfaction decay with time recovery creates variety-seeking behavior.
- **UCB Bandit Exploration** (Auer et al. 2002): Rarely-visited options receive an uncertainty bonus that decays with visits.

### FOUNDATIONS Motivation

- **FND-22 (Agent Diversity)**: Currently all agents follow the same "find optimal corridor, stay there" pattern regardless of personality. Per-agent curiosity parameters would create emergent diversity: some agents naturally scout while others settle.
- **FND-1 (Maximal Emergence)**: Resource diversification should emerge from agent-level drives, not from scenario design forcing agents to spread out.
- **FND-11 (Feedback Dampeners)**: Single-source dependency is a fragility — if the one known food source fails, agents have no backup beliefs. Proactive exploration is a natural dampener on resource concentration risk.

## Design

### DiversificationProfile

New **role-specific** ECS component (not universal — agents without this profile behave exactly as today). Registered on `EntityKind::Agent`.

```rust
/// Per-agent parameters governing proactive diversification exploration.
/// Absent agents never generate proactive exploration goals.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct DiversificationProfile {
    /// Base curiosity drive weight. Higher values produce more frequent
    /// exploration when needs are comfortable. Range: [0, 1000] permille.
    /// 100 = homebody, 500 = average, 800 = restless scout.
    pub base_curiosity: Permille,

    /// Maximum survival need pressure (any single need) that still permits
    /// proactive exploration. If max(all_needs) > this threshold, proactive
    /// exploration utility is zero. Range: [0, 1000] permille.
    pub comfort_threshold: Permille,

    /// Rate at which curiosity pressure accumulates per tick since last
    /// exploration. Curiosity pressure = min(1000, ticks_since * buildup_rate / 1000).
    /// Higher values = faster curiosity accumulation.
    pub curiosity_buildup_rate: Permille,

    /// Minimum ticks between proactive exploration attempts. Prevents
    /// thrashing by enforcing a cooldown after each exploration.
    pub exploration_cooldown_ticks: u32,

    /// Per-visit familiarity increase for the visited place. Higher values
    /// mean places become "boring" faster. Range: [0, 1000] permille.
    pub familiarity_per_visit: Permille,

    /// Per-tick familiarity recovery rate for unvisited places. Familiarity
    /// decays over time, making old places interesting again.
    /// Range: [0, 1000] permille. Applied as: familiarity -= recovery_per_tick.
    pub familiarity_recovery_per_tick: Permille,

    /// Floor below which familiarity cannot drop. Ensures frequently visited
    /// places retain some familiarity even after long absence.
    /// Range: [0, 1000] permille.
    pub familiarity_floor: Permille,

    /// Maximum graph distance (hops) for proactive exploration candidates.
    /// Limits travel cost of curiosity-driven exploration.
    pub max_exploration_hops: u16,
}
```

**Default impl** (for scenario convenience, though the profile is role-specific):

```rust
impl Default for DiversificationProfile {
    fn default() -> Self {
        Self {
            base_curiosity: Permille::new_unchecked(400),
            comfort_threshold: Permille::new_unchecked(450),
            curiosity_buildup_rate: Permille::new_unchecked(5),
            exploration_cooldown_ticks: 60,
            familiarity_per_visit: Permille::new_unchecked(150),
            familiarity_recovery_per_tick: Permille::new_unchecked(2),
            familiarity_floor: Permille::new_unchecked(50),
            max_exploration_hops: 3,
        }
    }
}
```

### PlaceVisitRecord

Per-place visit tracking stored in `AgentBeliefStore`. This is **concrete state** (FND-3) — visit counts and timestamps, not derived scores.

```rust
/// Tracks an agent's visit history for a believed place.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlaceVisitRecord {
    /// Total number of ticks the agent has spent at this place.
    pub ticks_present: u32,
    /// Tick of most recent arrival at this place.
    pub last_arrival_tick: Tick,
    /// Number of distinct visits (arrivals) at this place.
    pub visit_count: u16,
}
```

**Storage**: New field `place_visits: BTreeMap<EntityId, PlaceVisitRecord>` on `AgentBeliefStore`. Updated by the existing perception/location-tracking infrastructure whenever an agent arrives at or occupies a place.

**Update rules**:
- On arrival at a place: increment `visit_count`, set `last_arrival_tick` to current tick.
- Each tick at a place: increment `ticks_present`.
- Entries are never removed (places an agent has visited are permanent knowledge, per FND-18).

**Update mechanism**: This is **new behavior**, not a reuse of existing perception infrastructure. The current perception system (`record_entity_snapshot_claims` in `belief.rs`) records observation timestamps in `BelievedEntityState::presentation_ticks` but does not track discrete visit counts or ticks-present per place. Two new update paths are needed:

1. **Arrival tracking**: When an agent's `effective_place` changes (detected in the perception/location-tracking tick), insert or update the `PlaceVisitRecord` for the new place: increment `visit_count`, set `last_arrival_tick`. This hooks into the same location-change detection that already updates `BelievedEntityState`.
2. **Presence tracking**: Each tick, if the agent has a `PlaceVisitRecord` for its current `effective_place`, increment `ticks_present`. This can be a lightweight pass in the same perception tick or a dedicated system function.

Both updates are agent-local (the agent writes to its own belief store based on its own location), preserving FND-7 locality.

### Derived Familiarity Computation

Familiarity is **computed on query**, never stored (FND-3 / FND-27). Given a `PlaceVisitRecord` and `DiversificationProfile`:

```rust
fn compute_familiarity(
    record: &PlaceVisitRecord,
    current_tick: Tick,
    profile: &DiversificationProfile,
) -> Permille {
    // Base familiarity from visit history (clamped to Permille range)
    let raw_visit = u32::from(record.visit_count)
        * u32::from(profile.familiarity_per_visit.value());
    let visit_familiarity = Permille::new_unchecked(raw_visit.min(1000) as u16);

    // Time-based recovery (familiarity decays with absence)
    let ticks_away = current_tick.0.saturating_sub(record.last_arrival_tick.0);
    let raw_recovery = (ticks_away as u32)
        * u32::from(profile.familiarity_recovery_per_tick.value());
    let recovery = Permille::new_unchecked(raw_recovery.min(1000) as u16);

    // Effective familiarity: base minus recovery, floored
    let effective = visit_familiarity.saturating_sub(recovery);
    effective.max(profile.familiarity_floor)
}

/// Novelty is the inverse of familiarity.
fn compute_novelty(
    record: &PlaceVisitRecord,
    current_tick: Tick,
    profile: &DiversificationProfile,
) -> Permille {
    Permille::new_unchecked(1000).saturating_sub(
        compute_familiarity(record, current_tick, profile)
    )
}
```

For places the agent has never visited (no `PlaceVisitRecord`), novelty is `Permille(1000)` — maximum curiosity.

### Proactive Exploration Goal Emission

New function in `candidate_generation.rs`, called alongside the existing `emit_exploration_candidates`. Follows the established emitter pattern using `GenerationContext` and `emit_candidate_with_trace`:

```rust
fn emit_proactive_exploration_candidates(
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
    needs: Option<HomeostaticNeeds>,
) {
    let Some(needs) = needs else { return };
    let Some(profile) = ctx.view.diversification_profile(ctx.agent) else {
        return; // No DiversificationProfile — agent doesn't do proactive exploration
    };

    // Gate 1: Need-slack veto (max_value() returns u16, compare against permille value)
    let max_need = needs.max_value();
    if max_need > profile.comfort_threshold.value() {
        return; // Survival needs too high — suppress proactive exploration
    }

    // Gate 2: Cooldown
    let last_proactive_tick = ctx.view.last_proactive_exploration_tick(ctx.agent);
    if let Some(last_tick) = last_proactive_tick {
        if ctx.current_tick.0.saturating_sub(last_tick.0)
            < u64::from(profile.exploration_cooldown_ticks)
        {
            return; // Too soon since last proactive exploration
        }
    }

    // Gate 3: Curiosity pressure accumulation
    let ticks_since_explore = last_proactive_tick
        .map(|t| ctx.current_tick.0.saturating_sub(t.0) as u32)
        .unwrap_or(ctx.current_tick.0 as u32);
    let raw_curiosity = ticks_since_explore
        * u32::from(profile.curiosity_buildup_rate.value());
    let curiosity_pressure = Permille::new_unchecked(raw_curiosity.min(1000) as u16);

    // Gate 4: Need slack scaling (multiplicative dampener)
    let need_slack = Permille::new_unchecked(1000u16.saturating_sub(max_need));

    // Select target: highest-novelty believed place within hop limit
    let Some((target_place, novelty)) = select_proactive_target(ctx, &profile) else {
        return;
    };

    // Compute final utility (multiplicative: any factor at 0 vetoes)
    // Each factor is in [0, 1000] permille. Multiply pairwise with /1000 normalization.
    let utility_raw = u64::from(profile.base_curiosity.value())
        * u64::from(curiosity_pressure.value())
        * u64::from(need_slack.value())
        * u64::from(novelty.value())
        / (1000 * 1000 * 1000); // three divisions to keep result in [0, 1000]
    if utility_raw == 0 { return; }

    emit_candidate_with_trace(
        candidates,
        diagnostics,
        GoalKind::ExploreLocation {
            target_place,
            motivating_need: ExplorationMotivation::Proactive,
        },
        OpportunityAnchor::Place(target_place),
        Evidence::with_place(target_place),
        EvidenceTrace::default(),
    );
}
```

Note: Goal ranking (drive score / priority) is handled separately in `ranking.rs` via `motive_score` computation for `ExploreLocation`, not by the emitter. The emitter only determines whether to emit; ranking determines relative priority.

### ExplorationMotivation Extension

The current `ExploreLocation` goal uses `motivating_need: HomeostaticNeedId` to indicate which need drove the exploration. Proactive exploration has no specific need — it's driven by curiosity. Two options:

**Option A** — New enum replacing `HomeostaticNeedId` in `ExploreLocation`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ExplorationMotivation {
    NeedDriven(HomeostaticNeedId),
    Proactive,
}
```

**Option B** — Keep `HomeostaticNeedId` and use the need with highest pressure at time of emission as a proxy motivation.

**Recommendation**: Option A. It cleanly separates the two exploration drives, enables distinct observer reporting, and allows the planner to handle proactive exploration differently if needed (e.g., lower commitment, interruptible). The `GoalKey` derivation handles this naturally since `ExploreLocation` keys on `target_place`, not motivation.

**Blast radius**: Changing `motivating_need` from `HomeostaticNeedId` to `ExplorationMotivation` requires updating all exhaustive match sites on `ExploreLocation` across the AI crate. Key files needing match arm updates:

- `crates/worldwake-ai/src/goal_model.rs` — `GoalKindPlannerExt` methods (9 match sites)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` — `from_goal_kind` (3 sites)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` — dispatch declarations (6 sites)
- `crates/worldwake-ai/src/ranking.rs` — `motive_score` and priority class (5 sites)
- `crates/worldwake-ai/src/candidate_generation.rs` — existing `emit_exploration_candidates` (8 sites)
- `crates/worldwake-ai/src/feasibility.rs` — feasibility checks (1 site)
- `crates/worldwake-ai/tests/golden_exploration.rs` — explicit `ExploreLocation` literals/wrappers
- `crates/worldwake-ai/tests/golden_survival_baseline.rs` — recheck for typed fallout; live branch wildcard assertions compile unchanged
- `crates/worldwake-ai/tests/golden_survival_scattered.rs` — recheck for typed fallout; live branch wildcard assertions compile unchanged
- `crates/worldwake-systems/src/travel_actions.rs` — travel action handling (4 sites)

Most updates are mechanical: wrapping the existing `need_id` in `ExplorationMotivation::NeedDriven(need_id)` at emission sites, and adding `ExplorationMotivation::NeedDriven(need_id)` pattern destructuring at match sites. `ExplorationMotivation` must derive `Copy` (since `GoalKind` derives `Copy`).

### Target Selection

Reuses the existing BFS frontier machinery from `exploration_candidate_places()` (S80/S102), constrained to `max_exploration_hops`. The actual signature of `exploration_candidate_places` is `(view: &dyn GoalBeliefView, agent: EntityId, frontier_depth: u16) -> BTreeMap<EntityId, Option<Tick>>`:

```rust
fn select_proactive_target(
    ctx: &GenerationContext<'_>,
    profile: &DiversificationProfile,
) -> Option<(EntityId, Permille)> {
    // Reuse BFS frontier with max_exploration_hops as depth limit
    let candidates = exploration_candidate_places(
        ctx.view,
        ctx.agent,
        profile.max_exploration_hops,
    );

    let store = ctx.view.agent_belief_store(ctx.agent)?;
    let current_tick = ctx.current_tick;

    // Score each candidate by novelty (inverse familiarity)
    candidates
        .into_keys()
        .map(|place| {
            let novelty = store.place_visits.get(&place)
                .map(|record| compute_novelty(record, current_tick, profile))
                .unwrap_or(Permille::new_unchecked(1000)); // Never visited = max novelty
            (place, novelty)
        })
        .max_by_key(|(_, novelty)| *novelty)
}
```

### Last Proactive Exploration Tick Tracking

New runtime field on the agent (not a profile parameter — it's transient execution state):

```rust
/// Tick of the agent's most recent proactive exploration goal commitment.
pub struct LastProactiveExplorationTick(pub Option<Tick>);
```

Updated when a proactive `ExploreLocation` goal is committed. This is runtime-generated state (like `ActiveGoal`), not scenario-configured. Registered as an ECS component on `EntityKind::Agent`, set to `None` at spawn.

### Interaction with Existing Exploration (S80/S102)

The two exploration pathways are **independent and non-conflicting**:

| Aspect | S80/S102 (Reactive) | S107 (Proactive) |
|--------|-------------------|-----------------|
| Trigger | Needs above activation threshold | Needs below comfort threshold |
| Gate | Self-care fallback only | Need-slack veto + cooldown |
| Drive source | Acquisition exhaustion | Curiosity accumulation |
| Population | All agents (ExplorationProfile is universal) | Only agents with DiversificationProfile (role-specific) |
| Priority | High (need-driven) | Low (comfort-gated) |

The two pathways cannot fire simultaneously for the same agent: reactive requires `need_pressure >= activation_threshold`, proactive requires `max_need <= comfort_threshold`. If `activation_threshold > comfort_threshold`, there's a dead zone where neither fires — this is correct (moderate need pressure, not yet exploring reactively, not comfortable enough for proactive).

### SystemFn Integration

No new SystemFn is needed. The proactive exploration candidate emission integrates into the existing `generate_candidates` call in the agent decision tick. The `emit_proactive_exploration_candidates` function is called alongside `emit_exploration_candidates`, and both contribute to the same candidate pool for goal ranking.

**Consecutive exploration counting**: Proactive `ExploreLocation` goals count toward `ExplorationProfile.consecutive_exploration_count` and are subject to `max_consecutive_explorations`. This is automatic because both reactive and proactive pathways emit the same `GoalKind::ExploreLocation` variant — the counter increment logic in the agent tick matches on `ExploreLocation` regardless of `ExplorationMotivation`. This prevents a pathological case where unlimited proactive explorations bypass the existing S80 safety limit.

### Component Registration

| Component | Kind | EntityKind | Scenario Config | Default |
|-----------|------|-----------|----------------|---------|
| `DiversificationProfile` | Role-specific | Agent | `AgentDef.diversification_profile: Option<DiversificationProfile>` | `Default` impl provided for convenience |
| `PlaceVisitRecord` (in `AgentBeliefStore.place_visits`) | Runtime-generated | Agent | Not configured — populated by perception | N/A |
| `LastProactiveExplorationTick` | Runtime-generated | Agent | Not configured — set at spawn to `None` | `None` |

**Scenario wiring**:
- Add `diversification_profile: Option<DiversificationProfile>` to `AgentDef` in `types.rs` (no `*Def` wrapper needed — the profile contains no `EntityId` references, only `Permille`, `u32`, and `u16` fields)
- Add `if let Some(ref dp) = agent_def.diversification_profile { txn.set_component_diversification_profile(agent_id, *dp)?; txn.set_component_last_proactive_exploration_tick(agent_id, LastProactiveExplorationTick(None))?; }` in `spawn_agent()` (role-specific pattern, with runtime proactive state initialized at spawn)

**GoalBeliefView accessor** (in `worldwake-sim`):
- Add `fn diversification_profile(&self, agent: EntityId) -> Option<DiversificationProfile>` to `GoalBeliefView` trait
- Add `fn last_proactive_exploration_tick(&self, agent: EntityId) -> Option<Tick>` to `GoalBeliefView` trait
- Implement forwarding in `RuntimeBeliefView` and `PerAgentBeliefView`

## Authoritative-to-AI Impact Rule

This spec modifies candidate emission (new `emit_proactive_exploration_candidates`) and changes the `ExploreLocation` GoalKind field type (`HomeostaticNeedId` → `ExplorationMotivation`).

1. **`get_affordances`** — N/A. No new affordance type is introduced; proactive exploration uses the existing `ExploreLocation` goal kind.
2. **`generate_candidates`** — New emitter `emit_proactive_exploration_candidates` added alongside existing `emit_exploration_candidates`. Both feed the same candidate pool. Existing reactive emission updated to wrap `need_id` in `ExplorationMotivation::NeedDriven(need_id)`.
3. **`search_plan`** — Pass. `ExploreLocation` planning infrastructure (terminal ordering, barrier logic, op kinds) is unchanged. The planner keys on `GoalKey` which ignores `motivating_need`/`ExplorationMotivation`.
4. **`BestEffort` action start** — Pass. `ExploreLocation` uses existing travel actions; no new action type.
5. **`handle_plan_failure`** — Pass. Replanning logic is GoalKind-agnostic for ExploreLocation.
6. **Payload revalidation** — N/A. `ExploreLocation` does not use planner-synthesized payloads.
7. **Golden tests** — Recheck existing exploration-related goldens after the type migration. On the live branch, `golden_exploration.rs` needs explicit `ExplorationMotivation::NeedDriven(...)` wrapper updates, while `golden_survival_baseline.rs` and `golden_survival_scattered.rs` compile unchanged because their `ExploreLocation` assertions stay wildcard-based. New golden tests are still required for proactive diversification scenarios.

## Section H: FND-01 Analysis

### H.1 Information-Path Analysis

**What information does the agent use?**
1. **Visit history** (PlaceVisitRecord): Agent's own location tracking. Updated each tick via the existing perception/location system. No locality violation — the agent knows where it has been.
2. **Believed places**: Agent's belief store. Already subject to locality — agents only know about places they've perceived or heard about (FND-7, FND-14).
3. **Current needs**: Agent's own homeostatic state. Local by definition.
4. **Curiosity pressure**: Derived from ticks since last proactive exploration. Agent-local runtime state.

**Information path**: All inputs are agent-local. No global query, no cross-agent knowledge, no omniscient world access. The proactive exploration decision is fully explainable as "Agent X chose to explore place Y because they hadn't explored recently, their needs were comfortable, and place Y was the least familiar believed place."

### H.2 Positive-Feedback Analysis

**Potential loop: Exploration → Discovery → More believed places → More exploration targets → More exploration**

This is a self-limiting loop because:
- Visit count increases familiarity, reducing novelty of visited places
- `familiarity_floor` prevents novelty from fully recovering for well-known places
- Finite place graph means candidates are exhausted
- Need-slack gate means exploration is suppressed whenever needs rise

**No other amplifying loops identified.** The curiosity buildup is linear and clamped at 1000 permille. Familiarity recovery is linear and bounded. Neither creates exponential growth.

### H.3 Concrete Dampeners

| Loop | Dampener | Mechanism |
|------|----------|-----------|
| Exploration → more targets → more exploration | Visit familiarity | Each visit increases familiarity, reducing that place's novelty score |
| Curiosity accumulation | Permille clamp | `curiosity_pressure = min(1000, ...)` — asymptotic ceiling |
| Travel cost | Need accumulation during travel | Travel increases fatigue, thirst, bladder (metabolism profile), which eventually triggers the need-slack veto |
| Consecutive exploration | Cooldown | `exploration_cooldown_ticks` enforces minimum gap between proactive explorations |
| Over-exploration in general | `max_consecutive_explorations` on ExplorationProfile | Existing S80 limit applies to all `ExploreLocation` goals regardless of `ExplorationMotivation` — proactive explorations count toward the same `consecutive_exploration_count` counter |

### H.4 Stored State vs. Derived Read-Model

| Item | Category | Location |
|------|----------|----------|
| `DiversificationProfile` | Stored (scenario config) | ECS component on Agent |
| `PlaceVisitRecord` | Stored (runtime-generated) | `AgentBeliefStore.place_visits` |
| `LastProactiveExplorationTick` | Stored (runtime-generated) | ECS component on Agent |
| Familiarity | **Derived** (computed on query) | `compute_familiarity()` — never stored |
| Novelty | **Derived** (computed on query) | `compute_novelty()` — never stored |
| Curiosity pressure | **Derived** (computed on query) | Computed from tick delta — never stored |
| Need slack | **Derived** (computed on query) | `1000 - max(needs)` — never stored |
| Exploration utility | **Derived** (computed on query) | Multiplicative product — never stored |

## Golden Test Coverage

### Scenario: Proactive Diversification Discovery

A 2-location scenario where one location satisfies all immediate needs but a third location has an alternative food source reachable within `max_exploration_hops`. Agent with `DiversificationProfile` should eventually explore the third location after needs stabilize. Agent without the profile should not.

**Primary assertion**: Agent with diversification profile visits the third location within N ticks after needs stabilize below comfort threshold.

**Control assertion**: Agent without diversification profile never visits the third location (needs are met locally).

### Scenario: Need-Slack Veto

Agent with `DiversificationProfile` in a resource-scarce environment where needs are always above `comfort_threshold`. Proactive exploration should never fire.

**Primary assertion**: Zero proactive `ExploreLocation` goals emitted across the entire run.

### Scenario: Cooldown Enforcement

Agent with short cooldown explores multiple locations. Verify that exploration attempts are spaced by at least `exploration_cooldown_ticks`.

### H.5 FND-30 Causal Hooks Checklist (Supplementary Items)

Items 1-4 are covered by H.1-H.4. Remaining items:

- **5. Quantities conserved/transferred**: N/A — proactive exploration does not create, destroy, or transfer items or currency.
- **6. Scarce capacities/contention**: N/A — exploration uses standard travel which already handles path contention.
- **7. Partial failures/aftermath**: Exploration that fails (unreachable place, interrupted by need pressure) simply results in the agent returning to normal goal selection. No special aftermath state.
- **10. Agent learning/habit**: `PlaceVisitRecord` is the learning artifact — explicit, agent-local, with accountable origin (arrival events), scope (per-place), and no decay (permanent, per FND-18). Familiarity is derived, not stored.
- **11. Error/correction**: Agents cannot be "wrong" about their own visit history (it's their own experience). They can be wrong about what resources a place has — that is handled by existing belief staleness mechanisms.
- **12. Lifecycle states**: `DiversificationProfile` is static config (no lifecycle). `PlaceVisitRecord` entries are permanent (no removal). `LastProactiveExplorationTick` is transient runtime state reset at spawn.
- **13. Temporal resolution/scheduling**: Uses existing tick granularity. No new simultaneity or tie-breaking concerns.
- **14. Boundary conditions**: N/A — proactive exploration operates within the simulated place graph.
- **15. Derived views/caches**: Familiarity, novelty, curiosity pressure, need slack, and exploration utility are all derived on query from concrete state. Never stored.
- **16. Causal records/provenance**: Proactive `ExploreLocation` goals are emitted with `EvidenceTrace::default()` and `Evidence::with_place(target_place)`, preserving the standard evidence chain.
- **17. Falsification checks**: See Golden Test Coverage section. Primary falsification: agent without `DiversificationProfile` must never emit proactive exploration. Secondary: agent with profile but needs above comfort_threshold must never emit.
- **18. Save/load/replay**: `DiversificationProfile` is a standard ECS component (serialized). `PlaceVisitRecord` is in `AgentBeliefStore` (serialized via serde). `LastProactiveExplorationTick` is runtime-generated (reconstructed on replay). No world meaning is lost across save/load boundaries.

## Cross-System Interactions (FND-26)

| System | Interaction | Mediation |
|--------|------------|-----------|
| Perception | Updates `PlaceVisitRecord` when agent observes a place | Via `AgentBeliefStore` state mutation |
| Needs/Metabolism | Travel during proactive exploration increases fatigue, thirst, bladder | Via standard travel metabolism (already exists) |
| Goal Ranking | Proactive exploration competes with other goals via drive score | Via existing candidate pool and ranking |
| Existing Exploration (S80/S102) | Independent pathways — cannot fire simultaneously | Via disjoint need-pressure gates |

No direct system-to-system calls. All interaction through shared ECS state.
