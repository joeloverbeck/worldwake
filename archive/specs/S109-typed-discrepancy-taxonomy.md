**Status**: ✅ COMPLETED

# S109: Typed Discrepancy Taxonomy and BlockedIntentMemory Split

## Summary

Replace the overloaded `BlockingFact::Unknown` / `BlockingFact::AssumptionFailed` variants with a proper discrepancy taxonomy that distinguishes stale beliefs, contradicted beliefs, missing observations, route-unknowns, no-legal-binding failures (including S108's `RequestResolutionRejectionReason::ExactIdentityRequired`), search-budget exhaustion, no-willing-counterparty refusals, improper planning state, and partial-execution drift. Each discrepancy class carries its own retry policy, invalidation condition, learning update, and debug explanation. Concurrently, split the monolithic `BlockedIntentMemory` into four purpose-specific memories (`DiscrepancyMemory`, `BlockerMemory`, `RepairMemory`, `LearnedOpportunityMemory`) so that epistemic retreat (stale belief → reverify) is never conflated with contention backoff (`SellerOutOfStock`, `WorkstationBusy`) or missing routes. Contention losses and structural-impossibility-by-contention stay in `BlockerMemory` with the existing `BlockingFact` vocabulary — S109 does not rename or relocate them.

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Completed.

## Crates

- `worldwake-core` — new `Discrepancy` enum, `BeliefClaimKey` key type, `DiscrepancyMemory`, `BlockerMemory`, `RepairMemory`, `LearnedOpportunityMemory` components; migrate `BlockingFact::Unknown` / `AssumptionFailed` call sites to the new typing; preserve `BlockerClearingCondition` / `ClearingBaseline` / `BlockerDiagnostic` on `BlockerMemory`
- `worldwake-ai` — `failure_handling.rs` emits typed discrepancies through a new `classify_discrepancy` entry point; TTL lookup and clearing logic split by memory class; candidate generation and search readers migrated to the new memories; the planning trace now surfaces typed discrepancy data through `PlanningPipelineTrace::discrepancy_trace: Vec<DiscrepancyTrace>`
- `worldwake-sim` — belief-view accessors for the new read-only memory views
- `worldwake-cli` — scenario RON files updated to drop `unknown_block_ticks`; `AgentDef` unchanged (memories stay runtime-generated)

## Dependencies

- S108 (Per-Action Binding Strictness) — **landed** at `archive/specs/S108-per-action-binding-strictness.md`. S108 lands the `RequestResolutionRejectionReason::ExactIdentityRequired` rejection and maps it to the existing `BlockingFact::AssumptionFailed`. S109 refines that mapping into `Discrepancy::NoLegalBinding`.

## Design Goals

- Eliminate `BlockingFact::Unknown` as the catch-all bucket. Every currently-Unknown pathway must map to a specific discrepancy class in the new `Discrepancy` enum, a specific typed `BlockingFact` in the surviving vocabulary, or a typed recovery condition.
- Distinguish epistemic discrepancies (agent belief was wrong or missing) from world-state blockers (opportunity contended, depleted, or unreachable). The former live in `DiscrepancyMemory`; the latter remain in `BlockerMemory` using the existing `BlockingFact` variants.
- Route each failure to its correct memory: transient contention losses → `BlockerMemory` (preserves `WorkstationBusy` / `ReservationConflict` / `ExclusiveFacilityUnavailable` / `SellerOutOfStock` semantics); stale beliefs → `DiscrepancyMemory` with a reverify hook; successful alternate repairs → `RepairMemory`; learned opportunities discovered en route → `LearnedOpportunityMemory`.
- Preserve FOUNDATIONS P29A: the authoritative event log records the typed discrepancy or blocker at the moment it is observed, not a generic "Unknown." S110 will add the corresponding `EventTag::BlockerRecorded` variant.

## Non-Goals

- Full PolicyPlan branching on discrepancy class — deferred to a Phase 9 spec.
- Runtime decay / learning-rate tuning beyond setting per-class TTL defaults on `DiscrepancyMemory` and retaining the existing `transient_block_ticks` / `structural_block_ticks` buckets for `BlockerMemory`. Decay-shape tuning is a future adjunct.
- Changing the existing authored `BlockingFact` variants that are already correctly typed (`NoKnownPath`, `NoKnownSeller`, `SellerOutOfStock`, `WorkstationBusy`, `ReservationConflict`, `ExclusiveFacilityUnavailable`, `TargetGone`, `DangerTooHigh`, `CombatTooRisky`, `TooExpensive`, `MissingTool`, `MissingInput`, `PatienceExhausted`, `NoBuyer`, `SourceDepleted`). Those stay on `BlockerMemory`; only `Unknown` and `AssumptionFailed` are decomposed into `Discrepancy` variants.
- Per-variant TTL fields for `BlockerMemory`. The existing 3-bucket TTL policy (`transient_block_ticks`, `structural_block_ticks`, and the now-deleted `unknown_block_ticks`) stays for `BlockerMemory`.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-7 (Locality of Motion, Interaction, and Communication) | Each discrepancy and blocker is recorded against the agent that observed the failure. No cross-agent propagation; another agent learns by its own failed attempt or by a shared-belief carrier (separate path). |
| FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) | Per-class TTL on `DiscrepancyMemory` entries and the existing 3-bucket TTL on `BlockerMemory` entries are the concrete dampeners. Retry storms are bounded by explicit tick-cost retries, not invisible caps. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | Each discrepancy class is a distinct epistemic state: `BeliefStale`, `BeliefContradicted`, `MissingObservation`. The architecture no longer forces all ignorance into the same bucket. |
| FND-20 (Resource-Bounded Practical Reasoning) | Correct discrepancy classification drives correct retry policy. A `BeliefStale` failure triggers reverify; a contention loss (BlockerMemory) triggers short backoff; a `SearchBudgetExhausted` discrepancy triggers longer backoff. |
| FND-22A (Learning, Habits, Preference Shifts) | `LearnedOpportunityMemory` is the authoritative surface for opportunities discovered in transit, separate from blocker backoff. Decay and overwrite are explicit. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The old `BlockingFact::Unknown` and `BlockingFact::AssumptionFailed` variants are removed, not wrapped or aliased. Saved states using those variants are not decodable. |
| FND-29 (Debuggability Is a Product Feature) | The discrepancy class answers "why did this step fail?" without needing to dig into ad-hoc log messages. `PlanningPipelineTrace::discrepancy_trace` now surfaces typed discrepancy entries directly. |
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
    /// The planner/executor is in a state from which no further legal
    /// step can be taken (e.g. intermediate execution aborted partway).
    /// Requires proper-state recovery before resume.
    ImproperPlanningState,
    /// An observation that should have been made during a step did
    /// not arrive (perception failed, occluded, channel missing).
    MissingObservation,
    /// No legal binding exists for the request under the current
    /// `BindingStrictness` class. Includes S108's
    /// `RequestResolutionRejectionReason::ExactIdentityRequired` case.
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
    /// Execution partially applied effects before failing, leaving the
    /// agent in a state that is neither the old start nor the intended
    /// end. Needs bail-out before replan.
    PartialExecutionDrift,
}
```

Contention losses (`WorkstationBusy`, `ReservationConflict`, `ExclusiveFacilityUnavailable`, `SellerOutOfStock`) remain in the existing `BlockingFact` vocabulary on `BlockerMemory` — they are world-state facts with concrete clearing conditions (see D3), not epistemic states. Structural impossibility expressed as "no affordance of the requested kind exists anywhere known" continues to surface through `BlockingFact::NoKnownPath` / `NoKnownSeller` etc.

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

`BlockerKey` is the existing key from `crates/worldwake-core/src/blocker_memory.rs` (reused verbatim after T001's rename). `BeliefClaimKey` is introduced by D9 below. `DiscrepancyClearing` is intentionally simpler than `BlockerClearingCondition` because epistemic discrepancies do not compare concrete quantity baselines — they clear on reobservation, belief update, or time.

### D3: `BlockerMemory` component

Carries the world-state blocker variants from the old `BlockedIntentMemory` (`SellerOutOfStock`, `WorkstationBusy`, `ReservationConflict`, `ExclusiveFacilityUnavailable`, `TargetGone`, `DangerTooHigh`, `CombatTooRisky`, `TooExpensive`, `MissingTool`, `MissingInput`, `PatienceExhausted`, `NoBuyer`, `NoKnownPath`, `NoKnownSeller`, `SourceDepleted`).

`BlockerMemory` preserves the full world-state-aware clearing infrastructure of the current blocker-memory implementation:

- `BlockerClearingCondition` (all 8 variants: `CommodityAvailabilityChanged`, `InventoryChanged`, `UniqueItemAcquired`, `PathDiscovered`, `EntityReappeared`, `DangerReduced`, `ContentionChanged`, `TtlOnly`) and `ClearingBaseline` (all 6 variants) are carried over unchanged.
- `BlockerDiagnostic` attaches to entries whose corresponding `BlockingFact` needs action-def context.
- The `blocks_goal_generation()` gate, `is_blocked`, `is_blocked_for_search`, `find_blocked_for_search`, `record`, `expire`, `sweep_cleared`, `clear_for`, and `clear_all_for_goal` API surface on the current `BlockerMemory` is preserved with identical semantics.
- The sweep path (`clear_resolved_blockers` → `is_blocker_cleared` in `failure_handling.rs`) applies only to `BlockerMemory`. `DiscrepancyMemory` uses a separate, simpler clearing dispatch based on `DiscrepancyClearing`.

The migration is a rename plus the deletion of the two removed variants — no behavioral change in blocker clearing.

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

`RepairEntry` and `OpportunityEntry` both carry explicit `expires_tick: Tick` fields in addition to `observed_tick`, so liveness is authoritative stored state rather than a read-time heuristic. `OpportunityKey` already exists at `crates/worldwake-core/src/goal.rs:161`. `RepairKey` is new and its exact shape lands at implementation time. Size bounds: each memory caps at a per-agent `MemoryCapacityProfile` field (default 32 entries; eviction by oldest `observed_tick`). Retention is profile-driven via `CognitiveProfile::repair_memory_ticks` and `CognitiveProfile::learned_opportunity_memory_ticks`; caps are profile-driven, not hardcoded.

### D5: Migration of `BlockingFact::Unknown` and `AssumptionFailed` call sites

The `BlockingFact` enum loses its `Unknown` and `AssumptionFailed` variants after migration. Every runtime call site is rewritten against the surviving `BlockingFact` vocabulary or against the new `Discrepancy` enum.

**Emission sites (runtime, in `worldwake-ai`):**

- `failure_handling.rs::derive_blocking_fact` default fallthrough (currently line 177, returning `BlockingFact::Unknown`) → routed through a new `classify_discrepancy` helper (see F1). When no specific `BlockingFact` applies, the failure is recorded in `DiscrepancyMemory` under the appropriate `Discrepancy` variant instead of fabricating a catch-all blocker.
- `failure_handling.rs::classify_precondition_failure_detail` (currently line 568) — the `exactidentityrequired` / `targetatactorplace` / `targetdirectlypossessedbyactor` / `targetgrounded` branch returning `BlockingFact::AssumptionFailed` → returns `Discrepancy::NoLegalBinding` for `exactidentityrequired`; remaining precondition-assertion failures route to `Discrepancy::ImproperPlanningState`.
- `failure_handling.rs::map_handler_abort_reason` `SelfTargetForbidden` arm (currently line 590, returning `BlockingFact::Unknown`) → `Discrepancy::NoLegalBinding`.
- `failure_handling.rs::derive_clearing_condition` `Unknown | PatienceExhausted | AssumptionFailed | NoBuyer` arm (currently lines 745–748) → split: `PatienceExhausted` and `NoBuyer` stay on `BlockerMemory` with `BlockerClearingCondition::TtlOnly`; the `Unknown` and `AssumptionFailed` cases disappear from this function because their failures now route to `DiscrepancyMemory` (which has its own clearing dispatch, not `BlockerClearingCondition`).
- `failure_handling.rs::record_blocked_intent` `diagnostic_context` Unknown branch (currently line 66) — removed. `BlockerDiagnostic` only attaches to BlockerMemory entries; `DiscrepancyMemory` entries carry their action-def context through `blocker_key.action_def`.
- `failure_handling.rs::blocking_fact_ttl` (currently lines 992–1011) → drops the `Unknown => unknown_block_ticks` row. Remaining `BlockingFact` variants keep their bucket assignments (`transient_block_ticks`, `structural_block_ticks`). A new parallel function `discrepancy_ttl(&Discrepancy, &CognitiveProfile) -> u32` (see D6) handles `DiscrepancyMemory` entries.
- `agent_tick/frame.rs::record_assumption_failure_blocked_intent` (currently line 435, emitting `BlockingFact::AssumptionFailed`) → routes to `DiscrepancyMemory` with `Discrepancy::BeliefContradicted` when the broken assumption is an epistemic claim (target identity, belief claim) or `Discrepancy::PartialExecutionDrift` when execution has already committed partial effects. The routing decision is made from `frame.domain` state.
- `agent_tick/mod.rs::decision_outcome_planning` (currently line 837–852) — now reads `DiscrepancyMemory` directly into `PlanningPipelineTrace::discrepancy_trace: Vec<DiscrepancyTrace>` rather than filtering `BlockerMemory` on `BlockingFact::Unknown`.

**Reader migration (runtime, in `worldwake-ai`):**

- `candidate_generation.rs::generate_candidates` (currently lines 2030, 2264 call `blocked.is_blocked(...)`) → read both `BlockerMemory::is_blocked(...)` for world-state suppression and `DiscrepancyMemory::is_suppressed(...)` for epistemic suppression. Goal generation is suppressed when either memory has a live entry for the goal key scope.
- `search/candidates.rs` (currently line 1154 calls `blocked.find_blocked_for_search(...)`) → migrate to `BlockerMemory::find_blocked_for_search(...)`. Search does not consult `DiscrepancyMemory` directly; discrepancies are filtered at candidate-generation time before search runs.
- `feasibility.rs`, `planning_snapshot.rs`, `agent_tick/{active_action,candidates,execution,observation,planning,tests}.rs` (12 files total currently access `get_component_blocker_memory` or a local `blocker_memory` alias) → migrate read paths to `get_component_blocker_memory` plus `get_component_discrepancy_memory` as needed. Mutable access (primarily `failure_handling.rs`, `agent_tick/frame.rs`) migrates to the per-memory mutable accessors.

**Diagnostic trace:**

- `decision_trace.rs::PlanningPipelineTrace::discrepancy_trace` and `decision_trace.rs::DiscrepancyTrace` now carry the typed derived trace populated from `DiscrepancyMemory`. See F2.

**Test sites (in `worldwake-ai`, `worldwake-core`):**

- `failure_handling.rs` tests (boundary `#[cfg(test)]` at line 1014; sites at 1673, 1804, 1812, 2094, 2455, 2531, 2539, 2604, 2678) — rewrite to assert against the new memory's expected entry.
- `agent_tick/tests.rs` (sites at 4124, 6006, 6036) — rewrite.
- `candidate_generation.rs` tests (boundary at line 5200; sites at 8005, 15412, 16170) — rewrite.
- `search/tests.rs` (sites at 2323, 2340) — rewrite.
- `blocker_memory.rs` tests (post-T001 rename; sites at 402, 649, 669 in the pre-split draft) — delete or rewrite against `BlockerMemory` / `DiscrepancyMemory`. The `assumption_failed_blocks_goal_generation` test loses its meaning and is deleted.

This is not backward compatible (FND-28) — old save files with the removed variants fail decode. Migration: new save format only; prior runs are not decodable.

### D6: TTL policy per class

Introduce `discrepancy_ttl(&Discrepancy, &CognitiveProfile) -> u32` alongside the existing `blocking_fact_ttl`. `blocking_fact_ttl` keeps its current structure minus the removed `Unknown => cognitive.unknown_block_ticks` row; the remaining variants continue to use `transient_block_ticks` or `structural_block_ticks` as today.

| Discrepancy | Default TTL field (on `CognitiveProfile`) | Rough default |
|-------------|-------------------------------------------|----------|
| `BeliefStale` | `stale_belief_backoff_ticks` | 30 |
| `BeliefContradicted` | `contradicted_belief_backoff_ticks` | 60 |
| `ImproperPlanningState` | `improper_state_backoff_ticks` | 2 |
| `MissingObservation` | `missing_observation_backoff_ticks` | 20 |
| `NoLegalBinding` | `no_legal_binding_backoff_ticks` | 120 |
| `NoWillingCounterparty` | `counterparty_refusal_backoff_ticks` | 40 |
| `RouteUnknown` | `route_unknown_backoff_ticks` | 200 |
| `SearchBudgetExhausted` | `search_exhaustion_backoff_ticks` | 100 |
| `PartialExecutionDrift` | `partial_drift_backoff_ticks` | 4 |

All defaults are per-profile and overridable per agent via scenario. `transient_block_ticks` and `structural_block_ticks` remain on `CognitiveProfile` as the bucket TTLs for `BlockerMemory`; `unknown_block_ticks` is removed.

### D7: Belief-view accessors

Add read-only accessors to the appropriate belief-view sub-trait in `crates/worldwake-sim/src/belief_view.rs`:

```rust
fn discrepancy_memory(&self, agent: EntityId) -> Option<&DiscrepancyMemory> { None }
fn blocker_memory(&self, agent: EntityId) -> Option<&BlockerMemory> { None }
fn repair_memory(&self, agent: EntityId) -> Option<&RepairMemory> { None }
fn learned_opportunity_memory(&self, agent: EntityId) -> Option<&LearnedOpportunityMemory> { None }
```

Default implementations return `None`; `PerAgentBeliefView` / `RuntimeBeliefView` read the live components. These accessors are the surface used by AI-crate read-only consumers (candidate generation, ranking, diagnostic trace). Mutable access continues to flow directly through `get_component_blocker_memory_mut` / `get_component_discrepancy_memory_mut` / `get_component_repair_memory_mut` / `get_component_learned_opportunity_memory_mut` in `failure_handling.rs`, `agent_tick/frame.rs`, and other writer sites. This mirrors the current pattern: there is no existing `blocked_intent_memory` belief-view accessor — the 12 AI-crate files that read `BlockedIntentMemory` today do so through direct component access, and the read paths migrate to the new accessors as part of D5.

### D8: Scenario contract

Add `discrepancy_memory`, `blocker_memory`, `repair_memory`, `learned_opportunity_memory` as runtime-generated universal components (exempt from scenario-authored initialization per `docs/spec-drafting-rules.md` §5 — they start empty and accumulate from runtime experience).

Scenario RON migration (non-trivial because `unknown_block_ticks` is being removed):

- `scenarios/survival-baseline.ron` — 3 occurrences.
- `scenarios/survival-scattered.ron` — 3 occurrences.
- `scenarios/survival-contested.ron` — 4 occurrences.
- `scenarios/cli-evaluation.ron` — 1 occurrence.
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/acute_thirst_spike.ron` — 1 occurrence.
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/convergence_hub.ron` — 3 occurrences.
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/maintenance_starvation_wash_gap.ron` — 2 occurrences.
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/recipe_monoculture_apples_vs_grain.ron` — 2 occurrences.
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron` — 1 occurrence.
- `crates/worldwake-cli/src/scenario/types.rs:939` — test fixture literal.
- `crates/worldwake-core/src/delta.rs:582`, `crates/worldwake-core/src/cognitive_profile.rs:59, 117, 138`, `crates/worldwake-ai/src/{agent_tick/tests.rs,agent_tick/planning.rs,decision_runtime.rs,failure_handling.rs,goal_model.rs,search/tests.rs,lib.rs}` — Rust literal sites that currently assign or assert `unknown_block_ticks`.

All sites lose the field as part of the migration. New TTL fields land on `CognitiveProfile` with `#[serde(default)]` so existing scenarios that do not yet declare them continue to deserialize.

`CognitiveProfile` gains the new TTL fields from D6 and loses `unknown_block_ticks`. `transient_block_ticks` and `structural_block_ticks` remain.

### D9: `BeliefClaimKey` type

Introduce `BeliefClaimKey` in `crates/worldwake-core/` (module location to finalize at implementation time; likely `entity_belief_claim.rs` or a new `belief_claim_key.rs`):

```rust
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BeliefClaimKey {
    pub subject: EntityId,
    pub aspect: EntityBeliefAspect,
}
```

`EntityBeliefAspect` already exists and is re-exported through `crates/worldwake-core/src/belief.rs:5`. `BeliefClaimKey` is the identity under which a `DiscrepancyClearing::BeliefUpdate` request is registered: an observation of the same `(subject, aspect)` later in time clears the entry. S114 and S115 also reference `BeliefClaimKey`; S109 lands the type so later specs consume it without re-introducing it.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Each discrepancy and blocker is recorded against the agent that observed it. Neither `DiscrepancyMemory` nor `BlockerMemory` propagate between agents — another agent learns about a merchant stockout only by its own failed purchase or by a shared belief (`ShareBelief` — separate path). `RepairMemory` and `LearnedOpportunityMemory` are purely per-agent. Aligned with FND-7.
2. **Positive-feedback analysis**: The only loop is "failure → record discrepancy or blocker → suppress goal → reattempt after TTL → possibly record again." The class-specific TTL is the dampener: longer-TTL classes (`RouteUnknown` 200, `SearchBudgetExhausted` 100, `structural_block_ticks` 200) suppress for many ticks, preventing retry spam; shorter-TTL classes (`ImproperPlanningState` 2, `PartialExecutionDrift` 4, `transient_block_ticks` 20) permit quick retry because the world genuinely changes fast.
3. **Concrete dampeners**: Per-class TTL values are the dampener. They are not invisible caps — they encode how long an agent is justified in believing the failure classification without re-evidence. Agents with `stale_belief_backoff_ticks = 5` reverify quickly (impulsive); agents with `stale_belief_backoff_ticks = 50` reverify rarely (stubborn). BlockerMemory reuses the existing `transient_block_ticks` and `structural_block_ticks` dampeners unchanged.
4. **Stored state vs. derived read-model**: `DiscrepancyMemory`, `BlockerMemory`, `RepairMemory`, `LearnedOpportunityMemory`, and `BeliefClaimKey` are authoritative stored state (each entry records a specific observation against a specific key). The "is this goal blocked?" decision is derived from reading those memories plus the current tick. `PlanningPipelineTrace::discrepancy_trace` (F2) is a derived view for debuggability rather than authoritative state.

## SystemFn Integration

No new tick-phase SystemFn. Memory maintenance (TTL expiry) runs inline within `agent_tick` the same way `BlockerMemory::expire` runs today via `clear_resolved_blockers` at `failure_handling.rs:88-96`. Each of the four memories gets an analogous `expire(current_tick)` call at the start of the agent tick. `BlockerMemory` additionally keeps the existing `sweep_cleared` world-state invalidation pass driven by `is_blocker_cleared`; `DiscrepancyMemory` uses a separate, simpler clearing dispatch based on `DiscrepancyClearing` variants.

## Component Registration

Four new components on `EntityKind::Agent`, runtime-generated (exempt from scenario contract per `docs/spec-drafting-rules.md` §5):

- `DiscrepancyMemory` — universal, `Default::default()`
- `BlockerMemory` — universal, `Default::default()`
- `RepairMemory` — universal, `Default::default()`
- `LearnedOpportunityMemory` — universal, `Default::default()`

Each component is registered via the `component_schema!` macro pattern in `crates/worldwake-core/src/component_schema.rs` (with T001 already renaming the blocker registration to `BlockerMemory`) — four analogous blocks replace the one old pre-split registration.

Old `BlockedIntentMemory` component is removed (no backward-compat path — FND-28). All 12 AI-crate files currently accessing `get_component_blocker_memory` migrate to the new per-memory accessors.

`CognitiveProfile` gains the new TTL fields from D6; existing field `unknown_block_ticks` is removed. `transient_block_ticks` and `structural_block_ticks` remain unchanged.

## Cross-System Interactions

- **Sim dispatch ↔ failure classification**: `tick_step.rs` BestEffort failures (including the `RequestResolutionRejectionReason::ExactIdentityRequired` rejection at `tick_step.rs:289`) now produce typed discrepancies or specific blockers (not `Unknown` / `AssumptionFailed`). State-mediated.
- **Revalidation ↔ memories**: `plan_revalidation.rs` reads `DiscrepancyMemory` and `BlockerMemory` when classifying revalidation failure. This is new consumption — `plan_revalidation.rs` does not yet read blocker/discrepancy memory today.
- **Candidate generation ↔ memories**: `candidate_generation.rs` filters suppressed goals by reading `BlockerMemory::is_blocked` and `DiscrepancyMemory::is_suppressed`, and boosts alternatives via `RepairMemory` / `LearnedOpportunityMemory`.
- **Search ↔ BlockerMemory**: `search/candidates.rs` uses `BlockerMemory::find_blocked_for_search`; `DiscrepancyMemory` is filtered before search, not during.
- **S110 event log ↔ discrepancy/blocker recording**: Every recorded discrepancy or blocker emits `EventTag::BlockerRecorded { fact_or_discrepancy, blocker_key }` to the authoritative event log.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `stale_belief_backoff_ticks` | `CognitiveProfile` | `u32` | 30 | How long to trust a stale-belief classification before re-evidencing |
| `contradicted_belief_backoff_ticks` | `CognitiveProfile` | `u32` | 60 | Same for contradictions |
| `improper_state_backoff_ticks` | `CognitiveProfile` | `u32` | 2 | Short backoff for planner-internal state errors |
| `missing_observation_backoff_ticks` | `CognitiveProfile` | `u32` | 20 | Perception-gap backoff |
| `no_legal_binding_backoff_ticks` | `CognitiveProfile` | `u32` | 120 | ExactIdentity / strictness failure backoff |
| `counterparty_refusal_backoff_ticks` | `CognitiveProfile` | `u32` | 40 | Merchant/witness/office refused |
| `route_unknown_backoff_ticks` | `CognitiveProfile` | `u32` | 200 | Unknown route; wait for testimony/exploration |
| `search_exhaustion_backoff_ticks` | `CognitiveProfile` | `u32` | 100 | Planner budget exhausted |
| `partial_drift_backoff_ticks` | `CognitiveProfile` | `u32` | 4 | Bail-out recovery window |
| `transient_block_ticks` | `CognitiveProfile` | `u32` | 20 | Unchanged — BlockerMemory transient bucket |
| `structural_block_ticks` | `CognitiveProfile` | `u32` | 200 | Unchanged — BlockerMemory structural bucket |
| `repair_memory_ticks` | `CognitiveProfile` | `u32` | 120 | How long a successful alternate repair stays eligible to bias ranking |
| `learned_opportunity_memory_ticks` | `CognitiveProfile` | `u32` | 60 | How long an in-transit learned opportunity stays eligible to bias ranking |
| `memory_capacity` | `MemoryCapacityProfile` (new) | `u32` | 32 | Entry cap per memory |

`unknown_block_ticks` is removed.

## Follow-up Deliverables

### F1: `classify_discrepancy` entry point

Introduce a named classifier function in `worldwake-ai` that replaces the implicit default-fallthrough in `derive_blocking_fact`:

```rust
fn classify_discrepancy(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    goal_key: &GoalKey,
    step: &PlannedStep,
    execution_failure: Option<ExecutionFailure<'_>>,
) -> FailureClassification;

enum FailureClassification {
    Blocker(BlockingFact),
    Discrepancy(Discrepancy),
}
```

The caller writes to `BlockerMemory` or `DiscrepancyMemory` based on the classification. A test-time exhaustive match over `Discrepancy` variants enforces compile-time coverage (see Validation test 6).

### F2: `DiscrepancyTrace` replaces `UnknownBlockerTrace`

Replace `decision_trace.rs::PlanningPipelineTrace::unknown_blockers: Vec<UnknownBlockerTrace>` (currently lines 244–246) and the `UnknownBlockerTrace` struct (lines 279–285) with:

```rust
pub struct DiscrepancyTrace {
    pub discrepancy: Discrepancy,
    pub blocker_key: BlockerKey,
    pub expires_tick: Tick,
}
```

`PlanningPipelineTrace` gets a `discrepancy_trace: Vec<DiscrepancyTrace>` field. The filter at `agent_tick/mod.rs:837–852` reads from `DiscrepancyMemory` instead of `BlockedIntentMemory`. `BlockerMemory` entries are not surfaced in the trace — the observer consumes them through the existing blocker-memory snapshot path.

## Validation and Falsification

### Unit tests (in `discrepancy.rs` / memory modules)

1. `DiscrepancyMemory::record` + `expire(tick)` prunes expired entries.
2. `DiscrepancyClearing::ReobservationOf { target }` clears when perception system records a new observation of that target.
3. `DiscrepancyClearing::BeliefUpdate { claim_key }` clears when a new belief claim with matching `BeliefClaimKey` is recorded.
4. `BlockerMemory` preserves exact old `BlockedIntentMemory` semantics for migrated variants (carried over test coverage from `blocked_intent.rs` — the `SourceDepleted`, `ExclusiveFacilityUnavailable`, `TargetGone`, place-scoping, and `sweep_cleared` tests all apply unchanged).
5. `RepairMemory` overwrites a repair entry when a fresher successful alternate is recorded.
6. `LearnedOpportunityMemory` evicts oldest entry when `memory_capacity` is exceeded.

### Migration tests

7. Every `failure_handling.rs` code path that previously emitted `BlockingFact::Unknown` now emits either a specific surviving `BlockingFact` on `BlockerMemory` or a specific `Discrepancy` variant on `DiscrepancyMemory` (compile-time coverage via exhaustive match on `FailureClassification` and `Discrepancy`).
8. Every `BlockingFact::AssumptionFailed` call site maps to a specific `Discrepancy` variant (typically `BeliefContradicted`, `NoLegalBinding`, or `PartialExecutionDrift`) with no silent fallback to a catch-all.

### Golden test extension

9. Keep the generated golden surfaces green after the cleanup migration, but prove typed post-failure memory routing at the strongest honest lower layer unless a live golden seam already exposes those authoritative memories. On the current branch, `failure_handling` coverage proves `TargetGone` still records on `BlockerMemory`, and `agent_tick` coverage proves belief-contradiction routing records `Discrepancy::BeliefContradicted`; no stronger golden-level persisted-memory assertion is assumed unless that surface is explicitly added later.

## Outcome

Completed: 2026-04-19

What changed:
- Replaced the old overloaded blocker surface with the live split across `DiscrepancyMemory`, `BlockerMemory`, `RepairMemory`, and `LearnedOpportunityMemory`.
- Removed `BlockingFact::Unknown`, `BlockingFact::AssumptionFailed`, and `CognitiveProfile::unknown_block_ticks`, and migrated runtime failure routing to typed `Discrepancy` handling.
- Landed the supporting additive substrate for `BeliefClaimKey`, discrepancy TTL fields, memory-retention fields, read-only belief-view accessors, and the `DiscrepancyTrace` planning trace surface.
- Migrated authored scenarios, save/load format, observer fixtures, and generated docs to the post-split contract.

Deviations from original plan:
- The original work was decomposed across the `S109TYPDISTAX-*` ticket chain instead of landing as one pass.
- `RepairMemory` and `LearnedOpportunityMemory` initially landed as additive shells, then received their real expiry/retention semantics in follow-up ticket `S109TYPDISTAX-007`.
- The drafted golden-level assertion for persisted post-replan memory was narrowed to stronger lower-layer proof because the live golden seam on this branch does not expose that authoritative memory aftermath directly.

Verification results:
- `cargo test --workspace --no-run`
- `cargo build --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- Focused proof landed across the ticket chain for `worldwake-core`, `worldwake-ai`, `worldwake-sim`, and `worldwake-cli`, including blocker/discrepancy routing, memory retention, belief-view accessors, trace migration, scenario/save cleanup, and generated-doc refresh.

## Post-Merge Migration Correction (2026-04-19)

The initial TYPDISTAX-004 migration landed two correctness defects that were caught by the survival-baseline / contested / scattered goldens. The defects violated the spec's behavior-preservation contract for the migrated paths and were corrected in-scope on this PR:

1. **`derive_discrepancy_clearing` over-application of `ReobservationOf`.** The initial implementation returned `DiscrepancyClearing::ReobservationOf { target }` for every discrepancy that carried a `target`, regardless of class. `ReobservationOf` is only meaningful for classes whose authoritative resolution is "the agent saw the target again and now has a fresh observation that supersedes the recorded discrepancy" — i.e. `BeliefStale`, `BeliefContradicted` (when no typed `BeliefClaimKey` is derivable), and `MissingObservation`. For other classes (`ImproperPlanningState`, `PartialExecutionDrift`, `SearchBudgetExhausted`, `RouteUnknown`, `NoLegalBinding`, `NoWillingCounterparty`), re-perceiving the target proves nothing about whether the failure mode is resolved; applying `ReobservationOf` caused entries to clear the moment an agent traveled back into observation range, producing oscillation loops in survival scenarios. The corrected function scopes `ReobservationOf` to the three classes where re-perception genuinely supersedes the discrepancy, and falls through to `TtlExpiry` otherwise. Regression coverage: `failure_handling::tests::discrepancy_clearing_is_ttl_expiry_for_planner_state_classes`, `discrepancy_clearing_uses_reobservation_for_perceptive_classes`, `discrepancy_clearing_is_ttl_expiry_for_targetless_entries_in_all_classes`.

2. **`record_assumption_failure` lost effective suppression duration.** Pre-S109 the equivalent path emitted `BlockingFact::AssumptionFailed` recorded with `BlockerClearingCondition::TtlOnly` and TTL=`structural_block_ticks` (200 by default). The initial migration changed both: TTL became `partial_drift_backoff_ticks` (4) and clearing became `DiscrepancyClearing::ReobservationOf { target }` when a target was present. Combined, the effective suppression window collapsed from 200 ticks to ~0 — survival agents whose plan failed re-attempted the same broken plan as soon as they re-perceived the target. The corrected function uses TTL=`structural_block_ticks` with `DiscrepancyClearing::TtlExpiry`, preserving the pre-S109 suppression semantics that the survival baselines depend on. The classification (`Discrepancy::BeliefContradicted` when target present, `Discrepancy::PartialExecutionDrift` when not) is unchanged. Regression coverage: `agent_tick::frame::tests::record_assumption_failure_uses_structural_block_ticks_with_target`, `record_assumption_failure_uses_structural_block_ticks_without_target`, `record_assumption_failure_overwrites_prior_entry_for_same_key`.

3. **Throwaway `DiscrepancyMemory::default()` in `handle_active_action_phase` interrupt path.** The initial migration created a fresh `DiscrepancyMemory::default()` when handling an interrupt-for-replan and passed it to `reconcile_in_flight_state`. Any discrepancy recorded during interrupt reconciliation was silently lost when the throwaway went out of scope. Corrected by threading the agent's real `discrepancy_memory` parameter through. This is defense-in-depth: even after the two clearing/TTL corrections above, the throwaway would have continued to lose discrepancies on the interrupt path.

These corrections do not change the typed-taxonomy design or the per-class TTL fields. They restore the migration's behavior-preservation contract for the affected paths.

## Known follow-up work

The survival baseline / contested / scattered goldens still fail with these corrections. They expose a pre-S109 architectural gap that the typed-taxonomy split correctly stops masking: agents lack a perception-driven belief invalidation path for failed acquisitions. When a planner selects a `Travel + pick_up` plan based on a stale belief about a commodity lot, and the lot turns out to be empty/missing/inaccessible, no `FrameAssumption::CommodityAvailableAt` failure currently fires — the assumption is declared in `crates/worldwake-core/src/intention_frame.rs:69` but `crates/worldwake-ai/src/agent_tick/frame.rs::evaluate_assumptions` stubs it as always-true ("future work" comment) and `populate_assumptions` does not add it for `IntentionDomain::Travel`. Pre-S109 the 200-tick blanket `BlockingFact::AssumptionFailed` suppression masked the gap by simply benching the goal long enough for world state to drift; the typed-taxonomy split (correctly) gives each failure mode its own backoff and exposes the missing assumption.

The follow-up spec is `specs/S122-frame-assumption-commodity-availability.md`. Increasing `improper_state_backoff_ticks` beyond its spec-mandated default of 2 is intentionally NOT done here — it would be a band-aid that blurs the typed-taxonomy semantics and shifts other observer-anomaly tests. The survival goldens stay red until S122 lands.
