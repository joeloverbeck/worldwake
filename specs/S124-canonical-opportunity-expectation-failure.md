# S124: Canonical Opportunity Expectation Failure

## Summary

Unify source-backed opportunity failure handling across observation, planning, and action execution. The recent `SURVPREF-001` landing proved the live survival gap, but it also exposed an architectural split: source-reliability learning currently enters through several lawful seams (`candidate_generation`, read-phase local depletion, same-goal plan-search failure, and a shared persistence helper) rather than through one canonical expectation-failure substrate. This spec replaces that split with a single contract:

1. source-backed opportunities keep concrete provenance when they become the committed intention,
2. all expectation failures are recorded as one normalized runtime incident shape,
3. one attribution path decides whether the incident means "this source became less trusted", "this plan was blocked for a different reason", or both,
4. intention reconsideration and ranking consume that recorded outcome on the next tick.

The goal is not to preserve every current seam. The goal is to make "I expected this concrete source to satisfy this concrete opportunity, and reality contradicted that expectation" a first-class, inspectable, FOUNDATIONS-aligned concept.

## Phase and Status

Phase 8 Adjunct: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-core` - shared provenance/incident identity types if they belong in durable agent state or shared trace payloads
- `worldwake-ai` - canonical provenance capture, expectation-failure production, attribution, intention reconsideration, ranking integration, decision trace surfacing
- `worldwake-sim` - no new planner-facing authority channel; only existing authoritative action outcomes remain the source of execution-stage contradictions

## Dependencies

- S22 (generalized intention frames) - hard. This spec extends the committed-intention contract rather than creating a parallel current-plan memory lane.
- S109 (typed discrepancy taxonomy) - hard. Expectation failures must continue to route through typed, inspectable discrepancy memory rather than silent suppression.
- S110 (decision history events) - soft. If present, the canonical incident should surface through the existing decision-history/event path rather than bespoke logging.
- S112 (portfolio planning) - soft. Search/probe stages may produce expectation-failure incidents, but only through the shared substrate defined here.
- S122 (frame assumption commodity availability) - motivating evidence only. S122 proved that frame-level contradiction handling is required, but it did not unify source-backed attribution.

## Problem Statement

The live acquisition path currently has one concept expressed in multiple local forms:

- "local observation says the source is depleted or absent,"
- "search cannot build a same-goal plan from the current local source, but a sibling source still works,"
- "authoritative action start rejects the source-backed attempt,"
- "candidate-generation violation synthesis implies the source did not satisfy the expected commodity."

These are all contradictions of one practical expectation: a concrete source-backed opportunity was believed to satisfy a concrete goal, and that expectation failed. Today the code can learn correctly from several of these seams, but:

- the learning path is fragmented,
- provenance is partly reconstructed after the fact,
- current-plan retention requires `hydrate_reinstated_current_plan_source_entity(...)`,
- the architecture still answers "why did source trust change?" with seam-specific logic instead of one inspectable causal record.

That is the gap this spec closes.

## Design Goals

1. A committed source-backed opportunity must retain enough concrete provenance to survive replanning, plan retention, and trace inspection without synthetic rehydration.
2. Every expectation contradiction for that opportunity must be representable as one normalized incident type, regardless of whether it was detected during observation, search, or authoritative execution.
3. Source-reliability updates must flow through one attribution function, not multiple seam-specific writes.
4. The architecture must distinguish "the source failed" from "the goal remains valid but this source/path did not."
5. Intention reconsideration must remain stable but explicit: commitments persist until contradicted by concrete evidence, then revise through a lawful reconsideration path.
6. No global abstract trust scores, no magical confidence math, no backward-compatibility shadow paths.

## Non-Goals

- A generalized probabilistic confidence system for all planner beliefs.
- Faction-wide or commodity-wide reputation scores. Trust remains concrete and source-specific.
- Replacing `SourceReliability` with an abstract utility or score model.
- Creating a direct system-to-system call from authoritative action handlers into ranking. All effects still flow through stored state and the next decision tick.
- Solving every planner failure class. This spec covers source-backed opportunity expectation failure, not all search failures.

## External Research Notes

These references are not architectural authority over Worldwake, but they point in the same direction as `docs/FOUNDATIONS.md`:

- Goal-Driven Autonomy emphasizes explicit discrepancy detection, explanation, and goal-management rather than ad hoc repair hooks. That maps cleanly to FND-17 and FND-21. Source: Lehigh Goal-Driven Autonomy overview, https://www.cse.lehigh.edu/~munoz/projects/GDA/
- Continuous Planning and Execution Framework separates monitoring from repair: plans remain revisable artifacts, but contradictions are first-class monitored inputs instead of being buried inside replanning code. That maps cleanly to FND-20 and FND-21. Source: Karen Myers, *Towards a Framework for Continuous Planning and Execution*, https://www.sri.com/publication/artificial-intelligence-pubs/towards-a-framework-for-continuous-planning-and-execution/
- BDI/intention-management literature consistently treats commitments as stable but reconsiderable, and treats learning plus practical reasoning as complementary rather than interchangeable. That supports an explicit reconsideration policy and argues against silent source-score hacks. Sources:
  - *Behavioral flexibility in Belief-Desire-Intention (BDI) architectures*, https://journals.sagepub.com/doi/10.3233/MGS-200335
  - *The theory and practice of intention reconsideration*, https://www.tandfonline.com/doi/abs/10.1080/09528130412331309277

Worldwake should copy the architecture shape, not the academic abstractions: explicit discrepancy, explicit provenance, explicit reconsideration, concrete stored learning.

## FOUNDATIONS Alignment

| Principle | How S124 Satisfies It |
|-----------|------------------------|
| FND-3 (Concrete State) | Source learning remains concrete `SourceKey -> ReliabilityRecord`; failures are tied to concrete committed provenance, not abstract scores. |
| FND-7 (Locality) | Incidents are produced only from the agent's own observation, search from its own beliefs, or authoritative action outcomes from its own attempted plan. |
| FND-10 (Aftermath) | A failed expectation leaves durable aftermath in `SourceReliability` and discrepancy memory, affecting later ranking. |
| FND-14 / FND-14A | Observation-stage contradiction uses the agent's belief/perception boundary; execution-stage contradiction uses authoritative action outcomes already available to the acting agent's control loop. |
| FND-17 (Violated Expectation) | The spec makes expectation contradiction an explicit runtime artifact instead of implicit seam logic. |
| FND-20 (Bounded Practical Reasoning) | Search, observation, and execution may each detect contradictions, but the agent still updates through one bounded attribution path. No omniscient reconciliation pass. |
| FND-21 (Revisable Commitments) | Intentions stay committed until a canonical contradiction record says otherwise; reconsideration is explicit and inspectable. |
| FND-22A (Learning) | Learning is durable, concrete, source-scoped, and attributable to a named incident with phase and cause. |
| FND-26 (State-mediated interaction) | Action execution, perception, planning, and ranking communicate via runtime incident handling and stored state, not direct cross-module decisions. |
| FND-28 (No backward compatibility) | The canonical incident path replaces duplicated learning writes and removes provenance rehydration hacks instead of preserving them indefinitely. |

## Deliverables

### D1. Canonical committed provenance for source-backed opportunities

Source-backed `GoalOffer`s already carry concrete evidence in `GoalOffer::evidence_entities`, `GoalOffer::evidence_places`, `OpportunityAnchor`, and `OpportunityKey`. That information must stop disappearing when the agent commits to or retains a plan.

Add one canonical runtime contract for the currently committed source-backed opportunity:

```rust
pub struct CommittedOpportunityProvenance {
    pub opportunity: OpportunityKey,
    pub source: Option<SourceKey>,
    pub supporting_entity: Option<EntityId>,
    pub supporting_place: Option<EntityId>,
    pub expectation_kind: OpportunityExpectationKind,
}

pub enum OpportunityExpectationKind {
    AcquireCommodityFromConcreteSource,
    RestockCommodityFromConcreteSource,
    ConcreteTargetPresence,
}
```

Launch scope is only the source-backed acquisition family. The exact field names may differ, but the contract must satisfy these invariants:

- if ranking/candidate generation identified exactly one concrete source entity for a source-backed acquisition opportunity, the committed runtime state retains that exact source identity,
- the retained-current-plan path reuses stored provenance rather than reconstructing it from a fresh ranked candidate,
- if an opportunity is only place-anchored and has no concrete source identity yet, it may still commit, but it cannot generate a source-reliability update until it becomes concretized.

Implementation note: this provenance belongs on the committed intention/runtime path, not as a read-side helper in `observation.rs`. `hydrate_reinstated_current_plan_source_entity(...)` is the smell this deliverable removes.

### D2. Normalized expectation-failure incident

Introduce one normalized runtime incident for contradictions beneath a committed opportunity:

```rust
pub struct OpportunityExpectationFailureIncident {
    pub opportunity: OpportunityKey,
    pub provenance: CommittedOpportunityProvenance,
    pub detected_at_tick: Tick,
    pub phase: ExpectationFailurePhase,
    pub cause: ExpectationFailureCause,
}

pub enum ExpectationFailurePhase {
    Observation,
    Search,
    ExecutionStart,
    ExecutionOutcome,
}

pub enum ExpectationFailureCause {
    SourceAbsentLocally,
    SourceDepletedLocally,
    SameGoalSearchInfeasibleWhileSiblingSucceeded,
    AuthoritativeStartRejected,
    AuthoritativeOutcomeContradictedExpectation,
}
```

This incident is a runtime reasoning artifact, not a new abstract score. It exists so that all contradiction-producing seams emit the same shape before any learning or suppression happens.

### D3. Producers become detectors only

The current local seams become incident producers only:

- `candidate_generation.rs`
- `agent_tick/observation.rs`
- `agent_tick/planning.rs`
- authoritative action-start / plan-failure handling where a concrete source-backed attempt is rejected

These sites may detect and emit `OpportunityExpectationFailureIncident`, but they must not each decide independently whether to mutate `SourceReliability`.

Their responsibility ends at:

1. identify the contradiction,
2. attach the committed provenance,
3. emit the normalized incident.

This is the core architectural cleanup. After S124, there is one lawful write path for source-reliability learning.

### D4. Single attribution and persistence path

Add one shared attribution function in `worldwake-ai` that consumes normalized incidents and decides:

- whether the failure is attributable to a concrete source and therefore should increment `SourceReliability.failed_attempts`,
- whether it should also or instead record a typed blocker/discrepancy,
- whether it invalidates the current intention immediately or merely weakens its future ranking.

Required attribution rules:

1. A source-reliability update is only lawful when the incident points to one concrete `SourceKey`.
2. Same-goal sibling success is evidence against the current source-backed opportunity, not against the goal kind itself.
3. Route/path/precondition failures that are not attributable to the source must not be mislabeled as source unreliability.
4. Observation-stage absence/depletion and authoritative start rejection for the same concrete source map to the same durable source-learning substrate, even if their discrepancy causes differ.
5. Duplicate incidents for the same `(opportunity, source, phase, tick)` must be coalesced before persistence.

This attribution function becomes the only place that writes source-failure aftermath into durable memory.

### D5. Explicit intention reconsideration policy

The current architecture already supports revisable commitments through `IntentionFrame`. S124 defines the policy for source-backed contradictions:

- If the committed source-backed opportunity is contradicted, the frame is reconsidered as "current source invalidated", not "goal invalidated".
- If a same-goal sibling source remains viable, the agent may immediately replace the current opportunity while keeping the same goal kind.
- If no same-goal sibling is viable, the frame clears through the existing failure/discrepancy path and later ranking determines the next goal.

This distinction is mandatory. It prevents source-learning from masquerading as goal rejection and keeps the causal story faithful.

### D6. Decision-trace and explanation surface

Decision traces must surface the canonical contradiction directly. At minimum, the trace/explanation layer should be able to answer:

- which committed opportunity failed,
- which concrete source was being trusted,
- which phase detected the contradiction,
- what the contradiction cause was,
- whether the result was source-reliability learning, frame invalidation, same-goal source switch, or some combination.

This is required so future debugging does not regress back to inference from scattered hooks.

### D7. Current-plan/runtime contract cleanup

Remove the need for source-entity rehydration in retained-current-plan handling. After S124:

- `RetainedCurrentPlan` uses committed provenance already attached to the current frame/runtime state,
- read-phase logic consumes that provenance directly,
- no helper reconstructs the source entity from ranked candidates during reinstatement.

If an opportunity cannot preserve concrete provenance through commitment, that is a contract bug in the commitment path, not a reason to patch observation with recovery logic.

### D8. Scope of canonical source-backed opportunities

Launch scope for the unified substrate includes source-backed acquisition opportunities that currently feed `SourceReliability`:

- `GoalKind::AcquireCommodity`
- `GoalKind::RestockCommodity`

using concrete sources surfaced through `GoalOffer::evidence_entities` and represented as `SourceKey`.

This spec does not require immediate generalization to every target-backed goal. It does require the substrate to be shaped so future concrete expectation kinds can reuse it without inventing another parallel failure path.

## Proposed Architecture

### A. Canonical flow

1. Candidate generation/ranking produces a source-backed opportunity with concrete source evidence.
2. Commitment stores normalized committed provenance on the active intention/runtime path.
3. Observation/search/execution may each emit `OpportunityExpectationFailureIncident`.
4. A single attribution function translates incidents into:
   - `SourceReliability` mutation when the source is concretely implicated,
   - discrepancy/blocker memory mutation when appropriate,
   - intention invalidation/replacement outcome.
5. Next-tick ranking consumes the updated state and discounts the failed source-backed opportunity.

### B. Canonical boundary

The shared abstraction boundary under audit is:

`source-backed committed opportunity provenance -> expectation-failure incident -> attribution -> durable aftermath`

Any future feature in this family must either reuse that boundary or justify why it is not a source-backed opportunity expectation failure.

### C. Removed duplicate-path behavior

After S124, the following are not allowed as independent architectural paths:

- direct source-reliability writes from observation without incident normalization,
- direct source-reliability writes from planning without incident normalization,
- retained-current-plan source recovery via helper reconstruction,
- seam-specific private definitions of what counts as a "source failure."

## Information-Path Analysis

1. World change occurs: a source is depleted, absent, inaccessible to the current action, or otherwise contradicts the committed expectation.
2. The acting agent reaches that contradiction through one local path only:
   - co-located perception during observation,
   - same-goal plan search from its own belief state,
   - authoritative rejection/outcome of its own attempted action.
3. The detecting seam emits `OpportunityExpectationFailureIncident` tied to committed provenance.
4. The attribution function updates `SourceReliability` and discrepancy memory.
5. Ranking on a later tick reads those stored records and discounts the failed source-backed opportunity.

No agent receives this information by oracle. No ranking code reaches backward into authoritative systems to reinterpret the failure.

## Positive-Feedback Analysis

There is one relevant loop:

- failed source expectation lowers trust in a source,
- lower trust increases chance of selecting a sibling source,
- sibling selection may expose new failures and create more learning.

This is a lawful learning loop, not a bug, but it needs dampening so agents do not thrash or globally blacklist commodities.

## Concrete Dampeners

1. `SourceReliability` is keyed to concrete `SourceKey`, not commodity-wide class labels. Failure of one orchard does not poison all apples.
2. Existing memory retention and capacity limits on `SourceReliability` physically age out stale aftermath.
3. Commitment still binds the agent until a concrete contradiction is produced; ranking alone does not continuously churn the current source.
4. Same-goal switching only occurs when a concrete sibling opportunity exists. The world must physically provide another source.
5. Duplicate incidents for the same failure episode are coalesced before persistence.

These are all world/state/process dampeners, not arbitrary caps.

## Stored State vs. Derived Read-Model List

Authoritative stored state:

- `SourceReliability`
- existing typed discrepancy/blocker memories
- committed opportunity provenance stored on the active intention/runtime path

Runtime-derived only:

- `OpportunityExpectationFailureIncident`
- same-goal sibling comparison during selection/search
- coalescing of repeated incidents within a tick
- trace summaries and explanation strings

No derived ranking score or confidence summary becomes authoritative memory.

## SystemFn Integration

No new top-level simulation `SystemFn` is required.

This spec changes the AI tick pipeline only:

- commitment writes committed provenance once a source-backed opportunity is selected,
- observation/search/execution stages may emit normalized incidents,
- one shared attribution step runs inside the existing agent-tick decision/control flow before later ranking consumes the aftermath.

The authoritative simulation remains the source of execution outcomes; S124 only changes how the acting agent interprets those outcomes.

## Component Registration

No new scenario-definable behavior component is required.

`SourceReliability` already exists as the durable learning component. The new committed provenance field, if stored on `IntentionFrame` or adjacent runtime state, is runtime-generated and exempt from the scenario contract in `docs/spec-drafting-rules.md`.

If any shared provenance/incident identity type is moved into `worldwake-core`, that is a shared runtime type addition, not a new scenario-authored agent profile.

## Acceptance Criteria

1. There is exactly one shared function/path that mutates source-reliability memory from expectation contradiction.
2. Retained-current-plan handling no longer needs provenance rehydration.
3. Observation-, planning-, and execution-originated source contradictions are representable through one normalized incident type.
4. Same-goal sibling success records source failure against the current concrete source without rejecting the entire goal kind.
5. Decision traces expose the failed source-backed expectation with enough detail to explain the switch or discount.
6. The design remains concrete, local, and belief-first, with no abstract confidence score layer or compatibility shim.

## Implementation Notes

- The preferred landing is a small number of shared types and one shared attribution function, not another local helper cluster.
- If a naming choice conflicts with existing symbols, preserve the architectural boundary, not the draft names.
- If one existing seam proves not to be a true source expectation failure, remove it from the unified path rather than broadening the concept until it becomes vague.
