# S141: Motive Source Ledger and Desire Tokens

**Status**: Draft

## Summary

Today's goal ranking is a derived utility computation: emitters produce `GoalOffer`s with attached evidence references; `ranking::compare_ranked_goals` (`crates/worldwake-ai/src/ranking.rs`, made file-private by S123) reads needs, drives, learned opportunities, source reliability, and per-agent profile weights to produce a `motive_score: u32`. The score becomes the cross-goal ordering authority. The motive *source* — whether the agent committed because they're hungry, indebted, fearful, loyal to an office, or vengeful — is implicit in the score's components, not first-class state. Per FND-3 (concrete state over abstract scores), the architectural shape should be inverted: motive sources are the authoritative state; ranking is a derived view over motive sources, not a free-floating utility number.

S141 lands the `MotiveSource` enum and `DesireToken` carrier the assessor proposed. Each `GoalOffer` carries one or more `MotiveSourceRef`s naming the per-agent state that gives the goal weight: `NeedPressure(HomeostaticNeedId)`, `Pain(WoundId)`, `Fear(ThreatBeliefId)`, `Obligation(ContractId)`, `OfficeDuty(OfficeId, DutyId)`, `Debt(DebtId)`, `Loyalty(EntityId)`, `Greed(OpportunityId)`, `Habit(HabitId)`, `Curiosity(HypothesisId)`, `Shame(ReputationRecordId)`, `Revenge(ViolationId)`. `compare_ranked_goals` continues to produce the `motive_score: u32` (per S123, no parallel comparator), but the score becomes a *function over the motive sources* — derived, inspectable, traceable to specific per-agent state. The agenda manager (S115) consumes `DesireToken`s alongside the existing `AgendaEntry`. S136's `decisive_*` event-payload extensions reference the load-bearing motive sources rather than abstract score deltas.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — adds `motive_source` module owning `MotiveSource` enum, `MotiveSourceRef`, `DesireToken`. Extends `GoalOffer` (existing) with `motive_sources: SmallVec<MotiveSourceRef, 4>`. `Habit`, `Curiosity`, `Shame`, `Revenge` source variants reference per-agent state types either already present (`HomeostaticNeedId`, `WoundList`, `ContractId`, existing `OfficeAuthority`) or introduced as small typed wrappers without new authoritative state (a `RevengeRef` references an existing `ViolationRecord`).
- `worldwake-ai` — extends `ranking.rs` to derive `motive_score` as a function over `motive_sources`. `compare_ranked_goals` (file-private per S123) gains a per-`MotiveSource`-class contribution dispatch. Decision-trace `RankedGoalContext` records each motive source's contribution to the score for the chosen and top rejected goals. `agenda_manager.rs` carries `DesireToken` alongside existing `AgendaEntry`.
- `worldwake-systems` — no change. Existing systems' state already corresponds 1:1 to motive sources.
- `worldwake-cli` — observer Section 4 (Goals) renders motive sources per commit.

## Dependencies

- S112 (Portfolio Planning) — completed. The portfolio's three slots (Survival/Commitment/Economic) are aggregations over motive-source classes; S141 makes the aggregation explicit.
- S115 (Agenda Manager) — completed. `AgendaEntry` extended with `desire_token: DesireToken`.
- S123 (Preference Ordering Authority) — completed. `compare_ranked_goals` remains the single comparator. S141 changes its *internals* (read motive sources, derive score), not its identity.
- S136 (Decision Event Payload Extension) — completed and archived at `archive/specs/S136-decision-event-payload-extension.md`. Soft dependency satisfied: because S136 landed first, S141 owns adding `decisive_motive_sources: SmallVec<MotiveSourceRef, 4>` to the always-on payload.
- S107, S130, S131 (existing learning state) — `LearnedOpportunityMemory`, `SurveyMemory`, `SourceReliability` continue to feed motive contributions; S141 does not duplicate them.

## Design Goals

1. **Motive sources are concrete state references.** Each variant of `MotiveSource` references existing per-agent state via typed ID. No score lives in the source — the source names *what* drives the goal; the *strength* derives from reading the referenced state.
2. **`motive_score` becomes a derived view.** Existing comparator unchanged in identity; its body changes from "read needs/drives/profiles directly and produce a number" to "read motive sources from the goal offer, dispatch per-class scoring, sum into a number." The number is the same kind of `u32` it was before; the *provenance* is now traceable.
3. **Per-class scoring weights are profile-driven.** Existing `UtilityProfile` (per-agent) gains explicit per-`MotiveSource`-class weights. The total weighted sum produces the score deterministically.
4. **Decision-trace shows source contributions.** Each ranked candidate's trace records `(MotiveSourceRef, contribution_score)` pairs for inspection. Observer Section 4 surfaces this.
5. **No new authoritative state.** All `MotiveSource` variants reference existing state. `Greed(OpportunityId)` references the opportunity from S138 (or pre-S138, the existing `OpportunityKey`). `Habit(HabitId)` reuses existing `HabitMemory` (a placeholder type pre-merge — see Risks).
6. **Determinism.** `SmallVec<MotiveSourceRef, 4>` iteration is insertion-ordered; per-class scoring is a fixed dispatch; total score is integer arithmetic.
7. **Backward-compat-free migration.** Goal offers without explicit motive sources are *invalid* post-S141. All existing emitters in `candidate_generation.rs` are updated in the same change to attach motive sources. No fallback path.
8. **No silent privilege.** Motive sources do not invoke other systems; they are pure references to existing per-agent state read at scoring time.

## Non-Goals

- **A separate `DesireToken` lifecycle distinct from `GoalOffer`.** `GoalOffer` already carries the lifecycle (offered → ranked → committed → fulfilled / abandoned). `DesireToken` is the conceptual name for "a `GoalOffer` plus its motive sources"; the runtime type is the extended `GoalOffer`.
- **A `MotiveSource` source-of-truth refactor.** The references' targets (`HomeostaticNeedId`, etc.) remain authoritative wherever they currently live. S141 only adds the reference layer.
- **Cross-agent motive sharing.** `Loyalty(EntityId)` references a per-agent loyalty target; cross-agent loyalty propagation is out of scope.
- **A new event tag.** Motive sources are payload data on existing decision events.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `motive_score` becomes a derived view over concrete per-agent state references rather than a free-floating numeric truth. The comparator continues to use the score; the score's provenance is now inspectable. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | "Agent X chose Y because they cared about Z" becomes literally inspectable: the goal commits with `motive_sources: [NeedPressure(Hunger), Habit(MarketRoutine)]`. |
| FND-22 (Agent Diversity Through Concrete Variation) | Two agents with identical state but different `UtilityProfile` per-class weights rank the same motive sources differently. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | `Habit(HabitId)` references concrete habit state; preference shifts manifest as habit-strength changes that propagate naturally to the derived score. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Motive sources are state references read at scoring time, not cross-system commands. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `motive_score` is explicitly a derived summary; deleting it and recomputing from motive sources produces the same value. |
| FND-29A (Causal History Is Authoritative, Append-Only, and Queryable) | Decision events carry the motive-source references; history reconstructs the *why* across ticks. |

## Deliverables

### `worldwake-core::motive_source` (new module)

```rust
pub enum MotiveSource {
    NeedPressure { need: HomeostaticNeedKind },
    Pain { wound: WoundId },
    Fear { threat: ThreatBeliefId },
    Obligation { contract: ContractId },
    OfficeDuty { office: EntityId, duty: DutyId },
    Debt { debt: DebtId },
    Loyalty { other: EntityId },
    Greed { opportunity: OpportunityKey },
    Habit { habit: HabitId },
    Curiosity { hypothesis: HypothesisId },
    Shame { reputation_record: EntityId },
    Revenge { violation: ViolationId },
}

pub struct MotiveSourceRef {
    pub source: MotiveSource,
    pub introduced_tick: Tick,
}
```

### `GoalOffer` extension

```rust
pub struct GoalOffer {
    // existing fields preserved (goal_kind, payload, evidence_entities, ...)
    pub motive_sources: SmallVec<MotiveSourceRef, 4>,    // NEW (required)
}
```

Required, non-empty post-S141. A debug-assertion in test builds catches empty `motive_sources` at offer construction. The conformance test `every_goal_offer_has_motive_sources()` enforces it across all emitters.

### `ranking.rs` derivation refactor

```rust
fn motive_score(
    offer: &GoalOffer,
    agent_state: &AgentScoringState,
    profile: &UtilityProfile,
) -> u32 {
    offer
        .motive_sources
        .iter()
        .map(|src| score_motive_source(src, agent_state, profile))
        .sum()
}

fn score_motive_source(
    src: &MotiveSourceRef,
    agent_state: &AgentScoringState,
    profile: &UtilityProfile,
) -> u32 {
    match src.source {
        MotiveSource::NeedPressure { need } => 
            score_need_pressure(agent_state.needs.pressure(need), profile.need_weight(need)),
        MotiveSource::Pain { wound } => 
            score_pain(agent_state.wounds.severity(wound), profile.pain_weight),
        // ... per-class scoring
    }
}
```

`compare_ranked_goals` continues to call `motive_score` and order by it. The comparator is unchanged; its body is partitioned.

### `UtilityProfile` extension

```rust
pub struct UtilityProfile {
    // existing per-need weights (eat, drink, wash, sleep, ...)
    pub pain_weight: Permille,                     // NEW
    pub fear_weight: Permille,                     // NEW
    pub obligation_weight: Permille,               // NEW
    pub office_duty_weight: Permille,              // NEW
    pub debt_weight: Permille,                     // NEW
    pub loyalty_weight: Permille,                  // NEW
    pub greed_weight: Permille,                    // NEW
    pub habit_weight: Permille,                    // NEW
    pub curiosity_weight: Permille,                // NEW
    pub shame_weight: Permille,                    // NEW
    pub revenge_weight: Permille,                  // NEW
}
```

Per FND-22, two agents differ on these weights. The conformance test `utility_profile_default_for_motive_class()` ensures every class has a default.

### Decision-trace extension

`RankedGoalContext` (in decision_trace.rs) gains:

```rust
pub struct RankedGoalContext {
    // existing fields
    pub motive_source_contributions: SmallVec<(MotiveSourceRef, u32), 4>,    // NEW
}
```

### Observer Section 4

Render motive-source contributions per commit:
```
Tick 412 — Agent A — GoalCommitted: Eat (motive 18420)
  motive sources:
    NeedPressure(Hunger) → 14200 (need_weight=750, pressure=950)
    Habit(MarketRoutine) → 4220 (habit_weight=200, strength=420)
```

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** No new path. Motive sources are read references to existing per-agent state. The references propagate through the existing decision-event surface (S110 + S136).
2. **Positive-feedback analysis.** No amplification. The score is a deterministic sum over fixed-cardinality motive sources per offer.
3. **Concrete dampeners.** Not applicable.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state**: `MotiveSourceRef`s carried inside `GoalOffer` (which is already authoritative through the event log per S110).
   - **Derived read-model**: `motive_score` (existing); per-source contribution scores in decision trace.

## SystemFn Integration

No new `SystemFn`. Motive-source population happens at the existing emitter call sites in `candidate_generation.rs`; scoring happens in the existing ranking pass.

## Component Registration

No new ECS components. `MotiveSource` variants reference existing components (`HomeostaticNeeds`, `WoundList`, `OfficeAuthority`, etc.) that are already registered.

## Cross-System Interactions

- **AI ↔ Core**: emitters and ranking read existing per-agent state through the existing belief-view facade and ECS reads. Motive sources are state references, not state.
- **AI → Sim**: events emit through existing decision-event paths (S110 + S136).
- **Sim → CLI**: observer reads decision-trace.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

`UtilityProfile` (per-agent) gains 11 new `Permille` fields, one per non-need motive class. All must have defaults; per FND-22, scenarios opt into per-agent variation. The `ProfileHomogeneity` lint (S111) extends to detect cloned utility profiles across the new fields.

## Validation and Falsification

- **Golden coverage**: new `golden_motive_sources.rs` with five scenarios:
  1. Hunger-only commit → expects `motive_sources: [NeedPressure(Hunger)]` and contribution score == previous-pre-S141 motive score (parity).
  2. Hunger + Habit commit → expects two motive sources, sum-equals-score, observer renders both.
  3. Pain dominates Hunger under wound profile → expects `Pain(...)` contribution > `NeedPressure(Hunger)` contribution.
  4. `UtilityProfile.greed_weight` variation across two otherwise-identical agents → expects different commit choices for the same opportunity.
  5. Empty `motive_sources` debug-assert in test build → expects panic at offer construction.
- **Score parity regression**: every existing 1440-tick survival golden produces identical `motive_score` values pre/post-S141 for every commit (the score is the same, the provenance is the new layer). This is the strongest regression guard against derivation drift.
- **Conformance**: `every_goal_offer_has_motive_sources()` test fails on any emitter that constructs a `GoalOffer` without motive sources.

## Risks

- **`HabitId` placeholder.** Phase 11 has no committed `Habit` substrate; PR-21 (concrete learning state) was rejected. Mitigation: `Habit` motive source is implemented but `HabitId` is a stub type that always returns 0 contribution from `score_motive_source`. The variant is reserved for Phase 12 HTN/learning work; goldens do not exercise it.
- **Migration scope.** Every `emit_*` function in `candidate_generation.rs` must populate motive sources. Mitigation: ticket-001 audits every emitter and lands a default mapping (e.g., need-driven emitters always emit `NeedPressure`); ticket-002 adds the test-build conformance assertion.
- **Score drift.** The derivation refactor must produce bitwise-identical motive scores to preserve goldens. Mitigation: per-class scoring functions are direct extractions of the current `compare_ranked_goals` body; ticket-003 lands a dedicated motive-score parity test that exhaustively compares pre/post values across the soak harness.
- **`UtilityProfile` save-format growth.** 11 new `Permille` fields. Mitigation: `#[serde(default)]` on each; `SAVE_FORMAT_VERSION` increments by one for the schema bump.
