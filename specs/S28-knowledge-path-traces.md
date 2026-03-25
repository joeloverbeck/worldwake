**Status**: PENDING

# S28: Knowledge-Path Traces

## Summary

Extend the decision trace system to record the knowledge path behind each goal candidate: which specific beliefs motivated the candidate, how those beliefs were acquired (direct observation, report, rumor, inference, record consultation), and when. This completes FOUNDATIONS P27's requirement that both the causal path and the knowledge path be separately inspectable.

## Phase

Phase 3+: AI Architecture Overhaul (Step 13.5, Wave 2)

## Crates

- `worldwake-core` — `HomeostaticNeedId` enum (domain type, next to `HomeostaticNeeds` in `needs.rs`)
- `worldwake-sim` — `institutional_belief_claims()` default method on `GoalBeliefView` trait (in `belief_view.rs`)
- `worldwake-ai` — all diagnostic types (`BeliefProvenance`, `KnowledgePath`, etc.), candidate generation instrumentation, and trace output extension

## Dependencies

- S21 (Promote Causal Runtime State) — authoritative belief sources make traces more meaningful; with richer causal runtime state, knowledge paths trace through more informative provenance
- S22 (Generalized Intention Frames) — `DecisionOutcome::ActiveAction` now includes `frame_transition: Option<FrameTransitionTrace>`; knowledge paths coexist in the candidate evidence section, not the frame transition section
- S23 (Refined Blocked Intents) — `BlockedIntentMemory` is now `BTreeMap<BlockerKey, BlockedIntent>` and available in `GenerationContext`; future extension point for tracing which blockers suppressed candidates
- S24 (Typed Invalidation Domains) — knowledge path population does NOT add a new `DirtySet` domain; knowledge paths are purely diagnostic and do not trigger replan
- S25 (Feasibility Sketching) — `FeasibilityHint` is now on `RankedGoalSummary`; knowledge paths are keyed by `GoalKey` and independent of feasibility reordering

## FOUNDATIONS Alignment

- **P27** (Debuggability Is a Product Feature): "For any nontrivial event chain, you must be able to inspect both the causal path and the knowledge path separately." Current `DecisionTraceSink` traces record the causal path through the decision pipeline (candidates generated, ranking, plan search, selection, execution outcome). The knowledge path -- how the agent's beliefs justified each candidate -- is missing. This spec adds it.

## Motivation

The current `DecisionTraceSink` answers:
- "What candidates were generated?" -- via `CandidateTrace.generated`
- "Which entities/places contributed evidence?" -- via `CandidateEvidenceTrace`
- "How were candidates ranked?" -- via `RankedGoalSummary` with `RankedGoalProvenance`
- "What feasibility hint was assigned?" -- via `RankedGoalSummary.feasibility` (S25)
- "Which plan was selected?" -- via `SelectionTrace`
- "What was the execution outcome?" -- via `ExecutionTrace`
- "Was there a frame transition?" -- via `FrameTransitionTrace` (S22)

But it does NOT answer:
- "WHY did the agent generate this candidate?" -- Which specific belief motivated it?
- "WHERE did that belief come from?" -- Direct observation? Report from agent B? Record consultation?
- "HOW fresh is the motivating belief?" -- Was it observed this tick or 50 ticks ago?

These questions are essential for debugging emergence chains. Example: "Why did agent A travel to the market?" The causal path says "goal AcquireCommodity(Apple) was ranked highest, plan was Travel->Buy." The knowledge path (currently missing) would say "because agent B told A about apples at the market at tick 15, and B learned this from direct observation at tick 10."

### What already exists

The belief system already stores provenance. Each `BelievedEntityState` carries `source: PerceptionSource` and `observed_tick: Tick`. Each `BelievedInstitutionalClaim` carries `source: InstitutionalKnowledgeSource`, `learned_tick: Tick`, and `learned_at: Option<EntityId>`. The `CandidateEvidenceContributor` records which entities/places contributed to a candidate, but discards the belief provenance during candidate generation. The data is available; it is simply not threaded into the trace.

## Design

### BeliefProvenance Record

A new struct captures the knowledge path for one belief that contributed to a candidate:

```rust
/// One belief that motivated a goal candidate, with its acquisition provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeliefProvenance {
    /// The entity this belief is about.
    pub subject: EntityId,
    /// What aspect of the entity motivated the candidate.
    pub aspect: BeliefAspect,
    /// How the agent acquired this belief.
    pub source: PerceptionSource,
    /// When the belief was last updated.
    pub observed_tick: Tick,
}
```

`BeliefAspect` describes what facet of the believed entity state was relevant:

```rust
/// Which aspect of a believed entity contributed to candidate generation.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BeliefAspect {
    /// Entity believed to be at a place (used for co-location evidence).
    LocationAt { place: EntityId },
    /// Entity believed to have commodity inventory (seller, resource source).
    HasCommodity { commodity: CommodityKind },
    /// Entity believed to have a workstation tag.
    HasWorkstation { tag: WorkstationTag },
    /// Entity believed to be a resource source for a commodity.
    IsResourceSource { commodity: CommodityKind },
    /// Entity believed to be alive.
    Alive,
    /// Entity believed to be dead (corpse evidence).
    Dead,
    /// Entity believed to have wounds (care target).
    Wounded,
    /// Entity believed to be hostile.
    Hostile,
}
```

For institutional beliefs, a parallel record uses the existing `InstitutionalKnowledgeSource`:

```rust
/// One institutional belief that motivated a goal candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstitutionalBeliefProvenance {
    /// The institutional claim that motivated the candidate.
    pub claim: InstitutionalClaim,
    /// How the agent learned about this claim.
    pub source: InstitutionalKnowledgeSource,
    /// When the agent learned this.
    pub learned_tick: Tick,
    /// Where the agent learned this (place, if known).
    pub learned_at: Option<EntityId>,
}
```

For self-knowledge (own needs, own wounds, own inventory):

```rust
/// Self-knowledge that motivated a goal candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelfKnowledgeProvenance {
    /// Homeostatic need level.
    NeedLevel { need: HomeostaticNeedId, permille: Permille },
    /// Agent has wounds.
    OwnWounds { count: u16 },
    /// Agent possesses commodity.
    OwnCommodity { commodity: CommodityKind, quantity: Quantity },
    /// Agent has merchandise profile (merchant identity).
    MerchantIdentity,
}
```

Where `HomeostaticNeedId` is a simple enum matching the need fields, defined in `worldwake-core::needs` (next to `HomeostaticNeeds`):

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum HomeostaticNeedId {
    Hunger,
    Thirst,
    Fatigue,
    Bladder,
    Dirtiness,
}
```

### KnowledgePath Composite

The complete knowledge path for one candidate combines all three provenance kinds:

```rust
/// Complete knowledge path for one goal candidate.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KnowledgePath {
    /// Self-knowledge (needs, wounds, inventory) that motivated the candidate.
    pub self_knowledge: Vec<SelfKnowledgeProvenance>,
    /// Entity beliefs (with perception source) that motivated the candidate.
    pub entity_beliefs: Vec<BeliefProvenance>,
    /// Institutional beliefs that motivated the candidate.
    pub institutional_beliefs: Vec<InstitutionalBeliefProvenance>,
}
```

### Integration with CandidateEvidenceTrace

Extend the existing `CandidateEvidenceTrace` with the knowledge path:

```rust
pub struct CandidateEvidenceTrace {
    pub goal: GoalKey,
    pub contributors: Vec<CandidateEvidenceContributor>,
    pub exclusions: Vec<CandidateEvidenceExclusion>,
    /// Knowledge path: which beliefs motivated this candidate and where they came from.
    /// Empty when tracing is disabled.
    pub knowledge_path: KnowledgePath,
}
```

### Instrumentation Points

Candidate generation dispatches through top-level dispatcher functions that call specific emitters. The instrumentation adds `KnowledgePath` construction alongside the existing `EvidenceTrace` in each emitter:

| Dispatcher | Emitters | Self-Knowledge | Entity Beliefs | Institutional |
|---|---|---|---|---|
| `emit_need_candidates` | `emit_self_consume_candidates`, `emit_need_driven_candidates`, `emit_sleep_goal`, `emit_relieve_goal`, `emit_wash_goal` | `NeedLevel(hunger/thirst/fatigue/bladder/dirtiness)` | Commodity holders via `known_entity_beliefs()` lookup for sellers, resource sources, loose lots | -- |
| `emit_production_candidates` | `emit_produce_goals` | -- | Resource sources, workstations via belief lookup | -- |
| `emit_enterprise_candidates` | `emit_restock_goals`, `emit_move_cargo_goals` | `MerchantIdentity` for restock | Sellers, demand memory entities, cargo lots via belief lookup | -- |
| `emit_combat_candidates` | `emit_engage_hostile_goals`, `emit_reduce_danger_goal`, `emit_care_goals`, `emit_loot_goals`, `emit_bury_goals` | `OwnWounds` for danger reduction | Hostile targets, wounded entities, corpses via belief lookup | -- |
| `emit_social_candidates` | (direct) | -- | Known entity beliefs for Tell subjects via `known_entity_beliefs()` | -- |
| `emit_political_candidates` | `emit_claim_office_candidate`, `emit_support_candidate_goals` | -- | -- | Office holder beliefs, support declarations via `institutional_belief_claims()` |

#### Social and Political Candidate Trace Upgrade

Currently `emit_social_candidates` and `emit_political_candidates` use `emit_candidate()` which does NOT produce a `CandidateEvidenceTrace`. To attach knowledge paths, these must be upgraded to use `emit_candidate_with_trace()`:

- `emit_social_candidates`: Add `EvidenceTrace` construction with `CandidateEvidenceKind` entries for the listener and subject entities. Thread `CandidateGenerationDiagnostics` parameter.
- `emit_political_candidates` (and its sub-emitters `emit_claim_office_candidate`, `emit_support_candidate_goals`): Add `EvidenceTrace` construction with new `CandidateEvidenceKind` variants for office-holder and support-candidate entities. Thread `CandidateGenerationDiagnostics` parameter.

New `CandidateEvidenceKind` variants needed:

```rust
pub enum CandidateEvidenceKind {
    // ... existing variants ...
    /// Listener in a Tell/ShareBelief interaction.
    Listener,
    /// Subject of a Tell/ShareBelief interaction.
    TellSubject,
    /// Office holder or candidate in political candidate generation.
    OfficeParticipant,
}
```

### GoalBeliefView Extension

Candidate generation currently calls methods like `corpse_entities_at()`, `agents_selling_at()`, and `visible_hostiles_for()` which return entity IDs without provenance. To extract provenance, the instrumentation uses `known_entity_beliefs()` (already on the trait, returns `Vec<(EntityId, BelievedEntityState)>`) to look up the `PerceptionSource` and `observed_tick` for the entities that contributed to the candidate.

No new entity-belief methods are added to `GoalBeliefView`. The provenance extraction happens inside `candidate_generation.rs` by cross-referencing the contributing entity IDs against `known_entity_beliefs()` results.

For institutional beliefs, `believed_office_holder()` and related methods return `InstitutionalBeliefRead<T>` which does not expose provenance. A new trait method is needed on `GoalBeliefView` (defined in `worldwake-sim::belief_view`):

```rust
/// Return the raw institutional belief claims for a key, with provenance.
/// Default returns empty (backward compatible).
fn institutional_belief_claims(
    &self,
    agent: EntityId,
    key: InstitutionalBeliefKey,
) -> Vec<BelievedInstitutionalClaim> {
    let _ = (agent, key);
    Vec::new()
}
```

This is a narrow, optional extension. Implementations populate it from `AgentBeliefStore.institutional_beliefs`:
- `PerAgentBeliefView`: returns claims from the agent's belief store for the given key
- `OmniscientBeliefView`: returns empty (omniscient view has no institutional belief store)

### Integration with S22 (Intention Frames)

`DecisionOutcome::ActiveAction` now includes `frame_transition: Option<FrameTransitionTrace>`. Knowledge paths are part of candidate evidence, which is recorded during the candidate generation phase — before frame evaluation and before plan search. The `dump_agent()` output renders knowledge paths in the candidates section, separate from and before any frame transition output.

### Integration with S23 (Refined Blocked Intents)

`BlockedIntentMemory` (now `BTreeMap<BlockerKey, BlockedIntent>`) is available in `GenerationContext.blocked`. The existing candidate generation already uses blocker memory to suppress candidates (via `is_blocked()`). S28 does not trace which specific blockers suppressed which candidates — that information is already available in `CandidateTrace.suppressed`. Future extension: a `BlockerSuppression` diagnostic in the knowledge path could show which blocker entries caused suppression, but this is out of scope for S28.

### Integration with S24 (Typed Invalidation Domains)

Knowledge path population does NOT add a new `InvalidationDomain` variant to `DirtySet`. Knowledge paths are purely derived diagnostic data in `DecisionTraceSink`. They do not influence agent behavior, do not trigger replan, and do not persist between ticks. The `DirtySet` remains unchanged.

### Integration with S25 (Feasibility Sketching)

`RankedGoalSummary` now carries `feasibility: FeasibilityHint` (S25). Feasibility reordering happens after candidate generation but before plan search. Knowledge paths attach to `CandidateEvidenceTrace` which is keyed by `GoalKey`, independent of ranking position. Feasibility reordering does not disrupt knowledge paths.

### Trace Output Extension

`dump_agent()` includes knowledge path per candidate when non-empty:

```
[tick 5] PLAN: selected=AcquireCommodity(Apple, SelfConsume), ...
  Candidate: AcquireCommodity(Apple, SelfConsume) [feasibility=Likely]
    Evidence: Seller(OrchardFarmer @ OrchardFarm), ResourceSource(AppleTree @ OrchardFarm)
    Knowledge path:
      self: NeedLevel(Hunger, 900 permille)
      belief: OrchardFarmer at OrchardFarm — Report(from=Traveler, chain=1) @ tick 12
      belief: AppleTree has Apple — DirectObservation @ tick 8
  Candidate: ClaimOffice(TownSteward) [feasibility=Uncertain]
    Evidence: OfficeParticipant(CurrentSteward @ TownHall)
    Knowledge path:
      institutional: OfficeHolder(TownSteward, holder=CurrentSteward) — RecordConsultation(record=TownLedger, entry=3) @ tick 20, learned_at=TownHall
```

The `summary()` one-liner remains unchanged (already concise). The knowledge path is visible only in the detailed `dump_agent()` output.

### Zero-Cost When Disabled

Knowledge path construction only runs when tracing is enabled (checked via the existing `Option<DecisionTraceSink>` pattern in `AgentTickDriver`). When tracing is disabled:
- `EvidenceTrace` still collects `CandidateEvidenceContributor` as today (needed for non-trace purposes)
- `KnowledgePath` fields are not populated
- No belief lookups for provenance are performed

The `tracing_enabled: bool` flag is threaded through `GenerationContext` (sourced from `AgentTickDriver::trace_sink.is_some()`). Each emitter checks this flag before performing provenance lookups.

## Tickets

### S28-001: Define knowledge path types (core + sim + ai)

**worldwake-core** changes:
- Add `HomeostaticNeedId` enum to `needs.rs` (next to `HomeostaticNeeds`) with derives `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`. Re-export from crate root.

**worldwake-sim** changes:
- Add `institutional_belief_claims()` default method to `GoalBeliefView` trait in `belief_view.rs`.
- Implement on `PerAgentBeliefView` (returns claims from agent's belief store for the given key).
- Implement on `OmniscientBeliefView` (returns empty vec — omniscient view has no institutional belief store).

**worldwake-ai** changes:
- Add new `knowledge_path.rs` module with: `BeliefProvenance`, `BeliefAspect`, `InstitutionalBeliefProvenance` (with `learned_at`), `SelfKnowledgeProvenance`, `KnowledgePath`.
- Add new `CandidateEvidenceKind` variants: `Listener`, `TellSubject`, `OfficeParticipant`.
- Add `knowledge_path: KnowledgePath` field to `CandidateEvidenceTrace` (defaulting to empty via `Default`).

Verify: `cargo build --workspace` -- no breakage (new field has `Default`).

### S28-002: Instrument need and self-consume candidates

- Add `tracing_enabled: bool` to `GenerationContext` (sourced from `AgentTickDriver::trace_sink.is_some()`).
- Instrument `emit_self_consume_candidates`, `emit_need_driven_candidates`, `emit_sleep_goal`, `emit_relieve_goal`, `emit_wash_goal` to populate `KnowledgePath` when tracing is enabled:
  - Self-knowledge: `NeedLevel` with current permille for each relevant need
  - Entity beliefs: look up `PerceptionSource` and `observed_tick` for commodity holders (sellers, resource sources, loose lots) via `known_entity_beliefs()` cross-reference

Verify: `cargo test -p worldwake-ai` -- all existing tests pass, no behavioral change.

### S28-003: Instrument production and enterprise candidates

Instrument `emit_produce_goals`, `emit_restock_goals`, `emit_move_cargo_goals`:
- Entity beliefs: resource sources, workstations, sellers, demand memory entities — look up provenance via `known_entity_beliefs()` cross-reference
- Self-knowledge: `MerchantIdentity` for restock goals

Verify: `cargo test -p worldwake-ai` -- all existing tests pass.

### S28-004: Instrument combat, social, and political candidates

**Combat emitters** (already use `emit_candidate_with_trace()`):
- Instrument `emit_engage_hostile_goals`, `emit_reduce_danger_goal`, `emit_care_goals`, `emit_loot_goals`, `emit_bury_goals`
- Entity beliefs: hostile targets, wounded entities, corpses — look up provenance via `known_entity_beliefs()` cross-reference
- Self-knowledge: `OwnWounds` for danger reduction

**Social emitter** (upgrade required):
- Upgrade `emit_social_candidates` from `emit_candidate()` to `emit_candidate_with_trace()`
- Add `EvidenceTrace` with `Listener` + `TellSubject` evidence kinds
- Thread `CandidateGenerationDiagnostics` parameter
- Entity beliefs: known beliefs for Tell subjects — extract provenance from the `known_entity_beliefs()` tuples already iterated

**Political emitters** (upgrade required):
- Upgrade `emit_political_candidates`, `emit_claim_office_candidate`, `emit_support_candidate_goals` from `emit_candidate()` to `emit_candidate_with_trace()`
- Add `EvidenceTrace` with `OfficeParticipant` evidence kind
- Thread `CandidateGenerationDiagnostics` parameter
- Institutional beliefs: use `institutional_belief_claims()` to extract `InstitutionalKnowledgeSource`, `learned_tick`, and `learned_at` for office holder and support declaration claims

Verify: `cargo test -p worldwake-ai` -- all existing tests pass.

### S28-005: Extend trace dump with knowledge path output

Update `format_outcome()` and `dump_agent()` in `decision_trace.rs` to render knowledge paths:
- Show knowledge path per candidate in detailed output (after evidence, before next candidate)
- Format `PerceptionSource` variants as human-readable strings: `DirectObservation`, `Report(from=<name>, chain=N)`, `Rumor(chain=N)`, `Inference`
- Format `InstitutionalKnowledgeSource` variants similarly, including `learned_at` when present: `RecordConsultation(record=<name>, entry=N) @ tick T, learned_at=<place>`
- Show feasibility hint in candidate header line (coexists with S25)
- Keep `summary()` one-liner unchanged

Verify: enable tracing in an existing golden test, confirm `dump_agent()` output includes knowledge paths for all candidate types.

### S28-006: Workspace verification

- `cargo test --workspace` -- all pass
- `cargo clippy --workspace` -- no new warnings
- No behavioral changes -- traces are diagnostic only, zero-cost when disabled

## FND-01 Section H Analysis

Pure diagnostic extension — no new authoritative state, no system changes, no feedback loops.

### Information-path analysis

Not applicable. This spec adds diagnostic instrumentation to the existing decision pipeline. It does not introduce new information paths between agents.

### Positive-feedback analysis

No feedback loops introduced. The knowledge path is a read-only diagnostic record that does not influence agent decisions.

### Concrete dampeners

No feedback loops to dampen.

### Stored state vs. derived read-model list

- **Stored**: None. Knowledge path data lives only in `DecisionTraceSink`, which is opt-in, transient, and not part of authoritative world state or replay state. Knowledge paths do not add a new `InvalidationDomain` to `DirtySet` (S24) and do not participate in save/load or replay.
- **Derived**: `BeliefProvenance` records are derived from `BelievedEntityState.source` and `BelievedEntityState.observed_tick` during candidate generation. `SelfKnowledgeProvenance` is derived from `HomeostaticNeeds` component values. `InstitutionalBeliefProvenance` is derived from `BelievedInstitutionalClaim` records (including `learned_at`).

## Verification

1. `cargo test --workspace` -- all pass
2. `cargo clippy --workspace` -- no new warnings
3. `dump_agent()` output shows knowledge paths for all candidate families listed in the instrumentation table
4. Knowledge paths correctly attribute beliefs to `DirectObservation`, `Report`, `Rumor`, `Inference`, `WitnessedEvent`, `RecordConsultation`, or `SelfDeclaration` sources
5. Institutional belief provenance includes `learned_at` location when available
6. Self-knowledge entries correctly report need levels, wound counts, and commodity quantities
7. Knowledge paths are empty when tracing is disabled (zero-cost guarantee)
8. No behavioral changes -- all golden tests produce identical outcomes with and without tracing enabled
9. Social and political candidates now produce `CandidateEvidenceTrace` records (upgraded from trace-less `emit_candidate()`)
