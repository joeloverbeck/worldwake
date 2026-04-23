# S124: Canonical Opportunity Expectation Failure

## Summary

Unify AI-layer source-backed opportunity failure handling around one normalized incident shape. The recent `SURVPREF-001` landing (`archive/tickets/SURVPREF-001.md`) proved the live survival gap, and the follow-up `S124OPEXFAL-001` landing (`archive/tickets/S124OPEXFAL-001.md`) closed the committed-provenance carrier by adding `PlannedPlan.committed_source: Option<SourceKey>` and removing the `hydrate_reinstated_current_plan_source_entity(...)` reconstruction helper. What still remains is the normalization gap: three AI-layer detection sites (`observation.rs::pending_local_source_reliability_failures`, `candidate_generation.rs::emit_expectation_violation_candidates`, `planning.rs::same_goal_search_failed_source_keys`) coalesce into one AI-layer writer (`apply_source_reliability_failure_observations`), but they exchange only `BTreeSet<SourceKey>` — the incident cause, phase, and richer expectation kind never travel together. This spec closes that gap with a single contract:

1. source-backed committed opportunities carry an explicit `OpportunityExpectationKind` in addition to the already-landed `committed_source`,
2. all AI-layer expectation failures are recorded as one normalized runtime incident shape,
3. one attribution path (the evolved `apply_source_reliability_failure_observations`) decides whether the incident means "this source became less trusted", "this plan was blocked for a different reason", or both,
4. intention reconsideration and ranking consume that recorded outcome on the next tick.

The goal is not to preserve every current detection seam. The goal is to make "I expected this concrete source to satisfy this concrete opportunity, and reality contradicted that expectation" a first-class, inspectable, FOUNDATIONS-aligned concept on the AI-layer attribution path. Authoritative action outcomes recorded by the systems-layer helpers in `crates/worldwake-systems/src/experience_recording.rs` (called from `production_actions.rs` and `trade_actions.rs`) remain a separate lawful write path under FND-26 — the two paths mutate the same `SourceReliability` component but are not collapsed into one helper.

## Phase and Status

Phase 8 Adjunct: Belief-First Continual Planning Foundation. Status: Completed 2026-04-23; archive this spec.

## Crates

- `worldwake-ai` - canonical expectation-failure production, attribution evolution, intention reconsideration integration, ranking integration, decision trace surfacing. All new runtime types (`OpportunityExpectationFailureIncident`, `ExpectationFailurePhase`, `ExpectationFailureCause`, `OpportunityExpectationKind`) live here because they are runtime reasoning artifacts, not durable agent state. The already-landed `PlannedPlan.committed_source` carrier also lives here.
- `worldwake-core` - no new types. All provenance building blocks (`SourceKey`, `OpportunityKey`, `OpportunityAnchor`, `Tick`, `EntityId`) already exist here.
- `worldwake-sim` - no new planner-facing authority channel; systems-layer authoritative action outcomes continue to write through `experience_recording.rs` helpers as a separate lawful path (see `crates/worldwake-systems/src/experience_recording.rs`).

## Dependencies

- `archive/tickets/S124OPEXFAL-001.md` (landed 2026-04-23) - hard. Delivered the committed source-provenance carrier on `PlannedPlan` and removed `hydrate_reinstated_current_plan_source_entity(...)`. This ticket is the prerequisite that makes this spec's normalization work possible without re-litigating provenance preservation.
- `archive/tickets/SURVPREF-001.md` - hard (landed). Proved the live survival-time source-failure memory update; motivates the normalization substrate.
- `archive/specs/S22-generalized-intention-frames.md` - hard (landed). Defines `IntentionFrame`, `FrameState`, and the committed-intention lifecycle that D5's reconsideration policy extends.
- `archive/specs/S109-typed-discrepancy-taxonomy.md` - hard (landed). Defines `Discrepancy`, `BlockerMemory`, `DiscrepancyMemory`, `RepairMemory`, `LearnedOpportunityMemory`. Expectation failures must continue to route through typed, inspectable discrepancy memory rather than silent suppression.
- `archive/specs/S110-decision-history-events.md` - hard (landed). `DecisionEventPayload` (with variants `GoalOffered`, `GoalAbandoned`, `PlanInvalidated`, `ExpectationMismatch`, `BlockerRecorded`, etc.) is the canonical trace surface D6 extends.
- `archive/specs/S112-portfolio-planning.md` - soft (landed). Search/probe stages may produce expectation-failure incidents, but only through the shared substrate defined here.
- `archive/specs/S122-frame-assumption-commodity-availability.md` - motivating evidence only (landed). S122 proved that frame-level contradiction handling is required; `record_assumption_failure(...)` at `crates/worldwake-ai/src/agent_tick/frame.rs:496` is the S122 entrypoint whose module-local reconsideration seam D5 extends. S122 did not unify source-backed attribution.

## Problem Statement

The AI-layer acquisition path currently detects expectation failures at three distinct sites that all feed one coalesced writer:

- observation: `pending_local_source_reliability_failures(...)` at `crates/worldwake-ai/src/agent_tick/observation.rs:329` detects "source absent or depleted locally" using the retained-plan `committed_source` carrier,
- candidate generation: `emit_expectation_violation_candidates(...)` at `crates/worldwake-ai/src/candidate_generation.rs:4172` detects the `SupplyDepleted` violation (believed-available > 0, observed = 0) and emits a `SourceKey` into the pending-failure set,
- planning: `same_goal_search_failed_source_keys(...)` at `crates/worldwake-ai/src/agent_tick/planning.rs:341` detects "current plan search failed while a sibling opportunity succeeded."

All three feed `apply_source_reliability_failure_observations(...)` at `crates/worldwake-ai/src/agent_tick/mod.rs:1904`, which is the single AI-layer writer. The writer currently accepts only `&BTreeSet<SourceKey>` — it has no way to carry phase, cause, or expectation-kind information, so detection sites cannot tell the writer *why* the source failed, only that it did. The remaining architectural gap is:

- detections exchange an impoverished shape (`SourceKey` set) with the writer, so attribution rules collapse to "was a `SourceKey` present?" instead of distinguishing observation-stage absence from same-goal-sibling success from authoritative start rejection,
- decision traces cannot surface the canonical contradiction because the incident shape is not materialized,
- intention reconsideration cannot distinguish "source invalidated" from "goal invalidated" because both reach the reconsideration path through the same primitive set.

The systems-layer writes via `experience_recording.rs` (`record_failed_source_attempt`, `record_successful_source_acquisition`) called from `production_actions.rs:611,732` and `trade_actions.rs:394,421` remain a **separate lawful path** under FND-26. Authoritative action outcomes are the world's ground truth; their recording stays in systems. S124 does not attempt to fold these into the AI-layer attribution function.

That is the gap this spec closes.

## Design Goals

1. A committed source-backed opportunity must carry an explicit expectation kind alongside the already-landed concrete source identity, so the AI-layer attribution function can reason about what kind of expectation was violated rather than only which source entity was involved.
2. Every AI-layer expectation contradiction for a source-backed opportunity must be representable as one normalized runtime incident type, regardless of whether it was detected during observation, candidate generation, or search.
3. AI-layer source-reliability updates must flow through one evolved attribution function (`apply_source_reliability_failure_observations` taking normalized incidents), not multiple seam-specific primitives. Systems-layer writes from authoritative action outcomes remain a separate lawful path under FND-26.
4. The architecture must distinguish "the source failed" from "the goal remains valid but this source/path did not."
5. Intention reconsideration must remain stable but explicit: commitments persist until contradicted by concrete evidence, then revise through the existing frame-level reconsideration seam at `crates/worldwake-ai/src/agent_tick/frame.rs`, recording `Discrepancy::SourceInvalidated` and clearing the committed source plan for replanning where appropriate.
6. No global abstract trust scores, no magical confidence math, no backward-compatibility shadow paths.

## Non-Goals

- A generalized probabilistic confidence system for all planner beliefs.
- Faction-wide or commodity-wide reputation scores. Trust remains concrete and source-specific.
- Replacing `SourceReliability` with an abstract utility or score model.
- Unifying systems-layer authoritative action writers (`experience_recording.rs::record_failed_source_attempt`, `record_successful_source_acquisition`) into the AI-layer attribution function. Authoritative action outcomes remain a separate lawful write path under FND-26; both paths mutate the same `SourceReliability` component, but through distinct producers.
- Creating a direct system-to-system call from authoritative action handlers into ranking. All effects still flow through stored state and the next decision tick.
- Solving every planner failure class. This spec covers source-backed opportunity expectation failure, not all search failures.
- Moving provenance onto `IntentionFrame` in `worldwake-core`. `IntentionFrame` does not carry opportunity or source identity today; provenance stays on `PlannedPlan` in the AI-layer runtime carrier.

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
| FND-14 / FND-14A | Observation-stage contradiction uses the agent's belief/perception boundary; the candidate-generation `SupplyDepleted` case uses same-tick co-located observation of physical resource-source quantity (FND-14A). Social facts (ownership, rights) are never inferred from co-location. |
| FND-17 (Violated Expectation) | The spec makes expectation contradiction an explicit runtime artifact (`OpportunityExpectationFailureIncident`) instead of an impoverished `BTreeSet<SourceKey>` exchange. |
| FND-20 (Bounded Practical Reasoning) | Observation, candidate generation, and search may each detect contradictions, but the AI-layer attribution path updates reliability through one evolved bounded function. No omniscient reconciliation pass. |
| FND-21 (Revisable Commitments) | Intentions stay committed until a contradiction record says otherwise; reconsideration is explicit and routed through the S122-landed frame reconsideration seam, which records `Discrepancy::SourceInvalidated` and clears the committed source plan for replanning on source-backed failures. |
| FND-22A (Learning) | Learning is durable, concrete, source-scoped via `SourceKey`, and attributable to a named incident with phase and cause. |
| FND-26 (State-mediated interaction) | Action execution, perception, planning, and ranking communicate via stored state, not direct cross-module decisions. The two lawful write paths (AI-layer attribution vs. systems-layer authoritative-action recording) mutate the same `SourceReliability` component through non-overlapping triggering events; no direct cross-path call is introduced. |
| FND-28 (No backward compatibility) | The evolved `apply_source_reliability_failure_observations` replaces the `BTreeSet<SourceKey>` exchange shape without adding a parallel helper. No shim may be added that re-introduces source rehydration; the helper `hydrate_reinstated_current_plan_source_entity` is already gone. |

## Deliverables

### D1. Expectation-kind metadata on the committed source-backed carrier

`S124OPEXFAL-001` landed `PlannedPlan.committed_source: Option<SourceKey>` at `crates/worldwake-ai/src/planner_ops.rs:962`, with `with_committed_source(...)` at `:987` and `committed_source_for_offer(...)` at `:1018`, and removed the `hydrate_reinstated_current_plan_source_entity(...)` rehydration helper. That delivered the "preserve exact concrete source identity across retained plan ticks" contract.

What remains is metadata for the attribution function (D4) to reason about *what kind* of expectation was committed, not only *which source entity* was involved. Extend the AI-layer runtime carrier with an explicit expectation kind:

```rust
// crates/worldwake-ai/src/planner_ops.rs (or a sibling module in crates/worldwake-ai/src/)
pub enum OpportunityExpectationKind {
    AcquireCommodityFromConcreteSource,
    RestockCommodityFromConcreteSource,
}
```

Invariants:

- A source-backed acquisition opportunity with a concrete `SourceKey` gets a concrete `OpportunityExpectationKind` at adoption time in `planning.rs` (the same site that populates `committed_source`).
- A place-anchored opportunity with no concrete source key remains valid but does not carry a concrete expectation kind; it cannot generate a source-reliability update until it becomes concretized.
- The carrier lives on `PlannedPlan` (or immediately alongside it on the AI-layer runtime state). It does not propagate to `IntentionFrame` in `worldwake-core`; `IntentionFrame` is unchanged (`crates/worldwake-core/src/intention_frame.rs` carries no provenance today and this spec does not add any).

Launch scope is only the source-backed acquisition family — `GoalKind::AcquireCommodity` and `GoalKind::RestockCommodity`. Future expectation kinds (e.g., `ConcreteTargetPresence` for non-acquisition goals) are intentionally deferred; they can be added as new enum variants without re-shaping the attribution path.

Supporting-place/entity separation beyond `SourceKey.entity` is not added in this spec. Current detection sites already read `plan.opportunity.anchor` where needed (`OpportunityAnchor::Place(...)` vs `OpportunityAnchor::Entity(...)`), so there is no demonstrated gap that requires a richer `CommittedOpportunityProvenance` struct today. If a future deliverable proves the anchor is insufficient, extend the carrier at that time.

### D2. Normalized expectation-failure incident

Introduce one normalized runtime incident for AI-layer contradictions beneath a committed opportunity. The incident replaces the current `BTreeSet<SourceKey>` exchange shape between detection sites and the AI-layer writer:

```rust
// crates/worldwake-ai/src/ (runtime-only — NOT in worldwake-core)
pub struct OpportunityExpectationFailureIncident {
    pub opportunity: OpportunityKey,
    pub source: SourceKey,
    pub expectation_kind: OpportunityExpectationKind,
    pub detected_at_tick: Tick,
    pub phase: ExpectationFailurePhase,
    pub cause: ExpectationFailureCause,
}

pub enum ExpectationFailurePhase {
    Observation,
    CandidateGeneration,
    Search,
}

pub enum ExpectationFailureCause {
    SourceAbsentLocally,
    SourceDepletedLocally,
    SameGoalSearchInfeasibleWhileSiblingSucceeded,
}
```

Notes:

- Authoritative-start-rejected and authoritative-outcome-contradicted causes are intentionally **not** in this enum. Those are systems-layer events; `production_actions.rs:732` and `trade_actions.rs:421` already write them through `record_failed_source_attempt(...)` in `experience_recording.rs`. Keeping them out of the AI-layer incident enum makes the two lawful write paths visible at the type level.
- The incident is a runtime reasoning artifact, not durable state. It lives in `worldwake-ai/src/`.
- `source` is a concrete `SourceKey` rather than `Option<SourceKey>`. Incidents are only constructed when the detection site has a concrete source identity; place-only opportunities without a resolved source never produce incidents.

### D3. AI-layer detection sites emit incidents

The three AI-layer detection sites become incident producers, converting what they already detect into `Vec<OpportunityExpectationFailureIncident>` instead of `BTreeSet<SourceKey>`:

- `pending_local_source_reliability_failures(...)` at `crates/worldwake-ai/src/agent_tick/observation.rs:329` currently detects local depletion using `plan.committed_source`. It continues to read `committed_source`, but emits incidents tagged `phase=Observation` with cause `SourceDepletedLocally` or `SourceAbsentLocally` depending on whether the plan's target quantity was ever observed.
- `emit_expectation_violation_candidates(...)` at `crates/worldwake-ai/src/candidate_generation.rs:4172` currently inserts into `pending_source_reliability_failures` for the `SupplyDepleted` violation (`candidate_generation.rs:4247`). It emits incidents tagged `phase=CandidateGeneration` with cause `SourceDepletedLocally`. See M1 in the acceptance criteria for the attribution rule that distinguishes this case from a mere discrepancy.
- `same_goal_search_failed_source_keys(...)` at `crates/worldwake-ai/src/agent_tick/planning.rs:341` currently returns source keys when a sibling opportunity's search succeeded. It emits incidents tagged `phase=Search` with cause `SameGoalSearchInfeasibleWhileSiblingSucceeded`.

Each site's responsibility ends at: identify the contradiction, read the committed provenance + expectation kind from `PlannedPlan`, emit the normalized incident. The writer (D4) owns all reliability mutation and attribution decisions.

Systems-layer writers in `production_actions.rs` and `trade_actions.rs` are **not** modified; their direct calls to `record_failed_source_attempt(...)` / `record_successful_source_acquisition(...)` continue to record authoritative action outcomes as a separate lawful write path.

### D4. Evolve the single AI-layer attribution function

`apply_source_reliability_failure_observations(...)` at `crates/worldwake-ai/src/agent_tick/mod.rs:1904` is already the AI-layer single writer. Today it accepts `failed_sources: &BTreeSet<SourceKey>` and increments `failed_attempts` for each key. Evolve it — do not duplicate it — to accept `&[OpportunityExpectationFailureIncident]` and apply the following attribution rules inside one function:

1. A source-reliability update (`failed_attempts += 1`) is lawful only when the incident carries a concrete `SourceKey`. By D2's type shape, this is always true.
2. Same-goal sibling success (`cause=SameGoalSearchInfeasibleWhileSiblingSucceeded`) is evidence against the current source-backed opportunity, not against the goal kind itself.
3. Observation-proven local absence/depletion (`cause=SourceAbsentLocally`, `cause=SourceDepletedLocally`) with a concrete source and belief→observation mismatch → source-reliability update. A precondition mismatch that is *not* observation-proven absence (e.g., route blocked, carry capacity full) is out of scope for this function — those failures belong to the existing discrepancy memory path, not to source reliability.
4. The `SupplyDepleted` case in `candidate_generation.rs:4237-4250` (believed-available > 0, observed = 0, co-located with source) is the canonical example of observation-proven local absence and maps to rule 3; it is NOT a mislabeling of a discrepancy.
5. Duplicate incidents for the same `(opportunity, source, phase, tick)` must be coalesced before persistence.

This evolved function remains the single AI-layer writer of source-failure aftermath. The existing three call sites (`agent_tick/mod.rs:1045`, `planning.rs:1608`, `planning.rs:1966`) are updated to build incidents at the detection site and pass them through. Systems-layer writes via `experience_recording.rs` remain untouched.

### D5. Intention reconsideration policy via the frame reconsideration seam

S22 landed `IntentionFrame` and its lifecycle; S122 landed `record_assumption_failure(...)` at `crates/worldwake-ai/src/agent_tick/frame.rs:496` as the existing assumption-failure recorder inside the frame reconsideration module. S124 extends that module-level seam and its caller chain to distinguish source-invalidation from goal-invalidation for source-backed acquisitions:

- If the attribution function in D4 recorded a source-reliability decrement for the currently committed opportunity, the caller chain records `Discrepancy::SourceInvalidated` through sibling helper `record_source_invalidation(...)`, clears the committed plan for replanning, and preserves the goal kind for next-tick sibling-source selection.
- If a same-goal sibling source remains viable on the next tick, ranking naturally replaces the current opportunity while keeping the goal kind. No direct cross-tick manipulation of `IntentionFrame` state is required.
- If no same-goal sibling is viable, the frame clears through the existing failure/discrepancy path and later ranking determines the next goal.

This distinction is mandatory. It prevents source-learning from masquerading as goal rejection and keeps the causal story faithful. No parallel reconsideration mechanism is added — the deliverable is an extension of the existing frame reconsideration seam plus the writer-summary caller hook.

### D6. Decision-trace surface via `DecisionEventPayload`

S110 landed `DecisionEventPayload` with variants `GoalOffered`, `GoalAbandoned`, `PlanInvalidated`, `ExpectationMismatch`, `BlockerRecorded`, `RepairApplied`, `GoalSuspended`, `ReplanTriggered`, `GoalSuppressed`, etc. D6 extends that payload surface rather than introducing a parallel trace.

Options for surfacing incidents:

- Extend `DecisionEventPayload::ExpectationMismatch` with an optional `source_expectation_failure: Option<OpportunityExpectationFailureIncident>` field, OR
- Add a dedicated `DecisionEventPayload::SourceExpectationFailure(SourceExpectationFailurePayload)` variant that carries the incident directly.

Either approach must answer: which committed opportunity failed, which concrete source was being trusted, which phase detected the contradiction, what the cause was, and whether the attribution outcome was source-reliability learning, frame invalidation, same-goal source switch, or a combination.

The implementation ticket picks one approach at decomposition time; the spec requires only that no bespoke trace surface be introduced alongside `DecisionEventPayload`.

### D7. *Completed by `S124OPEXFAL-001` (2026-04-23).*

This deliverable is retained as a historical anchor for readers cross-referencing older drafts. The work that was originally scoped here landed through `archive/tickets/S124OPEXFAL-001.md`:

- `PlannedPlan.committed_source: Option<SourceKey>` added at `crates/worldwake-ai/src/planner_ops.rs:962`,
- retained-plan read-phase consumes `plan.committed_source` directly at `crates/worldwake-ai/src/agent_tick/observation.rs:346`,
- `hydrate_reinstated_current_plan_source_entity(...)` was removed (zero workspace matches),
- `SAVE_FORMAT_VERSION` bumped from 41 → 42 in `crates/worldwake-sim/src/save_load.rs`.

No further action is required for D7 under this spec. `reinstate_current_plan_candidate(...)` at `crates/worldwake-ai/src/agent_tick/observation.rs:357` still reconstructs `GoalOffer::evidence_entities` from `OpportunityAnchor` for candidate re-ranking — that is a distinct concern (restoring the ranked shape of a retained plan) and is not part of source-provenance preservation, so it stays as-is.

### D8. Scope of canonical source-backed opportunities

Launch scope for the normalized incident substrate includes source-backed acquisition opportunities that currently feed `SourceReliability`:

- `GoalKind::AcquireCommodity`
- `GoalKind::RestockCommodity`

using concrete sources surfaced through `GoalOffer::evidence_entities` and represented as `SourceKey` on `PlannedPlan.committed_source`.

This spec does not require immediate generalization to every target-backed goal. It does require the incident substrate and attribution function shape to accept new `OpportunityExpectationKind` variants without restructuring — future concrete expectation kinds (e.g., `ConcreteTargetPresence` for guard/hunt goals) can be added as new variants and new detection sites without a second normalization pass.

## Proposed Architecture

### A. Canonical AI-layer flow

1. Candidate generation/ranking produces a source-backed opportunity with concrete source evidence (`GoalOffer::evidence_entities`).
2. Adoption in `crates/worldwake-ai/src/agent_tick/planning.rs` stores `PlannedPlan.committed_source` (landed via `S124OPEXFAL-001`) and the new `OpportunityExpectationKind` on the runtime carrier.
3. AI-layer detection sites (`observation.rs:329`, `candidate_generation.rs:4172`, `planning.rs:341`) emit `OpportunityExpectationFailureIncident` values tagged with phase and cause.
4. The evolved `apply_source_reliability_failure_observations(...)` at `agent_tick/mod.rs:1904` applies attribution rules and writes to `SourceReliability`; when the committed source is among the applied failures, the caller chain records `Discrepancy::SourceInvalidated` via the frame module's sibling helper and clears the committed plan for replanning. Decision-event surfacing remains owned by D6.
5. Next-tick ranking consumes the updated `SourceReliability` component and discounts the failed source-backed opportunity.

### B. Canonical boundary

The AI-layer abstraction boundary is:

`PlannedPlan.committed_source + OpportunityExpectationKind -> OpportunityExpectationFailureIncident -> apply_source_reliability_failure_observations -> SourceReliability | DiscrepancyMemory | DecisionEventPayload`

Any future AI-layer feature in this family must either reuse that boundary or justify why it is not a source-backed opportunity expectation failure.

### C. Dual write paths, both lawful

`SourceReliability` is mutated by two distinct lawful paths, not unified:

- **AI-layer attribution path** (scope of this spec): `apply_source_reliability_failure_observations(...)` consumes normalized incidents from three detection sites within the agent's decision tick.
- **Systems-layer authoritative-action path** (unchanged by this spec): `record_failed_source_attempt(...)` and `record_successful_source_acquisition(...)` in `crates/worldwake-systems/src/experience_recording.rs` are called from `production_actions.rs:611, 732` (harvest commit / start failure) and `trade_actions.rs:394, 421` (trade acquisition / failure).

Both paths eventually mutate the same `SourceReliability` component. This is FND-26 compliant because the paths are non-overlapping in their triggering events: the AI path fires on belief/search contradictions *before* action attempts; the systems path fires on authoritative action outcomes. No direct cross-path call is introduced.

### D. Removed duplicate-path behavior

After S124, the following are not allowed as independent architectural paths:

- Detection sites within the AI layer must not each hold a private definition of "source failure" — all three must emit the same incident shape.
- Parallel AI-layer helpers that duplicate `apply_source_reliability_failure_observations` must not exist. The evolution is in-place.
- Retained-current-plan source recovery via helper reconstruction is already gone (`hydrate_reinstated_current_plan_source_entity` was removed by `S124OPEXFAL-001`); no equivalent shim may be re-introduced.

## Information-Path Analysis

1. World change occurs: a source is depleted or absent.
2. The acting agent reaches that contradiction through one of three AI-layer local paths:
   - co-located perception during the read phase (`observation.rs::pending_local_source_reliability_failures`),
   - candidate-generation-time belief-vs-observation mismatch (`candidate_generation.rs::emit_expectation_violation_candidates`, `SupplyDepleted` violation),
   - same-goal search from the agent's own belief state (`planning.rs::same_goal_search_failed_source_keys`).
3. The detecting site emits `OpportunityExpectationFailureIncident` tied to `PlannedPlan.committed_source` and the `OpportunityExpectationKind` carried on the retained plan.
4. `apply_source_reliability_failure_observations` applies attribution rules (D4), updates `SourceReliability`, and feeds the caller-side source-invalidation hook that records `Discrepancy::SourceInvalidated` plus committed-plan replanning where appropriate. `DecisionEventPayload` surfacing remains a separate D6 concern.
5. Ranking on a later tick reads `SourceReliability` via the existing belief-view accessor and discounts the failed source-backed opportunity.

Authoritative action outcomes reach `SourceReliability` through the parallel systems-layer path (`experience_recording.rs`); no AI-layer code queries that path directly, and no systems-layer code calls into AI-layer attribution. Both paths write the same component, and the next-tick ranking reads the resulting aggregate state.

No agent receives this information by oracle. No ranking code reaches backward into authoritative systems to reinterpret the failure.

## Positive-Feedback Analysis

There is one relevant loop:

- failed source expectation lowers trust in a source,
- lower trust increases chance of selecting a sibling source,
- sibling selection may expose new failures and create more learning.

This is a lawful learning loop, not a bug, but it needs dampening so agents do not thrash or globally blacklist commodities.

## Concrete Dampeners

1. `SourceReliability` is keyed to concrete `SourceKey` (`worldwake-core/src/experience.rs`), not commodity-wide class labels. Failure of one orchard does not poison all apples.
2. Existing memory retention and capacity limits are enforced by `SourceReliability::enforce_limits(...)` (called at `agent_tick/mod.rs:1935`), so stale aftermath physically ages out rather than accumulating.
3. Commitment still binds the agent until a concrete contradiction is produced; ranking alone does not continuously churn the current source.
4. Same-goal switching only occurs when a concrete sibling opportunity exists. The world must physically provide another source.
5. Duplicate incidents for the same `(opportunity, source, phase, tick)` are coalesced before persistence.

These are all world/state/process dampeners, not arbitrary caps.

## Stored State vs. Derived Read-Model List

Authoritative stored state:

- `SourceReliability` (ECS component, `worldwake-core/src/experience.rs`).
- `DiscrepancyMemory`, `BlockerMemory`, `RepairMemory`, `LearnedOpportunityMemory` (S109 typed discrepancy memories).
- `PlannedPlan.committed_source` (runtime agent state, persisted via save format v42 through `crates/worldwake-sim/src/save_load.rs`). The new `OpportunityExpectationKind` on the retained plan is also persisted as runtime agent state.
- `DecisionEventPayload` records (S110 decision-history events).

Runtime-derived only:

- `OpportunityExpectationFailureIncident` values produced and consumed within a single agent tick; never stored.
- Same-goal sibling comparison during candidate selection.
- Coalescing of repeated incidents within a tick.
- Trace summaries and explanation strings.

No derived ranking score or confidence summary becomes authoritative memory.

## SystemFn Integration

No new top-level simulation `SystemFn` is required.

This spec changes the AI tick pipeline only:

- plan adoption in `planning.rs` writes `OpportunityExpectationKind` alongside the already-landed `committed_source` on `PlannedPlan`,
- the three existing AI-layer detection sites emit `OpportunityExpectationFailureIncident` values instead of bare `SourceKey` sets,
- the evolved `apply_source_reliability_failure_observations(...)` runs from the existing call sites in `agent_tick/mod.rs` and `agent_tick/planning.rs` before later ranking consumes the aftermath.

The authoritative simulation remains the source of execution outcomes; S124 only changes how the acting agent interprets those outcomes within the AI tick.

## Component Registration

No new ECS component is introduced. `SourceReliability` already exists as the durable learning component. `OpportunityExpectationKind` is runtime agent state on `PlannedPlan` (persisted through the existing save format), not an ECS component, so the scenario contract in `docs/spec-drafting-rules.md` does not apply.

No new types are added to `worldwake-core`. All new runtime types live in `worldwake-ai/src/`.

## Acceptance Criteria

1. Exactly one AI-layer function (`apply_source_reliability_failure_observations`) mutates source-reliability memory from expectation contradiction; no parallel AI-layer helper is introduced. Systems-layer writers in `experience_recording.rs` remain the only writers of authoritative-action outcomes and are out of scope.
2. All three AI-layer detection sites (`observation.rs`, `candidate_generation.rs`, `planning.rs`) exchange `OpportunityExpectationFailureIncident` values with the writer, not bare `BTreeSet<SourceKey>`.
3. Same-goal sibling success records source failure against the current concrete source without rejecting the entire goal kind.
4. Observation-proven local absence/depletion (including the `SupplyDepleted` case from `candidate_generation.rs:4237-4250`) attributes to source reliability; precondition mismatches that are not observation-proven absence do not touch source reliability and continue to use the existing generic discrepancy path rather than the source-invalidation seam.
5. Retained-current-plan handling already consumes `PlannedPlan.committed_source` directly (delivered by `S124OPEXFAL-001`); this invariant must not regress.
6. Decision traces expose the failed source-backed expectation through `DecisionEventPayload` with enough detail to explain the switch or discount.
7. The design remains concrete, local, and belief-first, with no abstract confidence score layer or compatibility shim.

## Implementation Notes

- The preferred landing is an evolution of existing AI-layer infrastructure: extend `PlannedPlan` with the expectation-kind metadata, introduce the incident + enums in `worldwake-ai/src/`, and evolve `apply_source_reliability_failure_observations` in place. Do not introduce a parallel attribution helper.
- `reinstate_current_plan_candidate(...)` at `observation.rs:357` reconstructs `GoalOffer::evidence_entities` from `OpportunityAnchor` for candidate re-ranking. That is a distinct concern from source-provenance preservation and is not in S124's scope.
- If a naming choice conflicts with existing symbols, preserve the architectural boundary, not the draft names. In particular, the incident struct name may be shortened (e.g., `SourceExpectationFailure`) as long as all three detection sites exchange the same type.
- Save format already handles the committed_source field (v42). Adding `OpportunityExpectationKind` to `PlannedPlan` requires bumping the save format again; plan this bump as part of the D1 ticket, not as a separate deliverable.
- If future work proves a non-acquisition goal (e.g., `GoalKind::Attack` with `ConcreteTargetPresence`) needs the same substrate, extend `OpportunityExpectationKind` with a new variant and add a detection site. Do not generalize the expectation-kind enum until such a case is concrete.

## Outcome

Completed on 2026-04-23 through the archived ticket chain:

- `archive/tickets/S124OPEXFAL-001.md`
- `archive/tickets/S124CANOPPEXP-001.md`
- `archive/tickets/S124CANOPPEXP-002.md`
- `archive/tickets/S124CANOPPEXP-003.md`
- `archive/tickets/S124CANOPPEXP-004.md`

The landed implementation preserved this spec's canonical boundary: AI-layer detection sites now exchange one normalized expectation-failure incident shape, the AI-side attribution path records source-backed expectation failures through the decision-history surface, and source-reliability fallout remains distinct from the systems-layer authoritative action-outcome writers.

At decomposition time, D6 took the spec-permitted dedicated payload route rather than extending `ExpectationMismatch` in place. The shipped decision-history surface uses a dedicated `DecisionEventPayload::SourceExpectationFailure` variant with matching core event-tag support and observer rendering. That is within the explicit option set documented in D6.

The draft note that D1 would require a new persisted `OpportunityExpectationKind` on `PlannedPlan` did not survive reassessment as a necessary boundary. The truthful landed seam kept committed-source provenance on `PlannedPlan`, normalized runtime incidents in `worldwake-ai`, and recorded the contradiction through the canonical decision-history event family without widening the persisted retained-plan carrier beyond the already-landed committed-source field.

Verification recorded during the landing and closeout:

- `cargo test -p worldwake-core --lib decision_event_payload`
- `cargo test -p worldwake-ai --lib agent_tick::tests::apply_source_reliability_failure_observations_coalesces_duplicates_and_enforces_limits -- --exact`
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo fmt --all`

`./scripts/verify.sh` was not run as part of the archival handoff.
