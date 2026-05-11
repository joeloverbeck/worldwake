# S138: Affordance-to-Opportunity Compiler with Effect-Schema Indexing

**Status**: Draft

## Summary

Candidate generation in `crates/worldwake-ai/src/candidate_generation.rs` is purely top-down emitter-driven: ~15 `emit_*` functions read agent need/obligation/threat state and emit goal candidates. `relevant_ops` (`crates/worldwake-ai/src/goal_dispatch_decl.rs:57`) declares per-goal-kind which `PlannerOpKind`s are "relevant" — and a conformance test enforces that the declaration matches the live goal-model dispatch. As an architectural pattern, this works for ~40 goal kinds and hand-curated scenarios. It begins to fail when scenarios offer many emergent paths to the same end: `AcquireCommodity(Food)` only considers Trade/Harvest/Craft/Queue/Travel/MoveCargo because those are the declared `relevant_ops`. A starving agent next to an unguarded basket of bread cannot discover *steal*; a thirsty agent in a deserted hut cannot discover *beg from the household altar*; a hunter past their last reliable source cannot discover *raid an abandoned camp*.

The architectural shift the assessor proposes is bottom-up: for every perceived entity, record, place, route, social fact, and affordance, derive what effects it could enable, what motives those effects would satisfy, what risks or legal consequences they create, and what information they could reveal. The bottom-up pass produces `Opportunity` records that the existing emitters consume alongside the top-down pass. `relevant_ops` becomes a hint that biases search, not the authoritative gate of possibility. Authority moves to a queryable index over `ActionDef.effect_schema` (S134): "give me every action whose effect schema produces `EffectFact::OwnsCommodity(Food)` against an agent-accessible target."

S138 also folds in the assessor's PR-13 (richer travel pruning — detours that satisfy causal landmarks, reduce risk, gain information) and PR-20 (richer interrupt-layer opportunism). The opportunity compiler is the unifying surface: travel pruning becomes opportunity-aware, and the existing interrupt layer (`crates/worldwake-ai/src/interrupts.rs`) enriches its fired set from the same opportunity index. A panicked agent who sees a corpse, an unattended valuable, or a wounded ally generates opportunities through the same pass that emits "I see bread" and "I hear a cart on the road."

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-ai` — new `opportunity_compiler` module owning the `Opportunity` rich record, the per-tick compilation pass that runs before candidate generation, and the new typed enums it carries (`EffectFactKey`, `RiskFact`, `ClaimTopic`, `BelievedLegalStatus`, `SocialExposureBand`). New `effect_schema_index` module that builds a `BTreeMap<EffectFactKey, Vec<ActionDefId>>` from the current `ActionDefRegistry`. `candidate_generation.rs` consumes opportunities as a parallel input alongside existing emitters. `goal_dispatch_decl.rs` reclassifies `relevant_ops` from authoritative gate to ranking hint via a new `Authority { Gate, HintOnly }` enum. `interrupts.rs` extends to read opportunities. `search/heuristic.rs::prune_travel_away_from_goal_with_expansion_trace` consumes an opportunity-aware detour budget. `decision_trace.rs` extends `RootCandidateTrace` with a source field and adds an `OpportunityCompilerLoad` counter.
- `worldwake-core` — adds two new universal-on-Agent components (`RiskWeightProfile`, `LawAbidingProfile`); extends `PerceptionProfile` (`crates/worldwake-core/src/belief.rs:2644`) with an opportunity-salience floor field as a sibling to the existing `salience_policy`; extends `CognitiveProfile` (`crates/worldwake-core/src/cognitive_profile.rs`) with `detour_budget_permille` and the per-agent compiler/index soft caps. `OpportunityKey` and `OpportunityAnchor` (`crates/worldwake-core/src/goal.rs:192-204`) are unchanged — they remain the slim identity used by emitters and `LearnedOpportunityMemory`; all rich typed data is carried on the ai-side `Opportunity` record.
- `worldwake-sim` — `GoalBeliefView` (`crates/worldwake-sim/src/belief_view.rs:269`) gains accessors for `RiskWeightProfile` and `LawAbidingProfile`; the `RuntimeBeliefView` blanket impl forwards them; `PerAgentBeliefView` (`crates/worldwake-sim/src/per_agent_belief_view.rs:1493`) backs them.
- `worldwake-systems` — no change. Action effects already declare what they produce; the index reads the registry.
- `worldwake-cli` — `AgentDef` (`crates/worldwake-cli/src/scenario/types.rs:571`) gains optional `risk_weight_profile` and `law_abiding_profile` fields; `spawn_agent` (`crates/worldwake-cli/src/scenario/mod.rs:616-654`) gains the matching `set_component_*` calls following the existing `metabolism_profile`/`cognitive_profile` precedent. Observer Section 3 (`crates/worldwake-cli/src/bin/observer.rs:684`) renders compiled opportunities per agent per tick as a sibling sub-section coexisting with the existing decision-history table and with S137's `EventTag::RepairApplied` rendering.

## Dependencies

- S134 (Canonical Effect Schema) — completed and archived at `archive/specs/S134-canonical-effect-schema.md`. **Hard dependency satisfied.** The `EffectSchemaIndex` queries `ActionDef.effect_schema`; no transitional hand-maintained opportunity-effect key is needed.
- S105 (Observation Salience Filtering) — completed. Opportunity compilation runs per perceived entity; budget already governs which entities are perceived.
- S130 (Survey Records and Frontier Disconfirmation) — completed. `SurveyMemory` damps opportunities anchored on confirmed-empty places — the compiler reads survey memory before emitting an opportunity.
- S107 (Proactive Diversification) — completed. `DiversificationProfile` continues to bias `ExploreLocation` opportunities; the compiler consults it.
- S108 (Per-Action Binding Strictness) — completed. `BindingStrictness` continues to govern which entities can satisfy an opportunity's `required_actions`.
- S138 ↔ S137 soft relationship: with S137, repair's `RepairKind::RebindTarget` consumes opportunity-compiler output. Order-independent.
- S141 (Motive Source Ledger) — Phase 11 sibling. Soft dependency: opportunities can name `MotiveSource`s the action's effects would satisfy; if S141 lands first, those references are typed; if S138 lands first, opportunities carry the existing motive-derived matchers.

## Design Goals

1. **Bottom-up parity with top-down.** Every perceived entity, record, route, social fact, and place produces zero or more `Opportunity` records per tick. Opportunities feed candidate generation alongside existing emitters; they do not replace them.
2. **Effect-schema is the authority.** The `EffectSchemaIndex` answers "which actions produce this effect?" Goals query the index when their `relevant_ops` hints are exhausted or when motive urgency exceeds a per-agent threshold.
3. **`relevant_ops` becomes a hint.** The existing declaration stays for fast-path ranking. When the agent's urgency or learned-opportunity memory indicates the hint is insufficient, the compiler queries the effect-schema index and emits additional candidates. The conformance test (`test_declaration_relevant_ops_match_live_goal_model` at `crates/worldwake-ai/src/goal_dispatch_decl.rs:971`) continues to assert that `relevant_ops` matches the live goal model's `relevant_op_kinds()` exactly — the test's *assertion* is unchanged; only the *runtime authority* of the slice changes (from gate to hint). The static slice remains the curated fast-path; the effect-schema index becomes the broader authority when the hint is exhausted.
4. **Per-perception cost bound.** Compilation runs in O(|perceived entities| × |relevant action effects|), with `relevant action effects` bounded by `EffectSchemaIndex` lookup. Deterministic, bounded, no global scan.
5. **Risk and legality declared, not imposed.** Each opportunity declares believed legality (`BelievedLegalStatus`) and social exposure (`SocialExposureBand`) so ranking can weigh them per-agent. The compiler does not filter out illegal opportunities — that is the agent's per-profile decision.
6. **Travel pruning becomes opportunity-aware.** `prune_travel_away_from_goal_with_expansion_trace` consults the opportunity index along the candidate detour: a detour that produces a high-salience opportunity (witness encounter, prerequisite acquisition, risk reduction observation) is allowed even if `remaining_cost` increases, bounded by `CognitiveProfile.detour_budget_permille`.
7. **Interrupt layer enrichment.** `crates/worldwake-ai/src/interrupts.rs` reads the opportunity index for high-salience opportunities (dragon perception, wounded-ally perception, vulnerable-rival perception, unattended-valuables perception). The interrupt layer remains the existing fire-or-not gate; opportunities populate the gate's input set rather than being a separate channel.
8. **Determinism.** Opportunity emission iterates perceived entities in `BTreeMap`-stable order. The `EffectSchemaIndex` is a `BTreeMap`. Deterministic across runs.
9. **No silent privilege.** Opportunities cannot bypass FND-7 (locality) — they are anchored to the agent's perceived/believed state. The compiler does not query global truth.

## Non-Goals

- **HTN methods over opportunities.** Methods (Phase 12) decompose high-level procedures into opportunity-driven subtask plans. S138 only lands the opportunity substrate.
- **Cross-agent opportunity sharing.** Opportunities are per-agent. Cross-agent propagation flows through `ShareBelief` over the underlying beliefs, not through opportunity gossip.
- **Persistent `Opportunity` storage.** Opportunities are derived per-tick state (FND-27). The agent's existing `LearnedOpportunityMemory` (S109) records *which opportunities the agent has previously chosen and how that turned out*, not opportunities themselves.
- **Goal-kind expansion.** S138 does not add new `GoalKind` variants. Steal/Loot already exist as actions (`steal` is registered in `crates/worldwake-systems/src/transport_actions.rs:172`; `LootCorpse` is a `GoalKind` variant). Beg has no existing action; opportunities that would route through a Beg-style affordance fold into `AcquireCommodity` with risk/legality variation until a future spec introduces the action explicitly. The compiler does not synthesize action types it cannot bind to.
- **Transitional effect-schema index.** Not applicable; S134 is complete, so S138 uses the real `ActionDef.effect_schema` index directly.
- **`InspectContainer` action.** Spec scenario 3 originally cited it as a verification action; it does not currently exist in the codebase. Inspection-style verifications are scoped to a future spec — S138 only emits opportunities that bind to existing actions (`AskWitness`, `SearchPlace`, etc.).

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-1 (Maximal Emergence Through Local Causality) | The bottom-up pass is the architectural shape that lets agents exploit what scenarios offer rather than only what emitters were authored to consider. |
| FND-3 (Concrete State Over Abstract Scores) | Opportunities reference typed effect facts, typed risks, typed information topics. No abstract "opportunity score" promoted to truth. |
| FND-7 (Locality of Motion, Interaction, and Communication) | All inputs to the compiler are agent-local: perceived entities, believed records, observed routes, recalled habits. No global query. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Opportunities anchored on co-located entities use the same FND-14A read path the planner already uses for direct observation. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | Opportunities carry the underlying belief's provenance via `Opportunity.source_belief: BeliefRef`. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | The compiler is bounded by perceived-entity count × effect-index lookup. No script-driven "this scenario should offer this opportunity now." |
| FND-22 (Agent Diversity Through Concrete Variation) | Per-agent `RiskWeightProfile`, `LawAbidingProfile`, and existing utility weights cause two agents seeing the same bread to compile different rankings over the buy/steal/beg/wait opportunities. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | The compiler reads the agent's perception output, the action registry, and the opportunity-anchor entities. No cross-system imperatives. |

## Deliverables

### `worldwake-ai::opportunity_compiler` (new module)

```rust
pub struct Opportunity {
    pub key: OpportunityKey,                    // slim identity; carries goal_key + anchor
    pub perceived_at: Tick,
    pub source_belief: BeliefRef,
    pub possible_effects: Vec<EffectFactKey>,   // bounded by per-agent compiler cap (see Profile-Driven Parameters)
    pub possible_information: Vec<ClaimTopic>,
    pub required_actions: Vec<PlannerOpKind>,
    pub legal_status: BelievedLegalStatus,
    pub social_exposure: SocialExposureBand,
    pub risks: Vec<RiskFact>,
    pub salience: Permille,
}

pub fn compile_opportunities(
    agent: EntityId,
    belief_view: &impl RuntimeBeliefView,
    action_index: &EffectSchemaIndex,
) -> Vec<Opportunity> { /* … */ }
```

`compile_opportunities` reads perceived entities, observed records, observed routes, and recalled habits through `belief_view`'s existing accessors (`agent_belief_store`, `entities_at`, `effective_place`, `learned_opportunity_memory`, `survey_memory`, plus the new `risk_weight_profile` and `law_abiding_profile` accessors added in this spec). No separate `PerceptionState` parameter is needed — the agent's perception output is already in the belief store by the time the compiler runs. The result length is bounded by the per-agent `compile_opportunity_cap` field on `CognitiveProfile` (see Profile-Driven Parameters); truncation prefers higher-salience entries.

### `worldwake-ai::effect_schema_index` (new module)

```rust
pub struct EffectSchemaIndex {
    pub by_effect: BTreeMap<EffectFactKey, Vec<ActionDefId>>,
}

impl EffectSchemaIndex {
    pub fn build(registry: &ActionDefRegistry) -> Self { /* … */ }
    pub fn actions_producing(&self, fact: EffectFactKey) -> &[ActionDefId] { /* … */ }
}
```

`ActionDefRegistry` lives in `crates/worldwake-sim/src/action_def_registry.rs:6`. The index is built once at simulation startup (registry construction) and shared across the run, never per-tick.

### `relevant_ops` reclassification (in `goal_dispatch_decl.rs`)

Introduce a new `Authority` enum next to `GoalDispatchDeclaration` in `crates/worldwake-ai/src/goal_dispatch_decl.rs`:

```rust
pub enum Authority {
    Gate,      // the slice is the definitive answer; do not extend at runtime
    HintOnly,  // the slice biases search; the EffectSchemaIndex is the broader authority
}
```

The static `GoalDispatchDeclaration.relevant_ops` slice remains exactly as today. Add `relevant_ops_authority(goal: &GoalKind) -> Authority` returning `Authority::HintOnly` for all goal kinds at landing. Candidate generation queries the `EffectSchemaIndex` whenever `urgency_class >= GoalPriorityClass::HighPriority` AND the hint set is exhausted (`relevant_ops` did not bind a candidate). The conformance test `test_declaration_relevant_ops_match_live_goal_model` (`goal_dispatch_decl.rs:971`) continues to assert `relevant_ops` equals `relevant_op_kinds()` — the assertion is unchanged.

### Travel-pruning extension (in `search/heuristic.rs`)

The function at `crates/worldwake-ai/src/search/heuristic.rs:248` gains two new parameters:

```rust
pub(super) fn prune_travel_away_from_goal_with_expansion_trace(
    candidates: &mut Vec<SearchCandidate>,
    expansion_candidates: Option<&mut Vec<crate::decision_trace::ExpansionCandidateTrace>>,
    current_place: EntityId,
    goal_places: &[EntityId],
    snapshot: &PlanningSnapshot,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    detour_budget_permille: Permille,                // NEW (from CognitiveProfile)
    opportunity_index: &PerceivedOpportunityIndex,   // NEW
) -> Option<crate::decision_trace::TravelPruningTrace> { /* … */ }
```

A detour is allowed if the summed opportunity-salience along the detour path × `detour_budget_permille` exceeds the cost increase, deterministically (sorted-iteration over the salience contributions). `PerceivedOpportunityIndex` is a per-tick view defined alongside `compile_opportunities` (see "New typed enums and read-models" below).

### Interrupt-layer enrichment (in `interrupts.rs`)

The `evaluate_interrupt` function at `crates/worldwake-ai/src/interrupts.rs:31` already gates on `ranked_candidates`. Extend the candidate set seen by the interrupt layer with opportunity-derived candidates (salience × effect-to-motive-satisfaction). The interrupt fires when the opportunity's expected-motive-satisfaction exceeds the active commitment's expected-motive-satisfaction by at least the existing `frame_switch_margin`. No new interrupt channel is added; opportunities populate the existing `ranked_candidates` input set.

### Observer Section 3

The existing Section 3 (Decision History) at `crates/worldwake-cli/src/bin/observer.rs:684` is a markdown table (`| Tick | Agent | Event | Payload |`) and is shared with S137's `EventTag::RepairApplied` rendering. S138 adds a sibling sub-section **3a Opportunities** rendered before the decision-history table, with the existing decision-history becoming **3b Decision History**. The opportunity sub-section renders compiled opportunities per agent per tick (top-K by `Opportunity.salience`):

```
Section 3a — Opportunities

Tick 412 — Agent A:
  bread@bakery: salience 720 — effects: OwnsCommodity(Bread); legal: BelievedOwned(baker); exposure: Public
  bread@bakery: salience 540 — effects: OwnsCommodity(Bread); legal: BelievedOwned(baker)→Steal; exposure: Public+CriminalRisk
  altar@hut: salience 380 — effects: OwnsCommodity(Bread); legal: SociallyOpenToRequest; exposure: Public+ShameRisk
```

K is governed by `observer.section_3a_top_k` (existing observer-args precedent) or a fixed default if no flag exists at landing.

### New typed enums and read-models (in `worldwake-ai/src/opportunity_compiler`)

All new types live in `worldwake-ai` alongside `Opportunity`. None are core-side because none are stored as ECS components — they are per-tick derived view types.

```rust
// Discriminant mirror of EffectFact (worldwake-sim/src/effect_schema.rs:209) — payloadless
// keys for BTreeMap-indexing the EffectSchemaIndex.
pub enum EffectFactKey {
    CommodityTransfer,
    PartialQuantity,
    WoundApplied,
    ExpectationFulfilled,
    ContentionGrantConsumed,
    EventEmitted,
}

pub enum RiskFact {
    CriminalLiability { violation_kind: ViolationKind },
    SocialShameRisk,
    ThreatPresence { source: EntityId },
    InjuryRisk,
    PropertyForfeitureRisk,
}

pub enum ClaimTopic {
    EntityLocation { subject: EntityId },
    CommodityAvailability { commodity: CommodityKind, place: EntityId },
    OwnershipClaim { item: EntityId },
    HostilePresence { place: EntityId },
    RouteSafety { from: EntityId, to: EntityId },
}

pub enum BelievedLegalStatus {
    BelievedOwned { owner: EntityId },
    BelievedUnclaimed,
    BelievedContested,
    SociallyOpenToRequest,
    Forbidden { jurisdiction: EntityId },
}

pub enum SocialExposureBand {
    Private,
    Public,
    PublicWithCriminalRisk,
    PublicWithShameRisk,
}

// Per-tick view consumed by travel pruning and interrupts.
pub struct PerceivedOpportunityIndex {
    pub by_place: BTreeMap<EntityId, Vec<OpportunityHandle>>,
    pub by_anchor: BTreeMap<EntityId, OpportunityHandle>,
    pub all: Vec<Opportunity>,
}

pub struct OpportunityHandle(pub u32); // dense index into PerceivedOpportunityIndex.all
```

All derives match existing analog patterns: `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize` on payload-free enums; `Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize` on payload-bearing enums (matches `EffectFact` in sim). `EffectFactKey` is a payloadless discriminant — adding a new `EffectFact` variant in sim requires the matching `EffectFactKey` variant addition in ai, enforced via an exhaustiveness test.

### New universal-on-Agent components and their scenario contract

`RiskWeightProfile` and `LawAbidingProfile` follow `docs/spec-drafting-rules.md` Section 5 in full:

```rust
// crates/worldwake-core/src/risk_weight_profile.rs (new)
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RiskWeightProfile {
    pub theft_aversion: Permille,
    pub exposure_aversion: Permille,
    pub threat_aversion: Permille,
}

// crates/worldwake-core/src/law_abiding_profile.rs (new)
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct LawAbidingProfile {
    pub criminal_threshold: Permille,
    pub social_norm_weight: Permille,
}
```

Required wiring (per Section 5 + the "New Component on EntityKind::Agent" and "New Component Read by AI Crate" patterns):

1. **`component_schema.rs` registration** in `crates/worldwake-core/src/component_schema.rs` for both components with `|kind| kind == EntityKind::Agent` filter, mirroring the existing `cognitive_profile` and `metabolism_profile` entries.
2. **`AgentDef` fields** in `crates/worldwake-cli/src/scenario/types.rs` next to `cognitive_profile: Option<CognitiveProfile>` (line 592) and `metabolism_profile: Option<MetabolismProfile>` (line 618):
   ```rust
   #[serde(default)]
   pub risk_weight_profile: Option<RiskWeightProfile>,
   #[serde(default)]
   pub law_abiding_profile: Option<LawAbidingProfile>,
   ```
3. **`spawn_agent` set-calls** in `crates/worldwake-cli/src/scenario/mod.rs` near the existing `let cognitive = agent_def.cognitive_profile.unwrap_or_default(); …` (line 641) and `let metabolism = agent_def.metabolism_profile.unwrap_or_default(); txn.set_component_metabolism_profile(...)` (lines 616-617):
   ```rust
   let risk_weight = agent_def.risk_weight_profile.unwrap_or_default();
   txn.set_component_risk_weight_profile(agent_id, risk_weight)?;
   let law_abiding = agent_def.law_abiding_profile.unwrap_or_default();
   txn.set_component_law_abiding_profile(agent_id, law_abiding)?;
   ```
4. **`Default` impls** are required on both (universal classification) and are provided by `#[derive(Default)]` on the structs above (all fields are `Permille` which has `Default == ZERO`).
5. **`GoalBeliefView` accessors** in `crates/worldwake-sim/src/belief_view.rs` (trait at line 269, blanket impl at line 1416) — add accessors on the `ProfileBeliefView` sub-trait alongside the existing per-agent profile accessors:
   ```rust
   fn risk_weight_profile(&self, agent: EntityId) -> &RiskWeightProfile;
   fn law_abiding_profile(&self, agent: EntityId) -> &LawAbidingProfile;
   ```
   With matching backing impls on `PerAgentBeliefView` (`crates/worldwake-sim/src/per_agent_belief_view.rs:1493`) and the existing sub-trait forwarders.

### `PerceptionProfile` opportunity-floor field (in `worldwake-core/src/belief.rs`)

`SaliencePolicy` (`crates/worldwake-core/src/belief.rs:69`) is a one-variant enum acting as a discriminant on `PerceptionProfile.salience_policy` (`belief.rs:2666`). The opportunity-salience floor is a per-agent perception parameter, not a discriminant axis, so it joins `PerceptionProfile`'s existing sibling cluster of perception-budget and salience-shaping fields (`observation_budget`, `omission_log_capacity`, `need_salience_boost`, `need_salience_urgency_threshold`). Add one new field on `PerceptionProfile`:

```rust
#[serde(default = "default_opportunity_floor_permille")]
pub opportunity_floor_permille: Permille,
```

Opportunities below this floor are not emitted from `compile_opportunities`. Default value: `Permille::new_unchecked(100)` (10%). The `#[serde(default)]` annotation keeps existing scenarios deserializing without scenario-author churn. `PerceptionProfile`'s existing `#[serde(deny_unknown_fields)]` attribute is unaffected by the addition.

### Decision-trace surface (in `worldwake-ai/src/decision_trace.rs`)

Two additions:

1. `RootCandidateTrace` (`decision_trace.rs:820`) gains a `source: CandidateSource` field:
   ```rust
   pub enum CandidateSource {
       Emitter,
       OpportunityCompiler,
   }
   ```
2. New `OpportunityCompilerLoad` counter struct alongside the existing trace-sink counters:
   ```rust
   pub struct OpportunityCompilerLoad {
       pub compiled_count: u32,
       pub salience_floored: u32,
       pub learned_memory_damped: u32,
       pub cap_truncated: u32,
   }
   ```
   Recorded per-agent per-tick on the `DecisionTraceSink`. The observer's existing Section 9 (Budget Exhaustion) reports the counter alongside other per-tick load metrics.

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** No new cross-agent path. The compiler reads only the agent's perception/belief output. `EffectSchemaIndex` is registry data, not state. The opportunity-aware travel pruner reads the agent's own opportunity set.
2. **Positive-feedback analysis.** Potential loop: more perceived entities → more opportunities → wider candidate space → richer plans → agent visits more places → more perceived entities. **Concrete dampener:** S105 perception budget caps perceived entities per tick; `compile_opportunities` is bounded by that budget × per-entity effect-index lookup; `LearnedOpportunityMemory` (S109) damps repeatedly-failed opportunities. The `OpportunityCompilerLoad` decision-trace counter (defined in the decision-trace deliverable) records each tick's load for inspection.
3. **Concrete dampeners.**
   - Per-tick perceived-entity ceiling (S105).
   - `LearnedOpportunityMemory` decay damps repeated emission of low-yield opportunities.
   - `CognitiveProfile.detour_budget_permille` caps travel-pruning indulgence.
   - `CognitiveProfile.compile_opportunity_cap` caps the result length of `compile_opportunities` per tick per agent.
   - `PerceptionProfile.opportunity_floor_permille` floor below which opportunities are not emitted.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state**: `LearnedOpportunityMemory` (already in `worldwake-core/src/learned_opportunity_memory.rs`, S109); two new universal-on-Agent components `RiskWeightProfile` and `LawAbidingProfile` (sibling components, not extensions of `PreferenceProfile`); new field on `PerceptionProfile.opportunity_floor_permille`; new fields on `CognitiveProfile` (`detour_budget_permille`, `compile_opportunity_cap`).
   - **Derived read-model**: `Opportunity` records (per-tick, ai-side), `EffectSchemaIndex` (registry-time, rebuilt only on `ActionDefRegistry` change — i.e., simulation startup; no per-tick rebuild), `PerceivedOpportunityIndex` (per-tick view), `OpportunityCompilerLoad` (per-tick trace counter).

## SystemFn Integration

No new `SystemFn` in the simulation tick. `compile_opportunities` runs synchronously within `agent_tick` — specifically inside `crates/worldwake-ai/src/agent_tick/observation.rs` immediately before the existing `generate_candidates_with_*` invocation at line 273. The `AgentTickDriver` (`crates/worldwake-ai/src/agent_tick/mod.rs`) orchestrates `agent_tick`; this spec inserts the compiler call into the observation phase, not into a new top-level SystemFn. The `EffectSchemaIndex` is built once at simulation startup (registry construction in `ActionDefRegistry`) and shared across the run; the `PerceivedOpportunityIndex` is built per-tick from the compiler's output and consumed by travel pruning and interrupts within the same tick.

## Component Registration

- `LearnedOpportunityMemory` — already registered (S109).
- `RiskWeightProfile` (per-agent, universal) — register on `EntityKind::Agent` with default. Full Section 5 deliverables (component definition, `AgentDef` field, `spawn_agent` set-call, `Default` impl, `GoalBeliefView` accessor) are enumerated under "New universal-on-Agent components and their scenario contract" in Deliverables.
- `LawAbidingProfile` (per-agent, universal) — same Section 5 deliverables enumerated alongside `RiskWeightProfile`.
- `Opportunity`, `PerceivedOpportunityIndex`, `OpportunityCompilerLoad`, and `EffectSchemaIndex` are *not* stored components; they are derived per-tick (or per-startup) view types in `worldwake-ai`.

## Cross-System Interactions

- **AI → AI internal**: opportunity compiler → candidate generator → ranking.
- **AI → Sim**: no new cross-system call. The compiler reads the existing perception output and existing registry.
- **Sim → CLI**: observer reads decision-trace; no new sim API.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

- `RiskWeightProfile { theft_aversion: Permille, exposure_aversion: Permille, threat_aversion: Permille }` — universal per-agent.
- `LawAbidingProfile { criminal_threshold: Permille, social_norm_weight: Permille }` — universal per-agent.
- `CognitiveProfile.detour_budget_permille: Permille` — extension; default `Permille::new_unchecked(150)` (matches the `switch_margin: Permille::new_unchecked(100)` precedent at `crates/worldwake-core/src/cognitive_profile.rs:117`). Used by the travel-pruning extension.
- `CognitiveProfile.compile_opportunity_cap: u16` — extension; default `16`. Soft cap on the result length of `compile_opportunities` per tick per agent; the cap is the workspace-native substitute for the originally-considered `SmallVec<Opportunity, 16>` fixed inline size (matches the `decision_history_alternatives: u8` precedent at `cognitive_profile.rs:103`).
- `PerceptionProfile.opportunity_floor_permille: Permille` — new field on the existing core type; default `Permille::new_unchecked(100)`. Below-floor opportunities are not emitted.

Per FND-22, two agents seeing the same opportunity rank it differently because their profile values differ. The `CognitiveProfile` soft caps interact with the existing `CognitiveProfile` modulation surface (the wounded/exhausted modulation pattern is a downstream extension and is not delivered in S138 — it is noted here only as a future hook).

## Validation and Falsification

- **Golden coverage**: new `golden_opportunity_compiler.rs` with five scenarios:
  1. Starving agent + unguarded bread + no merchant → expects `Steal` opportunity emitted, ranked above `Wait`, ranked below `Buy` only when merchant is present.
  2. Thirsty agent + dry well + nearby alternative source via opportunity → expects detour-budget pruning to allow the alternative.
  3. Witness opportunity along travel route → expects detour-budget pruning to allow an `AskWitness` detour (action already registered at `crates/worldwake-systems/src/epistemic_actions.rs`), bounded.
  4. Effect-schema index miss → unknown effect produces no opportunity (negative test).
  5. `LearnedOpportunityMemory` damping after repeated failure → expects salience reduction over successive ticks.
- **`relevant_ops` regression**: pre-S138 emitter behavior on `survival-baseline.ron` produces identical event log. The bottom-up pass is additive at default profiles.
- **Performance**: per-tick opportunity-compilation duration ≤ 5% of agent_tick total under 1440-tick `survival-contested.ron` (4 agents). Soak measures.

## Risks

- **Effect-schema index timing.** S134 is complete, so S138's implementation should build the index from real `ActionDef.effect_schema` entries directly rather than landing a hand-maintained transitional subset.
- **Opportunity explosion in dense scenes.** A market with 80 vendors could compile 80+ opportunities. Mitigation: salience floor; per-tick opportunity cap (16 per agent); reuse of S105's perception budget upstream so the compiler input is already truncated.
- **Travel-detour budget mis-tuning.** A too-permissive `detour_budget_permille` could turn travel into wandering. Mitigation: default `Permille::new_unchecked(150)` is conservative; goldens (scenario 2 above) lock the boundary; observer surfaces detour decisions with attribution.
