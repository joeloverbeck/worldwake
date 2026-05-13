# S137: Plan Causal Links and Localized Repair Search

**Status**: Draft

## Summary

S114 (Plan Step Guards) added `PlanGuard`, `PlanExpectation`, and `Invalidator` (`crates/worldwake-ai/src/plan_guard.rs:8,35,52`) — declarative per-step preconditions and expected post-conditions, classified into `Invalidator` variants on breach. S109 (Typed Discrepancy Taxonomy) added `Discrepancy`, `DiscrepancyClearing`, `DiscrepancyMemory`, `RepairMemory`, `BlockerMemory`, and `LearnedOpportunityMemory`. Together they tell the agent *what* broke and *whether* a discrepancy has cleared, but they do not record *which step's effect* a later step depends on, nor do they actively *attempt repair* before the planner discards the active plan and re-searches from scratch.

Before D8 landed, the codebase classified replanned plans into a 4-variant `RepairKind` (`AlternateTarget`, `AlternateRoute`, `AlternateMerchant`, `AlternateRecipe`) and emitted `EventTag::RepairApplied` when the new plan differed from the failed plan along a recognized axis (`classify_accepted_repair` at `crates/worldwake-ai/src/agent_tick/planning.rs:1452-1526`). D8 migrated that surface to the 5-variant S137 set while preserving the same post-hoc classification role — the agent still pays the full-replan cost; the repair label is applied after the fact based on plan-shape diffing until the localized repair path lands.

S137 inserts *bounded localized repair before full replan*. UCPOP-style causal links explicitly record "this step's precondition is supported by that earlier step's effect," and partial-order planners use those links to locate the smallest sub-plan that needs repair. S137 adopts the same idea while keeping total-order execution: each `PlanGuard` precondition gets a `provider` reference (the plan step or belief/observation/record/expectation that originated the supporting fact). When a guard breaks, the `PlanRepairContext` walks the broken link, identifies the smallest failing prefix, and runs a localized repair search bounded by the agent's existing expansion budget — bind a different target, replace the provider step with a sibling, insert a verification step, or escalate to full replan only if repair fails.

PR-15's typed-blocker clearing folds in here: the repair search consults `DiscrepancyEntry.clearing_condition` (per-instance `DiscrepancyClearing` already populated when a discrepancy is recorded) to decide whether the broken link is repairable in place or must be replanned around. S137 does not introduce a parallel per-variant clearing template; the existing per-instance value is the authoritative form (FND-3).

`RepairKind` migrates from 4 post-hoc-classification variants to 5 search-axis variants: `RebindTarget` (rename of `AlternateTarget`, subsuming `AlternateMerchant` and `AlternateRecipe` since the alternative is carried in `RepairAppliedPayload.substitute_target`/`substitute_recipe`), `ReplaceProvider` (rename of `AlternateRoute`, generalized because route knowledge is a precondition provider), `InsertVerification` (new), `DowngradeToProgressBarrier` (new), and `Abandon` (new — equivalent to today's full-discard path). `SubstituteMethodBranch` is deferred to Phase 12 with HTN methods rather than added now as fossilized logic.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — extends `DiscrepancyClearing` (`crates/worldwake-core/src/discrepancy.rs:57-70`) with any new clearing variants the repair search requires beyond the existing five (`TtlExpiry`, `ReobservationOf`, `BeliefUpdate`, `CommodityAvailabilityChanged`, `WorldStructureChange`). Adds `CausalLink`, `CausalProvider`, `PlanningFact`, `RecordTopic`, and `BreachSignature` types. D8 migrated `RepairKind` (`crates/worldwake-core/src/decision_event_payload.rs`) from 4 post-hoc-classification variants to 5 search-axis variants. Migrates `RepairMemory` (`crates/worldwake-core/src/repair_memory.rs:19-22`) from `BTreeMap<RepairKey, RepairEntry>` to `BTreeMap<BreachSignature, RepairEntry>` with discriminated success/failure recording. D8 extended `RepairAppliedPayload` with a `substitute_recipe: Option<RecipeId>` field so the rebinding alternative is preserved through the variant merge. Extends `CognitiveProfile` (`crates/worldwake-core/src/cognitive_profile.rs:23-114`) with `repair_budget_fraction: Permille` and `causal_links_per_step_cap: u8`, both with `#[serde(default)]` for scenario/text-serde omission and updated `Default` impl. Because persisted runtime/profile shape changes are current-format-only, tickets 002, 003, 004, and 005 bumped `SAVE_FORMAT_VERSION` `79→80→81→82→83`; later S137 persisted-shape tickets continue the version chain.
- `worldwake-ai` — extends `PlanGuard` (`crates/worldwake-ai/src/plan_guard.rs:8`) to carry `causal_links: Vec<CausalLink>` per plan step (capped by `CognitiveProfile.causal_links_per_step_cap`). Adds `plan_repair` module owning `PlanRepairContext`, `RepairAttempt`, and the bounded repair search. `agent_tick/execution.rs:90-146` and `plan_revalidation.rs` route invalidator breaches through `attempt_repair_then_replan` instead of falling straight through to `handle_current_step_failure`. The existing post-hoc `classify_accepted_repair` at `planning.rs:1452-1526` is preserved as the fall-through case for the full-replan branch but is no longer the primary repair surface. The existing `pending_repair_context` on `AgentDecisionRuntime` is reused by the new pre-failure repair path. `decision_trace.rs` adds `RepairAttemptTrace`.
- `worldwake-sim` — no repair-search behavior change. S137 persisted-shape tickets update `SAVE_FORMAT_VERSION` as runtime/profile payloads change; repair search itself is internal to the AI crate, and calls to `apply_effects_with_context` (`crates/worldwake-sim/src/effect_schema.rs`) use the existing shared-service surface (FND-26).
- `worldwake-cli` — observer Section 3b (`crates/worldwake-cli/src/bin/observer.rs:828` `render_decision_history_section`) renders the chosen `RepairKind` and the rejected alternatives. Migration of the 20 `RepairKind::*` call sites includes observer and scenario test references.

## Dependencies

- S114 (Plan Step Guards) — completed and archived at `archive/specs/S114-plan-step-guards.md`. Provides `PlanGuard`, `PlanExpectation`, `Invalidator`, `InvalidatorTag`, `ExpectationMismatchPayload`, and the revalidation seam. S137 extends `PlanGuard` with causal links.
- S109 (Typed Discrepancy Taxonomy) — completed. Provides `Discrepancy`, `DiscrepancyClearing`, `DiscrepancyMemory`, `BlockerMemory`, `RepairMemory`. S137 consumes existing `DiscrepancyEntry.clearing_condition` and migrates `RepairMemory` shape.
- S110 (Decision History Events) — completed. `EventTag::RepairApplied` already exists; S137 widens the existing `RepairAppliedPayload` (adds `substitute_recipe: Option<RecipeId>`) and changes which code path emits it.
- S134 (Canonical Effect Schema) — completed and archived at `archive/specs/S134-canonical-effect-schema.md`. `RepairKind::ReplaceProvider` benefits from the queryable effect-schema surface (`apply_effects_with_context`) when picking a replacement step.
- S136 (Decision Event Payload Extension) — completed and archived at `archive/specs/S136-decision-event-payload-extension.md`. `RepairAttemptTrace` reuses S136's representable `decisive_*` ref families; record refs should be populated only when the repair emission seam carries a lawful record entity.
- S138 (Affordance-to-Opportunity Compiler) — completed and archived at `archive/specs/S138-opportunity-compiler.md`. Soft dependency satisfied: `RepairKind::RebindTarget` reads the opportunity index this spec produced when picking a sibling target.
- S139 (Epistemic Sensing Subgoals) — Phase 11 sibling draft (`specs/S139-epistemic-sensing-subgoals.md`). Soft dependency in the inverse direction: `RepairKind::InsertVerification` splices `AskWitness`/`InspectContainer` goals as repair steps. Order-independent: without S139, `InsertVerification` returns `RepairFailure::NoEpistemicSubstrate` and the search falls through to the next `RepairKind`.

## Design Goals

1. **Causal links record provenance, not state.** A `CausalLink` names the step or evidence whose effect supports a later step's precondition. The link itself is a per-tick derived structure attached to the active plan; persistence is in runtime state (`AgentDecisionRuntime.current_plan.steps[i].guard.causal_links`), not the event log. Event-log records of repair attempts (`RepairAppliedPayload`) reference causal links by `BreachSignature`, not by embedding the link payload.
2. **Localized repair before full replan.** Every guard breach first attempts `PlanRepairContext::attempt_repair`. Only if repair fails does the agent fall through to the existing full-replan path (`handle_current_step_failure`) and the post-hoc `classify_accepted_repair` classification.
3. **Bounded repair search.** Repair runs under a fraction of the agent's `CognitiveProfile.max_node_expansions` budget (default `repair_budget_fraction = Permille::new_unchecked(250)`, i.e., 25%), capped at 5 `RepairKind` attempt classes. Repair never starves the planner.
4. **Typed `RepairKind` set (5 variants).** `RebindTarget`, `ReplaceProvider`, `InsertVerification`, `DowngradeToProgressBarrier`, `Abandon`. All five preserve a non-trivial prefix except `Abandon`, which surrenders. `SubstituteMethodBranch` is deferred to Phase 12 with HTN methods; not introduced in S137. The migration absorbs the existing 4 variants per the mapping in Deliverables D8.
5. **Single-truth clearing condition.** The repair search consults `DiscrepancyEntry.clearing_condition` (the existing per-instance `DiscrepancyClearing` value populated at discrepancy-record time) to decide whether the broken link is repairable in place or must be replanned around. No per-variant constant template is added — that would create dual representation (FND-28).
6. **Repair memory feedback (single-key migration).** `RepairMemory` migrates from `BTreeMap<RepairKey, RepairEntry>` to `BTreeMap<BreachSignature, RepairEntry>`, where `BreachSignature` captures the invalidator tag, step target, and goal key. `RepairEntry` gains direct discriminators `kind: RepairKind` and `succeeded: bool`. The repair search consults this memory to skip recently-failed `RepairKind` variants for the same breach signature. The migration is single-truth; no parallel `successful_kinds`/`failed_kinds` collections coexist with the existing `repairs` field.
7. **Determinism.** Repair attempts run in a fixed `RepairKind` order (declared `Ord` derive sequence); ties resolve by `BTreeMap`-stable iteration over candidate provider steps. `BTreeMap`/`BTreeSet` only in authoritative state per CLAUDE.md determinism invariant.
8. **No silent privilege.** Repair only writes to the same plan/belief/discrepancy state full replan would write to. Repair uses `apply_effects_with_context` (worldwake-sim shared service, FND-26-allowed) for hypothetical effect evaluation; no bypass of contention, locality, or S134's effect schema.

## Non-Goals

- **Cross-tick repair continuation.** Repair runs to completion within the same revalidation tick or returns failure. PR-14 (cross-tick search continuation) is rejected and not folded here.
- **HTN-method substitution.** `SubstituteMethodBranch` is deferred to Phase 12 entirely. The variant is not introduced in S137 (FND-28 — no fossilized logic in live authority paths).
- **A new event tag.** `EventTag::RepairApplied` already exists (S110). S137 changes which code path emits it and widens `RepairAppliedPayload` by one field; it does not add new tags.
- **Repair without S114 guards.** Plan steps without a guard chain (pre-S114 paths, if any remain) bypass repair and fall through to full replan, exactly as today. S114 guard coverage is the prerequisite.
- **Speculative repair.** Repair only fires on actual breach. The planner does not pre-compute repair plans.
- **Discarding the existing 4-variant `RepairKind` without migration.** The migration enumerated in Deliverables D8 is mandatory; the 20 call sites of the existing variants must be updated atomically with the rename + subsumption. No deprecated wrappers (FND-28).
- **Per-variant `Discrepancy::clearing_condition()` template.** The existing per-instance `DiscrepancyClearing` is authoritative (FND-3). No per-variant constant function is added.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `CausalProvider` is a typed enum (`PriorStep`, `Belief`, `Observation`, `Record`, `CarriedItem`, `Expectation`), never a numeric "support score." `DiscrepancyClearing` remains per-instance authoritative; no per-variant derived constant promotes itself to truth. |
| FND-7 (Locality of Motion, Interaction, and Communication) | Repair searches over the same agent-local belief state full replan uses. No cross-agent reads. |
| FND-14 (World State Is Not Belief State) | `CausalProvider::Belief`/`Observation`/`Record` reference belief-store-backed claims, not authoritative world state. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent; Social Facts Are Not) | `CausalProvider::Observation { observed_entity, aspect }` is valid only when `aspect` is a physically perceivable property (kind, item-lot commodity, workstation tag, resource availability) on a co-located entity. Social/relational facts (ownership, rights, jurisdiction) always require `CausalProvider::Belief` provenance, even on co-located subjects. The `OfficeRule` variant is *not* included in `CausalProvider` because no concrete local-acquisition path exists for office authority without going through `Record` or `Belief`. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | `CausalProvider::Belief`/`Observation`/`Record` carry the same provenance metadata the agent's belief store carries; repair does not bypass provenance. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | `CausalLink.confidence` records the agent's confidence in the supporting fact; low confidence biases repair toward `InsertVerification`. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Repair budget is bounded by `repair_budget_fraction × max_node_expansions`; failed repair falls through to bounded full replan. No unbounded reasoning. |
| FND-21 (Intentions Are Revisable Commitments) | Repair *is* the architectural shape of "monitoring assumptions and revising plans when assumptions break." Today's post-hoc classification approximates this; S137 lands the proper pre-failure shape. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | `RepairMemory.repairs` (post-migration) is concrete per-agent state with explicit acquisition (the breach), explicit content (the `RepairKind` and outcome), and existing decay path via `expires_tick`. |
| FND-26 (Systems Interact Through State) | Repair calls `apply_effects_with_context` (worldwake-sim shared service) — allowed shared-domain computation, not a privileged cross-system call. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | `RepairKind` migration is single-pass: existing 4 variants are renamed/subsumed atomically across all 20 call sites; no deprecated wrappers remain. `RepairMemory` shape is migrated, not augmented. `DiscrepancyClearing` is reused, not duplicated. `SubstituteMethodBranch` is deferred entirely, not added as a stub. |
| FND-29 (Debuggability Is a Product Feature) | Repair attempts and chosen `RepairKind` are surfaced in `EventTag::RepairApplied` and observer Section 3b. "Why did this agent recover from the merchant moving?" becomes inspectable. |
| FND-29A (Causal History Is Authoritative, Append-Only, and Queryable) | Each repair attempt emits an event; repair memory is per-agent state, not history rewriting. |

## Deliverables

### D1. `worldwake-core::DiscrepancyClearing` extension (conditional)

The repair search consumes `DiscrepancyEntry.clearing_condition: DiscrepancyClearing` directly (`crates/worldwake-core/src/discrepancy.rs:53`). Audit the five existing variants (`TtlExpiry`, `ReobservationOf`, `BeliefUpdate`, `CommodityAvailabilityChanged`, `WorldStructureChange`) against the breach-recovery signals the search needs. If a recovery signal cannot be expressed by the existing five, extend the enum with the missing variant — likely candidates: `OnRelationshipShift { other: EntityId }`, `OnPriceShift { commodity: CommodityKind, place: EntityId }`, `OnThreatLifted { source: EntityId }`, `OnNeedRecovered { need: HomeostaticNeedId }`. Variants must derive `Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize` per the existing enum derives. No new `Discrepancy::clearing_condition()` method is added; the per-instance value remains authoritative.

### D2. `worldwake-core::CausalLink` (new type)

```rust
pub struct CausalLink {
    pub provider: CausalProvider,
    pub fact: PlanningFact,
    pub consumer_step_index: u16,
    pub source_tick: Tick,
    pub confidence: Permille,
}

pub enum CausalProvider {
    PriorStep { step_index: u16 },
    Belief { claim_key: BeliefClaimKey },
    Observation { observed_entity: EntityId, aspect: EntityBeliefAspect },
    Record { record_entity: EntityId, topic: RecordTopic },
    CarriedItem { item_lot: EntityId },
    Expectation { expectation_id: ExpectationId },
}
```

Step indices use `u16` to match `ExpectationMismatchPayload.step_index`, `PlanInvalidationReason::ExpectationMismatch.step_index`, and `RepairAppliedPayload.step_index`. Derives: `Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize`. `OfficeRule` is omitted from `CausalProvider` per FND-14A — office authority enters the agent's reasoning through `Record` (the record-entity carrying the rule) or `Belief` (the agent's belief about an institutional claim), never as a discriminant-only authority reference.

`CausalLink`s live on `PlanGuard.causal_links` in runtime state (`AgentDecisionRuntime.current_plan.steps[i].guard`). They are not persisted in the event log. `RepairAppliedPayload` references the breach via `BreachSignature` (see D9) rather than embedding the broken `CausalLink` payload.

### D3. `worldwake-core::PlanningFact` and `RecordTopic` (new types)

```rust
pub enum PlanningFact {
    TargetPresent { target: EntityId, at_place: EntityId },
    CommodityAvailable { place: EntityId, kind: CommodityKind, min_quantity: Quantity },
    RouteKnown { from: EntityId, to: EntityId },
    ResourceAccess { resource: EntityId, agent_holds_permission: bool },
}

pub enum RecordTopic {
    PriceObserved { commodity: CommodityKind },
    RouteSafety,
    OfficeRule { office: EntityId },
    BountyExists,
    TestifiedAbout { subject: EntityId },
}
```

`PlanningFact` mirrors `RequiredFact` (`crates/worldwake-ai/src/plan_guard.rs:14-33`) but lives in core because `CausalLink` is core-resident. Consider whether `RequiredFact` should relocate to core and be aliased to `PlanningFact`; if so, this is a unification deliverable rather than a sibling type. Decision deferred to ticket-decomposition time. Both types derive `Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize`.

### D4. `worldwake-ai::PlanGuard` extension

```rust
pub struct PlanGuard {
    pub required_facts: Vec<RequiredFact>,
    pub min_confidence: Permille,
    pub invalidators: Vec<Invalidator>,
    #[serde(default)]
    pub causal_links: Vec<CausalLink>,    // NEW; capped at runtime by CognitiveProfile.causal_links_per_step_cap
}
```

`Vec<CausalLink>` per the workspace-external-dependency rule (smallvec is not a workspace dependency). The cap is enforced at construction time by the planner emitter and validated at revalidation time. Note the distinction in prose: `GuardTemplateSpec.invalidators: Vec<InvalidatorTag>` is the action-definition registration surface; `PlanGuard.invalidators: Vec<Invalidator>` stores the AI-crate payload-carrying breach-detection surface raised when revalidation finds a mismatch.

### D5. `worldwake-ai::plan_repair` (new module)

```rust
pub struct PlanRepairContext<'a> {
    pub failed_step: u16,
    pub broken_link: CausalLink,
    pub preserved_prefix: &'a [PlannedStep],
    pub reusable_suffix: &'a [PlannedStep],
    pub new_evidence: &'a [BeliefRef],
    pub discrepancy_entry: &'a DiscrepancyEntry,    // includes per-instance clearing_condition
}

pub enum RepairKind {
    RebindTarget,                // pick a sibling target satisfying the same shape (subsumes legacy AlternateTarget / AlternateMerchant / AlternateRecipe)
    ReplaceProvider,             // pick a different prior step satisfying the consumer's need (subsumes legacy AlternateRoute since route knowledge is a precondition provider)
    InsertVerification,          // splice an AskWitness/InspectContainer step (S139) before the breach
    DowngradeToProgressBarrier,  // accept partial progress and continue
    Abandon,                     // surrender — equivalent to today's full-discard path
}

pub enum RepairOutcome {
    Repaired { kind: RepairKind, new_plan: PlannedPlan },
    Failed { tried: Vec<(RepairKind, RepairFailure)> },    // capped by repair_budget; ordered by RepairKind Ord
}

pub fn attempt_repair_then_replan(...) -> AgentTickAction { /* … */ }
```

The new module reuses `AgentDecisionRuntime.pending_repair_context` rather than introducing a parallel field. The existing post-hoc `classify_accepted_repair` (`planning.rs:1452-1526`) remains as the fall-through path that runs after `RepairOutcome::Failed` triggers full replan; D8's migration updates its variant mapping but does not delete the function.

`PlannedPlan` and `PlannedStep` are the existing types at `crates/worldwake-ai/src/planner_ops.rs:256-307`.

### D6. Revalidation routing

In `crates/worldwake-ai/src/agent_tick/execution.rs:90-146`, replace the unconditional fall-through to `handle_current_step_failure` on `RevalidationOutcome::Invalidated` with:

```rust
match attempt_repair_then_replan(&context) {
    RepairOutcome::Repaired { kind, new_plan } => {
        emit_event(EventTag::RepairApplied, RepairAppliedPayload {
            agent, goal_key, step_index, repair_kind: kind,
            substitute_target, substitute_recipe,
        });
        replace_plan(new_plan);
    }
    RepairOutcome::Failed { tried } => {
        record_repair_attempts(&tried);
        // existing path: handle_current_step_failure → replan → classify_accepted_repair
        handle_current_step_failure(...);
    }
}
```

The Authoritative-to-AI Impact Rule applies (see Section "Authoritative-to-AI Checklist" below).

### D7. Repair memory migration

`RepairMemory` migrates from `BTreeMap<RepairKey, RepairEntry>` (`crates/worldwake-core/src/repair_memory.rs:19-22`) to `BTreeMap<BreachSignature, RepairEntry>`:

```rust
pub struct RepairMemory {
    pub repairs: BTreeMap<BreachSignature, RepairEntry>,
}

pub struct RepairEntry {
    pub signature: BreachSignature,
    pub kind: RepairKind,
    pub succeeded: bool,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub success_count: u32,    // preserved from existing entry for FND-22A compatibility
}
```

`RepairKey` is removed; the legacy `(goal_key, alternate_target)` keying is absorbed into `BreachSignature` (see D9). The repair search consults `repairs.get(&signature)` to skip recently-failed `RepairKind` for the same breach (TTL governed by `CognitiveProfile.repair_memory_ticks`). Migration is single-truth — no parallel `successful_kinds`/`failed_kinds` maps coexist with `repairs`.

### D8. `RepairKind` migration across call sites

Variant mapping:

| Legacy variant | New variant | Notes |
|----------------|-------------|-------|
| `AlternateTarget` | `RebindTarget` | rename; `substitute_target` field unchanged |
| `AlternateMerchant` | `RebindTarget` | subsumed; `substitute_target` carries the merchant entity |
| `AlternateRecipe` | `RebindTarget` | subsumed; new `RepairAppliedPayload.substitute_recipe: Option<RecipeId>` field preserves the alternative |
| `AlternateRoute` | `ReplaceProvider` | rename; route knowledge is a precondition provider |
| (no analog) | `InsertVerification` | new |
| (no analog) | `DowngradeToProgressBarrier` | new |
| (no analog) | `Abandon` | new — equivalent to today's full-discard path |

Call sites to migrate (20):
- `crates/worldwake-core/src/decision_event_payload.rs:418-423` — enum definition
- `crates/worldwake-core/src/decision_event_payload.rs:705` — test construction
- `crates/worldwake-ai/src/agent_tick/planning.rs:1481` (AlternateRecipe)
- `crates/worldwake-ai/src/agent_tick/planning.rs:1495` (AlternateMerchant)
- `crates/worldwake-ai/src/agent_tick/planning.rs:1505` (AlternateTarget)
- `crates/worldwake-ai/src/agent_tick/planning.rs:1520` (AlternateRoute)
- `crates/worldwake-ai/src/agent_tick/planning.rs:3392,3456,3498` — tests
- `crates/worldwake-ai/src/agent_tick/mod.rs:2375` (AlternateTarget)
- `crates/worldwake-ai/src/decision_runtime.rs:653` (AlternateMerchant)
- `crates/worldwake-ai/src/agent_tick/tests.rs:8882,8926,8946,8962,8977,8993,9006,9022` — tests
- `crates/worldwake-sim/src/save_load.rs:1157` (AlternateMerchant)
- `crates/worldwake-cli/src/bin/observer.rs:5477` (AlternateMerchant)

Migration is atomic with the variant rename — no compatibility wrappers (FND-28). `classify_accepted_repair` (`planning.rs:1452-1526`) updates its mapping to emit `RebindTarget` for all three legacy variants and `ReplaceProvider` for `AlternateRoute`. `substitute_recipe` is populated when the legacy `AlternateRecipe` branch fires.

`RepairAppliedPayload` widens by one field:

```rust
pub struct RepairAppliedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub repair_kind: RepairKind,
    pub substitute_target: Option<EntityId>,
    pub substitute_recipe: Option<RecipeId>,    // NEW; #[serde(default)] for omitted-field serde payloads; save files still version-bump
}
```

### D9. `BreachSignature` type (new)

```rust
pub struct BreachSignature {
    pub goal_key: GoalKey,
    pub invalidator: InvalidatorTag,
    pub step_target: Option<EntityId>,
}
```

Lives in `crates/worldwake-core/src/repair_memory.rs` adjacent to `RepairMemory`. Derives `Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize`. Replaces `RepairKey` in `RepairMemory.repairs` (D7).

### D10. `worldwake-ai::decision_trace` extension

Add `RepairAttemptTrace` to `crates/worldwake-ai/src/decision_trace.rs` carrying the chosen `RepairKind`, the breach signature, the rejected `RepairKind` set with per-kind `RepairFailure` reason, and the budget consumed. Compose with existing `AgentDecisionTrace` per the `PortfolioSlotTrace`/`PlanAttemptTrace` precedent (lines ~80-110). Sink installation follows the existing `DecisionTraceSink` pattern.

### D11. `CognitiveProfile` extension

`crates/worldwake-core/src/cognitive_profile.rs` gains two fields:

```rust
pub struct CognitiveProfile {
    // ... existing fields ...
    #[serde(default = "default_repair_budget_fraction")]
    pub repair_budget_fraction: Permille,    // default Permille::new_unchecked(250) = 25%
    #[serde(default = "default_causal_links_per_step_cap")]
    pub causal_links_per_step_cap: u8,       // default 8
}

fn default_repair_budget_fraction() -> Permille { Permille::new_unchecked(250) }
fn default_causal_links_per_step_cap() -> u8 { 8 }
```

`Default` impl at `cognitive_profile.rs:117-155` is updated. AgentDef at `crates/worldwake-cli/src/scenario/types.rs:593` carries `cognitive_profile: Option<CognitiveProfile>` and continues to deserialize existing scenarios cleanly because both new fields have `#[serde(default)]`. Saved bincode state is not back-compatible; ticket 002 bumps the current save format to `80` so pre-80 saves remain rejected at the version gate. Per FND-22, repair budget is per-agent: a panicked, wounded agent might have lower repair budget; a rested strategist higher.

### D12. Observer Section 3b extension

`crates/worldwake-cli/src/bin/observer.rs:828` `render_decision_history_section` renders `EventTag::RepairApplied` events with `repair_kind`, the new `substitute_recipe` field, and the breach signature. Rejected `RepairKind` alternatives are read from the new `RepairAttemptTrace` (D10) when the trace sink is installed:

```
Tick 412 — Agent A — RepairApplied: ReplaceProvider
  breach: Invalidator::TargetMoved(target=M3) at step 3
  substitute_target: None
  substitute_recipe: None
  rejected: RebindTarget (no sibling found), InsertVerification (recently failed t=380)
```

The format follows the existing decision-history-table convention with multi-line detail rendering matching analogous events.

## FND-01 Section H — Causal Hooks Declaration (abbreviated)

S137 is a (b) system extension + (a) new-type hybrid. Section H covers only the new declarations; items inherited from S114 (guards), S109 (discrepancy taxonomy), and S110 (event tags) are not restated.

1. **Information-path analysis.** No new cross-agent path. Repair reads the agent's own belief store, repair memory, opportunity index (S138), discrepancy memory, and authoritative plan state. `CausalLink.confidence` is derived from the existing belief-confidence machinery. `CausalProvider::Record` and `CausalProvider::Observation` inherit FND-14A and FND-15 information-path semantics from S109 and the belief-view layer.
2. **Positive-feedback analysis.** No amplifying loop. Each repair attempt is bounded by `repair_budget_fraction × max_node_expansions`; failed repairs become memory entries that *suppress*, never amplify.
3. **Concrete dampeners.** Bounded `repair_budget_fraction × max_node_expansions` repair budget; capped 5 `RepairKind` attempt classes (4 with `Abandon` excluded from search-axis count); `RepairMemory.repairs[signature].expires_tick` TTL prevents repeat thrashing.
4. **Stored authoritative state vs. derived read-model.**
   - **Stored authoritative state**: `RepairMemory.repairs` keyed by `BreachSignature` (per-agent), `DiscrepancyEntry.clearing_condition` (existing per-instance), `CausalLink` carried by `PlannedStep.guard.causal_links` in `AgentDecisionRuntime.current_plan`.
   - **Derived read-model**: `PlanRepairContext` (per-revalidation transient), repair search frontier (per-attempt transient), `RepairAttemptTrace` (per-attempt diagnostic surface).

## SystemFn Integration

No new `SystemFn`. Repair runs within the existing `agent_tick` system at the revalidation seam (`agent_tick/execution.rs:90-146`).

## Component Registration

- `RepairMemory` — already registered (S109) at `crates/worldwake-core/src/component_schema.rs:784-802`. Field-shape migration (D7) preserves the registration; bincode-roundtrip tests in `repair_memory.rs:97-115` update to the new shape.
- `CognitiveProfile` — already registered at `component_schema.rs:1009-1027`. New fields (D11) deserialize cleanly from scenario/text-serde inputs via `#[serde(default)]`; save/load uses the current-format version bump rather than a legacy bincode shim.
- `CausalLink`, `CausalProvider`, `PlanningFact`, `RecordTopic`, `BreachSignature` — payload types on existing components and events, not standalone components.
- `DiscrepancyClearing` extension (D1, conditional) — variant addition to an existing enum; no new registration.

## Cross-System Interactions

- **AI ↔ AI internal**: repair calls `apply_effects_with_context` (`crates/worldwake-sim/src/effect_schema.rs`) in hypothetical mode for effect evaluation, the existing belief-view queries, the existing opportunity index, and `DiscrepancyMemory::is_suppressed` / `clear_by_condition`.
- **AI → Sim**: emit `EventTag::RepairApplied` through the existing event-log path. Widen `RepairAppliedPayload` per D8.
- **Sim → CLI**: observer reads the event payload at `observer.rs:828`.

No direct cross-system calls (FND-26). `apply_effects_with_context` is a shared-domain service.

## Profile-Driven Parameters

`CognitiveProfile` gains:
- `repair_budget_fraction: Permille` — default `Permille::new_unchecked(250)` (25% of `max_node_expansions`).
- `causal_links_per_step_cap: u8` — default 8 (cap on `PlanGuard.causal_links` length).

Both have `#[serde(default)]` for scenario/text-serde omitted-field tolerance. Because the profile is persisted component state, save/replay compatibility is handled by advancing `SAVE_FORMAT_VERSION` and rejecting older versions rather than adding a migration shim. Per FND-22, both are per-agent: an attention-impaired agent might have a smaller causal-link cap (records fewer dependencies, repairs less precisely); a rested strategist may have a larger budget.

## Authoritative-to-AI Checklist

Per CLAUDE.md's Authoritative-to-AI Impact Rule (D6 modifies revalidation routing):

1. `get_affordances` — N/A (no affordance-query changes).
2. `generate_candidates` — N/A (no new `GoalKind`).
3. `search_plan` — **applies**: the repair search shares terminal-ordering and barrier logic with `search_plan` but operates over the preserved-prefix subgraph. Repair-search budget is `repair_budget_fraction × max_node_expansions`; full `search_plan` budget is unchanged. Goldens must cover the case where repair budget exhausts and falls through to full replan.
4. `BestEffort` action start — N/A.
5. `handle_plan_failure` — **applies**: D6 inserts `attempt_repair_then_replan` before `handle_current_step_failure`. The latter remains the fall-through. Decision-trace surfaces both paths.
6. Payload revalidation — **applies for `RebindTarget`**: when repair synthesizes a new payload (different target entity), the action handler's `with_payload_override_validator` must accept it. Audit affected handlers (travel, trade, harvest, craft) and confirm validators handle the rebinding case. Document the audit as a sub-deliverable under D5.
7. Golden tests — see Validation section.

## Validation and Falsification

- **Golden coverage**: new `golden_plan_repair.rs` under `crates/worldwake-ai/tests/` with five scenarios:
  1. Merchant-moved breach → expect `RepairKind::RebindTarget` selecting a sibling merchant; preserved prefix retained.
  2. Stale belief breach → expect `RepairKind::InsertVerification` splicing `AskWitness` (S139, soft-dep — if S139 is unimplemented at golden-write time, this scenario asserts `RepairFailure::NoEpistemicSubstrate` and the search falls to the next kind).
  3. Repeat-failed repair → expect `RepairKind::Abandon` after `RepairMemory.repairs[signature].succeeded == false` records exist within TTL for the same breach.
  4. Discrepancy entry with `DiscrepancyClearing::CommodityAvailabilityChanged` cleared → expect blocker cleared structurally rather than via TTL.
  5. Repair budget exhaustion → expect fall-through to full replan with `RepairOutcome::Failed { tried: [...] }` recorded and `classify_accepted_repair` running on the resulting plan.
- **Plan-survival metric (Phase 11 gate)**: `survival-baseline.ron` 1440-tick replay shows ≥30% reduction in `EventTag::ReplanTriggered` count compared to a pre-S137 baseline run captured on the same scenario and seed. Define the baseline as the count of `ReplanTriggered` events emitted by `handle_current_step_failure` in the current `main` branch immediately before S137 lands; record the absolute number in the golden test fixture for reproducibility. The corresponding rise in `EventTag::RepairApplied` events should approximately balance the reduction (within ±10%).
- **No regression**: existing 1440-tick goldens (`survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron` under `scenarios/`) continue to pass without modification — repair is additive at the agent-tick level and the migration is semantics-preserving for the existing 4 `RepairKind` variants.

## Risks

- **Causal-link enumeration cap.** `Vec<CausalLink>` capped by `causal_links_per_step_cap` may truncate links for complex plans. Mitigation: only load-bearing causal links are recorded (the precondition-supporting subset, not the full transitive closure). Ticket-001 audits which `RequiredFact` kinds genuinely require links. If the cap is hit, the planner emits a `DecisionTrace::CausalLinkCapHit` annotation so debuggers see the truncation; the surviving links are the most recent ones (BTreeMap-stable order).
- **Repair-memory pollution.** A class of breach that is structurally unrepairable could fill `repairs` and mask future repairs. Mitigation: per-`BreachSignature` keying (D9) bounds collisions; TTL via `CognitiveProfile.repair_memory_ticks` decays entries; `enforce_capacity` (existing `MemoryCapacityProfile` path) evicts oldest.
- **Determinism under concurrent breaches.** Multiple guards can break in the same revalidation pass. Mitigation: repair attempts process breaches in `step_index` order; concurrent breaches at the same step process by `Invalidator` enum-discriminant order. Both orderings are deterministic given the `Ord` derive on `Invalidator` and the canonical step indexing.
- **Migration churn under D8.** The 20 call sites span 6 files across 4 crates. Migration must land atomically in one PR to satisfy FND-28; partial migration leaves `RepairKind` in an inconsistent state. Ticket-decomposition should bundle D8 as a single ticket rather than splitting per call site.
- **S139 soft-dep degradation.** `RepairKind::InsertVerification` requires S139's `AskWitness`/`InspectContainer` goal kinds to splice. Without S139, the variant short-circuits to `NoEpistemicSubstrate` and the search falls through. Goldens must cover both branches.
