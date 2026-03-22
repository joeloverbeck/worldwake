**Status**: PENDING

# S28: Knowledge-Path Traces

## Summary

Extend the decision trace system to record the knowledge path behind each goal candidate: which specific beliefs motivated the candidate, how those beliefs were acquired (direct observation, report, rumor, inference, record consultation), and when. This completes FOUNDATIONS P27's requirement that both the causal path and the knowledge path be separately inspectable.

## Phase

Phase 3+: AI Architecture Overhaul (Step 13.5, Wave 2)

## Crate

`worldwake-ai`

## Dependencies

- S21 (authoritative belief sources make traces more meaningful -- with richer causal runtime state, knowledge paths trace through more informative provenance)

## FOUNDATIONS Alignment

- **P27** (Debuggability Is a Product Feature): "For any nontrivial event chain, you must be able to inspect both the causal path and the knowledge path separately." Current `DecisionTraceSink` traces record the causal path through the decision pipeline (candidates generated, ranking, plan search, selection, execution outcome). The knowledge path -- how the agent's beliefs justified each candidate -- is missing. This spec adds it.

## Motivation

The current `DecisionTraceSink` answers:
- "What candidates were generated?" -- via `CandidateTrace.generated`
- "Which entities/places contributed evidence?" -- via `CandidateEvidenceTrace`
- "How were candidates ranked?" -- via `RankedGoalSummary` with `RankedGoalProvenance`
- "Which plan was selected?" -- via `SelectionTrace`
- "What was the execution outcome?" -- via `ExecutionTrace`

But it does NOT answer:
- "WHY did the agent generate this candidate?" -- Which specific belief motivated it?
- "WHERE did that belief come from?" -- Direct observation? Report from agent B? Record consultation?
- "HOW fresh is the motivating belief?" -- Was it observed this tick or 50 ticks ago?

These questions are essential for debugging emergence chains. Example: "Why did agent A travel to the market?" The causal path says "goal AcquireCommodity(Apple) was ranked highest, plan was Travel->Buy." The knowledge path (currently missing) would say "because agent B told A about apples at the market at tick 15, and B learned this from direct observation at tick 10."

### What already exists

The belief system already stores provenance. Each `BelievedEntityState` carries `source: PerceptionSource` and `observed_tick: Tick`. Each `BelievedInstitutionalClaim` carries `source: InstitutionalKnowledgeSource` and `learned_tick: Tick`. The `CandidateEvidenceContributor` records which entities/places contributed to a candidate, but discards the belief provenance during candidate generation. The data is available; it is simply not threaded into the trace.

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

Where `HomeostaticNeedId` is a simple enum matching the need fields:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
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

Each `emit_*` function in `candidate_generation.rs` already reads beliefs that motivate the candidate. The instrumentation adds `KnowledgePath` construction alongside the existing `EvidenceTrace`:

| Emitter | Self-Knowledge | Entity Beliefs | Institutional Beliefs |
|---------|---------------|----------------|----------------------|
| `emit_need_driven_candidates` | `NeedLevel(hunger/thirst)` | Commodity holders via `known_entity_beliefs` lookup for sellers, resource sources, loose lots | -- |
| `emit_sleep_goal` | `NeedLevel(Fatigue)` | -- | -- |
| `emit_relieve_goal` | `NeedLevel(Bladder)` | -- | -- |
| `emit_wash_goal` | `NeedLevel(Dirtiness)` | -- | -- |
| `emit_produce_goals` | -- | Resource sources, workstations via belief lookup | -- |
| `emit_restock_goals` | `MerchantIdentity` | Sellers, demand memory entities via belief lookup | -- |
| `emit_move_cargo_goals` | -- | Cargo lot beliefs | -- |
| `emit_engage_hostile_goals` | -- | Hostile targets via `visible_hostiles_for` belief lookup | -- |
| `emit_reduce_danger_goal` | `OwnWounds` | Attackers via belief lookup | -- |
| `emit_care_goals` | -- | Wounded entities via belief lookup | -- |
| `emit_loot_goals` | -- | Corpse entities via belief lookup | -- |
| `emit_bury_goals` | -- | Corpse entities via belief lookup | -- |
| `emit_social_candidates` | -- | Known entity beliefs for Tell subjects | -- |
| `emit_political_candidates` | -- | -- | Office holder beliefs, support declarations |

### GoalBeliefView Extension

Candidate generation currently calls methods like `corpse_entities_at()`, `agents_selling_at()`, and `visible_hostiles_for()` which return entity IDs without provenance. To extract provenance, the instrumentation uses `known_entity_beliefs()` (already on the trait, returns `Vec<(EntityId, BelievedEntityState)>`) to look up the `PerceptionSource` and `observed_tick` for the entities that contributed to the candidate.

No new methods are added to `GoalBeliefView`. The provenance extraction happens inside `candidate_generation.rs` by cross-referencing the contributing entity IDs against `known_entity_beliefs()` results.

For institutional beliefs, `believed_office_holder()` and related methods return `InstitutionalBeliefRead` which does not expose provenance. A new trait method is needed:

```rust
/// Return the raw institutional belief claims for an office, with provenance.
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

This is a narrow, optional extension that belief view implementations can populate from the `AgentBeliefStore.institutional_beliefs` map.

### Trace Output Extension

`dump_agent()` includes knowledge path per candidate when non-empty:

```
[tick 5] PLAN: selected=AcquireCommodity(Apple, SelfConsume), ...
  Candidate: AcquireCommodity(Apple, SelfConsume)
    Evidence: Seller(OrchardFarmer @ OrchardFarm), ResourceSource(AppleTree @ OrchardFarm)
    Knowledge path:
      self: NeedLevel(Hunger, 900 permille)
      belief: OrchardFarmer at OrchardFarm — Report(from=Traveler, chain=1) @ tick 12
      belief: AppleTree has Apple — DirectObservation @ tick 8
```

The `summary()` one-liner remains unchanged (already concise). The knowledge path is visible only in the detailed `dump_agent()` output.

### Zero-Cost When Disabled

Knowledge path construction only runs when tracing is enabled (checked via the existing `Option<DecisionTraceSink>` pattern in `AgentTickDriver`). When tracing is disabled:
- `EvidenceTrace` still collects `CandidateEvidenceContributor` as today (needed for non-trace purposes)
- `KnowledgePath` fields are not populated
- No belief lookups for provenance are performed

## Tickets

### S28-001: Define knowledge path types

Add to `worldwake-ai` (in `decision_trace.rs` or a new `knowledge_path.rs` module):
- `BeliefProvenance`, `BeliefAspect`
- `InstitutionalBeliefProvenance`
- `SelfKnowledgeProvenance`, `HomeostaticNeedId`
- `KnowledgePath`

Add `knowledge_path: KnowledgePath` field to `CandidateEvidenceTrace` (defaulting to empty).

Add `institutional_belief_claims()` default method to `GoalBeliefView`.

Verify: `cargo build --workspace` -- no breakage (new field has `Default`).

### S28-002: Instrument need and self-consume candidates

Instrument `emit_need_driven_candidates`, `emit_sleep_goal`, `emit_relieve_goal`, `emit_wash_goal` to populate `KnowledgePath` when tracing is enabled:
- Self-knowledge: `NeedLevel` with current permille
- Entity beliefs: look up `PerceptionSource` and `observed_tick` for commodity holders (sellers, resource sources, loose lots) via `known_entity_beliefs()`

Thread a `tracing_enabled: bool` flag through `GenerationContext` (sourced from `AgentTickDriver`).

Verify: `cargo test -p worldwake-ai` -- all existing tests pass, no behavioral change.

### S28-003: Instrument production and enterprise candidates

Instrument `emit_produce_goals`, `emit_restock_goals`, `emit_move_cargo_goals`:
- Entity beliefs: resource sources, workstations, sellers, demand memory entities
- Self-knowledge: `MerchantIdentity` for restock goals

Verify: `cargo test -p worldwake-ai` -- all existing tests pass.

### S28-004: Instrument combat, social, and political candidates

Instrument `emit_engage_hostile_goals`, `emit_reduce_danger_goal`, `emit_care_goals`, `emit_loot_goals`, `emit_bury_goals`, `emit_social_candidates`, `emit_political_candidates`:
- Entity beliefs: hostile targets, wounded entities, corpses, Tell subjects
- Institutional beliefs: office holder claims, support declarations (using new `institutional_belief_claims()`)
- Self-knowledge: `OwnWounds` for danger reduction

Verify: `cargo test -p worldwake-ai` -- all existing tests pass.

### S28-005: Extend trace dump with knowledge path output

Update `format_outcome()` and `dump_agent()` in `decision_trace.rs` to render knowledge paths:
- Show knowledge path per candidate in detailed output
- Format `PerceptionSource` variants as human-readable strings
- Format `InstitutionalKnowledgeSource` variants similarly
- Keep `summary()` one-liner unchanged

Verify: enable tracing in an existing golden test, confirm `dump_agent()` output includes knowledge paths for all candidate types.

### S28-006: Workspace verification

- `cargo test --workspace` -- all pass
- `cargo clippy --workspace` -- no new warnings
- No behavioral changes -- traces are diagnostic only, zero-cost when disabled

## FND-01 Section H Analysis

N/A -- pure diagnostic extension, no new authoritative state, no system changes, no feedback loops.

### Information-path analysis

Not applicable. This spec adds diagnostic instrumentation to the existing decision pipeline. It does not introduce new information paths between agents.

### Positive-feedback analysis

No feedback loops introduced. The knowledge path is a read-only diagnostic record that does not influence agent decisions.

### Concrete dampeners

No feedback loops to dampen.

### Stored state vs. derived read-model list

- **Stored**: None. Knowledge path data lives only in `DecisionTraceSink`, which is opt-in, transient, and not part of authoritative world state or replay state.
- **Derived**: `BeliefProvenance` records are derived from `BelievedEntityState.source` and `BelievedEntityState.observed_tick` during candidate generation. `SelfKnowledgeProvenance` is derived from `HomeostaticNeeds` component values. `InstitutionalBeliefProvenance` is derived from `BelievedInstitutionalClaim` records.

## Verification

1. `cargo test --workspace` -- all pass
2. `cargo clippy --workspace` -- no new warnings
3. `dump_agent()` output shows knowledge paths for all candidate families listed in the instrumentation table
4. Knowledge paths correctly attribute beliefs to `DirectObservation`, `Report`, `Rumor`, `Inference`, `WitnessedEvent`, `RecordConsultation`, or `SelfDeclaration` sources
5. Self-knowledge entries correctly report need levels, wound counts, and commodity quantities
6. Knowledge paths are empty when tracing is disabled (zero-cost guarantee)
7. No behavioral changes -- all golden tests produce identical outcomes with and without tracing enabled
