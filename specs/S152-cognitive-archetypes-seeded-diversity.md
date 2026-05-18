# S152: Cognitive Archetypes for Seeded Diversity

**Status**: Draft

## Summary

PR-13 (Default seeded personality/profile generation) from `reports/ai-architecture-improvements.md` flags that FND-22 (Agent Diversity Through Concrete Variation) is under-served by the current engine: scenarios may diversify agents through explicit per-agent profile overrides, but absent explicit authoring, every agent receives identical defaults. S111 (Scenario Homogeneity Lints, archived) added load-time detection of cloned profiles, but lints flag the problem rather than supplying a solution. The assessment proposes seeded `CognitiveArchetype` assignment at agent spawn that varies multiple profile fields together along narrative-coherent axes (Cautious, Bold, Stubborn, Methodical, Opportunistic, Sociable, Skeptical, Dutiful, Greedy, Fearful).

S152 lands a `CognitiveArchetype` enum with 10 variants and an `ArchetypeAssignmentPolicy` per-scenario that specifies how to seed agents with archetypes at spawn. Each archetype carries a template of `Permille` modifiers applied to existing universal profiles (`CognitiveProfile`, `PerceptionProfile`, `UtilityProfile`, `RiskWeightProfile`, `TestimonyTrustProfile` (S151), `PortfolioWeightsProfile` (S148)) and a small set of new archetype-specific profile fields (`backoff_ttl_multiplier`, `willingness_to_ask`, `willingness_to_detour`, `repair_budget_multiplier`). Assignment emits a `PersonalityAssigned` event with the archetype, seed, resolved profile snapshot, and source, making the assignment fully replayable and inspectable per FND-22A.

The archetypes are *not* personalities in the literary sense; they are *concrete state templates* that produce behavioral diversity. An agent of archetype `Cautious` plans more carefully (higher `repair_budget_fraction`), waits longer before retrying after failure (higher `backoff_ttl_multiplier`), and prefers safer routes (boosted `RoutePreferenceProfile.dangerous_traversal_penalty`). An agent of archetype `Bold` does the opposite. No archetype changes goal kinds or unlocks new actions — they only modulate how the agent reasons over the same affordance set as every other agent (per FND-19 agent symmetry).

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-core` — owns `CognitiveArchetype` enum, `ArchetypeProfileTemplate`, `PersonalityAssignedPayload`, and the new archetype-specific profile fields.
- `worldwake-ai` — consumes archetype settings at agent-state initialization; no new runtime decision pipeline.
- `worldwake-sim` — owns `PersonalityAssigned` event emission at agent spawn.
- `worldwake-systems` — no change.
- `worldwake-cli` — scenario loader supports `ArchetypeAssignmentPolicy`; observer renders agent archetype.

## Dependencies

- S111 (Scenario Homogeneity Lints, archived) — provides the homogeneity detection lint S152 substitutes a positive remedy for. The lint stays; S152 makes scenarios less likely to trip it by default.
- S148 (Portfolio Slot Expansion, archived at `archive/specs/S148-portfolio-and-motive-backed-intentions.md`) — `PortfolioWeightsProfile` is one of the archetype-modulated profiles.
- S151 (Testimony Reliability and Route Preferences, archived at `archive/specs/S151-testimony-reliability-and-route-preferences.md`) — `TestimonyTrustProfile` and `RoutePreferenceProfile` are archetype-modulated.
- S146 (Goal Schema and Per-Goal Budgets, archived at `archive/specs/S146-goal-schema-and-per-goal-budgets.md`) — `AgentSchemaContextProfile.budget_overrides` may be archetype-driven.

## Design Goals

1. **Archetypes are concrete state templates.** Each archetype is a deterministic `Permille`-valued template that applies modifiers to existing universal profiles plus a small set of archetype-specific fields.
2. **Deterministic seeded assignment.** Same scenario + same seed → identical archetype assignment per agent → identical resolved profile values.
3. **No new affordances per archetype.** Per FND-19 — every agent has the same action set; archetypes only modulate how the agent reasons.
4. **Authoring control.** Scenarios specify how to assign archetypes: uniform random, role-keyed, frequency-weighted, explicit per-agent. Default policy when nothing is specified: uniform random over a curated set of 5 archetypes (Cautious, Bold, Methodical, Opportunistic, Sociable) so the diversity is broad but not overwhelming.
5. **PersonalityAssigned event replayable.** Spawn-time emission captures the archetype, seed, and resolved profile snapshot. Replay reconstructs identical agents.
6. **Inspectable.** Observer surfaces archetype per agent and uses it in narrative rendering ("Agent A (Cautious) hesitated to enter the contested route").

## Non-Goals

- **No personality system beyond profile modulation.** Archetypes are not psychological models; they are profile templates.
- **No archetype switching at runtime.** An agent's archetype is set at spawn and persists. Runtime adaptation happens through S151 (testimony reliability, route preferences) and S22A learning state.
- **No archetype-keyed action restrictions.** A Cowardly archetype can still fight; they just have higher `RiskWeightProfile.combat_danger_weight`. Per FND-19.
- **No omniscient archetype detection.** Other agents cannot read another's archetype; they observe behavior.
- **No "rare" or "secret" archetypes.** All archetypes are equally available; rarity comes from `ArchetypeAssignmentPolicy.weights`.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-2 (No Ungrounded Triggers or Probabilities) | Archetype assignment uses a seeded ChaCha8Rng per `ArchetypeAssignmentPolicy.seed` derived from `ScenarioDef.seed`; assignment is deterministic and replayable. No naked probability constants. |
| FND-19 (Agent Symmetry) | Archetypes do not change action sets, world constraints, or any rule the engine enforces. They only modulate how the agent's reasoning weights itself. |
| FND-22 (Agent Diversity Through Concrete Variation) | Directly satisfies the principle — archetypes are concrete per-agent parameters that diversify needs, skills, values, courage, patience, and risk tolerance. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Initial archetype assignment is concrete state with `EventId` provenance (`PersonalityAssigned`); later learning sits on top per S151. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | When scenarios specify no archetype policy, the engine applies the default 5-archetype uniform policy; no legacy "all-identical" path is preserved. |
| FND-29 (Debuggability Is a Product Feature) | `PersonalityAssigned` event records the assignment; observer surfaces archetype in narrative; S144 diagnostics could aggregate archetype distribution (extension). |

## Deliverables

### D1: `CognitiveArchetype` enum

```rust
// crates/worldwake-core/src/cognitive_archetype.rs (new)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CognitiveArchetype {
    Cautious,
    Bold,
    Stubborn,
    Methodical,
    Opportunistic,
    Sociable,
    Skeptical,
    Dutiful,
    Greedy,
    Fearful,
}
```

Closed enum. Adding a variant requires a follow-up spec.

### D2: `ArchetypeProfileTemplate`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArchetypeProfileTemplate {
    pub archetype: CognitiveArchetype,

    // Cognitive
    pub max_plan_depth_delta: i8,
    pub repair_budget_fraction_delta: i32,    // permille delta
    pub margin_switch_delta: i32,             // permille delta
    pub guard_confidence_ceiling_delta: i32,  // permille delta

    // Perception
    pub observation_budget_delta: i8,
    pub testimony_confirmation_weight_delta: i32,
    pub testimony_refutation_penalty_delta: i32,

    // Risk and preferences
    pub combat_danger_weight_delta: i32,
    pub route_preference_penalty_delta: i32,
    pub backoff_ttl_multiplier: Permille,         // 1000 = unchanged
    pub willingness_to_ask: Permille,
    pub willingness_to_detour: Permille,

    // Portfolio
    pub portfolio_survival_delta: i32,
    pub portfolio_obligation_delta: i32,
    pub portfolio_economic_delta: i32,
    pub portfolio_social_delta: i32,
    pub portfolio_opportunistic_delta: i32,

    // Method enablement (S147)
    pub method_enable: Vec<MethodSchemaId>,
    pub method_disable: Vec<MethodSchemaId>,
}
```

Deltas are signed integers in `Permille` (where applicable) or absolute counts. The resolver clamps applied results to legal `Permille` ranges [0, 1000] and to per-profile bounds.

### D3: Archetype template table

```rust
// crates/worldwake-core/src/cognitive_archetype/templates.rs (new)
pub fn cautious_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Cautious,
        max_plan_depth_delta: 2,                              // plans more deeply
        repair_budget_fraction_delta: 100,                    // higher repair budget
        margin_switch_delta: 100,                             // higher switch threshold (stickier)
        guard_confidence_ceiling_delta: -100,                 // less confident in guards
        observation_budget_delta: 1,                          // slightly more observation
        testimony_confirmation_weight_delta: -50,             // less easily convinced
        testimony_refutation_penalty_delta: 100,              // strong refutation penalty
        combat_danger_weight_delta: 200,                      // strongly danger-averse
        route_preference_penalty_delta: 150,                  // dangerous routes hit harder
        backoff_ttl_multiplier: Permille::new(1500),          // waits 50% longer
        willingness_to_ask: Permille::new(800),               // happy to ask
        willingness_to_detour: Permille::new(900),            // happy to detour
        portfolio_survival_delta: 50,
        portfolio_obligation_delta: 0,
        portfolio_economic_delta: -50,
        portfolio_social_delta: 0,
        portfolio_opportunistic_delta: -150,                  // avoids opportunism
        method_enable: vec![],
        method_disable: vec![ /* method ids for group-confrontation methods */ ],
    }
}

// ... templates for Bold, Stubborn, Methodical, Opportunistic, Sociable, Skeptical, Dutiful, Greedy, Fearful
```

All ten templates ship in this spec. The table is build-time data, like S146's `GoalSchema` registry.

### D4: Archetype-specific profile fields

These are new fields on existing universal profiles to receive the archetype modifications:

```rust
// crates/worldwake-core/src/cognitive_profile.rs (extended)
pub struct CognitiveProfile {
    // ... existing fields
    pub backoff_ttl_multiplier: Permille,    // default 1000
    pub willingness_to_ask: Permille,        // default 500
    pub willingness_to_detour: Permille,     // default 500
}
```

Each is consulted by exactly one existing system:
- `backoff_ttl_multiplier`: applied to S109 / S150 discrepancy and blocker TTLs.
- `willingness_to_ask`: scales `AskWitness` (S139) candidate emission.
- `willingness_to_detour`: scales detour decision-making in `route_threat.rs`.

### D5: `ArchetypeAssignmentPolicy`

```rust
// crates/worldwake-core/src/archetype_assignment_policy.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub enum ArchetypeAssignmentPolicy {
    #[default]
    DefaultUniformFive,                                            // Cautious/Bold/Methodical/Opportunistic/Sociable
    Uniform(BTreeSet<CognitiveArchetype>),                         // uniform over given set
    Weighted(BTreeMap<CognitiveArchetype, u32>),                   // weighted by integer weights
    PerRole(BTreeMap<RoleTag, ArchetypeAssignmentPolicy>),        // per-role nested policy
    Explicit(BTreeMap<AgentName, CognitiveArchetype>),             // explicit by scenario agent name
}
```

`RoleTag` is the existing scenario-role tag (`crates/worldwake-cli/src/scenario/types.rs`). Per-role policy lets scenarios authoring patrolling guards have a different archetype distribution from peasants.

### D6: `PersonalityAssigned` event

```rust
// crates/worldwake-sim/src/event_log.rs (event tag added)
EventTag::PersonalityAssigned

// payload
pub struct PersonalityAssignedPayload {
    pub agent: EntityId,
    pub archetype: CognitiveArchetype,
    pub seed: u64,
    pub source: ArchetypeAssignmentSource,
    pub resolved_profile_snapshot: ArchetypeResolvedProfileSnapshot,
}

pub enum ArchetypeAssignmentSource {
    Scenario(ArchetypeAssignmentPolicy),     // policy carried by ScenarioDef
    Default,                                  // engine default
}

pub struct ArchetypeResolvedProfileSnapshot {
    pub cognitive_profile_hash: blake3::Hash,
    pub portfolio_weights_hash: blake3::Hash,
    pub utility_profile_hash: blake3::Hash,
    pub risk_weights_hash: blake3::Hash,
}
```

Emitted at agent spawn. Hashes (blake3) replace the full profile snapshot to keep event payload size bounded; hashes are deterministic per profile bytes and verifiable.

### D7: Scenario integration

`ScenarioDef` gains:

```rust
pub archetype_assignment_policy: Option<ArchetypeAssignmentPolicy>,
```

`None` (the default) applies `ArchetypeAssignmentPolicy::DefaultUniformFive`. Existing scenarios continue to load; their agents receive archetypes from the default policy unless they explicitly override per-agent profile fields (legacy authoring path still works).

### D8: Spawn-time resolution

In `crates/worldwake-cli/src/scenario/mod.rs` `spawn_agent()`:

1. Determine archetype: consult `ArchetypeAssignmentPolicy` via deterministic seeded RNG derived from `ScenarioDef.seed + agent_index`.
2. Look up `ArchetypeProfileTemplate` for that archetype.
3. Apply deltas to each universal profile (starting from default values or scenario-author overrides). Clamp results to legal `Permille` ranges.
4. Apply `method_enable` / `method_disable` to `AgentSchemaContextProfile.enabled_methods` (S147).
5. Emit `PersonalityAssigned` event with the resolved snapshot.

Explicit scenario-author overrides on individual profile fields (e.g., a scenario specifying `cognitive_profile.max_plan_depth = 14` for a specific agent) take precedence over archetype deltas. The archetype provides defaults; authors can still tune.

### D9: Observer rendering

Observer Section 1 (Agent Overview) prepends archetype to each agent line:
```
Agent A (Cautious) — health=900, hunger=480, ...
```

Section 3b (Decision History) includes archetype context in narrative renderings.

### D10: S144 diagnostics extension

`ScenarioDiagnosticsReport.agent_archetypes: BTreeMap<CognitiveArchetype, u64>` — count per archetype in the scenario.

### D11: Golden coverage

`golden_archetypes.rs` covers:
- Same scenario + seed → identical archetype assignment.
- Cautious agent waits longer after failure than Bold agent (backoff TTL behavior).
- Sociable agent emits more AskWitness candidates than Skeptical agent under identical belief state.
- Greedy agent's `OpportunisticLocal` slot wins more often than Cautious agent's in dense opportunity scenarios.
- `PersonalityAssigned` event is emitted exactly once per agent at spawn.
- Save/load preserves resolved profile values and archetype.

## FND-01 Section H Analysis

### Information-Path Analysis

Archetype is *assigned* at spawn; it does not propagate through perception. Other agents who interact with an archetype-shaped agent observe behavior, not the archetype label. Per FND-19 / FND-29, the archetype is inspectable through observer / debug tooling but not through in-world perception.

### Positive-Feedback Analysis

Not applicable. Archetype is initialization-time state; it does not amplify itself.

### Concrete Dampeners

Not applicable.

### Stored State vs. Derived Read-Model List

**Stored state**:
- `CognitiveArchetype` on the agent (per-agent runtime state, save/load round-trip).
- Extended universal profile fields (`backoff_ttl_multiplier`, `willingness_to_ask`, `willingness_to_detour`).
- `ArchetypeAssignmentPolicy` on `ScenarioDef`.

**Derived read-model**:
- `ArchetypeProfileTemplate` lookup is build-time data; resolved per-agent profile values are computed once at spawn and then stored on the agent.

## SystemFn Integration

No new `SystemFn`. Resolution runs at agent spawn (scenario load).

## Component Registration

- **New universal component**: `CognitiveArchetypeComponent { archetype: CognitiveArchetype }` on `EntityKind::Agent`. Default impl returns `CognitiveArchetype::Methodical` (a neutral default).
- **Existing universal profile field extensions** per D4 (no new component, just new fields).

## Cross-System Interactions

- `worldwake-cli` scenario loader writes profile-resolved values + emits `PersonalityAssigned` event.
- `worldwake-sim` records the event in the append-only log.
- All consuming systems (planning, perception, ranking, exploration) read existing profile fields. Archetype is *not* read at decision time; only its resolved profile values are read.

State-mediated per FND-26.

## Profile-Driven Parameters

All archetype template fields are `Permille` deltas or `i8` integer deltas. No floats.

`ArchetypeAssignmentPolicy.Weighted` uses `u32` integer weights.

## Test Plan

- D11 golden coverage (6 scenarios).
- Determinism: 10 archetype templates × 5 seeds × per-archetype regression tests of resolved profile snapshots.
- Save/load coverage for `CognitiveArchetypeComponent` and resolved profile fields.
- Scenario loader test: `ScenarioDef` with no archetype policy → uniform default-five applied.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
