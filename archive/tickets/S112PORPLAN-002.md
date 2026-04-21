# S112PORPLAN-002: Portfolio types and slot categorization

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `worldwake-ai` module `agent_tick/portfolio.rs`
**Deps**: archive/tickets/S112PORPLAN-001.md (needs `PortfolioSlotWeights`)

## Problem

S112's portfolio replaces flat top-N candidate selection with a diversified agenda slice: survival / commitment / economic slots assembled from already-ranked candidates. This ticket introduces the pure-data and pure-function substrate — `Portfolio`, `SlotKind`, `PortfolioSlot`, `FeasibilityVerdict`, `assemble_portfolio`, and `plausible_slots_by_score` — without yet wiring them into the planning loop.

Decoupling type declaration from loop integration keeps 002 independently testable (slot-category assignment, tie-breaking, score-weighted ordering) and keeps 005's diff focused on the loop rewrite.

## Assumption Reassessment (2026-04-20)

1. Ranked candidates flow from `rank_candidates` producing `Vec<RankedGoal>` (`crates/worldwake-ai/src/goal_model.rs:2528` — fields: `grounded`, `priority_class`, `motive_score`, `feasibility`, provenance). `GoalKey::kind` (a `GoalKind` variant) is how categorization decides slot assignment. Today, the top-N path calls `prioritize_same_goal_replan_candidates` at `planning.rs:303` then `.take(max_candidates_to_plan)` — both are replaced by portfolio assembly in ticket 005.
2. Spec S112 D1 and D2 define the types and categorization rules. Category priority via `SlotKind::Ord` is declared explicitly. `committed_opportunity: Option<OpportunityKey>` is the commitment-slot anchor (tracked on `AgentDecisionRuntime` via `planning.rs:275`).
3. Shared boundary: `Portfolio` is a per-tick derivation (not authoritative state) per Section H item 4. `assemble_portfolio(&ranked, &probe_ctx, &cognitive)` consumes already-ranked candidates and `PortfolioSlotWeights` from ticket 001.
4. `GoalKind` variants referenced by categorization rules all exist and are read from `crates/worldwake-core/src/goal.rs:24-121`: survival (`ConsumeOwnedCommodity`, `AcquireCommodity { purpose: SelfConsume }`, `Sleep`, `Relieve`, `Wash`, `TreatWounds`, `ReduceDanger`, `FreeCarryCapacity`), obligations (`PostNotice`, `PostBounty`, `ReportMissing`, `ReportFound`), economic (`AcquireCommodity { purpose: Restock | RecipeInput(_) }`, `ProduceCommodity`, `SellCommodity`, `RestockCommodity`, `MoveCargo`, `EstablishBanditCamp`).
5. `FreeCarryCapacity` is a unit variant — categorization does not re-gate on capacity ratio; by the time it appears in the ranked list, emission (`DisposalProfile`-gated in `goal_model.rs:468-543`) has already decided the agent is over threshold. Spec D2 was corrected during reassessment to drop the ratio caveat.
6. Reassessment mismatch: the drafted test `category_priority_survival_wins_over_economic` no longer maps to a truthful live overlap case once survival/economic categories are bound to the current `GoalKind` surface. The focused proof was corrected to assert the live self-care boundary instead: `AcquireCommodity { purpose: SelfConsume }` populates the survival slot while a separate restock acquire still populates economic.

## Architecture Check

1. Pure data + pure function — assembly reads `Vec<RankedGoal>` and writes a `BTreeMap<SlotKind, PortfolioSlot>`. No authoritative state is written; no cross-system calls. FND-26 aligned (systems interact through state).
2. Deterministic by construction — `SlotKind` derives `Ord`, `BTreeMap` iteration is stable, score-weighted ordering uses integer math (`u32::saturating_mul / 1000`). No `HashMap`/`HashSet`, no floats.
3. Diversity substrate for FND-22: per-agent slot weights (from ticket 001) let two agents with identical motives differ on which slot wins the plausible-slots ranking.

## Verification Layers

1. Categorization correctness → focused unit tests on `assemble_portfolio` (survival picks highest-motive survival candidate; commitment slot picks `committed_opportunity` when still ranked; category priority handled via `SlotKind::Ord` tie-break).
2. Score-weighted ordering → focused unit test on `plausible_slots_by_score` with a concrete numeric fixture (e.g., survival 500 × 1000 vs. economic 600 × 700 → survival wins at 500 vs. 420).
3. Single-layer ticket — `Portfolio` assembly is not called by the runtime planning loop yet (ticket 005 integrates). No decision/action/event-log assertions are reachable here; that's by design.

## What to Change

### 1. Create `crates/worldwake-ai/src/agent_tick/portfolio.rs`

Declare:

```rust
use std::collections::BTreeMap;
use worldwake_core::{Discrepancy, GoalKey, PortfolioSlotWeights};
use crate::goal_model::RankedGoal;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum SlotKind {
    Survival,
    Commitment,
    Economic,
}

pub(crate) struct Portfolio {
    pub(crate) slots: BTreeMap<SlotKind, PortfolioSlot>,
}

pub(crate) struct PortfolioSlot {
    pub(crate) ranked: RankedGoal,
    pub(crate) feasibility: FeasibilityVerdict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FeasibilityVerdict {
    Plausible,
    RejectedBeforeSearch { reason: Discrepancy },
}
```

### 2. Implement `assemble_portfolio`

Signature:

```rust
pub(crate) fn assemble_portfolio(
    ranked: &[RankedGoal],
    committed: Option<OpportunityKey>,
    probe: impl Fn(&RankedGoal) -> FeasibilityVerdict,
) -> Portfolio
```

Rules per D2:

- Survival slot = highest-motive candidate whose `GoalKind` matches survival list; ties broken by `GoalKey` order.
- Commitment slot = the candidate whose `(GoalKey, anchor)` equals `committed` when present in `ranked`; fallback to highest-motive obligation candidate when `committed` is absent from the list.
- Economic slot = highest-motive economic candidate not already winning survival or commitment.
- Category priority: if a candidate ties between categories, the lower `SlotKind` variant wins (Survival > Commitment > Economic lexicographically, enforced by `SlotKind`'s derived `Ord`).

Each populated slot carries the probe's verdict for its winner.

### 3. Implement `plausible_slots_by_score`

```rust
impl Portfolio {
    pub(crate) fn plausible_slots_by_score<'a>(
        &'a self,
        weights: &PortfolioSlotWeights,
    ) -> Vec<(SlotKind, &'a PortfolioSlot)> {
        // Filter FeasibilityVerdict::Plausible only.
        // Sort by u32::from(slot.ranked.motive_score)
        //     .saturating_mul(u32::from(weight_for(slot_kind).value())) / 1000
        // descending; ties broken by SlotKind's Ord (Survival > Commitment > Economic).
    }
}
```

`weight_for(slot_kind)` is a private helper that reads the matching field from `PortfolioSlotWeights`.

### 4. Register module

Add `pub(crate) mod portfolio;` to `crates/worldwake-ai/src/agent_tick/mod.rs`.

### 5. Unit tests

Place in `#[cfg(test)]` block in `portfolio.rs`:

1. `survival_slot_picks_highest_motive_survival` — two survival candidates, highest wins; tie broken by `GoalKey` order.
2. `commitment_slot_picks_committed_opportunity_when_ranked` — committed opportunity present in ranked list wins commitment slot even when a higher-motive obligation exists.
3. `commitment_slot_falls_back_to_highest_obligation_when_commitment_unranked` — committed opportunity absent from ranked list → highest-motive obligation candidate wins commitment.
4. `self_consume_acquire_populates_survival_slot` — `AcquireCommodity { purpose: SelfConsume }` is classified into the survival slot, while a separate restock acquire remains economic.
5. `plausible_slots_by_score_applies_weights` — survival 500 × 1000 vs. economic 600 × 700 → survival wins (500 > 420).

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (new)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — add `mod portfolio;`)

## Out of Scope

- Feasibility probe implementation (ticket 003 owns `feasibility_probe::probe`; 002 accepts the probe as a closure argument).
- Planning loop integration (ticket 005 — `assemble_portfolio` is not called from any runtime path yet).
- `PortfolioTrace` on `PlanningPipelineTrace` (ticket 004).
- Information slot and its `SlotKind` variant (deferred to S113 follow-up per spec Non-Goals).
- Removal of `prioritize_same_goal_replan_candidates` (ticket 005 — commitment-slot subsumption is proven in 005, not here).

## Acceptance Criteria

### Tests That Must Pass

1. The 5 new unit tests in `portfolio.rs` pass.
2. Existing suite: `cargo test -p worldwake-ai`, `cargo test --workspace`.
3. `cargo clippy --workspace --all-targets -- -D warnings` remains clean.

### Invariants

1. No `HashMap`/`HashSet` in authoritative storage (the `slots` field uses `BTreeMap`).
2. `Portfolio` and `PortfolioSlot` are never written to authoritative world state — they exist only as per-tick derivations.
3. Score-weighted ordering uses integer math only; no floating-point comparisons.
4. `SlotKind` derive order (`Survival`, `Commitment`, `Economic`) is fixed — changing it would silently reorder tie-breaking.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/portfolio.rs` — inline `#[cfg(test)]` module with 5 unit tests per the What to Change section.

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::portfolio::tests`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-21.

- Added `crates/worldwake-ai/src/agent_tick/portfolio.rs` with the staged `Portfolio`, `SlotKind`, `PortfolioSlot`, `FeasibilityVerdict`, `assemble_portfolio`, and `plausible_slots_by_score` substrate.
- Registered the new module in `crates/worldwake-ai/src/agent_tick/mod.rs`.
- Kept the landed surface intentionally staged for ticket 005 by marking the new module's unused scaffolding explicitly, and preserved the ticket/spec `&PortfolioSlotWeights` API shape with narrow clippy allowances.

## Deviations

- Corrected the live slot-classification boundary so `AcquireCommodity { purpose: SelfConsume }` follows the existing self-care grouping used elsewhere in `worldwake-ai`; the original ticket text omitted that current live survival case.
- Replaced the drafted survival-vs-economic overlap test with a truthful self-consume classification proof because the current live category set does not expose a lawful survival/economic overlap candidate.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib agent_tick::portfolio::tests`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
