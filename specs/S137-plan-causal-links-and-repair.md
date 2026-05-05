# S137: Plan Causal Links and Localized Repair Search

**Status**: Draft

## Summary

S114 (Plan Step Guards) added `PlanGuard` and `PlanExpectation` (`crates/worldwake-ai/src/plan_guard.rs:8`) — declarative per-step preconditions and expected post-conditions, classified into `Invalidator` variants on breach. S109 (Typed Discrepancy Taxonomy) added `RepairMemory`, `BlockerMemory`, `DiscrepancyMemory`, and `LearnedOpportunityMemory`. Together they tell the agent *what* broke and *whether* repair has been tried before, but they do not record *which step's effect* a later step depends on, nor do they actively *attempt repair* before discarding the plan and full-replanning.

When step 3 of a 7-step plan fails because the merchant moved, today's agent discards the entire plan, pushes a blocker, and replans from scratch. The route, the prerequisite acquisition (steps 1–2), and the post-purchase return-leg intent (steps 4–7) are abandoned. UCPOP-style causal links explicitly record "this step's precondition is supported by that earlier step's effect," and partial-order planners use those links to locate the smallest sub-plan that needs repair. S137 adopts the same idea while keeping total-order execution: each `PlanGuard` precondition gets a `provider` reference (the plan step or belief/observation/record/expectation that originated the supporting fact). When a guard breaks, the `PlanRepairContext` walks the broken link, identifies the smallest failing prefix, and runs a localized repair search bounded by the agent's existing expansion budget — bind a different target, replace the provider step with a sibling, insert a verification step, or escalate to full replan only if repair fails.

PR-15's typed-blocker clearing folds in here: each `Discrepancy` variant gets a per-variant `clearing_condition` (CommodityUnavailable clears on observed restock, NoKnownPath clears on consulted route record, CounterpartyRefused clears on relationship/price/threat shift) consumed by the repair search to decide whether the broken link is repairable in place or must be replanned around.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — extends `Discrepancy` (`discrepancy.rs:8`) variants with per-variant `clearing_condition: ClearingCondition`. Adds `ClearingCondition` enum naming the in-world signal that resets the discrepancy. Adds `CausalLink { provider: CausalProvider, fact: PlanningFact, consumer_step_index: u8, source_tick: Tick, confidence: Permille }` and `CausalProvider` enum.
- `worldwake-ai` — extends `PlanGuard` to carry `causal_links: SmallVec<CausalLink, 8>` per plan step. Adds `plan_repair` module owning `PlanRepairContext`, `RepairAttempt`, and the bounded repair search. `agent_tick/execution.rs` and `plan_revalidation.rs` route invalidator breaches through `attempt_repair_then_replan` instead of unconditional full-replan. Decision-trace adds `RepairAttemptTrace` and `EventTag::RepairApplied` (already exists, S110) carries the typed `RepairKind` that succeeded.
- `worldwake-sim` — no change. Repair search is internal to the AI crate.
- `worldwake-cli` — observer Section 3 (Decision History) renders the chosen `RepairKind` and the rejected alternatives.

## Dependencies

- S114 (Plan Step Guards) — completed. Provides `PlanGuard`, `PlanExpectation`, `Invalidator`, `ExpectationMismatchPayload`, and the revalidation seam. S137 extends `PlanGuard` with causal links.
- S109 (Typed Discrepancy Taxonomy) — completed. Provides `Discrepancy`, `BlockerMemory`, `RepairMemory`. S137 adds `clearing_condition` per variant and consumes `RepairMemory` to suppress repeat repair attempts.
- S110 (Decision History Events) — completed. `EventTag::RepairApplied` already exists; S137 populates its payload with the chosen `RepairKind`.
- S134 (Canonical Effect Schema) — completed and archived at `archive/specs/S134-canonical-effect-schema.md`. `RepairKind::ReplaceProvider` benefits from the queryable effect-schema surface when picking a replacement step.
- S138 (Affordance-to-Opportunity Compiler) — Phase 11 sibling. Soft dependency: with S138, `RepairKind::RebindTarget` can pick a sibling target the opportunity compiler surfaced; absent S138, rebind walks the agent's existing belief observations.
- S136 (Decision Event Payload Extension) — Phase 11 sibling. Soft dependency: `RepairAppliedPayload` benefits from `decisive_beliefs`/`decisive_records` populated by S136; if S136 lands first, the repair-attempt trace inherits the same fields.

## Design Goals

1. **Causal links record provenance, not state.** A `CausalLink` names the step or evidence whose effect supports a later step's precondition. The link itself is a per-tick derived structure attached to the active plan; persistence follows the plan's persistence (already in event log per S110), not a separate component.
2. **Localized repair before full replan.** Every guard breach first attempts `PlanRepairContext::attempt_repair`. Only if repair fails does the agent fall through to the existing full-replan path.
3. **Bounded repair search.** Repair runs under a fraction of the agent's `max_node_expansions` budget (default 1/4), capped at 4 `RepairKind` attempt classes. Repair never starves the planner.
4. **Typed `RepairKind` set.** Six kinds: `RebindTarget`, `ReplaceProvider`, `InsertVerification`, `SubstituteMethodBranch` (deferred to Phase 12 with HTN), `DowngradeToProgressBarrier`, `Abandon`. The first five preserve the prefix; the sixth surrenders.
5. **Per-variant `ClearingCondition`.** Each `Discrepancy` variant declares the world signal that resets it. `BlockerMemory` consumed by repair as a TTL-aware filter rather than only as a plain block.
6. **Repair memory feedback.** Successful repairs record `RepairMemory::Successful { kind, breach }`; failed repairs record `RepairMemory::Failed { kind, breach }`. The repair search reads this memory to skip recently-failed `RepairKind` variants for the same breach signature.
7. **Determinism.** Repair attempts run in a fixed `RepairKind` order; ties resolve by `BTreeMap`-stable iteration over candidate provider steps.
8. **No silent privilege.** Repair only writes to the same plan/belief/discrepancy state full replan would write to. No bypass of contention, locality, or S134's effect schema.

## Non-Goals

- **Cross-tick repair continuation.** Repair runs to completion within the same revalidation tick or returns failure. PR-14 (cross-tick search continuation) is rejected and not folded here.
- **HTN-method substitution.** `RepairKind::SubstituteMethodBranch` is named for the Phase 12 HTN spec but is not implemented in S137. The variant exists; its handler returns `RepairFailure::NoMethodSubstrate` until HTN methods land.
- **A new event tag.** `EventTag::RepairApplied` already exists (S110). S137 populates its payload, does not add new tags.
- **Repair without S114 guards.** Plan steps without a guard chain (any pre-S114 paths) bypass repair and fall through to full replan, exactly as today. S114 guard coverage is the prerequisite.
- **Speculative repair.** Repair only fires on actual breach. The planner does not pre-compute repair plans.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `CausalProvider` is a typed enum (PriorStep, Belief, Observation, Record, CarriedItem, OfficeRule, Expectation), never a numeric "support score." |
| FND-7 (Locality of Motion, Interaction, and Communication) | Repair searches over the same agent-local belief state full replan uses. No cross-agent reads. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | `CausalProvider::Belief`/`Observation`/`Record` carry the same provenance metadata the agent's belief store carries; repair does not bypass provenance. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | `CausalLink.confidence` records the agent's confidence in the supporting fact; low confidence biases repair toward `InsertVerification`. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Repair budget is bounded; failed repair falls through to bounded full replan. No unbounded reasoning. |
| FND-21 (Intentions Are Revisable Commitments) | Repair *is* the architectural shape of "monitoring assumptions and revising plans when assumptions break." Today's full-discard path approximates this poorly; S137 lands the proper shape. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | `RepairMemory::{Successful, Failed}` is concrete per-agent state with explicit acquisition (the breach), explicit content (the `RepairKind`), and existing decay path. |
| FND-29 (Debuggability Is a Product Feature) | Repair attempts and chosen `RepairKind` are surfaced in `EventTag::RepairApplied` and observer Section 3. "Why did this agent recover from the merchant moving?" becomes inspectable. |
| FND-29A (Causal History Is Authoritative, Append-Only, and Queryable) | Each repair emits an event; repair memory is per-agent state, not history rewriting. |

## Deliverables

### `worldwake-core::Discrepancy` extension

```rust
pub enum Discrepancy {
    // existing variants preserved (CommodityUnavailable, NoLegalBinding, NeedHorizonExceeded, ...)
}

impl Discrepancy {
    pub fn clearing_condition(&self) -> ClearingCondition {
        match self {
            Self::CommodityUnavailable { commodity, place } => 
                ClearingCondition::ObservedRestock { commodity, place },
            Self::NoLegalBinding { ... } => 
                ClearingCondition::OnEvidenceUpdate,
            Self::NeedHorizonExceeded { .. } => 
                ClearingCondition::OnNeedRecovered,
            // ... per-variant clearing rules
        }
    }
}

pub enum ClearingCondition {
    ObservedRestock { commodity: CommodityKey, place: EntityId },
    ObservedRouteOpen { from: EntityId, to: EntityId },
    OnEvidenceUpdate,
    OnNeedRecovered,
    OnRelationshipShift { other: EntityId },
    OnPriceShift { commodity: CommodityKey },
    OnThreatLifted { source: EntityId },
    TtlExpiry,                       // existing default
}
```

### `worldwake-core::CausalLink` (new type)

```rust
pub struct CausalLink {
    pub provider: CausalProvider,
    pub fact: PlanningFact,
    pub consumer_step_index: u8,
    pub source_tick: Tick,
    pub confidence: Permille,
}

pub enum CausalProvider {
    PriorStep { step_index: u8 },
    Belief { claim_key: BeliefClaimKey },
    Observation { observed_entity: EntityId, aspect: EntityBeliefAspect },
    Record { record_entity: EntityId, topic: RecordTopic },
    CarriedItem { item_lot: EntityId },
    OfficeRule { authority: AuthorityKind },
    Expectation { expectation_id: ExpectationId },
}
```

### `worldwake-ai::PlanGuard` extension

```rust
pub struct PlanGuard {
    pub required_facts: Vec<RequiredFact>,
    pub min_confidence: Permille,
    pub invalidators: Vec<InvalidatorTag>,
    pub causal_links: SmallVec<CausalLink, 8>,    // NEW
}
```

### `worldwake-ai::plan_repair` (new module)

```rust
pub struct PlanRepairContext<'a> {
    pub failed_step: u8,
    pub broken_link: CausalLink,
    pub preserved_prefix: &'a [PlanStep],
    pub reusable_suffix: &'a [PlanStep],
    pub new_evidence: &'a [BeliefRef],
    pub blocker: &'a Discrepancy,
}

pub enum RepairKind {
    RebindTarget,                // pick a sibling target satisfying the same shape
    ReplaceProvider,             // pick a different prior step satisfying the consumer's need
    InsertVerification,          // splice an AskWitness/InspectContainer step (S139) before the breach
    SubstituteMethodBranch,      // Phase 12: HTN method substitution; returns NoMethodSubstrate today
    DowngradeToProgressBarrier,  // accept partial progress and continue
    Abandon,                     // surrender — equivalent to today's full-discard path
}

pub enum RepairOutcome {
    Repaired { kind: RepairKind, new_plan: Plan },
    Failed { tried: SmallVec<(RepairKind, RepairFailure), 6> },
}

pub fn attempt_repair_then_replan(...) -> AgentTickAction { /* … */ }
```

### Revalidation routing

In `crates/worldwake-ai/src/agent_tick/execution.rs:140,222`, replace the unconditional full-replan path on `Invalidator` breach with:

```rust
match attempt_repair(&context) {
    RepairOutcome::Repaired { kind, new_plan } => {
        emit_event(EventTag::RepairApplied, RepairAppliedPayload { kind, ... });
        replace_plan(new_plan);
    }
    RepairOutcome::Failed { tried } => {
        record_repair_attempts(&tried);
        full_replan(...);  // existing path
    }
}
```

### Repair memory feedback

`RepairMemory` (already in `worldwake-core/src/repair_memory.rs:20`) gains `successful_kinds: BTreeMap<BreachSignature, BTreeSet<RepairKind>>` and `failed_kinds: BTreeMap<BreachSignature, BTreeSet<(RepairKind, Tick)>>`. The repair search consults `failed_kinds` to skip recently-failed kinds (TTL governed by existing `RepairMemory` decay).

### Observer Section 3 extension

Render `EventTag::RepairApplied` events with `kind`, `breach`, and the rejected `RepairKind` alternatives:
```
Tick 412 — Agent A — RepairApplied: ReplaceProvider
  breach: TargetMoved(Merchant=M3) at step 3
  rejected: RebindTarget (no sibling found), InsertVerification (recently failed t=380)
```

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** No new cross-agent path. Repair reads the agent's own belief store, repair memory, opportunity index (S138 if available), and authoritative plan state. `CausalLink.confidence` is derived from the existing belief-confidence machinery.
2. **Positive-feedback analysis.** No amplifying loop. Each repair attempt is bounded by budget; failed repairs become memory entries that *suppress*, never amplify.
3. **Concrete dampeners.** Bounded `max_node_expansions / 4` repair budget; capped 6 `RepairKind` attempt classes; `RepairMemory::failed_kinds` TTL prevents repeat thrashing.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state**: `RepairMemory.{successful_kinds, failed_kinds}` (per-agent), `Discrepancy::clearing_condition()` (per-variant constant function), `CausalLink` carried by the active plan in the existing plan-event payload.
   - **Derived read-model**: `PlanRepairContext` (per-revalidation transient), repair search frontier (per-attempt transient).

## SystemFn Integration

No new `SystemFn`. Repair runs within the existing `agent_tick` system at the revalidation seam.

## Component Registration

- `RepairMemory` — already registered (S109). Field additions migrate via the existing belief-store-diff path.
- `CausalLink` and `CausalProvider` — payload types on the existing plan event, not standalone components.
- `Discrepancy::clearing_condition()` — function-on-enum, not a component.

## Cross-System Interactions

- **AI ↔ AI internal**: repair calls the schema-backed planner effect evaluator (`apply_effects` / `apply_effects_with_context` in hypothetical mode), existing belief queries, and the existing opportunity index.
- **AI → Sim**: emit `EventTag::RepairApplied` through the existing event-log path.
- **Sim → CLI**: observer reads the event payload.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

`CognitiveProfile` gains `repair_budget_fraction: Permille` (default `pm(250)` — 25% of `max_node_expansions`). Per FND-22, repair budget is per-agent: a panicked, wounded agent might have lower repair budget; a rested strategist higher.

## Validation and Falsification

- **Golden coverage**: new `golden_plan_repair.rs` with five scenarios:
  1. Merchant-moved breach → expect `RepairKind::RebindTarget` selecting a sibling merchant; preserved prefix retained.
  2. Stale belief breach → expect `RepairKind::InsertVerification` splicing `AskWitness` (S139) before the broken step.
  3. Repeat-failed repair → expect `RepairKind::Abandon` after `RepairMemory::failed_kinds` records two prior failures within TTL.
  4. CommodityUnavailable with `ClearingCondition::ObservedRestock` met → expect blocker cleared structurally rather than via TTL.
  5. Repair budget exhaustion → expect fall-through to full replan with `RepairOutcome::Failed { tried: [...] }` recorded.
- **Plan-survival metric**: 1440-tick survival-baseline replay shows aggregate "plan steps preserved on breach" rate increases (target: ≥30% reduction in full-replan triggers vs pre-S137 baseline). Recorded in observer Section 3 summary.
- **No regression**: existing 1440-tick goldens (`survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron`) continue to pass without modification — repair is additive.

## Risks

- **Causal-link enumeration explosion.** Some plan steps depend on many prior facts; `SmallVec<CausalLink, 8>` may overflow for complex plans. Mitigation: only load-bearing causal links are recorded (the precondition-supporting subset, not the full transitive closure). Ticket-001 audits which precondition kinds genuinely require links.
- **Repair-memory pollution.** A scenario where a class of breach is unrepairable could fill `failed_kinds` and mask future repairs. Mitigation: TTL inherited from existing `RepairMemory` decay; per-`RepairKind` failure recording is signature-keyed not class-keyed.
- **Determinism under concurrent breaches.** Multiple guards can break in the same revalidation pass. Mitigation: repair attempts process breaches in `step_index` order; concurrent breaches at the same step process by `Invalidator` enum-discriminant order.
