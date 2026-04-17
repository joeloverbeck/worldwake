# S112: Portfolio Planning with Feasibility Probes

## Summary

Replace flat top-N candidate selection (`max_candidates_to_plan`, default 2) with a small *portfolio* — a diversified agenda slice composed of: the best urgent survival goal, the best current commitment or obligation, the best feasible background economic goal, and the best information-gathering fallback when confidence is low. Before committing full tactical search budget, each slot runs a cheap feasibility probe (belief-grounded target reachability, BestEffort affordance existence check, discrepancy-memory filter). Slots that fail the probe are dropped; remaining slots proceed to full search in score order. This prevents the pathology where the top two candidates are infeasible but the third is trivial — today the agent wastes a tick and looks stuck.

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-ai` — `agent_tick/planning.rs` portfolio assembly; `ranking.rs` slot-aware scoring; new `feasibility_probe.rs`
- `worldwake-core` — `CognitiveProfile` fields for slot weights and probe budget
- `worldwake-cli` — scenario authoring for the new cognitive profile fields

## Dependencies

- S109 (Typed Discrepancy Taxonomy) — feasibility probe reads `DiscrepancyMemory` / `BlockerMemory` to reject already-suppressed goals. Soft: S112 can land before S109 by reading the current `BlockedIntentMemory`.

## Design Goals

- Candidate selection is *diverse*, not just top-scored. The portfolio always considers multiple categories so a single high-score goal class (e.g., saturated PostNotice obligations pre-S96) does not occupy all candidate slots.
- Infeasibility is caught cheaply, before the full tactical search. A portfolio slot that fails its feasibility probe never consumes tactical search budget.
- Deterministic. Slot assembly, probe order, and fallback ordering are all `BTreeMap`-driven.
- Backward compatible with existing `max_candidates_to_plan` per agent: agents that set `max_candidates_to_plan = 1` skip portfolio assembly entirely (single-slot behavior preserved).

## Non-Goals

- Parallel search across portfolio slots. Slots are searched sequentially in score order; the first slot that produces a plan wins.
- PolicyPlan branching or contingent plans — deferred to a Phase 9 spec (S114 lands the step-guard substrate that branching will build on).
- Replacing the `max_candidates_to_plan` field. Portfolio is an enhancement to candidate *selection*, not a replacement for the search-budget cap.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-20 (Resource-Bounded Practical Reasoning) | Feasibility probes burn cheap tokens before committing to expensive tactical search. An agent finds *a* feasible plan within budget instead of burning budget on the two infeasible top options. |
| FND-22 (Agent Diversity Through Concrete Variation) | Slot weights per `CognitiveProfile` allow one agent to prioritize the survival slot, another the obligation slot. Two agents with the same motives can differ on portfolio shape. |
| FND-16 (Ignorance, Uncertainty, Contradiction First-Class) | The information-gathering fallback slot is always considered when confidence is low, even if its raw motive is outscored by action goals. This is the structural surface through which "verify first when uncertain" becomes a planning outcome. |
| FND-21 (Intentions Are Revisable Commitments) | Portfolio still runs every tick; the current commitment is surfaced via the "current commitment/obligation" slot, which margin-based commitment (S74) decides whether to keep. |

## Deliverables

### D1: `Portfolio` type

New type in `crates/worldwake-ai/src/agent_tick/portfolio.rs`:

```rust
/// A small diversified slice of agenda candidates assembled per tick.
/// Slots may be `None` if no candidate of that category is available.
pub(crate) struct Portfolio {
    /// Best urgent survival goal (hunger, thirst, wounds, sleep, danger).
    pub survival: Option<PortfolioSlot>,
    /// Best goal tied to a current commitment or obligation
    /// (committed goal persisted from prior tick, or a notice/bounty
    /// the agent is obliged to post / fulfill).
    pub commitment: Option<PortfolioSlot>,
    /// Best feasible background economic goal (produce, buy, sell,
    /// consume-owned) that isn't an urgent need.
    pub economic: Option<PortfolioSlot>,
    /// Best information-gathering fallback when agent confidence in
    /// relevant beliefs is below `confidence_probe_threshold`.
    pub information: Option<PortfolioSlot>,
}

pub(crate) struct PortfolioSlot {
    pub ranked: RankedGoal,
    pub feasibility: FeasibilityVerdict,
}

pub(crate) enum FeasibilityVerdict {
    /// Probe passed — proceed to full tactical search.
    Plausible,
    /// Probe failed for a reason the agent already knows
    /// (discrepancy memory hit, no known target).
    RejectedBeforeSearch { reason: Discrepancy },
}
```

### D2: Slot categorization

Categorization rules (evaluated at slot-assembly time, reading from already-ranked candidates):

- **Survival slot**: `GoalKind::ConsumeOwnedCommodity { Survival* }`, `Sleep`, `Relieve`, `Wash`, `TreatWounds { patient == self }`, `ReduceDanger`, `FreeCarryCapacity` when capacity ratio ≥ agent's `free_carry_threshold`. Scored by its existing motive; survival is picked by highest motive within category.
- **Commitment slot**: the goal the agent committed to last tick (if still ranked), plus any `PostNotice`/`PostBounty`/`ReportMissing`/`ReportFound`/warrant-adjacent obligation goals.
- **Economic slot**: `AcquireCommodity { purpose != Survival }`, `ConsumeOwnedCommodity { non-survival }`, `ProduceCommodity`, trade goals, `EstablishBanditCamp`, faction-economic goals. Filtered: if a slot goal is also the survival or commitment slot winner, the economic slot picks the next candidate.
- **Information slot**: only considered when `agent_confidence_summary() < confidence_probe_threshold` (new `CognitiveProfile` field). Categories: `ExploreLocation`, `Patrol`, `InvestigateViolation`, `SearchForMissing`. When confidence is high, the slot is `None`.

If a goal ties between categories (e.g., `PostNotice` is both an obligation and — with high weight — an economic goal), the earlier category wins. Deterministic via category ordering.

### D3: Feasibility probe

New function in `crates/worldwake-ai/src/feasibility_probe.rs`:

```rust
pub(crate) fn probe(
    ranked: &RankedGoal,
    context: &ProbeContext<'_>,
) -> FeasibilityVerdict;
```

Probes are shallow:

1. **Discrepancy/blocker memory check**: if `DiscrepancyMemory` or `BlockerMemory` records a non-expired suppressive entry for `(goal_key, place, target, action_def)`, the slot is rejected with the recorded discrepancy. No search budget consumed.
2. **Known-target check**: the goal must have at least one candidate target that the agent believes exists (from the agent's belief store). Goals whose anchors reference unknown targets are rejected (`RouteUnknown` / `StructurallyImpossible`).
3. **Affordance existence check**: at least one affordance of the goal's action-kind must be believed-reachable from the agent's current place. Does not verify the full chain — only that the first step type is plausible.

The probe does **not** run tactical search. It is O(candidates × belief-lookup), not O(search budget).

### D4: Portfolio-driven planning loop

`agent_tick/planning.rs` currently takes the top `max_candidates_to_plan` ranked goals and attempts to plan each in order. Replace with:

```rust
let portfolio = assemble_portfolio(&ranked_goals, &probe_context, &cognitive);
let plausible_slots = portfolio.plausible_slots_by_score();
for slot in plausible_slots.iter().take(cognitive.max_candidates_to_plan as usize) {
    match try_plan(slot.ranked, &planning_context) {
        PlanOutcome::Success(plan) => return Some(plan),
        PlanOutcome::Failure(discrepancy) => {
            record_blocker_or_discrepancy(...);
            continue;
        }
    }
}
None
```

`max_candidates_to_plan` bounds the *number of portfolio slots we actually search*, not the ranking depth. The portfolio itself is always 4 slots (with `None` permitted); `max_candidates_to_plan = 2` means "try the top 2 plausible slots."

### D5: `CognitiveProfile` extensions

Add to `CognitiveProfile`:

```rust
/// Confidence summary below which the information-gathering slot is
/// considered. Uses the belief-store's aggregate freshness/confidence
/// measure (see the GoalBeliefView accessor).
pub confidence_probe_threshold: Permille,
/// Relative weights by slot category when ordering plausible slots
/// for tactical search. Survival usually dominates; background
/// agents may weight economic higher.
pub slot_weights: PortfolioSlotWeights,

pub struct PortfolioSlotWeights {
    pub survival: Permille,
    pub commitment: Permille,
    pub economic: Permille,
    pub information: Permille,
}
```

Defaults: `survival = 1000`, `commitment = 900`, `economic = 700`, `information = 600`, `confidence_probe_threshold = 400` (low-confidence threshold; below this the information slot activates).

### D6: Decision-trace extension

The existing `AgentDecisionTrace` gains a `portfolio` summary field per tick:

```rust
pub struct PortfolioTrace {
    pub survival_slot: Option<PortfolioSlotTrace>,
    pub commitment_slot: Option<PortfolioSlotTrace>,
    pub economic_slot: Option<PortfolioSlotTrace>,
    pub information_slot: Option<PortfolioSlotTrace>,
    pub slots_attempted: u8,
}

pub struct PortfolioSlotTrace {
    pub goal_key: GoalKey,
    pub motive_score: u32,
    pub feasibility: FeasibilityVerdict,
}
```

S110's `GoalCommittedPayload::rejected_alternatives` also records the probe verdicts for rejected slots.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: The feasibility probe reads only from the agent's own belief store and memories. It does not consult world state. Aligned with FND-14 (World State Is Not Belief State).
2. **Positive-feedback analysis**: A loop exists if "probe fails → record discrepancy → next tick probe fails for same reason → record discrepancy again." The dampener is S109's typed TTL: once a discrepancy is recorded, the same goal does not re-enter the portfolio until the TTL expires.
3. **Concrete dampeners**: Discrepancy memory TTL (from S109) dampens retry loops. Probe budget is bounded: at most `4 × max_candidates_to_plan × small_constant` belief-store lookups per tick, independent of search budget.
4. **Stored state vs. derived read-model**: `Portfolio`, `PortfolioSlot`, and `FeasibilityVerdict` are transient per-tick derivations. No authoritative state is added. `PortfolioTrace` is recorded into the optional decision-trace sink; S110's per-commit alternatives summary is the authoritative log entry.

## SystemFn Integration

No new SystemFn. Portfolio assembly runs inline in the existing agent tick planning phase.

## Component Registration

No new components. `CognitiveProfile` gains `confidence_probe_threshold` and `slot_weights` fields, both serde-default so existing scenarios remain valid.

## Cross-System Interactions

- **Ranking ↔ portfolio assembly**: Ranking produces the `Vec<RankedGoal>` as today; portfolio assembly categorizes and re-orders that list. State-mediated.
- **S109 discrepancy memory ↔ feasibility probe**: Probe reads memories, writes nothing. State-mediated.
- **S110 event log ↔ rejected alternatives**: Slot rejections appear in `GoalCommittedPayload::rejected_alternatives` via `GoalRejectionReason::FeasibilityProbeFailed`.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `confidence_probe_threshold` | `CognitiveProfile` | `Permille` | `Permille::new(400)` | Below this belief-confidence summary, activate information-gathering slot |
| `slot_weights.survival` | `CognitiveProfile::slot_weights` | `Permille` | `Permille::new(1000)` | Relative weight of survival slot when ordering plausible slots |
| `slot_weights.commitment` | `CognitiveProfile::slot_weights` | `Permille` | `Permille::new(900)` | Commitment/obligation slot weight |
| `slot_weights.economic` | `CognitiveProfile::slot_weights` | `Permille` | `Permille::new(700)` | Economic slot weight |
| `slot_weights.information` | `CognitiveProfile::slot_weights` | `Permille` | `Permille::new(600)` | Information slot weight (when active) |

## Validation and Falsification

### Unit tests

1. Survival slot always picks the highest-motive survival goal; ties broken deterministically by `GoalKey` order.
2. If agent confidence summary ≥ threshold, information slot is `None`.
3. A ranked goal that is both survival and economic populates only survival (category priority).
4. `FeasibilityVerdict::RejectedBeforeSearch` when discrepancy memory has non-expired entry.
5. `plausible_slots_by_score` orders by `score * slot_weight / 1000`, not raw score.

### Integration tests

6. Scenario with 2 infeasible high-motive goals + 1 trivial feasible goal: pre-S112 agent wastes a tick on the infeasible pair; post-S112 agent probes, rejects both, and plans the trivial goal in the same tick.
7. Scenario with low agent confidence: information slot activates, survival still wins, but information slot ranks high enough to commit when survival is satiated.
8. Existing `survival-baseline.ron` and `survival-contested.ron` goldens pass unchanged (portfolio with survival-dominant weights produces the same commit decisions the existing scenarios assert).

### Golden test

9. New scenario proof in `golden_portfolio_planning.rs`: Agent with (infeasible-high-motive goal A, infeasible-high-motive goal B, feasible-low-motive goal C). Agent commits goal C within 2 ticks. Pre-S112 regression baseline: without portfolio, agent would waste ≥5 ticks on A/B before reaching C.

## Outcome

To be filled in at completion.
