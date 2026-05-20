# S149: Partial Plan Segments and Typed Plan Terminals

**Status**: Draft

## Summary

Folds in PR-10 (Partial plans as first-class objects) and PR-5 (Information barriers as first-class plan outcomes) from `reports/ai-architecture-improvements.md`.

Before S149PARPLASEG-001, the planner had three `PlanTerminalKind` variants: `GoalSatisfied`, `ProgressBarrier`, `CombatCommitment`, and every non-success terminal path used `ProgressBarrier`. S149PARPLASEG-001 replaced that generic terminal with typed terminals: `GoalSatisfied`, `CombatCommitment`, `InformationBarrier`, `CoordinationBarrier`, `ResourceBarrier`, `JurisdictionBarrier`, `SearchBudgetExhausted` (seven variants), and re-keyed diagnostics by `PlanTerminalKindDiscriminant`. Remaining S149 work builds partial-plan storage and resumption on top of those typed terminals. S139 added `GoalKind::AskWitness` as a sensing *goal*, but the planner cannot yet express "I made partial progress and stopped at an information barrier — when I learn fact F, I can resume."

S149 ships both as the same architectural layer: typed terminal barriers and first-class `PartialPlanSegment` storage. When a plan attempts goal G and reaches a typed barrier B, the planner stores a `PartialPlanSegment` carrying the prefix steps that did succeed, the barrier type, the resume conditions that would clear the barrier, and the abandon conditions that would invalidate the partial plan. S149PARPLASEG-005 lands the agenda-manager lifecycle slice for stored segments: evaluating resume/abandon conditions, incrementing bounded retry state, and returning a typed resumed segment. Executable suffix re-entry from `remaining_skeleton` is deferred to S149PARPLASEG-010 because the current skeleton carrier is not yet sufficient to reconstruct lawful `PlannedStep`s without a planner resolver contract.

The typed barriers map onto the *existing* failure-attribution surfaces so the failure-handling pipeline absorbs them uniformly. Three barriers reuse existing `Discrepancy` variants (S109): `InformationBarrier` ⇒ `Discrepancy::MissingObservation`; `ResourceBarrier` ⇒ `Discrepancy::BeliefStale`; `JurisdictionBarrier` ⇒ `Discrepancy::NoLegalBinding`; `SearchBudgetExhausted` ⇒ `Discrepancy::SearchBudgetExhausted`. `CoordinationBarrier` reuses the existing `BlockingFact::ReservationConflict` blocker surface (`crates/worldwake-core/src/blocker_memory.rs:241`) rather than `Discrepancy`, because contention attribution is carried by the live blocker taxonomy, not the discrepancy taxonomy (see D6).

**Resume and abandon conditions reuse the S148 types already in core**: `IntentionResumeCondition` and `IntentionAbandonCondition` (`crates/worldwake-core/src/intention_condition.rs:7,24`). S149 introduces no parallel condition types — that would be two live authoritative representations of the same fact (FND-28).

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-core` — no new condition types (reuses existing `IntentionResumeCondition` / `IntentionAbandonCondition` from S148). No new component. `TellTopic` (already core, `belief.rs:1737`) is reused as the information-gap topic.
- `worldwake-sim` — `SAVE_FORMAT_VERSION` was bumped to 91 by S149PARPLASEG-001 for the typed-terminal serialized-format break, then to 92 by S149PARPLASEG-003 because adding `AgendaEntry.partial_plan_segment` changes the bincode shape of the ai runtime payload. Version 91 saves are rejected at the existing save-header boundary; no compatibility decoder is introduced.
- `worldwake-systems` — no change.
- `worldwake-ai` — `PlanTerminalKind`, the payload-free `PlanTerminalKindDiscriminant` histogram key, and all `ProgressBarrier` removal sites landed in S149PARPLASEG-001; S149PARPLASEG-002 added the `PartialPlanSegment` carrier type (and `PartialPlanSegmentId`, `BarrierFact`, `PlannedSkeletonStep`) here because their fields reference ai-resident types (`PlanTerminalKind`, `PlannedStep`, `PlannerOpKind`, `GoalOffer`, `BeliefPredicate`); S149PARPLASEG-003 added `PartialPlanSegment` storage on `AgendaEntry`; S149PARPLASEG-005 adds the suspended-entry lifecycle evaluator; S149PARPLASEG-010 owns executable segment writing and tactical suffix re-entry.
- `worldwake-cli` — observer renders barrier type per terminal in the planning-diagnostic sections; S144 diagnostics aggregate barrier-kind distribution.

## Dependencies

- S88 (Two-Phase Landmark Planning, archived at `archive/specs/S88-two-phase-landmark-planning.md`) — provides the strategic + tactical search the typed terminals attach to.
- S109 (Typed Discrepancy Taxonomy, archived at `archive/specs/S109-typed-discrepancy-taxonomy.md`) — three barriers reuse existing `Discrepancy` variants.
- S115 (Agenda Manager, archived at `archive/specs/S115-agenda-manager.md`) — manages suspended intentions; S149 adds partial-plan-aware resumption.
- S132 (Frontier-Exhaustion Strategy as Goal-Kind Property, archived at `archive/specs/S132-frontier-exhaustion-strategy.md`) — current generic `ProgressBarrier` dispatch; S149 refines.
- S137 (Plan Causal Links and Localized Repair, archived at `archive/specs/S137-plan-causal-links-and-repair.md`) — provides repair substrate; barrier-typed plans guide repair scope.
- S139 (AskWitness Goal Layer, archived at `archive/specs/S139-epistemic-sensing-subgoals.md`) — provides `GoalKind::AskWitness { witness, topic: TellTopic }` (`goal.rs:145`) that `InformationBarrier` resume conditions can produce.
- S148 (Portfolio Slot Expansion, archived at `archive/specs/S148-portfolio-and-motive-backed-intentions.md`) — provides the already-landed `IntentionResumeCondition` / `IntentionAbandonCondition` enums, `IntentionFrame.resume_conditions` / `abandon_conditions` / `patience_limit` fields, and the `SlotKind::SocialMotive` slot. S149 reuses these; it does not redefine them.

## Design Goals

1. **Every plan terminal carries diagnostic shape.** Observer and S144 can answer "what stopped this plan?" with one of seven specific types, not a generic `ProgressBarrier`.
2. **Partial plans resume, don't restart.** When the resume condition holds, the agenda manager continues from the partial-plan suffix.
3. **No barrier is silent.** Each barrier type produces a typed failure record on the existing surfaces (`Discrepancy` for four kinds, `BlockingFact` for contention), flowing through the existing blocker/repair pipeline.
4. **Backward-compat-free.** The generic `ProgressBarrier` is removed entirely; specific barrier types replace it. No alias, no shim (FND-28).
5. **Deterministic resumption.** Same resume condition + same belief update → same suffix retry.
6. **Bounded resume retries.** A partial plan that re-fails its tail enters `IntentionAbandonCondition::PatienceExhausted` per S148.

## Non-Goals

- **No method-decomposition resumption.** S147's `MethodSchema` already supports method-driven decomposition; S149 only handles flat tactical-prefix resumption.
- **No cross-tick search continuation.** That is PR-14 (rejected as YAGNI).
- **No new event tag for barrier transitions.** Existing `PlanFinalized` / decision-event payloads carry the typed terminal.
- **No invariants beyond per-goal.** A `PartialPlanSegment` is owned by its `IntentionFrame`; no shared partial-plan pool.
- **No `SafetyBarrier` terminal in this spec.** A safety/danger barrier requires a danger-projection system that does not yet exist; mapping it to `Discrepancy::NeedHorizonExceeded` would be a phantom hook. `SafetyBarrier` and any `SafetyHazard` type are deferred to the future danger-projection spec, which will add the terminal variant, its discriminant, and its resume path at that time. S149 ships seven terminals without it.
- **No new resume/abandon condition types.** S149 reuses the S148 `IntentionResumeCondition` / `IntentionAbandonCondition` enums in core. Introducing parallel types would violate FND-28.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Typed barriers replace a single abstract terminal label; partial plans store concrete prefix steps. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | `InformationBarrier` makes ignorance an explicit plan outcome rather than a planning failure. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Partial plans express *how* the agent partially advanced; the typed barrier expresses *what is blocking* further progress. Resumption re-enters plain GOAP tactical search over the recorded prefix; no scripted rails (see Section H Planner-Formalism Analysis). |
| FND-21 (Intentions Are Revisable Commitments) | Resume/abandon conditions on the segment make every partial intention explicitly revisable. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Partial-plan resumption reads belief-view state to evaluate resume conditions; no cross-system command. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | Generic `ProgressBarrier` is removed and all sites migrated (D3); resume/abandon conditions reuse the single existing core types rather than introducing a parallel taxonomy. |
| FND-29 (Debuggability Is a Product Feature) | Observer planning-diagnostic sections (Section 9 Budget Exhaustion Snapshots and Section 13 Scenario Diagnostics) surface per-attempt barrier types; S144 aggregates `terminal_kind_distribution` keyed by the payload-free discriminant. |

## Deliverables

### D1: Typed `PlanTerminalKind`

```rust
// crates/worldwake-ai/src/planner_ops.rs (replacing existing 3-variant enum)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PlanTerminalKind {
    GoalSatisfied,
    CombatCommitment,
    InformationBarrier { topic: TellTopic },
    CoordinationBarrier { contested_resource: EntityId },
    ResourceBarrier { commodity: CommodityKind, place: EntityId },
    JurisdictionBarrier { authority: EntityId, jurisdiction: EntityId },
    SearchBudgetExhausted { budget_consumed: u16, budget_total: u16 },
}
```

`PlanTerminalKind` currently derives `Copy` (planner_ops.rs:387); every payload type above is `Copy` (`TellTopic`, `EntityId`, `CommodityKind`, `u16`), so the derive is preserved. `TellTopic` (`crates/worldwake-core/src/belief.rs:1737`) is the existing testimony-topic enum reused for the information-gap topic; no new `InformationGapTopic` type is introduced.

The legacy generic `ProgressBarrier` is removed (migration in D3). `SearchBudgetExhausted` is the explicit typed terminal for terminal-bearing budget-exhaustion contexts such as repair/discrepancy terminal records. Direct no-plan search-budget exhaustion remains the existing `PlanSearchResult::BudgetExhausted` outcome after S149PARPLASEG-001 unless a terminal-bearing partial-segment path is later populated. The other typed barriers replace the cases that meant "missing fact" / "contested" / "depleted" / "out of authority."

### D2: `PlanTerminalKindDiscriminant` (histogram key)

`PlanTerminalKind` is payload-bearing, so it cannot be a `BTreeMap` aggregation key without fragmenting the histogram per payload value (every distinct `EntityId` / `CommodityKind` would become its own bucket). Add a payload-free discriminant:

```rust
// crates/worldwake-ai/src/planner_ops.rs (new, alongside PlanTerminalKind)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum PlanTerminalKindDiscriminant {
    GoalSatisfied,
    CombatCommitment,
    InformationBarrier,
    CoordinationBarrier,
    ResourceBarrier,
    JurisdictionBarrier,
    SearchBudgetExhausted,
}

impl From<&PlanTerminalKind> for PlanTerminalKindDiscriminant { /* 1:1 match */ }
```

`PlanningMetrics.terminal_kind_distribution` (`crates/worldwake-ai/src/scenario_diagnostics/mod.rs:43`) is re-keyed from `BTreeMap<PlanTerminalKind, u64>` to `BTreeMap<PlanTerminalKindDiscriminant, u64>`. The expected-diagnostics fixture (`crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json`) updates to the discriminant form.

### D3: `ProgressBarrier` removal migration

Removing `PlanTerminalKind::ProgressBarrier` touches ~20 files / 60+ sites. This deliverable enumerates the migration explicitly (FND-28: no aliasing the removed variant):

- **Creation sites**: `crates/worldwake-ai/src/search/transition.rs` and `crates/worldwake-ai/src/search/mod.rs` fallback — each former `ProgressBarrier` construction is replaced by the specific typed terminal that matches the failure cause at that site. Direct no-plan budget exhaustion remains `PlanSearchResult::BudgetExhausted`; terminal-bearing budget-exhaustion contexts use `SearchBudgetExhausted`.
- **`DowngradeToProgressBarrier` repair kind** (`crates/worldwake-ai/src/plan_repair.rs`, ~8 sites): rename to `DowngradeToTypedBarrier` (or equivalent) so the repair pipeline downgrades to the specific terminal the failure produced rather than the generic one. The repair-kind enum, its match arms, and its trace rendering migrate together.
- **Terminal handling** in `agent_tick/planning.rs`, `agent_tick/execution.rs`, `agent_tick/observation.rs`, `agent_tick/active_action.rs`, `failure_handling.rs`, `plan_selection.rs`: each `ProgressBarrier` match arm is replaced by arms over the typed terminals (or a catch-all over the barrier-bearing subset where the handling is uniform).
- **Tests / fixtures / UI**: `search/tests.rs` assertions, `goal_model.rs` test assertions, `candidate_generation.rs` fixtures, `visualizer/tabs/plan.rs`, observer test data, golden scenario files (`htn_methods.rs`, `offices.rs`, `plan_repair.rs`), and `expected-scenario-diagnostics.json` migrate to the typed terminals.

Implementation must satisfy the `Authoritative-to-AI Impact Analysis` checklist below (`search_plan` terminal ordering, `handle_plan_failure` replan).

### D4: `PartialPlanSegment`

```rust
// crates/worldwake-ai/src/partial_plan.rs (new — lives in worldwake-ai because its
// fields reference ai-resident types PlanTerminalKind, PlannedStep, PlannerOpKind,
// GoalOffer, BeliefPredicate; worldwake-core cannot depend on worldwake-ai)
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PartialPlanSegment {
    pub id: PartialPlanSegmentId,
    pub goal: GoalOffer,
    pub completed_prefix: Vec<PlannedStep>,
    pub remaining_skeleton: Option<Vec<PlannedSkeletonStep>>,
    pub terminal_barrier: PlanTerminalKind,
    pub barrier_fact: BarrierFact,
    pub resume_conditions: Vec<IntentionResumeCondition>,   // reused from S148 core type
    pub abandon_conditions: Vec<IntentionAbandonCondition>, // reused from S148 core type
    pub created_tick: Tick,
    pub last_resume_attempt_tick: Option<Tick>,
    pub resume_attempt_count: u8,
    pub causal_links: Vec<EventId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedSkeletonStep {
    pub op: PlannerOpKind,
    pub target_template: PayloadTemplate,        // crates/worldwake-ai/src/htn/method_schema.rs:151
    pub expected_pre: Vec<BeliefPredicate>,      // crates/worldwake-ai/src/htn/method_schema.rs:72
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BarrierFact {
    MissingBelief(BeliefPredicate),
    ContestedReservation(EntityId),
    DepletedResource { commodity: CommodityKind, place: EntityId },
    NoAuthorityForAction(EntityId),
    BudgetExhausted { remaining_stages: u16 },
}
```

`PartialPlanSegmentId` is a typed newtype with `Tick`-and-counter provenance for deterministic ID assignment. `expected_pre` uses the existing `BeliefPredicate` enum; the fictional `PreconditionPredicate` is dropped. `BarrierFact::HazardPresent` is dropped along with `SafetyBarrier` (see Non-Goals).

### D5: Agenda-manager resumption path

```rust
// crates/worldwake-ai/src/agenda_manager.rs (extended)
fn try_resume_partial_plan(
    state: &mut AgendaState,
    actor: EntityId,
    belief_view: &dyn RuntimeBeliefView,   // crates/worldwake-sim/src/belief_view.rs:1596
    tick: Tick,
    patience_limit: u32,
) -> Option<ResumedPlan>;
```

For each suspended `AgendaEntry` (in `AgendaState.suspended`, `crates/worldwake-ai/src/agenda_types.rs:18`) carrying a `PartialPlanSegment`:
1. Check `abandon_conditions` — if any holds, abandon the intention and clear the segment.
2. Check `resume_conditions` — if any holds, return the segment for retry.
3. Otherwise leave suspended.

A returned `ResumedPlan` identifies the suspended agenda entry and updated `PartialPlanSegment` that is eligible for retry. S149PARPLASEG-010 owns the executable planner re-entry contract that applies `completed_prefix` to planning state and completes `remaining_skeleton` against the new world state.

`PartialPlanSegment.resume_attempt_count` increments per try; when it exceeds the supplied `IntentionFrame.patience_limit` (per S148, `crates/worldwake-core/src/intention_frame.rs:141`), the segment is abandoned (`IntentionAbandonCondition::PatienceExhausted`). The agenda entry does not store its own patience limit.

### D6: Barrier → failure-attribution mapping

The agenda manager records each barrier on the existing failure surface and derives resume conditions from the barrier fact. Four barriers reuse `Discrepancy` variants; `CoordinationBarrier` reuses `BlockingFact::ReservationConflict` because contention attribution lives on the blocker taxonomy, not the discrepancy taxonomy:

```rust
fn terminal_to_discrepancy(terminal: &PlanTerminalKind) -> Option<Discrepancy> {
    match terminal {
        PlanTerminalKind::GoalSatisfied => None,
        PlanTerminalKind::CombatCommitment => None,
        PlanTerminalKind::InformationBarrier { .. } => Some(Discrepancy::MissingObservation),
        PlanTerminalKind::ResourceBarrier { .. } => Some(Discrepancy::BeliefStale),
        PlanTerminalKind::JurisdictionBarrier { .. } => Some(Discrepancy::NoLegalBinding),
        PlanTerminalKind::SearchBudgetExhausted { .. } => Some(Discrepancy::SearchBudgetExhausted),
        // CoordinationBarrier is NOT a Discrepancy — it routes to BlockingFact (see below).
        PlanTerminalKind::CoordinationBarrier { .. } => None,
    }
}
```

`CoordinationBarrier { contested_resource }` records a `BlockingFact::ReservationConflict { affordance, contention_event }` (`crates/worldwake-core/src/blocker_memory.rs:241`) through the existing blocker-memory path, with `affordance` derived from the contested affordance and `contention_event` from the contention event that blocked the step.

The four `Discrepancy`-mapped barriers record through the existing `DiscrepancyMemory` path (S109). Resume conditions are derived from the barrier fact using the *existing* `IntentionResumeCondition` variants (`crates/worldwake-core/src/intention_condition.rs:7`):
- `MissingBelief(pred)` → `IntentionResumeCondition::BeliefStatusChanged { subject, target_status }`, where `subject` is the belief subject and `target_status` is the `BeliefStatusTag` the agent must reach (e.g. `Known`).
- `ContestedReservation(target)` → `IntentionResumeCondition::ArtifactLegalEffectActive(target)` (the grant/claim becomes active) or `IntentionResumeCondition::OpportunityVisible(anchor)` when the contested affordance is re-offered.
- `DepletedResource { place, commodity }` → `IntentionResumeCondition::BeliefStatusChanged { subject: place, target_status }` once the agent believes the commodity is available at `place`.
- `NoAuthorityForAction(actor)` → `IntentionResumeCondition::ArtifactLegalEffectActive(authority)` or a `BeliefStatusChanged` on the authority artifact.
- `BudgetExhausted` → `IntentionResumeCondition::TickElapsed(cognitive.search_exhaustion_backoff_ticks)`.

`search_exhaustion_backoff_ticks` (`crates/worldwake-core/src/cognitive_profile.rs:56`, "TTL for search-budget-exhaustion discrepancies before retry") is the existing per-agent profile field for budget-cooldown timing. No new `budget_cooldown_ticks` field is introduced — that would duplicate an existing field (FND-28).

### D7: Information-barrier subgoal synthesis

When a plan terminal is `InformationBarrier { topic }`, the agenda manager spawns an auxiliary `GoalKind::AskWitness { witness, topic }` (S139 substrate, `crates/worldwake-core/src/goal.rs:145`) as a *companion intention* slot-typed `SlotKind::SocialMotive` (S148). The `topic` is the `TellTopic` carried by the barrier; the `witness` is chosen from co-located or known agents the belief view exposes as plausible sources for that topic (the synthesis must name a concrete `witness: EntityId`, since `AskWitness` requires one). The companion intention is owned by the suspended primary intention; abandoning the primary cancels the companion. Successful `AskWitness` commit updates the agent's belief store; the `BeliefStatusChanged` resume condition on the suspended primary then fires.

### D8: Coordination-barrier resume listening

When a plan terminal is `CoordinationBarrier { contested_resource }`, the agenda manager adds the suspended intention to a per-actor "watching" list keyed on `contested_resource`. Resume is triggered by the existing contention lifecycle: `ContentionGrant` (`crates/worldwake-core/src/contention.rs:43`) tracks expiry via `expires_at`, and its lifecycle is queue-state-mediated rather than carried by a discrete invalidation event. The resume path therefore checks the watching list against contention-queue state transitions (grant expiry / re-grant / queue-head promotion) and fires the `ArtifactLegalEffectActive` / `OpportunityVisible` resume condition when the contested resource becomes available again. No new event tag is introduced. (Implementation must confirm the precise queue-state signal it hooks; if the lifecycle does not expose the needed transition, this deliverable adds the minimal read-side hook on the existing contention state — never a new authoritative representation.)

### D9: Observer rendering

The observer's planning-diagnostic surface renders per-attempt barrier types. The relevant existing sections are **Section 9 — Budget Exhaustion Snapshots** (`crates/worldwake-cli/src/bin/observer.rs:1418`) and **Section 13 — Scenario Diagnostics** (`observer.rs:4002`, which carries `terminal_kind_distribution`); there is no "Section 7 (planning)" — Section 7 is "End-State Inventory & Resources". Extend the appropriate section to print, following the existing `## Section <N> — <Title>` header convention:

```
Plan terminal: ResourceBarrier(commodity=Grain, place=ThornwallMarket#42)
  Barrier fact: DepletedResource — observed stock = 0 at tick 1247
  Resume on: BeliefStatusChanged(ThornwallMarket#42 -> Known: commodity available)
  Abandon if: PatienceExhausted (3 resume attempts left)
```

S144's `PlanningMetrics.terminal_kind_distribution` is keyed by `PlanTerminalKindDiscriminant` (D2) and gains all seven typed kinds.

### D10: Partial-plan storage

`AgendaEntry` (`crates/worldwake-ai/src/agenda_types.rs:22`) gains:

```rust
pub partial_plan_segment: Option<PartialPlanSegment>,
```

`AgendaState.suspended` map (`agenda_types.rs:18`) stores the suspended `AgendaEntry`s; segments persist with their entries through the existing `AgentDecisionRuntime` runtime payload. `SAVE_FORMAT_VERSION` was already bumped to `91` by S149PARPLASEG-001 because removing `ProgressBarrier` changed serialized decision payloads; S149PARPLASEG-003 bumps it again to `92` because live reassessment proved bincode cannot decode the pre-field version-91 runtime shape with `#[serde(default)]` alone. Version 91 saves are rejected at the save header rather than supported by a compatibility shim.

### D11: Golden coverage

A golden scenario module (under the post-S154 `golden_ai` test target — `crates/worldwake-ai/tests/golden_ai.rs` routes to `tests/scenarios/`; there is no standalone `golden_typed_plan_terminals.rs` path) covers:
- `InformationBarrier` end-to-end: agent lacks target location → barrier raised → companion `AskWitness` commits → primary intention resumes → completion.
- `CoordinationBarrier`: agent loses oven reservation → barrier raised → `BlockingFact::ReservationConflict` recorded → grant re-available → resume.
- `ResourceBarrier`: market depleted → barrier raised → resupply observed → resume.
- `JurisdictionBarrier`: agent attempts arrest outside jurisdiction → barrier raised → travel to jurisdiction → resume.
- `SearchBudgetExhausted`: budget runs out → eligible suspended segment receives a typed `SearchBudgetExhausted` terminal per the segment-construction ticket → `search_exhaustion_backoff_ticks` TTL → resume.
- Abandon-condition flow: patience-exhausted abandons; observer surfaces the abandonment.

(`SafetyBarrier` coverage is deferred with the variant — see Non-Goals.)

## FND-01 Section H Analysis

### Information-Path Analysis

`PartialPlanSegment.resume_conditions` evaluate against existing belief-view reads. The companion `AskWitness` intention (D7) follows S139's existing testimony-acquisition path. Coordination-barrier resumption (D8) reads existing contention-queue state. No new world-information path is introduced.

### Positive-Feedback Analysis

Potential loop: failed resume → retry → fails again → retry. Bounded by `IntentionFrame.patience_limit` (per S148) and `PartialPlanSegment.resume_attempt_count` cap.

### Concrete Dampeners

- `patience_limit` per S148.
- `resume_attempt_count` per-segment cap.
- `IntentionAbandonCondition::PatienceExhausted` is the lawful exit path.

### Stored State vs. Derived Read-Model List

**Stored state**:
- `PartialPlanSegment` (in `AgendaState.suspended[].partial_plan_segment`, ai-crate runtime state persisted through the existing agenda-state save path).
- `PlanTerminalKind` enum (extended; no new authoritative ECS storage, but typed-terminal data persists in decision-history payloads).

**Derived read-model**:
- `PlanTerminalKindDiscriminant` histogram keys (derived from `PlanTerminalKind` via `From`).
- Resume / abandon evaluation per-tick.

### Planner-Formalism Analysis

S149 is plain GOAP/affordance search. Resumption remains intended to re-enter the *existing* tactical search over the recorded `completed_prefix` and `remaining_skeleton`; it must not register any HTN `MethodSchema` and must not encode goal-specific decomposition (the Non-Goals exclude method-decomposition resumption — that lives in S147). Live reassessment during S149PARPLASEG-005 showed that the current `PlannedSkeletonStep` carrier is not executable by itself, so executable re-entry is deferred to S149PARPLASEG-010 rather than inventing a parallel planner path. The typed terminals are search outcomes, not method contracts. No method-required goal is introduced, so no schema contract or fallback-invalidity argument is needed.

## SystemFn Integration

No new top-level `SystemFn`. Resumption runs inside the agenda manager's existing tick pass.

## Component Registration

No new ECS component. `PartialPlanSegment` lives within the existing `AgendaState` ai-crate runtime structure on the agent (analogous to other per-agent runtime state held off the ECS).

## Cross-System Interactions

- Agenda manager (S115) consults `AgentBeliefStore` (S101/S113) via the belief view to evaluate resume conditions.
- Partial plans reference `ContentionGrant` (S140) lifecycle via contention-queue state reads (D8).
- Typed terminals produce S109 `Discrepancy`s (four kinds) and `BlockingFact::ReservationConflict` (coordination) through existing failure-handling.

State-mediated.

## Profile-Driven Parameters

No new profile field. Budget-cooldown timing reuses the existing `CognitiveProfile.search_exhaustion_backoff_ticks` (`crates/worldwake-core/src/cognitive_profile.rs:56`).

## Authoritative-to-AI Impact Analysis

The typed terminals are produced in `search` and feed replan; no `validate_*`/precondition is modified, but terminal ordering and replan routing change.

1. `get_affordances` — N/A (no precondition change).
2. `generate_candidates` — N/A.
3. `search_plan` — **flag**: typed terminals replace `ProgressBarrier` in `search/transition.rs` + `search/mod.rs`; verify terminal ordering and barrier logic (D3).
4. `BestEffort` action start — N/A (no synthesized-payload action added).
5. `handle_plan_failure` — **flag**: resumption (D5) + barrier→failure-attribution routing (D6) change replan behavior; verify the `agent_tick` replan path handles each typed terminal.
6. Payload revalidation — N/A.
7. Golden tests — **flag**: D11 must pass under the `golden_ai` target; full `cargo test -p worldwake-ai` must pass after the `ProgressBarrier` migration.

## Test Plan

- D11 golden coverage (6 scenarios above).
- Determinism: same resume condition + same belief update → same suffix retry.
- Save/load coverage for `PartialPlanSegment` against current format version 92, with explicit rejection of S149PARPLASEG-001 baseline version 91 at the save-header boundary.
- `cargo test -p worldwake-ai` clean after the `ProgressBarrier` migration (D3).
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
