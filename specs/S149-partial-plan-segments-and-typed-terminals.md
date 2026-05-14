# S149: Partial Plan Segments and Typed Plan Terminals

**Status**: Draft

## Summary

Folds in PR-10 (Partial plans as first-class objects) and PR-5 (Information barriers as first-class plan outcomes) from `reports/ai-architecture-improvements.md`.

The current planner has three `PlanTerminalKind` variants at `crates/worldwake-ai/src/planner_ops.rs:387-391`: `GoalSatisfied`, `ProgressBarrier`, `CombatCommitment`. Every non-success path terminates as `ProgressBarrier` — the planner cannot distinguish "I lack information about the target's location" from "the resource is depleted" from "another agent holds the reservation" from "I lack jurisdiction." The assessment proposes seven typed terminals: `GoalSatisfied`, `ProgressBarrier`, `InformationBarrier`, `CoordinationBarrier`, `ResourceBarrier`, `JurisdictionBarrier`, `SafetyBarrier`. S139 added `GoalKind::AskWitness` as a sensing *goal*, but the planner cannot express "I made partial progress and stopped at an information barrier — when I learn fact F, I can resume."

S149 ships both as the same architectural layer: typed terminal barriers and first-class `PartialPlanSegment` storage. When a plan attempts goal G and reaches a typed barrier B, the planner stores a `PartialPlanSegment` carrying the prefix steps that did succeed, the barrier type, the resume conditions that would clear the barrier, and the abandon conditions that would invalidate the partial plan. The agenda manager (S115) gains the ability to resume a suspended intention from its `PartialPlanSegment` when its resume conditions hold, picking up at the prefix-tail rather than replanning from scratch.

The typed barriers map to typed `Discrepancy` variants (S109) so the existing failure-handling pipeline absorbs them uniformly. `InformationBarrier` ⇒ `Discrepancy::MissingObservation`; `CoordinationBarrier` ⇒ `Discrepancy::ReservationConflict`; `ResourceBarrier` ⇒ `Discrepancy::BeliefStale`; `JurisdictionBarrier` ⇒ `Discrepancy::NoLegalBinding`; `SafetyBarrier` ⇒ `Discrepancy::NeedHorizonExceeded` (when danger projection is added).

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — extends `PlanTerminalKind`, adds `PartialPlanSegment` storage on `AgendaEntry`, extends agenda manager resumption.
- `worldwake-core` — adds `PartialPlanSegment` carrier type and `ResumeCondition`/`AbandonCondition` types (shared with S148).
- `worldwake-sim` — no change.
- `worldwake-systems` — no change.
- `worldwake-cli` — observer renders barrier type per terminal; S144 diagnostics aggregate barrier-kind distribution.

## Dependencies

- S88 (Two-Phase Landmark Planning, archived) — provides the strategic + tactical search the typed terminals attach to.
- S109 (Typed Discrepancy Taxonomy, archived) — barriers map to existing `Discrepancy` variants.
- S115 (Agenda Manager, archived) — manages suspended intentions; S149 adds partial-plan-aware resumption.
- S132 (Frontier-Exhaustion Strategy as Goal-Kind Property, archived) — current generic `ProgressBarrier` dispatch; S149 refines.
- S137 (Plan Causal Links and Localized Repair, archived) — provides repair substrate; barrier-typed plans guide repair scope.
- S139 (AskWitness Goal Layer, archived) — provides epistemic-sensing goals that `InformationBarrier` resume conditions can produce.
- S148 (Portfolio Slot Expansion, Phase 12) — `IntentionFrame.resume_conditions` / `abandon_conditions` types shared.

## Design Goals

1. **Every plan terminal carries diagnostic shape.** Observer and S144 can answer "what stopped this plan?" with one of seven specific types, not a generic `ProgressBarrier`.
2. **Partial plans resume, don't restart.** When the resume condition holds, the agenda manager continues from the partial-plan suffix.
3. **No barrier is silent.** Each barrier type produces a typed `Discrepancy` per S109, flowing through the existing blocker/repair pipeline.
4. **Backward-compat-free.** The generic `ProgressBarrier` becomes a vanishing case; specific barrier types replace it. No alias.
5. **Deterministic resumption.** Same resume condition + same belief update → same suffix retry.
6. **Bounded resume retries.** A partial plan that re-fails its tail enters `AbandonCondition::PatienceExhausted` per S148.

## Non-Goals

- **No method-decomposition resumption.** S147's `MethodSchema` already supports method-driven decomposition; S149 only handles flat tactical-prefix resumption.
- **No cross-tick search continuation.** That is PR-14 (rejected as YAGNI).
- **No new event tag for barrier transitions.** Existing `PlanFinalized` / decision-event payloads carry the typed terminal.
- **No invariants beyond per-goal.** A `PartialPlanSegment` is owned by its `IntentionFrame`; no shared partial-plan pool.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Typed barriers replace a single abstract terminal label; partial plans store concrete prefix steps. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | `InformationBarrier` makes ignorance an explicit plan outcome rather than a planning failure. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Partial plans express *how* the agent partially advanced; the typed barrier expresses *what is blocking* further progress. |
| FND-21 (Intentions Are Revisable Commitments) | Resume/abandon conditions on the segment make every partial intention explicitly revisable. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Partial-plan resumption reads belief-view state to evaluate resume conditions; no cross-system command. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | Generic `ProgressBarrier` is removed; specific barriers replace it. |
| FND-29 (Debuggability Is a Product Feature) | Observer Section 7 surfaces per-attempt barrier types; S144 aggregates `terminal_kind_distribution`. |

## Deliverables

### D1: Typed `PlanTerminalKind`

```rust
// crates/worldwake-ai/src/planner_ops.rs (replacing existing 3-variant enum)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PlanTerminalKind {
    GoalSatisfied,
    CombatCommitment,
    InformationBarrier { topic: InformationGapTopic },
    CoordinationBarrier { contested_resource: EntityId },
    ResourceBarrier { commodity: CommodityKind, place: EntityId },
    JurisdictionBarrier { authority: EntityId, jurisdiction: EntityId },
    SafetyBarrier { hazard: SafetyHazard },
    SearchBudgetExhausted { budget_consumed: u16, budget_total: u16 },
}
```

The legacy generic `ProgressBarrier` is removed. `SearchBudgetExhausted` is the new explicit "I ran out of search budget" terminal; the previous `ProgressBarrier` cases that meant "budget out" now report this explicitly. The other typed barriers replace the cases that meant "missing fact" / "contested" / "depleted" / "out of authority" / "unsafe."

### D2: `PartialPlanSegment`

```rust
// crates/worldwake-core/src/partial_plan.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PartialPlanSegment {
    pub id: PartialPlanSegmentId,
    pub goal: GoalOffer,
    pub completed_prefix: Vec<PlannedStep>,
    pub remaining_skeleton: Option<Vec<PlannedSkeletonStep>>,
    pub terminal_barrier: PlanTerminalKind,
    pub barrier_fact: BarrierFact,
    pub resume_conditions: Vec<ResumeCondition>,    // shared with S148
    pub abandon_conditions: Vec<AbandonCondition>,  // shared with S148
    pub created_tick: Tick,
    pub last_resume_attempt_tick: Option<Tick>,
    pub resume_attempt_count: u8,
    pub causal_links: Vec<EventId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedSkeletonStep {
    pub op: PlannerOpKind,
    pub target_template: PayloadTemplate,
    pub expected_pre: Vec<PreconditionPredicate>,
}

pub enum BarrierFact {
    MissingBelief(BeliefPredicate),
    ContestedReservation(EntityId),
    DepletedResource { commodity: CommodityKind, place: EntityId },
    NoAuthorityForAction(EntityId),
    HazardPresent(SafetyHazard),
    BudgetExhausted { remaining_stages: u16 },
}
```

`PartialPlanSegmentId` is a typed newtype with `Tick`-and-counter provenance for deterministic ID assignment.

### D3: Agenda-manager resumption path

```rust
// crates/worldwake-ai/src/agenda_manager.rs (extended)
fn try_resume_partial_plan(
    state: &mut AgendaState,
    actor: EntityId,
    belief_view: &dyn RuntimeBeliefView,
    tick: Tick,
) -> Option<ResumedPlan>;
```

For each `Suspended` intention with a `PartialPlanSegment`:
1. Check `abandon_conditions` — if any holds, abandon the intention and clear the segment.
2. Check `resume_conditions` — if any holds, return the segment for retry.
3. Otherwise leave suspended.

A returned `ResumedPlan` re-enters the planner at the tactical phase with the `completed_prefix` already applied to the planning state. The tactical planner attempts to complete the `remaining_skeleton` against the new world state.

`PartialPlanSegment.resume_attempt_count` increments per try; when it exceeds `IntentionFrame.patience_limit` (per S148), the segment is abandoned (`AbandonCondition::PatienceExhausted`).

### D4: Barrier → Discrepancy mapping

```rust
fn terminal_to_discrepancy(terminal: &PlanTerminalKind) -> Option<Discrepancy> {
    match terminal {
        PlanTerminalKind::GoalSatisfied => None,
        PlanTerminalKind::CombatCommitment => None,
        PlanTerminalKind::InformationBarrier { .. } => Some(Discrepancy::MissingObservation),
        PlanTerminalKind::CoordinationBarrier { .. } => Some(Discrepancy::ReservationConflict),
        PlanTerminalKind::ResourceBarrier { .. } => Some(Discrepancy::BeliefStale),
        PlanTerminalKind::JurisdictionBarrier { .. } => Some(Discrepancy::NoLegalBinding),
        PlanTerminalKind::SafetyBarrier { .. } => Some(Discrepancy::NeedHorizonExceeded),
        PlanTerminalKind::SearchBudgetExhausted { .. } => Some(Discrepancy::SearchBudgetExhausted),
    }
}
```

The agenda manager records the discrepancy through the existing `DiscrepancyMemory` path (S109). Resume conditions are derived from the barrier fact:
- `MissingBelief(pred)` → `ResumeCondition::BeliefUpdated(pred)`.
- `ContestedReservation(target)` → `ResumeCondition::ArtifactValid(target)` once the existing reservation invalidator clears.
- `DepletedResource { place, commodity }` → `ResumeCondition::BeliefUpdated(commodity_available_at(place))`.
- `NoAuthorityForAction(actor)` → `ResumeCondition::BeliefUpdated(authority_holds(actor))`.
- `HazardPresent(hazard)` → `ResumeCondition::BeliefUpdated(hazard_cleared(hazard))`.
- `BudgetExhausted` → `ResumeCondition::TickElapsed(cognitive.budget_cooldown_ticks)`.

### D5: Information-barrier subgoal synthesis

When a plan terminal is `InformationBarrier { topic }`, the agenda manager spawns an auxiliary `GoalKind::AskWitness { topic, ... }` (S139 substrate) as a *companion intention* slot-typed `SocialEpistemic` (S148). The companion intention is owned by the suspended primary intention; abandoning the primary cancels the companion. Successful AskWitness commit updates the agent's belief store; the resume condition on the suspended primary fires.

### D6: Coordination-barrier queue listening

When a plan terminal is `CoordinationBarrier { contested_resource }`, the agenda manager adds the suspended intention to a per-actor "watching" list keyed on `contested_resource`. Existing `ContentionGrant` invalidation events (S140 lifecycle) check the watching list and emit `ResumeCondition::ArtifactValid` triggers. No new event tag.

### D7: Observer rendering

Observer Section 7 (planning) extends to print:
```
Plan terminal: ResourceBarrier(commodity=Grain, place=ThornwallMarket#42)
  Barrier fact: DepletedResource — observed stock = 0 at tick 1247
  Resume on: BeliefUpdated(commodity_available_at(ThornwallMarket#42, Grain))
  Abandon if: PatienceExhausted (3 resume attempts left)
```

S144's `PlanningMetrics.terminal_kind_distribution` gains all seven typed kinds.

### D8: Partial-plan storage

`AgendaEntry` (`crates/worldwake-ai/src/agenda_types.rs:22`) gains:

```rust
pub partial_plan_segment: Option<PartialPlanSegment>,
```

`AgendaState.suspended` map stores the suspended `AgendaEntry`s; segments persist with their entries. Save/load coverage extends; `SAVE_FORMAT_VERSION` bumps (latest version visible in the codebase plus 1).

### D9: Golden coverage

`golden_typed_plan_terminals.rs` covers:
- `InformationBarrier` end-to-end: agent lacks target location → barrier raised → companion AskWitness commits → primary intention resumes → completion.
- `CoordinationBarrier`: agent loses oven reservation → barrier raised → grant invalidated by holder → resume after new grant.
- `ResourceBarrier`: market depleted → barrier raised → resupply observed → resume.
- `JurisdictionBarrier`: agent attempts arrest outside jurisdiction → barrier raised → travel to jurisdiction → resume.
- `SafetyBarrier`: dangerous travel route → barrier raised → danger cleared → resume.
- `SearchBudgetExhausted`: budget runs out → typed terminal → budget-cooldown TTL → resume.
- Abandon-condition flow: patience-exhausted abandons; observer surfaces the abandonment.

## FND-01 Section H Analysis

### Information-Path Analysis

`PartialPlanSegment.resume_conditions` evaluate against existing belief-view reads. The companion `AskWitness` intention (D5) follows S139's existing testimony-acquisition path. `ContentionGrant` invalidation events (D6) flow through S140's existing lifecycle path. No new world-information path.

### Positive-Feedback Analysis

Potential loop: failed resume → retry → fails again → retry. Bounded by `IntentionFrame.patience_limit` (per S148) and `PartialPlanSegment.resume_attempt_count` cap.

### Concrete Dampeners

- `patience_limit` per S148.
- `resume_attempt_count` per-segment cap.
- `AbandonCondition::PatienceExhausted` is the lawful exit path.

### Stored State vs. Derived Read-Model List

**Stored state**:
- `PartialPlanSegment` (in `AgendaState.suspended[].partial_plan_segment`).
- `PlanTerminalKind` enum (extended; no new authoritative storage, but typed-terminal data persists in decision history payloads).

**Derived read-model**:
- Resume / abandon evaluation per-tick.

## SystemFn Integration

No new top-level `SystemFn`. Resumption runs inside the agenda manager's existing tick pass.

## Component Registration

No new ECS component. `PartialPlanSegment` lives within the existing `AgendaState` runtime structure on the agent.

## Cross-System Interactions

- Agenda manager (S115) consults `AgentBeliefStore` (S101/S113) to evaluate resume conditions.
- Partial plans reference `ContentionGrant` (S140) lifecycle invalidation.
- Typed terminals produce S109 `Discrepancy`s through existing failure-handling.

State-mediated.

## Profile-Driven Parameters

`CognitiveProfile.budget_cooldown_ticks` (new, default `60`) — how long after a `SearchBudgetExhausted` terminal before the agent re-attempts the goal.

## Test Plan

- D9 golden coverage (7 scenarios above).
- Determinism: same resume condition + same belief update → same suffix retry.
- Save/load coverage for `PartialPlanSegment`.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
