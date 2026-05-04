# S138: Affordance-to-Opportunity Compiler with Effect-Schema Indexing

**Status**: Draft

## Summary

Candidate generation in `crates/worldwake-ai/src/candidate_generation.rs` is purely top-down emitter-driven: ~15 `emit_*` functions read agent need/obligation/threat state and emit goal candidates. `relevant_ops` (`crates/worldwake-ai/src/goal_dispatch_decl.rs:57`) declares per-goal-kind which `PlannerOpKind`s are "relevant" — and a conformance test enforces that the declaration matches the live goal-model dispatch. As an architectural pattern, this works for ~40 goal kinds and hand-curated scenarios. It begins to fail when scenarios offer many emergent paths to the same end: `AcquireCommodity(Food)` only considers Trade/Harvest/Craft/Queue/Travel/MoveCargo because those are the declared `relevant_ops`. A starving agent next to an unguarded basket of bread cannot discover *steal*; a thirsty agent in a deserted hut cannot discover *beg from the household altar*; a hunter past their last reliable source cannot discover *raid an abandoned camp*.

The architectural shift the assessor proposes is bottom-up: for every perceived entity, record, place, route, social fact, and affordance, derive what effects it could enable, what motives those effects would satisfy, what risks or legal consequences they create, and what information they could reveal. The bottom-up pass produces `Opportunity` records that the existing emitters consume alongside the top-down pass. `relevant_ops` becomes a hint that biases search, not the authoritative gate of possibility. Authority moves to a queryable index over `ActionDef.effect_schema` (S134): "give me every action whose effect schema produces `EffectFact::OwnsCommodity(Food)` against an agent-accessible target."

S138 also folds in the assessor's PR-13 (richer travel pruning — detours that satisfy causal landmarks, reduce risk, gain information) and PR-20 (richer interrupt-layer opportunism). The opportunity compiler is the unifying surface: travel pruning becomes opportunity-aware, and the existing interrupt layer (`crates/worldwake-ai/src/interrupts.rs`) enriches its fired set from the same opportunity index. A panicked agent who sees a corpse, an unattended valuable, or a wounded ally generates opportunities through the same pass that emits "I see bread" and "I hear a cart on the road."

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-ai` — new `opportunity_compiler` module owning `Opportunity`, `OpportunityKey` (extended), and the per-tick compilation pass that runs before candidate generation. New `EffectSchemaIndex` module that builds a `BTreeMap<EffectFactKey, SmallVec<ActionDefId, 4>>` from the current `ActionDef` registry. `candidate_generation.rs` consumes opportunities as a parallel input alongside existing emitters. `goal_dispatch_decl.rs` reclassifies `relevant_ops` from authoritative gate to ranking hint. `interrupts.rs` extends to read opportunities. `search/heuristic.rs::prune_travel_away_from_goal_with_expansion_trace` consumes an opportunity-aware detour budget.
- `worldwake-core` — extends `OpportunityKey` (`crates/worldwake-core/src/goal.rs`) with the typed `effect_facts: SmallVec<EffectFactKey, 4>`, `risks: SmallVec<RiskFact, 2>`, `information: SmallVec<ClaimTopic, 2>`. Adds `EffectFactKey` (a discriminant over `EffectFact` from S134; if S134 is not yet landed, a transitional `OpportunityEffectKey` covers the named subset).
- `worldwake-systems` — no change. Action effects already declare what they produce; the index reads the registry.
- `worldwake-cli` — observer Section 3 renders compiled opportunities per agent per tick (top-K by salience). Decision-trace `RootCandidateTrace` annotates which root candidates originated from emitters vs the opportunity compiler.

## Dependencies

- S134 (Canonical Effect Schema) — Phase 11 sibling. **Hard dependency.** The `EffectSchemaIndex` queries `ActionDef.effect_schema`. S134 must land in or before S138's first ticket. If S134 schedule slips, S138 can land a transitional `OpportunityEffectKey` over a hand-maintained subset of action effects (Acquire/Trade/Harvest/Steal/Beg/Loot/Confiscate/Receive); the transitional path is a documented compatibility shim removed when S134 lands.
- S105 (Observation Salience Filtering) — completed. Opportunity compilation runs per perceived entity; budget already governs which entities are perceived.
- S130 (Survey Records and Frontier Disconfirmation) — completed. `SurveyMemory` damps opportunities anchored on confirmed-empty places — the compiler reads survey memory before emitting an opportunity.
- S107 (Proactive Diversification) — completed. `DiversificationProfile` continues to bias `ExploreLocation` opportunities; the compiler consults it.
- S108 (Per-Action Binding Strictness) — completed. `BindingStrictness` continues to govern which entities can satisfy an opportunity's `required_actions`.
- S138 ↔ S137 soft relationship: with S137, repair's `RepairKind::RebindTarget` consumes opportunity-compiler output. Order-independent.
- S141 (Motive Source Ledger) — Phase 11 sibling. Soft dependency: opportunities can name `MotiveSource`s the action's effects would satisfy; if S141 lands first, those references are typed; if S138 lands first, opportunities carry the existing motive-derived matchers.

## Design Goals

1. **Bottom-up parity with top-down.** Every perceived entity, record, route, social fact, and place produces zero or more `Opportunity` records per tick. Opportunities feed candidate generation alongside existing emitters; they do not replace them.
2. **Effect-schema is the authority.** The `EffectSchemaIndex` answers "which actions produce this effect?" Goals query the index when their `relevant_ops` hints are exhausted or when motive urgency exceeds a per-agent threshold.
3. **`relevant_ops` becomes a hint.** The existing declaration stays for fast-path ranking. When the agent's urgency or learned-opportunity memory indicates the hint is insufficient, the compiler queries the effect-schema index and emits additional candidates. The conformance test (`test_declaration_relevant_ops_match_live_goal_model`) is preserved as a self-consistency check, not a possibility gate.
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
- **Goal-kind expansion.** S138 does not add new `GoalKind` variants. Steal/Beg/Loot/etc. *actions* exist or will exist (steal already exists in `bandit_camp_actions`); the compiler routes them through existing goal kinds (`AcquireCommodity` with risk/legality variation).
- **Pre-S134 effect-schema index.** If S134 schedule slips, the transitional `OpportunityEffectKey` covers the subset needed for the canonical regression scenarios. The transition is removed when S134 lands.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-1 (Maximal Emergence Through Local Causality) | The bottom-up pass is the architectural shape that lets agents exploit what scenarios offer rather than only what emitters were authored to consider. |
| FND-3 (Concrete State Over Abstract Scores) | Opportunities reference typed effect facts, typed risks, typed information topics. No abstract "opportunity score" promoted to truth. |
| FND-7 (Locality of Motion, Interaction, and Communication) | All inputs to the compiler are agent-local: perceived entities, believed records, observed routes, recalled habits. No global query. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Opportunities anchored on co-located entities use the same FND-14A read path the planner already uses for direct observation. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | Opportunities carry the underlying belief's provenance via `OpportunityKey.source_belief: BeliefRef`. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | The compiler is bounded by perceived-entity count × effect-index lookup. No script-driven "this scenario should offer this opportunity now." |
| FND-22 (Agent Diversity Through Concrete Variation) | Per-agent `RiskWeightProfile`, `LawAbidingProfile`, and existing utility weights cause two agents seeing the same bread to compile different rankings over the buy/steal/beg/wait opportunities. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | The compiler reads the agent's perception output, the action registry, and the opportunity-anchor entities. No cross-system imperatives. |

## Deliverables

### `worldwake-ai::opportunity_compiler` (new module)

```rust
pub struct Opportunity {
    pub anchor: EntityId,
    pub perceived_at: Tick,
    pub source_belief: BeliefRef,
    pub possible_effects: SmallVec<EffectFactKey, 4>,
    pub possible_information: SmallVec<ClaimTopic, 2>,
    pub required_actions: SmallVec<PlannerOpKind, 4>,
    pub legal_status: BelievedLegalStatus,
    pub social_exposure: SocialExposureBand,
    pub salience: Permille,
}

pub fn compile_opportunities(
    agent: EntityId,
    belief_view: &RuntimeBeliefView,
    perception_state: &PerceptionState,
    action_index: &EffectSchemaIndex,
    survey_memory: &SurveyMemory,
    learned_memory: &LearnedOpportunityMemory,
) -> SmallVec<Opportunity, 16> { /* … */ }
```

### `worldwake-ai::effect_schema_index` (new module)

```rust
pub struct EffectSchemaIndex {
    pub by_effect: BTreeMap<EffectFactKey, SmallVec<ActionDefId, 4>>,
}

impl EffectSchemaIndex {
    pub fn build(registry: &ActionRegistry) -> Self { /* … */ }
    pub fn actions_producing(&self, fact: EffectFactKey) -> &[ActionDefId] { /* … */ }
}
```

### `OpportunityKey` extension (in `worldwake-core/src/goal.rs`)

The existing `OpportunityKey` and `OpportunityAnchor` types gain typed effect/risk/information fields. Existing emitters that produce `OpportunityKey` continue to function; the compiler emits new instances with the additional context.

### `relevant_ops` reclassification (in `goal_dispatch_decl.rs`)

The static slice remains. A new method `relevant_ops_authority()` returns `Authority::HintOnly` (vs current implicit `Authority::Gate`). Candidate generation queries the `EffectSchemaIndex` whenever `urgency_class >= GoalPriorityClass::HighPriority` AND the hint set is exhausted. The conformance test `test_declaration_relevant_ops_match_live_goal_model` continues to assert hint accuracy.

### Travel-pruning extension (in `search/heuristic.rs`)

```rust
pub fn prune_travel_away_from_goal_with_expansion_trace(
    // existing args
    detour_budget_permille: Permille,           // NEW (from CognitiveProfile)
    opportunity_index: &PerceivedOpportunityIndex,  // NEW
) -> TravelPruneOutcome { /* … */ }
```

A detour is allowed if the opportunity-salience along the detour path × `detour_budget_permille` exceeds the cost increase, deterministically.

### Interrupt-layer enrichment (in `interrupts.rs`)

Existing fire-rules are extended to consume `Opportunity.salience` × `Opportunity.possible_effects` against the agent's current motive set. The interrupt fires when the opportunity's expected-motive-satisfaction exceeds the active commitment's expected-motive-satisfaction.

### Observer Section 3

Render compiled opportunities per agent per tick, top-K by salience:
```
Tick 412 — Agent A — Opportunities:
  bread@bakery: salience 720 — effects: OwnsCommodity(Bread); legal: BelievedOwned(baker); exposure: Public
  bread@bakery: salience 540 — effects: OwnsCommodity(Bread); legal: BelievedOwned(baker)→Steal; exposure: Public+CriminalRisk
  altar@hut: salience 380 — effects: OwnsCommodity(Bread)→Beg; legal: SociallyOpenToRequest; exposure: Public+ShameRisk
```

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** No new cross-agent path. The compiler reads only the agent's perception/belief output. `EffectSchemaIndex` is registry data, not state. The opportunity-aware travel pruner reads the agent's own opportunity set.
2. **Positive-feedback analysis.** Potential loop: more perceived entities → more opportunities → wider candidate space → richer plans → agent visits more places → more perceived entities. **Concrete dampener:** S105 perception budget caps perceived entities per tick; `compile_opportunities` is bounded by that budget × per-entity effect-index lookup; `LearnedOpportunityMemory` (S109) damps repeatedly-failed opportunities. Observer-side `OpportunityCompilerLoad` metric (decision-trace counter) flags excess.
3. **Concrete dampeners.**
   - Per-tick perceived-entity ceiling (S105).
   - `LearnedOpportunityMemory` decay damps repeated emission of low-yield opportunities.
   - `detour_budget_permille` caps travel-pruning indulgence.
   - Per-`SaliencePolicy` salience floor below which opportunities are not emitted.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state**: `LearnedOpportunityMemory` (already in `worldwake-core/src/learned_opportunity_memory.rs`, S109), `RiskWeightProfile`/`LawAbidingProfile` extensions of `PreferenceProfile`.
   - **Derived read-model**: `Opportunity` records (per-tick), `EffectSchemaIndex` (registry-time, rebuilt only on `ActionRegistry` change — i.e., test-binary startup; no per-tick rebuild), `PerceivedOpportunityIndex` (per-tick view).

## SystemFn Integration

No new `SystemFn` in the simulation tick. `compile_opportunities` runs synchronously within the agent_tick before `candidate_generation`. The `EffectSchemaIndex` is built once at registry construction and shared across the run.

## Component Registration

- `LearnedOpportunityMemory` — already registered (S109).
- `RiskWeightProfile` (per-agent, universal) — register on `EntityKind::Agent` with default. Profile contract per `docs/spec-drafting-rules.md` Section 5.
- `LawAbidingProfile` (per-agent, universal) — register on `EntityKind::Agent` with default.
- `Opportunity` itself is *not* a stored component; it is derived per-tick.

## Cross-System Interactions

- **AI → AI internal**: opportunity compiler → candidate generator → ranking.
- **AI → Sim**: no new cross-system call. The compiler reads the existing perception output and existing registry.
- **Sim → CLI**: observer reads decision-trace; no new sim API.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

- `RiskWeightProfile { theft_aversion: Permille, exposure_aversion: Permille, threat_aversion: Permille }` — universal per-agent.
- `LawAbidingProfile { criminal_threshold: Permille, social_norm_weight: Permille }` — universal per-agent.
- `CognitiveProfile.detour_budget_permille` — extension; default `pm(150)`. Wounded/exhausted agents get lower budgets via existing `CognitiveProfile` modulation.

Per FND-22, two agents seeing the same opportunity rank it differently because their profile values differ.

## Validation and Falsification

- **Golden coverage**: new `golden_opportunity_compiler.rs` with five scenarios:
  1. Starving agent + unguarded bread + no merchant → expects `Steal` opportunity emitted, ranked above `Wait`, ranked below `Buy` only when merchant is present.
  2. Thirsty agent + dry well + nearby alternative source via opportunity → expects detour-budget pruning to allow the alternative.
  3. Witness opportunity along travel route → expects detour-budget pruning to allow the witness inquiry (S139 `AskWitness`) detour, bounded.
  4. Effect-schema index miss → unknown effect produces no opportunity (negative test).
  5. `LearnedOpportunityMemory` damping after repeated failure → expects salience reduction over successive ticks.
- **`relevant_ops` regression**: pre-S138 emitter behavior on `survival-baseline.ron` produces identical event log. The bottom-up pass is additive at default profiles.
- **Performance**: per-tick opportunity-compilation duration ≤ 5% of agent_tick total under 1440-tick `survival-contested.ron` (4 agents). Soak measures.

## Risks

- **Effect-schema index timing.** Pre-S134, the index is hand-maintained over a subset. Mitigation: ticket-001 lands the transitional `OpportunityEffectKey` covering Acquire/Trade/Harvest/Steal/Beg/Loot/Confiscate/Receive — the canonical-scenario-A and stored-gold-canonical-scenario-C subsets. Removed when S134 lands.
- **Opportunity explosion in dense scenes.** A market with 80 vendors could compile 80+ opportunities. Mitigation: salience floor; per-tick opportunity cap (16 per agent); reuse of S105's perception budget upstream so the compiler input is already truncated.
- **Travel-detour budget mis-tuning.** A too-permissive `detour_budget_permille` could turn travel into wandering. Mitigation: default `pm(150)` is conservative; goldens (scenario 2 above) lock the boundary; observer surfaces detour decisions with attribution.
