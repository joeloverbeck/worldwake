# S112: Portfolio Planning with Feasibility Probes

## Summary

Replace flat top-N candidate selection (`max_candidates_to_plan`, default 2) with a small *portfolio* — a diversified agenda slice composed of: the best urgent survival goal, the best current commitment or obligation, and the best feasible background economic goal. Before committing full tactical search budget, each slot runs a cheap feasibility probe (belief-grounded target reachability, BestEffort affordance existence check, discrepancy-memory filter). Slots that fail the probe are dropped; remaining slots lead the search order by priority class first and score-weighted slot priority second, with later admitted ranked opportunities still eligible behind those slot winners until the normal search-attempt cap stops the pass. This prevents the pathology where the top two candidates are infeasible but the third is trivial — today the agent wastes a tick and looks stuck.

An information-gathering slot is *deferred to S113*: that slot requires a per-belief confidence/freshness envelope accessor that does not yet exist. Once S113 lands, S112's portfolio gains the information slot as a follow-up.

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-ai` — `agent_tick/planning.rs` portfolio assembly; `ranking.rs` slot-aware scoring; new `feasibility_probe.rs`; subsumes `prioritize_same_goal_replan_candidates` into the commitment slot
- `worldwake-core` — `CognitiveProfile` fields for slot weights

## Dependencies

- S109 (Typed Discrepancy Taxonomy) — **landed** (`archive/specs/S109-typed-discrepancy-taxonomy.md`). The feasibility probe reads the existing `DiscrepancyMemory` (`crates/worldwake-core/src/discrepancy.rs`) and `BlockerMemory` (`crates/worldwake-core/src/blocker_memory.rs`) directly; no split workaround is needed.
- S110 (Decision History Events) — **landed** (`archive/specs/S110-decision-history-events.md`). The existing `GoalRejectionReason::FeasibilityProbeFailed` variant (`crates/worldwake-core/src/decision_event_payload.rs:96`, currently unused) is the rejection reason S112 populates.
- S113 (Belief Envelope) — **soft forward dependency for the information slot only**. S112 ships with three slots (survival / commitment / economic). The fourth (information) is deferred until S113 lands the per-belief confidence accessor.

## Design Goals

- Candidate selection is *diverse*, not just top-scored. The portfolio always considers multiple categories so a single high-score goal class (e.g., saturated `PostNotice` obligations pre-S96) does not occupy all search budget.
- Infeasibility is caught cheaply, before the full tactical search. A portfolio slot that fails its feasibility probe never consumes tactical search budget.
- Deterministic. Slot assembly, probe order, and fallback ordering are all `BTreeMap`-driven via an `Ord`-deriving `SlotKind` key.
- `max_candidates_to_plan` retains its role as a *search-attempt cap*: it bounds the number of plausible slots that consume tactical search budget. A value of `1` does not disable the portfolio — it simply caps searched slots at one. This preserves FND-28 (no parallel live authority paths).

## Non-Goals

- Parallel search across portfolio slots. Slots are searched sequentially in score-weighted order; the first slot that produces a plan wins.
- PolicyPlan branching, contingent plans, or per-anchor retry of a failed commitment — deferred to a Phase 9 spec (S114 lands the step-guard substrate that branching will build on).
- Replacing the `max_candidates_to_plan` field. Portfolio is an enhancement to candidate *selection*, not a replacement for the search-budget cap.
- The information-gathering slot. Shipped as a follow-up once S113's belief envelope provides the per-belief confidence accessor.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-20 (Resource-Bounded Practical Reasoning) | Feasibility probes burn cheap tokens before committing to expensive tactical search. An agent finds *a* feasible plan within budget instead of burning budget on the top two infeasible options. |
| FND-21 (Intentions Are Revisable Commitments) | The commitment slot picks `committed_opportunity` explicitly when it is still ranked, so commitments persist across ticks unless the goal is no longer ranked or the slot's feasibility probe rejects it. Margin-based commitment (S74) still decides whether to keep the commitment once it wins the slot. |
| FND-22 (Agent Diversity Through Concrete Variation) | Slot weights per `CognitiveProfile` allow one agent to prioritize the survival slot, another the commitment or economic slot. Two agents with the same motives can differ on portfolio shape. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | Portfolio assembly always runs; there is no bypass branch for `max_candidates_to_plan = 1`. The replaced `prioritize_same_goal_replan_candidates` clustering is subsumed by the commitment slot — only one live candidate-selection path remains. |

## Deliverables

### D1: `Portfolio` and `SlotKind` types

New types in `crates/worldwake-ai/src/agent_tick/portfolio.rs`:

```rust
/// Ordered category key for portfolio slots.
/// Derive order dictates deterministic iteration; Economic > Survival lexical
/// order would be incorrect — ordering is by category priority when scores tie.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum SlotKind {
    Survival,
    Commitment,
    Economic,
    // Information — reserved; populated once S113 lands the belief envelope.
}

/// A small diversified slice of agenda candidates assembled per tick.
/// Slots may be absent from the map if no candidate of that category is
/// available this tick.
pub(crate) struct Portfolio {
    pub slots: BTreeMap<SlotKind, PortfolioSlot>,
}

pub(crate) struct PortfolioSlot {
    pub ranked: RankedGoal,
    pub feasibility: FeasibilityVerdict,
}

pub(crate) enum FeasibilityVerdict {
    /// Probe passed — proceed to full tactical search.
    Plausible,
    /// Probe failed for a reason the agent already knows
    /// (discrepancy memory hit, no known target, no reachable affordance).
    RejectedBeforeSearch { reason: Discrepancy },
}
```

`Portfolio::plausible_slots_by_score(&self, weights: &PortfolioSlotWeights) -> Vec<(SlotKind, &PortfolioSlot)>` returns only `FeasibilityVerdict::Plausible` slots, ordered by `weighted_score = u32::from(slot.ranked.motive_score).saturating_mul(u32::from(weight_for(slot_kind).value())) / 1000`, descending. Ties break by `SlotKind`'s derived `Ord` (Survival > Commitment > Economic).

### D2: Slot categorization

Categorization rules (evaluated at slot-assembly time, reading from already-ranked candidates):

- **Survival slot**: `GoalKind::ConsumeOwnedCommodity { Survival* }`, `AcquireCommodity { purpose: SelfConsume }`, `Sleep`, `Relieve`, `Wash`, `TreatWounds { patient == self }`, `ReduceDanger`, `FreeCarryCapacity`. `FreeCarryCapacity` is categorized unconditionally — emission (`crates/worldwake-ai/src/goal_model.rs:468-543`, `DisposalProfile`-gated) has already decided the agent is over its disposal threshold by the time the goal appears in the ranked list. Slot winner is the highest-motive survival candidate; ties broken by `GoalKey` order.
- **Commitment slot**: picks `committed_opportunity` explicitly when that `(goal_key, anchor)` pair is still present in the ranked list. Reads `committed_opportunity: Option<OpportunityKey>` already tracked by `AgentDecisionRuntime` (`crates/worldwake-ai/src/agent_tick/planning.rs:275, 993, 1026`). If the committed opportunity is no longer ranked, the commitment slot falls back to the highest-motive candidate whose kind is an obligation goal (`PostNotice`, `PostBounty`, `ReportMissing`, `ReportFound`, warrant-adjacent). If neither applies, the slot is absent from the `Portfolio::slots` map.
- **Economic slot**: `AcquireCommodity { purpose: Restock | RecipeInput(_) }`, `ProduceCommodity`, `SellCommodity`, `RestockCommodity`, `MoveCargo`, `EstablishBanditCamp`, faction-economic goals. If a candidate is also the survival or commitment slot winner, the economic slot picks the next-ranked economic candidate. Slot winner is the highest-motive remaining economic candidate.

If a goal ties between categories (e.g., `PostNotice` is both an obligation and — with a high weight — an economic goal), the earlier `SlotKind` variant wins. This is deterministic because `SlotKind` derives `Ord`.

The information slot is **not categorized in this spec**. Its `SlotKind` variant is reserved (commented out in D1) and added in the S113 follow-up once a per-belief confidence accessor exists.

This deliverable replaces the current `prioritize_same_goal_replan_candidates` pre-step (`crates/worldwake-ai/src/agent_tick/planning.rs`). The commitment slot now subsumes that **pre-search clustering** role by explicitly surfacing the current commitment when still ranked. The later `same_goal_trace` continuation contract documented in `docs/planner-contracts.md` remains live after portfolio admission determines the searched opportunity order. Multi-anchor retry beyond that admitted sequence is explicitly deferred to Phase 9 per Non-Goals.

### D3: Feasibility probe

New function in `crates/worldwake-ai/src/feasibility_probe.rs`:

```rust
pub(crate) fn probe(
    ranked: &RankedGoal,
    context: &ProbeContext<'_>,
) -> FeasibilityVerdict;
```

Probes are shallow:

1. **Discrepancy/blocker memory check**: if `DiscrepancyMemory::is_suppressed` returns true for the goal's blocker key or if `BlockerMemory::is_blocked` returns true, the slot is rejected with the recorded `Discrepancy` (for `DiscrepancyMemory` hits) or with `Discrepancy::PartialExecutionDrift` (mapping the `BlockerMemory` hit to the taxonomy). In the landed standalone probe, this blocker lookup is goal/anchor-scoped and uses `action_def: None` because root candidate expansion has not happened yet. No search budget consumed.
2. **Known-target check**: the goal must have at least one candidate target that the agent believes exists (from the agent's belief store via the existing `RuntimeBeliefView` surface). Goals whose anchors reference unknown targets are rejected with `Discrepancy::MissingObservation`. Goals whose target place is known but unreachable from the agent's current place (no known route in belief) are rejected with `Discrepancy::RouteUnknown`.
3. **Affordance existence check**: at least one affordance of the goal's action-kind must be believed-reachable from the agent's current place. Does not verify the full chain — only that the first step type is plausible. If no such affordance exists in belief, reject with `Discrepancy::MissingObservation`.

The probe does **not** run tactical search. It is O(candidates × belief-lookup), not O(search budget).

### D4: Portfolio-driven planning loop

`agent_tick/planning.rs` currently takes the top `max_candidates_to_plan` ranked goals and attempts to plan each in order (after a `prioritize_same_goal_replan_candidates` clustering pre-step at line 405). Replace with:

```rust
let portfolio = assemble_portfolio(
    &ranked_goals,
    committed_opportunity,
    |ranked| feasibility_probe::probe(ranked, &probe_context),
);
let plausible_slots = portfolio.plausible_slots_by_score(&cognitive.slot_weights);
let mut search_order = plausible_slots
    .iter()
    .map(|(kind, _)| portfolio.slots[kind].ranked.opportunity_key())
    .collect::<Vec<_>>();
for ranked in &ranked_goals {
    if !search_order.contains(&ranked.opportunity_key()) {
        search_order.push(ranked.opportunity_key());
    }
}
for opportunity in search_order
    .iter()
    .take(usize::from(cognitive.max_candidates_to_plan))
{
    match try_plan(opportunity, &planning_context) {
        PlanOutcome::Success(plan) => return Some(plan),
        PlanOutcome::Failure(_) => continue,
    }
}
None
```

`max_candidates_to_plan` bounds the *number of search attempts we actually run*, not the ranking depth. The assembled portfolio still leads the order, but weaker admitted candidates can be searched later in the same pass if they fit under the normal cap. The portfolio itself may hold up to 3 slots in this spec (survival / commitment / economic); `max_candidates_to_plan = 2` means "try at most 2 opportunities after the portfolio has led the search order." Among plausible slot winners, higher `GoalPriorityClass` still preempts lower-priority commitments before the weighted slot score tie-break applies.

Portfolio assembly always runs; there is no `max_candidates_to_plan = 1` bypass. The single-slot case is expressed naturally as "the top plausible slot is the only one searched."

The earlier draft mentioned a `record_blocker_or_discrepancy` helper in this loop. The live `plan_and_validate_next_step*` planning path does not classify search failures or mutate blocker/discrepancy memory here, so ticket 005 integrates portfolio admission, probe rejection tracing, and `FeasibilityProbeFailed` decision-history output without adding a second blocker-recording seam.

### D5: `CognitiveProfile` extensions

Add to `CognitiveProfile` (`crates/worldwake-core/src/cognitive_profile.rs`):

```rust
/// Relative weights by slot category when ordering plausible slots
/// for tactical search. Survival usually dominates; background
/// agents may weight economic higher.
#[serde(default)]
pub slot_weights: PortfolioSlotWeights,
```

New type in the same module:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PortfolioSlotWeights {
    pub survival: Permille,
    pub commitment: Permille,
    pub economic: Permille,
    // `information` reserved for S113 follow-up.
}

impl Default for PortfolioSlotWeights {
    fn default() -> Self {
        Self {
            survival: Permille::new_unchecked(1000),
            commitment: Permille::new_unchecked(900),
            economic: Permille::new_unchecked(700),
        }
    }
}
```

`slot_weights` uses `#[serde(default)]` so existing scenarios deserialize unchanged (existing scenarios fall back to the default above). `PortfolioSlotWeights` must be added to the `CognitiveProfile` round-trip and default-matches tests in the same module.

### D6: `PortfolioTrace` on `PlanningPipelineTrace`

The portfolio is assembled only during planning ticks (not active-action ticks). The trace therefore belongs on `PlanningPipelineTrace` (reached through `DecisionOutcome::Planning(Box<PlanningPipelineTrace>)` at `crates/worldwake-ai/src/decision_trace.rs:96`), not on the top-level `AgentDecisionTrace`.

Extend `PlanningPipelineTrace` with:

```rust
pub portfolio: Option<PortfolioTrace>,
```

`None` only when the tick is dead or no candidates ranked.

```rust
pub struct PortfolioTrace {
    pub slots: BTreeMap<SlotKind, PortfolioSlotTrace>,
    pub slots_attempted: u8,
}

pub struct PortfolioSlotTrace {
    pub goal_key: GoalKey,
    pub motive_score: u32,
    pub feasibility: FeasibilityVerdict,
}
```

`GoalRejectionReason::FeasibilityProbeFailed` already exists in `crates/worldwake-core/src/decision_event_payload.rs:96` and is currently unused. Ticket 004 lands only the staged `PortfolioTrace` sink on `PlanningPipelineTrace`; ticket 005 begins using that variant in `GoalCommittedPayload::rejected_alternatives` for slot winners rejected by the probe, surfacing probe verdicts on the authoritative decision-history event log (S110) alongside the populated `PortfolioTrace`. Later admitted fallback opportunities are still searchable in the same planning pass, but they do not create extra `PortfolioTrace` slots or `FeasibilityProbeFailed` summaries because they are not portfolio slots.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: The feasibility probe reads only from the agent's own `DiscrepancyMemory`, `BlockerMemory`, and belief store (via `RuntimeBeliefView`, plus the existing action-def, handler, and planner-semantics tables needed for affordance enumeration). It does not consult world state. Aligned with FND-14 (World State Is Not Belief State).
2. **Positive-feedback analysis**: A loop would exist if "probe fails → record discrepancy → next tick probe fails for same reason → record discrepancy again." S109 already provides typed TTLs per discrepancy class (stored on `DiscrepancyEntry::expires_tick`); the same `(blocker_key)` stays suppressed until its TTL expires, preventing re-recording each tick.
3. **Concrete dampeners**: S109's typed TTLs (`CognitiveProfile::*_backoff_ticks` fields) dampen retry loops. Probe budget is bounded: at most `max_candidates_to_plan × (number_of_slot_categories)` belief-store lookups per tick, independent of search budget.
4. **Stored state vs. derived read-model**: `Portfolio`, `PortfolioSlot`, `SlotKind`, and `FeasibilityVerdict` are transient per-tick derivations — no authoritative state is added beyond the profile fields in core. `PortfolioTrace` is recorded into the optional decision-trace sink; S110's `GoalCommittedPayload::rejected_alternatives` is the authoritative log entry. No new event types are emitted; rejection reasons reuse existing `GoalRejectionReason::FeasibilityProbeFailed`.

## SystemFn Integration

No new SystemFn. Portfolio assembly runs inline in the existing agent tick planning phase, replacing the current flat top-N loop and the `prioritize_same_goal_replan_candidates` clustering step.

## Component Registration

No new components. `CognitiveProfile` gains `slot_weights: PortfolioSlotWeights` with `#[serde(default)]` so existing scenarios remain valid without modification.

## Cross-System Interactions

- **Ranking ↔ portfolio assembly**: Ranking produces the `Vec<RankedGoal>` as today; portfolio assembly categorizes into `BTreeMap<SlotKind, PortfolioSlot>` and orders plausible slots by weighted score. State-mediated.
- **S109 discrepancy memory ↔ feasibility probe**: Probe reads `DiscrepancyMemory::is_suppressed` and `BlockerMemory::is_blocked`, writes nothing during the probe phase itself. State-mediated.
- **S110 event log ↔ rejected alternatives**: Slot rejections appear in `GoalCommittedPayload::rejected_alternatives` via the pre-existing `GoalRejectionReason::FeasibilityProbeFailed` variant.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `slot_weights.survival` | `CognitiveProfile::slot_weights` | `Permille` | `Permille::new(1000)` | Relative weight of survival slot when ordering plausible slots |
| `slot_weights.commitment` | `CognitiveProfile::slot_weights` | `Permille` | `Permille::new(900)` | Commitment/obligation slot weight |
| `slot_weights.economic` | `CognitiveProfile::slot_weights` | `Permille` | `Permille::new(700)` | Economic slot weight |

Once S113 lands, an `information: Permille` field (default `Permille::new(600)`) and an activation threshold will be added via the S113 follow-up; neither is part of S112's scope.

## Validation and Falsification

### Unit tests

1. Survival slot always picks the highest-motive survival goal; ties broken deterministically by `GoalKey` order.
2. Commitment slot picks `committed_opportunity` when still ranked, regardless of whether a higher-scoring obligation candidate exists this tick.
3. Commitment slot falls back to highest-motive obligation candidate when `committed_opportunity` is no longer ranked.
4. `AcquireCommodity { purpose: SelfConsume }` populates the survival slot while a separate restock or recipe-input acquisition remains economic.
5. `FeasibilityVerdict::RejectedBeforeSearch` is produced when `DiscrepancyMemory::is_suppressed` returns true for the goal's `BlockerKey`.
6. `plausible_slots_by_score` orders by `u32::from(motive_score).saturating_mul(u32::from(weight.value())) / 1000`, not raw score. Verify with a survival candidate scoring 500 × weight 1000 vs. economic 600 × weight 700: survival wins at 500 vs 420.
7. Portfolio assembly always runs (no `max_candidates_to_plan = 1` bypass); `max_candidates_to_plan = 1` results in at most one slot being searched but the full portfolio being assembled and traced.

### Integration tests

8. Scenario with 2 infeasible high-motive goals + 1 trivial feasible goal: pre-S112 agent wastes a tick on the infeasible pair; post-S112 agent probes, rejects both, and plans the trivial goal in the same tick.
9. Existing `survival-baseline.ron` and `survival-contested.ron` goldens (`crates/worldwake-ai/tests/golden_survival_baseline.rs`, `golden_survival_contested.rs`) pass unchanged — the portfolio with survival-dominant weights produces the same commit decisions the existing scenarios assert.

### Golden test

10. New scenario proof in `golden_portfolio_planning.rs`: Agent with (infeasible-high-motive goal A, infeasible-high-motive goal B, feasible-low-motive goal C). Agent commits goal C within 2 ticks. Pre-S112 regression baseline: without portfolio, agent would waste ≥5 ticks on A/B before reaching C. The rejected A and B appear in `GoalCommittedPayload::rejected_alternatives` with `GoalRejectionReason::FeasibilityProbeFailed`.

## Outcome

Landed in `worldwake-ai` as a portfolio-led planning loop integration. `prioritize_same_goal_replan_candidates` is removed, `PlanningPipelineTrace::portfolio` is populated during planning ticks, and probe-rejected portfolio slots now surface `GoalRejectionReason::FeasibilityProbeFailed` in `GoalCommittedPayload::rejected_alternatives`.

The live integration keeps one truthful deviation from the earlier draft: plausible slot winners lead the searched opportunity order, but remaining admitted ranked opportunities still stay eligible behind them until `max_candidates_to_plan` stops the pass. Within the slot-winner front of that order, higher `GoalPriorityClass` preempts lower-priority commitments before weighted slot score breaks equal-priority ties. This preserves the staged slot substrate and same-goal continuation contract while avoiding a second candidate-admission path.

The landed probe/planning boundary also adds a narrow same-place harvest guard to prevent stale local acquisition beliefs from producing immediately-invalid first steps that would otherwise record spurious contradicted-belief memory before a feasible fallback can win.
