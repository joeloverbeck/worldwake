# S152: Cognitive Archetypes for Seeded Diversity

**Status**: Draft

## Summary

PR-13 (Default seeded personality/profile generation) from `reports/ai-architecture-improvements.md` flags that FND-22 (Agent Diversity Through Concrete Variation) is under-served by the current engine: scenarios may diversify agents through explicit per-agent profile overrides, but absent explicit authoring, every agent receives identical defaults. S111 (Scenario Homogeneity Lints, archived) added load-time detection of cloned profiles, but lints flag the problem rather than supplying a solution. The assessment proposes seeded `CognitiveArchetype` assignment at agent spawn that varies multiple profile fields together along narrative-coherent axes (Cautious, Bold, Stubborn, Methodical, Opportunistic, Sociable, Skeptical, Dutiful, Greedy, Fearful).

S152 lands a `CognitiveArchetype` enum with 10 variants and an `ArchetypeAssignmentPolicy` per-scenario that specifies how to seed agents with archetypes at spawn. Each archetype carries a template of `Permille`/integer **deltas applied to existing universal profile fields** plus a `BackoffScalePermille` backoff scale — `CognitiveProfile`, `PerceptionProfile`, `RiskWeightProfile`, `TestimonyTrustProfile` (S151), `RoutePreferenceProfile` (S151), `EpistemicDispositionProfile`, `PortfolioWeightsProfile` (S148), and `AgentSchemaContextProfile.disabled_methods` (S147). **No new behavioral profile fields are introduced**: archetypes vary the concrete parameters that already drive reasoning, so no consuming system needs new wiring (FND-3 / FND-28). Assignment emits a `PersonalityAssigned` event with the archetype, seed, a resolved-profile verification hash, and source, making the assignment replayable and inspectable per FND-22A / FND-29.

The archetypes are *not* personalities in the literary sense; they are *concrete state templates* that produce behavioral diversity. An agent of archetype `Cautious` plans more carefully (higher `repair_budget_fraction`), waits longer before retrying after failure (longer `*_backoff_ticks` via a backoff scale), and prefers safer routes (boosted `RoutePreferenceProfile.dangerous_traversal_penalty`). An agent of archetype `Bold` does the opposite. No archetype changes goal kinds or unlocks new actions — they only modulate how the agent reasons over the same affordance set as every other agent (per FND-19 agent symmetry). Method disabling (`AgentSchemaContextProfile.disabled_methods`) narrows HTN decomposition strategies while flat GOAP fallback remains lawful, so it too leaves the action set unchanged.

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-core` — owns `CognitiveArchetype` enum, `ArchetypeProfileTemplate`, the build-time template table, `CognitiveArchetypeComponent`, `ArchetypeAssignmentPolicy`, `PersonalityAssignedPayload`, `ArchetypeAssignmentSource`, the `EventTag::PersonalityAssigned` variant, and the `EventPayload` field carrying the payload.
- `worldwake-ai` — extends `ScenarioDiagnosticsReport` with an archetype distribution; no new runtime decision pipeline (resolved profile values drive existing systems).
- `worldwake-sim` — records the `PersonalityAssigned` event in the append-only log (no new emission logic; the tag and payload live in core).
- `worldwake-systems` — no change.
- `worldwake-cli` — scenario loader resolves archetypes and applies deltas at spawn, sets `CognitiveArchetypeComponent`, and emits `PersonalityAssigned`; observer renders agent archetype.

## Dependencies

- S111 (Scenario Homogeneity Lints, archived at `archive/specs/S111-scenario-homogeneity-lints.md`) — provides the homogeneity detection lint S152 substitutes a positive remedy for. The lint stays; S152 makes scenarios less likely to trip it by default.
- S148 (Portfolio Slot Expansion, archived at `archive/specs/S148-portfolio-and-motive-backed-intentions.md`) — `PortfolioWeightsProfile` (fields `need_survival`, `pain_care`, `obligation_duty`, `economic_opportunity`, `social_motive`) is one of the archetype-modulated profiles.
- S151 (Testimony Reliability and Route Preferences, archived at `archive/specs/S151-testimony-reliability-and-route-preferences.md`) — `TestimonyTrustProfile` (`confirmation_weight`, `refutation_penalty`) and `RoutePreferenceProfile` (`dangerous_traversal_penalty`) are archetype-modulated.
- S147 (Method Schemas, archived) — `MethodSchemaId` and `AgentSchemaContextProfile.disabled_methods` are the surface archetype templates use to narrow HTN decomposition.
- S146 (Goal Schema and Per-Goal Budgets, archived at `archive/specs/S146-goal-schema-and-per-goal-budgets.md`) — `AgentSchemaContextProfile.budget_overrides` exists but is **not** archetype-driven in this spec (see Non-Goals).

## Design Goals

1. **Archetypes are concrete state templates over existing fields.** Each archetype is a deterministic template of `Permille`/integer deltas applied to existing universal profile fields plus a uniform `BackoffScalePermille` backoff scale and an optional method-disable set. No new behavioral profile field is introduced.
2. **Deterministic seeded assignment.** Same scenario + same seed → identical archetype assignment per agent → identical resolved profile values.
3. **No new affordances per archetype.** Per FND-19 — every agent has the same action set; archetypes only modulate how the agent reasons (including which HTN methods it considers, with flat GOAP fallback intact per FND-20).
4. **Authoring control.** Scenarios specify how to assign archetypes: uniform random over a curated default five, uniform over an authored set, or frequency-weighted. A per-agent `AgentDef.archetype` override pins a specific agent's archetype, mirroring the existing per-agent profile-override idiom. Default policy when nothing is specified: uniform random over a curated set of 5 archetypes (Cautious, Bold, Methodical, Opportunistic, Sociable) so the diversity is broad but not overwhelming.
5. **PersonalityAssigned event replayable.** Spawn-time emission captures the archetype, seed, source, and a resolved-profile verification hash. Replay reconstructs identical agents deterministically from `archetype + seed + build-time template`; the hash detects divergence.
6. **Inspectable.** Observer surfaces archetype per agent and uses it in narrative rendering ("Agent A (Cautious) hesitated to enter the contested route").

## Non-Goals

- **No personality system beyond profile modulation.** Archetypes are not psychological models; they are delta templates over existing concrete parameters.
- **No new behavioral profile fields.** Archetypes reuse the existing levers (`*_backoff_ticks`, `dangerous_traversal_penalty`, `ask_memory_retention_ticks`, testimony weights, portfolio slot weights, etc.). No `willingness_to_*` or `backoff_ttl_multiplier` field is added — those behaviors are realized by delta-ing fields the runtime already reads.
- **No archetype switching at runtime.** An agent's archetype is set at spawn and persists. Runtime adaptation happens through S151 (testimony reliability, route preferences) and S22A learning state.
- **No archetype-keyed action restrictions.** A Fearful archetype can still fight; it just has higher `RiskWeightProfile.threat_aversion`. Method disabling narrows HTN decomposition only, never the action set (FND-19, FND-20).
- **No omniscient archetype detection.** Other agents cannot read another's archetype; they observe behavior.
- **No "rare" or "secret" archetypes.** All archetypes are equally available; rarity comes from `ArchetypeAssignmentPolicy::Weighted`.
- **No per-role or per-name assignment policy in this spec.** A `RoleTag` substrate does not exist in scenario types; per-role distributions are deferred to a future sibling spec. Explicit per-agent control is provided through `AgentDef.archetype` instead of a name-keyed policy map.
- **No budget-override modulation.** `AgentSchemaContextProfile.budget_overrides` (S146) is left untouched by archetypes; archetypes shape reasoning weights and patience, not per-goal planning budgets.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-2 (No Ungrounded Triggers or Probabilities) | Archetype assignment uses a seeded `DeterministicRng` whose per-agent sub-seed is derived by hashing `ScenarioDef.seed` with the stable agent index (via `seed_from_u64`-style derivation, not naked addition); assignment is deterministic and replayable. No naked probability constants. |
| FND-3 (Concrete State Over Abstract Scores) | Archetypes modulate existing concrete profile fields rather than introducing new abstract scores; no derived summary becomes a source of truth. |
| FND-19 (Agent Symmetry) | Archetypes do not change action sets, world constraints, or any rule the engine enforces. They only modulate the agent's reasoning weights and HTN method availability. |
| FND-20 (Resource-Bounded Practical Reasoning) | `disabled_methods` narrows HTN decomposition (search control) while flat GOAP fallback remains lawful; no archetype encodes plot rails. See Planner-Formalism Analysis in Section H. |
| FND-22 (Agent Diversity Through Concrete Variation) | Directly satisfies the principle — archetypes are concrete per-agent parameters that diversify needs, skills, values, courage, patience, and risk tolerance, all expressed as deltas to existing fields. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Initial archetype assignment is concrete state with `EventId` provenance (`PersonalityAssigned`); later learning sits on top per S151. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | When scenarios specify no archetype policy, the engine applies the default 5-archetype uniform policy; no legacy "all-identical" path is preserved, and no redundant new field shadows an existing lever. |
| FND-29 (Debuggability Is a Product Feature) | `PersonalityAssigned` event records the assignment and a resolved-profile verification hash; observer surfaces archetype in narrative; S144 diagnostics aggregate archetype distribution. |

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

The template carries **signed deltas to existing profile fields** plus a uniform backoff scale and a method-disable set. The resolver (D7) applies these to the default or scenario-authored profile values and clamps the results. Field names mirror the real fields they modulate; verified target fields are noted in comments.

```rust
// crates/worldwake-core/src/cognitive_archetype.rs
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArchetypeProfileTemplate {
    pub archetype: CognitiveArchetype,

    // CognitiveProfile (crates/worldwake-core/src/cognitive_profile.rs)
    pub max_plan_depth_delta: i8,                 // -> max_plan_depth (u8)
    pub repair_budget_fraction_delta: i32,        // -> repair_budget_fraction (Permille), permille delta
    pub switch_margin_delta: i32,                 // -> switch_margin (Permille), permille delta
    pub planning_switch_margin_delta: i32,        // -> planning_switch_margin (Permille), permille delta
    pub guard_min_confidence_ceiling_delta: i32,  // -> guard_min_confidence_ceiling (Permille), permille delta
    pub backoff_ticks_scale: BackoffScalePermille, // 1000 = unchanged; scales ALL existing *_backoff_ticks / *_block_ticks fields uniformly at resolution

    // PerceptionProfile (crates/worldwake-core/src/belief.rs)
    pub observation_budget_delta: i8,             // -> observation_budget (u8)

    // TestimonyTrustProfile (crates/worldwake-core/src/testimony_trust_profile.rs)
    pub testimony_confirmation_weight_delta: i32, // -> confirmation_weight (Permille), permille delta
    pub testimony_refutation_penalty_delta: i32,  // -> refutation_penalty (Permille), permille delta

    // RiskWeightProfile (crates/worldwake-core/src/risk_weight_profile.rs)
    pub threat_aversion_delta: i32,               // -> threat_aversion (Permille), permille delta

    // RoutePreferenceProfile (crates/worldwake-core/src/route_preference_profile.rs)
    pub dangerous_traversal_penalty_delta: i32,   // -> dangerous_traversal_penalty (Permille), permille delta (the "detour" lever)

    // EpistemicDispositionProfile (crates/worldwake-core/src/epistemic.rs)
    pub ask_memory_retention_ticks_delta: i32,    // -> ask_memory_retention_ticks (u32); negative = asks more often (the "willingness to ask" lever)

    // PortfolioWeightsProfile (crates/worldwake-core/src/portfolio_weights_profile.rs)
    pub portfolio_need_survival_delta: i32,       // -> need_survival (Permille), permille delta
    pub portfolio_pain_care_delta: i32,           // -> pain_care (Permille), permille delta
    pub portfolio_obligation_duty_delta: i32,     // -> obligation_duty (Permille), permille delta
    pub portfolio_economic_opportunity_delta: i32,// -> economic_opportunity (Permille), permille delta
    pub portfolio_social_motive_delta: i32,       // -> social_motive (Permille), permille delta

    // Method narrowing (AgentSchemaContextProfile.disabled_methods, crates/worldwake-core/src/agent_schema_context_profile.rs)
    pub method_disable: Vec<MethodSchemaId>,      // inserted into disabled_methods (denylist; default empty = all methods enabled)
}
```

Deltas are signed integers interpreted as `Permille` deltas (where the target field is `Permille`) or as absolute integer deltas (where the target is `u8`/`u32`). The resolver clamps applied `Permille` results to [0, 1000] and clamps integer fields to their non-negative ranges. There is no `method_enable` direction: `disabled_methods` is a denylist that defaults to empty (all methods enabled), so disabling is the only meaningful lever.

### D3: Archetype template table

```rust
// crates/worldwake-core/src/cognitive_archetype/templates.rs (new) — or an inline `mod templates` in cognitive_archetype.rs
pub fn cautious_template() -> ArchetypeProfileTemplate {
    ArchetypeProfileTemplate {
        archetype: CognitiveArchetype::Cautious,
        max_plan_depth_delta: 2,                              // plans more deeply
        repair_budget_fraction_delta: 100,                    // higher repair budget
        switch_margin_delta: 100,                             // stickier goal commitment
        planning_switch_margin_delta: 100,                    // stickier plan commitment
        guard_min_confidence_ceiling_delta: -100,             // less confident in guards
        backoff_ticks_scale: BackoffScalePermille::new_unchecked(1500), // waits 50% longer after failure
        observation_budget_delta: 1,                          // slightly more observation
        testimony_confirmation_weight_delta: -50,             // less easily convinced
        testimony_refutation_penalty_delta: 100,              // strong refutation penalty
        threat_aversion_delta: 200,                           // strongly danger-averse
        dangerous_traversal_penalty_delta: 150,               // dangerous routes hit harder (prefers detour)
        ask_memory_retention_ticks_delta: -4,                 // re-asks sooner (happy to confirm)
        portfolio_need_survival_delta: 50,
        portfolio_pain_care_delta: 0,
        portfolio_obligation_duty_delta: 0,
        portfolio_economic_opportunity_delta: -50,
        portfolio_social_motive_delta: 0,
        method_disable: vec![ /* method ids for group-confrontation methods, if any */ ],
    }
}

// ... templates for Bold, Stubborn, Methodical, Opportunistic, Sociable, Skeptical, Dutiful, Greedy, Fearful
```

All ten templates ship in this spec. The table is build-time data, like S146's `GoalSchema` registry. Greedy and Opportunistic archetypes express their bias through positive `portfolio_economic_opportunity_delta`; there is no separate "opportunistic" portfolio slot.

### D4: `ArchetypeAssignmentPolicy` and per-agent override

```rust
// crates/worldwake-core/src/archetype_assignment_policy.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub enum ArchetypeAssignmentPolicy {
    #[default]
    DefaultUniformFive,                       // Cautious/Bold/Methodical/Opportunistic/Sociable
    Uniform(BTreeSet<CognitiveArchetype>),    // uniform over the given set
    Weighted(BTreeMap<CognitiveArchetype, u32>), // weighted by integer weights
}
```

Per-role and explicit-by-name policies are out of scope (see Non-Goals): no `RoleTag` substrate exists in scenario types, and agent names are plain `String`. Explicit per-agent control is provided by the `AgentDef.archetype: Option<CognitiveArchetype>` override (D6), which mirrors the existing per-agent profile-override idiom and takes precedence over the policy.

### D5: `PersonalityAssigned` event

```rust
// crates/worldwake-core/src/event_tag.rs (variant added to the existing core EventTag enum)
EventTag::PersonalityAssigned

// crates/worldwake-core/src/cognitive_archetype.rs (payload + source, defined in core)
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersonalityAssignedPayload {
    pub agent: EntityId,
    pub archetype: CognitiveArchetype,
    pub seed: u64,                               // per-agent sub-seed used for this assignment
    pub source: ArchetypeAssignmentSource,
    pub resolved_profile_hash: StateHash,        // blake3 over the resolved archetype-affected profiles (crates/worldwake-core/src/canonical.rs)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArchetypeAssignmentSource {
    Policy(ArchetypeAssignmentPolicy),           // assigned by the scenario/default policy
    Explicit,                                    // pinned by AgentDef.archetype
}
```

The payload is carried by a new `Option<PersonalityAssignedPayload>` field on the shared `EventPayload` struct (`crates/worldwake-core/src/event_record.rs`), following the existing optional-payload convention used by `ContentionEventPayload`, `DecisionEventPayload`, and `ArtifactTransitionPayload` — there is no inline-variant payload mechanism on `EventTag`. The verification hash uses `StateHash([u8; 32])` via `canonical::hash_serializable` (`crates/worldwake-core/src/canonical.rs`); `blake3::Hash` is not used directly because it does not derive the `Eq`/`Ord`/`Serialize` bounds event payloads require. A single hash over the resolved archetype-affected profiles is sufficient for replay-divergence detection; the resolved values themselves persist on the agent's components and survive save/load. Replay reconstructs identical agents from `archetype + seed + build-time template` — the hash verifies, it does not reconstruct.

Emitted once per agent at spawn (scenario load).

### D6: Scenario integration

`ScenarioDef` (`crates/worldwake-cli/src/scenario/types.rs`) gains:

```rust
#[serde(default)]
pub archetype_assignment_policy: Option<ArchetypeAssignmentPolicy>,
```

`None` (the default) applies `ArchetypeAssignmentPolicy::DefaultUniformFive`. `ScenarioDef.seed: u64` already exists and seeds the assignment RNG.

`AgentDef` (`crates/worldwake-cli/src/scenario/types.rs`) gains:

```rust
#[serde(default)]
pub archetype: Option<CognitiveArchetype>,
```

When set, it pins that agent's archetype with `ArchetypeAssignmentSource::Explicit`, bypassing the policy draw. Existing scenarios continue to load (both fields default to `None`); their agents receive archetypes from the default policy unless an `AgentDef.archetype` override or per-field profile overrides are present.

### D7: Spawn-time resolution

`spawn_agent()` currently takes `(txn, recipes, agent_def, names, agent_locations)` (`crates/worldwake-cli/src/scenario/mod.rs:590`) and has no RNG or index, and the simulation `DeterministicRng` is constructed (`mod.rs:1493`) *after* the agent-spawning loop (`mod.rs:313`). This spec adds a dedicated archetype-assignment RNG:

1. Before the agent-spawning loop, construct an archetype-assignment RNG seeded from `def.seed` (independent of the simulation RNG so ordering is stable).
2. Iterate agents with a stable index (`for (agent_index, agent_def) in def.agents.iter().enumerate()`), threading `agent_index` and the assignment RNG (or a per-agent sub-seed) into `spawn_agent`.
3. Determine the archetype:
   - If `agent_def.archetype` is `Some`, use it with `source = Explicit`.
   - Otherwise draw from the effective `ArchetypeAssignmentPolicy` using a per-agent sub-seed derived by hashing `def.seed` with `agent_index` (via `seed_from_u64`-style derivation, not naked addition), with `source = Policy(policy)`.
4. Look up the `ArchetypeProfileTemplate` for that archetype.
5. Apply deltas to each affected profile, starting from the scenario-authored value if present, otherwise the profile default. Clamp `Permille` results to [0, 1000] and integer fields to their valid ranges. Apply `backoff_ticks_scale` uniformly to every `*_backoff_ticks` / `*_block_ticks` field on `CognitiveProfile`.
6. Insert `method_disable` ids into `AgentSchemaContextProfile.disabled_methods`.
7. Set `CognitiveArchetypeComponent` on the agent.
8. Emit `PersonalityAssigned` with the resolved-profile hash and source.

Explicit scenario-author overrides on individual profile fields take precedence over archetype deltas (the delta applies to the authored base value). The archetype provides defaults; authors can still tune. Because the deltas land on the existing profile fields the runtime already reads, no consuming system (`failure_handling.rs` TTLs, `route_threat.rs`, `candidate_generation.rs` AskWitness emitter, ranking) requires any change.

### D8: Observer rendering

Observer Section 1 (`## Section 1 — Run Metadata`, `crates/worldwake-cli/src/bin/observer.rs`) now renders an agent table with an `Archetype` column (landed by `archive/tickets/S152COGARCSEE-006.md`):
```
| Name | Archetype | EntityId |
```

Section 3b (`## Section 3b — Decision History`, `observer.rs`) appends archetype context to decision-history agent labels, e.g. `Agent A (Cautious)`.

### D9: S144 diagnostics extension

`ScenarioDiagnosticsReport` (`crates/worldwake-ai/src/scenario_diagnostics/mod.rs:14`) gains:

```rust
pub agent_archetypes: BTreeMap<CognitiveArchetype, u64>, // count per archetype in the scenario
```

Populated in `build_scenario_diagnostics` by counting `CognitiveArchetypeComponent` values across agents.

### D10: Golden coverage

`golden_archetypes.rs` covers:
- Same scenario + seed → identical archetype assignment.
- Cautious agent waits longer after failure than Bold agent (longer resolved `*_backoff_ticks` via `backoff_ticks_scale`).
- Sociable agent re-asks witnesses sooner than Skeptical agent under identical belief state (lower resolved `ask_memory_retention_ticks`).
- Greedy agent's `EconomicOpportunity` portfolio slot wins more often than Cautious agent's in dense opportunity scenarios.
- `PersonalityAssigned` event is emitted exactly once per agent at spawn.
- `AgentDef.archetype` override pins the archetype regardless of policy.
- Save/load preserves resolved profile values and `CognitiveArchetypeComponent`.

## FND-01 Section H Analysis

### Information-Path Analysis

Archetype is *assigned* at spawn; it does not propagate through perception. Other agents who interact with an archetype-shaped agent observe behavior, not the archetype label. Per FND-19 / FND-29, the archetype is inspectable through observer / debug tooling and the `PersonalityAssigned` event, but not through in-world perception.

### Positive-Feedback Analysis

Not applicable. Archetype is initialization-time state; it does not amplify itself, and it modulates static profile values rather than any runtime loop.

### Concrete Dampeners

Not applicable (no positive-feedback loop introduced).

### Stored State vs. Derived Read-Model List

**Stored state**:
- `CognitiveArchetypeComponent { archetype: CognitiveArchetype }` on the agent (per-agent runtime state, save/load round-trip).
- The resolved profile values themselves (existing components `CognitiveProfile`, `PerceptionProfile`, `RiskWeightProfile`, `TestimonyTrustProfile`, `RoutePreferenceProfile`, `EpistemicDispositionProfile`, `PortfolioWeightsProfile`, `AgentSchemaContextProfile`), mutated once at spawn.
- `ArchetypeAssignmentPolicy` on `ScenarioDef`; `AgentDef.archetype` override.
- `PersonalityAssignedPayload` recorded in the append-only event log.

**Derived read-model**:
- `ArchetypeProfileTemplate` lookup is build-time data; resolved per-agent profile values are computed once at spawn and then stored on the existing profile components.
- `resolved_profile_hash` is a verification digest over stored state, never a source of truth.
- `ScenarioDiagnosticsReport.agent_archetypes` is a derived count over `CognitiveArchetypeComponent` values.

### Planner-Formalism Analysis

The only planner-facing lever is `method_disable`, which inserts ids into `AgentSchemaContextProfile.disabled_methods`. This is HTN **search control**, not method-required behavior: disabling a method removes a decomposition strategy from consideration, and the HTN selector already falls back to flat GOAP / remaining methods for the same goal. No archetype encodes plot rails, scene-specific success paths, or target-specific logic; disabling is a per-agent expression of how that kind of agent prefers to pursue goals (FND-20). No new `GoalKind`, method schema, or affordance is introduced, so no method-required schema contract is needed.

## SystemFn Integration

No new `SystemFn`. Resolution runs at agent spawn (scenario load) in `worldwake-cli`.

## Component Registration

- **New universal component**: `CognitiveArchetypeComponent { archetype: CognitiveArchetype }`, defined in `worldwake-core` and registered on `EntityKind::Agent` via the `with_component_schema_entries!` macro (`crates/worldwake-core/src/component_schema.rs`) with insert/get accessors and the `|kind| kind == EntityKind::Agent` filter. Classification: **scenario-authorable universal** — every agent has one. `Default` returns `CognitiveArchetype::Methodical` (a neutral default). Authored via the `AgentDef.archetype: Option<CognitiveArchetype>` field (D6); applied in `spawn_agent` (D7). No `*Def` wrapper is needed (no `EntityId` references). The component is read only by observer rendering (D8) and diagnostics (D9), never at decision time.
- **No new profile fields**: archetype effects land entirely on existing profile components.

## Cross-System Interactions

- `worldwake-cli` scenario loader resolves archetypes, mutates the existing profile components, sets `CognitiveArchetypeComponent`, and emits `PersonalityAssigned`.
- `worldwake-sim` records the event in the append-only log.
- All consuming systems (planning, perception, ranking, exploration, failure handling, route threat) read the existing profile fields exactly as they do today — the archetype only changed the resolved values written at spawn. Archetype is *not* read at decision time.

State-mediated per FND-26.

## Profile-Driven Parameters

All archetype template fields are `Permille` deltas, `i8`/`i32` integer deltas, a `BackoffScalePermille` backoff scale, or a `Vec<MethodSchemaId>`. No floats. `BackoffScalePermille` is a typed integer scale where `1000 == 1x`; it supports both below-identity and above-identity retry/backoff variation without overloading bounded `Permille`.

`ArchetypeAssignmentPolicy::Weighted` uses `u32` integer weights. Assignment iterates `BTreeSet`/`BTreeMap` in deterministic order (CLAUDE.md determinism invariant).

## Test Plan

- D10 golden coverage (7 scenarios).
- Determinism: 10 archetype templates × 5 seeds × per-archetype regression tests of resolved profile values.
- Save/load coverage for `CognitiveArchetypeComponent` and resolved profile fields.
- Scenario loader test: `ScenarioDef` with no archetype policy → uniform default-five applied; `AgentDef.archetype` override pins the archetype.
- Resolver clamp tests: deltas that would push a `Permille` field past [0, 1000] clamp correctly.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
