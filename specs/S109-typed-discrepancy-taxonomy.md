# S109: Typed Discrepancy Taxonomy and BlockedIntentMemory Split

## Summary

Replace the overloaded `BlockingFact::Unknown` / `BlockingFact::AssumptionFailed` variants with a proper discrepancy taxonomy that distinguishes stale beliefs, contradicted beliefs, contention losses, missing observations, route-unknowns, no-legal-binding failures (including S108's `ExactIdentityRequired`), search-budget exhaustion, structural impossibility, and partial-execution drift. Each discrepancy class carries its own retry policy, invalidation condition, learning update, and debug explanation. Concurrently, split the monolithic `BlockedIntentMemory` into four purpose-specific memories (`DiscrepancyMemory`, `BlockerMemory`, `RepairMemory`, `LearnedOpportunityMemory`) so that epistemic retreat (stale belief → reverify) is never conflated with contention backoff (`SellerOutOfStock`) or structural impossibility (`NoKnownPath`).

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-core` — new `Discrepancy` enum, `DiscrepancyMemory`, `BlockerMemory`, `RepairMemory`, `LearnedOpportunityMemory` components; migrate `BlockingFact::Unknown` / `AssumptionFailed` call sites to the new typing
- `worldwake-ai` — `failure_handling.rs` emits typed discrepancies; TTL lookup and clearing logic split by memory class; candidate generation reads from the appropriate memory
- `worldwake-sim` — belief-view accessors for the new memories
- `worldwake-cli` — scenario `AgentDef` carries the new components (universal, with `Default`)

## Dependencies

- S108 (Per-Action Binding Strictness) — provides the `MatchOutcome::ExactIdentityRequired` signal that feeds `Discrepancy::NoLegalBinding`. Soft dependency; S109 can land in parallel if S108 lands first, otherwise they interlock.

## Design Goals

- Eliminate `BlockingFact::Unknown` as the catch-all bucket. Every currently-Unknown pathway must map to a specific discrepancy class.
- Distinguish epistemic discrepancies (agent belief was wrong) from world-state discrepancies (opportunity vanished) from structural discrepancies (impossible affordance).
- Route each discrepancy to its correct memory: transient contention losses → `BlockerMemory` with short TTL; stale beliefs → `DiscrepancyMemory` with a reverify hook; successful alternate repairs → `RepairMemory`; learned opportunities discovered en route → `LearnedOpportunityMemory`.
- Preserve FOUNDATIONS P29A: the authoritative event log records the typed discrepancy at the moment it is observed, not a generic "Unknown." S110 will add the corresponding `EventTag::BlockerRecorded` variant.

## Non-Goals

- Full PolicyPlan branching on discrepancy class — deferred to a Phase 9 spec.
- Runtime decay / learning-rate tuning beyond setting per-class TTL defaults on each memory. Decay-shape tuning is a future adjunct.
- Changing the existing authored `BlockingFact` variants that are already correctly typed (`NoKnownPath`, `NoKnownSeller`, `SellerOutOfStock`, etc.). Those stay; only `Unknown` and `AssumptionFailed` are decomposed.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | Each discrepancy class is a distinct epistemic state: `BeliefStale`, `BeliefContradicted`, `MissingObservation`. The architecture no longer forces all ignorance into the same bucket. |
| FND-20 (Resource-Bounded Practical Reasoning) | Correct discrepancy classification drives correct retry policy. A `BeliefStale` failure triggers reverify; a `ContentionLost` failure triggers backoff; a `StructurallyImpossible` failure triggers permanent goal suppression. |
| FND-22A (Learning, Habits, Preference Shifts) | `LearnedOpportunityMemory` is the authoritative surface for opportunities discovered in transit, separate from blocker backoff. Decay and overwrite are explicit. |
| FND-29 (Debuggability Is a Product Feature) | The discrepancy class answers "why did this step fail?" without needing to dig into ad-hoc log messages. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | The typed discrepancy is recorded in the append-only event log (S110) with its provenance (which step, which belief, which observation). |

## Deliverables

### D1: `Discrepancy` enum

New type in `crates/worldwake-core/src/discrepancy.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Discrepancy {
    /// An agent-held belief is older than its tolerated freshness window
    /// and must be reverified before reuse.
    BeliefStale,
    /// Two beliefs about the same proposition disagree; neither is
    /// trusted until a new observation arbitrates.
    BeliefContradicted,
    /// A contended affordance (queue, reservation, exclusive facility)
    /// was won by another actor. Short-TTL backoff; retry plausible.
    ContentionLost,
    /// The planner/executor is in a state from which no further legal
    /// step can be taken (e.g. intermediate execution aborted partway).
    /// Requires proper-state recovery before resume.
    ImproperPlanningState,
    /// An observation that should have been made during a step did
    /// not arrive (perception failed, occluded, channel missing).
    MissingObservation,
    /// No legal binding exists for the request under the current
    /// `BindingStrictness` class. Includes S108's
    /// `MatchOutcome::ExactIdentityRequired` case.
    NoLegalBinding,
    /// A counterparty required for the step refused (merchant unwilling,
    /// witness declined, office-holder denied).
    NoWillingCounterparty,
    /// The route to the destination is not known to the agent (not in
    /// its route beliefs). Distinct from a known-but-unsafe route.
    RouteUnknown,
    /// The planner exhausted its search budget without finding a
    /// feasible plan. Longer TTL; re-evaluate when context shifts.
    SearchBudgetExhausted,
    /// The goal is structurally impossible given the current world
    /// (no affordance of the requested kind exists anywhere). Long TTL;
    /// suppress until the world changes.
    StructurallyImpossible,
    /// Execution partially applied effects before failing, leaving the
    /// agent in a state that is neither the old start nor the intended
    /// end. Needs bail-out before replan.
    PartialExecutionDrift,
}
```

### D2: `DiscrepancyMemory` component

```rust
/// Records typed discrepancies per goal/place/target, each with a
/// class-specific TTL. Replaces the `Unknown` / `AssumptionFailed`
/// slice of the old `BlockedIntentMemory`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyMemory {
    pub entries: BTreeMap<BlockerKey, DiscrepancyEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyEntry {
    pub blocker_key: BlockerKey,
    pub discrepancy: Discrepancy,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub clearing_condition: DiscrepancyClearing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiscrepancyClearing {
    /// Retry after expiry tick.
    TtlExpiry,
    /// Clear when the agent observes the place/target again.
    ReobservationOf { target: EntityId },
    /// Clear when a specific belief claim is updated.
    BeliefUpdate { claim_key: BeliefClaimKey },
    /// Clear when the world crosses a structural threshold (new
    /// affordance appears, new route learned).
    WorldStructureChange,
}
```

### D3: `BlockerMemory` component

Carries the world-state blocker variants from the old `BlockedIntentMemory` (`SellerOutOfStock`, `WorkstationBusy`, `ReservationConflict`, `ExclusiveFacilityUnavailable`, `TargetGone`, `DangerTooHigh`, `CombatTooRisky`, `TooExpensive`, `MissingTool`, `MissingInput`, `PatienceExhausted`, `NoBuyer`, `NoKnownPath`, `NoKnownSeller`, `SourceDepleted`). Shape matches the current `BlockedIntentMemory` exactly — this is the direct migration target, less the `Unknown` / `AssumptionFailed` variants.

### D4: `RepairMemory` and `LearnedOpportunityMemory`

```rust
/// Records successful alternate repairs (merchant B worked after
/// merchant A failed; route C after route A was hostile). Feeds
/// ranking to boost preferred_operator_boost on the alternative.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairMemory {
    pub repairs: BTreeMap<RepairKey, RepairEntry>,
}

/// Records opportunities the agent perceived en route to a different
/// goal (a new well observed during a travel step, a new merchant seen
/// during market travel). Decays with time; overwritten by revisits.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LearnedOpportunityMemory {
    pub opportunities: BTreeMap<OpportunityKey, OpportunityEntry>,
}
```

Exact key and entry shapes land at implementation time. Size bounds: each memory caps at a per-agent `MemoryCapacityProfile` field (default 32 entries; eviction by oldest `observed_tick`). Caps are profile-driven, not hardcoded.

### D5: Migration of `BlockingFact::Unknown` and `AssumptionFailed` call sites

Grep sweep: `BlockingFact::Unknown` appears in `failure_handling.rs`, `blocked_intent.rs`, `agent_tick/mod.rs`, `agent_tick/frame.rs`. Each call site maps to a specific `Discrepancy` variant:

- `failure_handling.rs:66` (diagnostic filter) → `Discrepancy::ImproperPlanningState`
- `failure_handling.rs:176` (default branch) → replaced by explicit classifier
- `failure_handling.rs:574` (`ActionAbortRequestReason::SelfTargetForbidden`) → `Discrepancy::NoLegalBinding`
- `agent_tick/mod.rs:840` (filter on unexpired unknowns) → migrated to query `DiscrepancyMemory` instead
- `agent_tick/frame.rs:435` (`AssumptionFailed`) → `Discrepancy::BeliefContradicted` or `Discrepancy::TargetGone` depending on specific break (routed through `BlockerMemory::TargetGone` where applicable)

The `BlockingFact` enum loses `Unknown` and `AssumptionFailed` variants after migration. This is not backward compatible (FND-28) — old save files with those variants fail decode. Migration: new save format only; prior runs are not decodable.

### D6: TTL policy per class

`failure_handling.rs::blocking_fact_ttl` currently maps every `BlockingFact` variant to a `CognitiveProfile` field. Replace with `discrepancy_ttl(&Discrepancy, &CognitiveProfile)` returning per-class TTL:

| Discrepancy | Default TTL field (on `CognitiveProfile`) | Rough default |
|-------------|-------------------------------------------|----------|
| `BeliefStale` | `stale_belief_backoff_ticks` | 30 |
| `BeliefContradicted` | `contradicted_belief_backoff_ticks` | 60 |
| `ContentionLost` | `contention_backoff_ticks` | 8 |
| `ImproperPlanningState` | `improper_state_backoff_ticks` | 2 |
| `MissingObservation` | `missing_observation_backoff_ticks` | 20 |
| `NoLegalBinding` | `no_legal_binding_backoff_ticks` | 120 |
| `NoWillingCounterparty` | `counterparty_refusal_backoff_ticks` | 40 |
| `RouteUnknown` | `route_unknown_backoff_ticks` | 200 |
| `SearchBudgetExhausted` | `search_exhaustion_backoff_ticks` | 100 |
| `StructurallyImpossible` | `structural_impossibility_backoff_ticks` | 400 |
| `PartialExecutionDrift` | `partial_drift_backoff_ticks` | 4 |

All defaults are per-profile and overridable per agent via scenario.

### D7: Belief-view accessors

Add `discrepancy_memory(agent)`, `blocker_memory(agent)`, `repair_memory(agent)`, `learned_opportunity_memory(agent)` accessors on the appropriate belief-view sub-trait in `crates/worldwake-sim/src/belief_view.rs`. Default returns `Default::default()`; `PerAgentBeliefView` reads the live component.

### D8: Scenario contract

Add `discrepancy_memory`, `blocker_memory`, `repair_memory`, `learned_opportunity_memory` as runtime-generated universal components (exempt from scenario-authored initialization per spec-drafting-rules.md §5 — they start empty and accumulate from runtime experience). Add the new TTL fields to `CognitiveProfile` with serde defaults so existing scenarios remain valid.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Each discrepancy is recorded against the agent that observed it. Discrepancies do not propagate between agents — another agent learns about a merchant stockout only by its own failed purchase or by a shared belief (`ShareBelief` — separate path). Aligned with FND-7.
2. **Positive-feedback analysis**: The only loop is "failure → record discrepancy → suppress goal → reattempt after TTL → possibly record again." The class-specific TTL is the dampener: `StructurallyImpossible` suppresses for 400+ ticks, preventing infinite retry spam; `ContentionLost` permits quick retry because the world genuinely changes fast.
3. **Concrete dampeners**: Per-class TTL values are the dampener. They are not invisible caps — they encode how long an agent is justified in believing the failure classification without re-evidence. Agents with `stale_belief_backoff_ticks = 5` reverify quickly (impulsive); agents with `stale_belief_backoff_ticks = 50` reverify rarely (stubborn).
4. **Stored state vs. derived read-model**: `DiscrepancyMemory`, `BlockerMemory`, `RepairMemory`, `LearnedOpportunityMemory` are authoritative stored state (each entry records a specific observation). The "is this goal blocked?" decision is derived from reading those memories plus the current tick.

## SystemFn Integration

No new tick-phase SystemFn. Memory maintenance (TTL expiry) runs inline within `agent_tick` the same way `BlockedIntentMemory::expire` runs today. Each memory gets an analogous `expire(current_tick)` call at the start of the agent tick.

## Component Registration

Four new components on `EntityKind::Agent`, runtime-generated (exempt from scenario contract per spec-drafting-rules.md §5):

- `DiscrepancyMemory` — universal, `Default::default()`
- `BlockerMemory` — universal, `Default::default()`
- `RepairMemory` — universal, `Default::default()`
- `LearnedOpportunityMemory` — universal, `Default::default()`

Old `BlockedIntentMemory` component is removed (no backward-compat path — FND-28).

`CognitiveProfile` gains new TTL fields; existing field `unknown_block_ticks` is removed or renamed.

## Cross-System Interactions

- **Sim dispatch ↔ discrepancy recording**: `tick_step.rs` BestEffort failures now produce typed discrepancies (not `Unknown`). State-mediated.
- **Revalidation ↔ discrepancy memory**: `plan_revalidation.rs` reads `DiscrepancyMemory` when classifying revalidation failure.
- **Candidate generation ↔ memories**: `candidate_generation.rs` filters suppressed goals by reading `BlockerMemory`, `DiscrepancyMemory`, and boosts alternatives via `RepairMemory` / `LearnedOpportunityMemory`.
- **S110 event log ↔ discrepancy recording**: Every recorded discrepancy emits `EventTag::BlockerRecorded { discrepancy, blocker_key }` to the authoritative event log.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `stale_belief_backoff_ticks` | `CognitiveProfile` | `u32` | 30 | How long to trust a stale-belief classification before re-evidencing |
| `contradicted_belief_backoff_ticks` | `CognitiveProfile` | `u32` | 60 | Same for contradictions |
| `contention_backoff_ticks` | `CognitiveProfile` | `u32` | 8 | Short backoff for race losses |
| `improper_state_backoff_ticks` | `CognitiveProfile` | `u32` | 2 | Short backoff for planner-internal state errors |
| `missing_observation_backoff_ticks` | `CognitiveProfile` | `u32` | 20 | Perception-gap backoff |
| `no_legal_binding_backoff_ticks` | `CognitiveProfile` | `u32` | 120 | ExactIdentity / strictness failure backoff |
| `counterparty_refusal_backoff_ticks` | `CognitiveProfile` | `u32` | 40 | Merchant/witness/office refused |
| `route_unknown_backoff_ticks` | `CognitiveProfile` | `u32` | 200 | Unknown route; wait for testimony/exploration |
| `search_exhaustion_backoff_ticks` | `CognitiveProfile` | `u32` | 100 | Planner budget exhausted |
| `structural_impossibility_backoff_ticks` | `CognitiveProfile` | `u32` | 400 | Nothing of the kind exists anywhere known |
| `partial_drift_backoff_ticks` | `CognitiveProfile` | `u32` | 4 | Bail-out recovery window |
| `memory_capacity` | `MemoryCapacityProfile` (new) | `u32` | 32 | Entry cap per memory |

## Validation and Falsification

### Unit tests (in `discrepancy.rs` / memory modules)

1. `DiscrepancyMemory::record` + `expire(tick)` prunes expired entries.
2. `DiscrepancyClearing::ReobservationOf { target }` clears when perception system records a new observation of that target.
3. `BlockerMemory` preserves exact old `BlockedIntentMemory` semantics for migrated variants.
4. `RepairMemory` overwrites a repair entry when a fresher successful alternate is recorded.
5. `LearnedOpportunityMemory` evicts oldest entry when `memory_capacity` is exceeded.

### Migration tests

6. Every `failure_handling.rs` code path that previously emitted `BlockingFact::Unknown` now emits a specific `Discrepancy` variant (compile-time coverage via exhaustive match).
7. Every `BlockingFact::AssumptionFailed` call site maps to a specific variant with no silent fallback to a catch-all.

### Golden test extension

8. Extend an existing replan golden (e.g., `golden_healer_acquires_remote_ground_medicine_for_patient` or `golden_planner_pathology` scenarios) with an assertion that after a target-gone replan, the agent's `DiscrepancyMemory` or `BlockerMemory` contains a typed entry (not `Unknown`).

## Outcome

To be filled in at completion.
