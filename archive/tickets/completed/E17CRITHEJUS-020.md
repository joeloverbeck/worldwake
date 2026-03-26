# E17CRITHEJUS-020: Make exact-goal terminal operator surfacing an explicit planner contract

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` planner root-candidate synthesis and conformance coverage
**Deps**: archive/tickets/completed/E17CRITHEJUS-008.md

## Problem

`E17CRITHEJUS-008` exposed a broader planner contradiction than the original `accuse` gap: several live grounded goals already carry enough identity to determine their exact terminal action payloads, but root-candidate surfacing still depends partly on incidental affordance enumeration. That leaves the architecture brittle. A goal can be valid, its payload can be derivable from `GoalKind`, and yet search can still fail to surface the terminal operator unless some other layer happens to enumerate the right affordance payload first.

That is not a one-off bug. It is an unclear planner contract. The fix should be architectural: define one explicit rule for when a live goal may synthesize a root candidate from goal identity, centralize that rule, and test it across all non-deferred exact-bound goal families.

## Assumption Reassessment (2026-03-26)

1. Current code already exposes exact terminal operators in `crates/worldwake-ai/src/goal_model.rs` for multiple live goals. `GoalKind::ShareBelief`, `GoalKind::InvestigateViolation`, `GoalKind::ClaimOffice`, and `GoalKind::Accuse` each advertise exact terminal ops via `relevant_op_kinds()` and already have payload override builders for `PlannerOpKind::Tell`, `PlannerOpKind::Investigate`, `PlannerOpKind::PressForceClaim`, and `PlannerOpKind::Accuse`.
2. Current root-candidate surfacing remains partly ad hoc. `crates/worldwake-ai/src/search/candidates.rs` uses `goal_synthesized_candidates()` and `synthesized_candidate_for_goal()` to synthesize only specific operator/goal pairs after affordance enumeration yields no candidate. That is a hidden contract, not a declared planner rule.
3. The shared abstraction boundary under audit is: `GroundedGoal` + `GoalKindPlannerExt` -> root-candidate synthesis in `crates/worldwake-ai/src/search/candidates.rs` -> successor construction in `crates/worldwake-ai/src/search/transition.rs`. Reassessment correction: for some families the lawful terminal binding comes from `GoalKind` alone, but `PlannerOpKind::Trade` currently depends on `GroundedGoal.evidence_entities`, not on `GoalKind` identity by itself.
4. The intended invariant is: if a live non-deferred grounded goal already carries the canonical terminal binding and the planner can lawfully derive its authoritative targets and payload from the grounded goal boundary, search must be able to surface that terminal operator without depending on incidental affordance payload enumeration.
5. The live goal/operator surface relevant to this ticket is not just `GoalKind::Accuse`. Current code paths show the same contract matters for:
   - exact-goal families whose terminal binding is encoded directly in `GoalKind`: `GoalKind::ShareBelief` / `PlannerOpKind::Tell`, `GoalKind::InvestigateViolation` / `PlannerOpKind::Investigate`, `GoalKind::ClaimOffice` / `PlannerOpKind::PressForceClaim`
   - evidence-bound acquisition families whose terminal binding currently rides on grounded evidence rather than raw `GoalKind`: `GoalKind::AcquireCommodity`, `GoalKind::ConsumeOwnedCommodity`, `GoalKind::RestockCommodity`, and `GoalKind::TreatWounds` through `PlannerOpKind::Trade`
6. This is an AI planner/search ticket, not a candidate-generation ticket. The intended verification layer is focused search coverage, targeted goal-model unit coverage, planner conformance coverage, and `cargo test -p worldwake-ai`; this is not satisfied by candidate-generation tests alone.
7. The current architecture still includes deliberate deferred justice surface. `GoalKind::PunishAccused` remains deferred in `crates/worldwake-ai/src/goal_model.rs`; this ticket must not silently broaden scope into undeferred punishment planning.
8. The heuristic under reassessment is the scattered per-operator root-candidate synthesis in `crates/worldwake-ai/src/search/candidates.rs`. The missing substrate is an explicit exact-goal terminal operator contract. This ticket adds that substrate; it should not add more operator-specific fallback branches.
9. First failure boundary for the regressions exposed by `E17CRITHEJUS-008` was plan search root-candidate surfacing, not authoritative action start. The relevant shared symbols are `search_candidates()`, `goal_synthesized_candidates()`, `matches_binding()`, and successor construction in `search/transition.rs`.
10. Existing coverage confirms the gap is only partially protected today. `crates/worldwake-ai/tests/planner_conformance.rs` covers `conformance_press_force_claim` and still contains the explicit `conformance_trade_noop_coverage_gap`; it has no live `conformance_tell`, `conformance_investigate`, or `conformance_accuse` coverage today. `crates/worldwake-ai/src/search/tests.rs` already proves `ClaimOffice` force-claim root tracing and that `Accuse` is live while `PunishAccused` stays deferred, but it does not yet serve as a complete exact-goal root-synthesis contract suite for `tell`, `investigate`, and evidence-bound `trade`.
11. Adjacent contradictions exposed during reassessment are separate follow-ups, not in-scope consequences here:
    - planning snapshot completeness for planner-visible duration dependencies -> follow-up ticket `E17CRITHEJUS-021`
    - decision-trace provenance for omitted root operators and missing prerequisites -> follow-up ticket `E17CRITHEJUS-022`
12. Mismatch + correction: the stale narrative was “`accuse` is missing.” Current code already implements `accuse`. The remaining architectural problem is that exact-goal operator surfacing is still encoded as narrow search fallbacks instead of a first-class planner contract, and the current `Trade` fallback especially proves the contract is grounded-goal based rather than `GoalKind`-only.

## Architecture Check

1. The clean fix is to make exact-goal terminal surfacing a named planner contract owned by the grounded-goal / goal-model boundary, then route all eligible live goals through that one contract.
2. That is cleaner than preserving a growing list of special cases in `search/candidates.rs`, because the rule then lives where terminal binding and payload derivation are defined rather than where one caller happens to notice an empty affordance result.
3. This aligns with `docs/FOUNDATIONS.md`: explicit causal rules, no workaround branches, and no compatibility aliases between “affordance surfaced it” and “goal synthesized it.”
4. No backwards-compatibility shims or parallel operator paths should be introduced. Replace the implicit rule with the explicit one.

## Verification Layers

1. Exact-goal root operator is surfaced when lawful even without incidental affordance payload enumeration -> focused `worldwake-ai` search tests
2. Goal-derived or grounded-evidence-derived authoritative targets and payload overrides remain correct for each exact-goal family -> focused `goal_model.rs` unit coverage
3. Planner hypothetical transitions remain aligned with authoritative action handlers for exact-goal terminals -> `crates/worldwake-ai/tests/planner_conformance.rs`
4. Existing golden scenarios that already depend on `tell`, `investigate`, `press_force_claim`, or trade-backed exact execution continue to pass -> targeted golden regression in `cargo test -p worldwake-ai`
5. Traceability improvements are not the proof surface for this ticket; if current traces remain too coarse, the stronger proof surface is focused search/conformance coverage, with follow-up traceability work in `E17CRITHEJUS-022`.

## What to Change

### 1. Define a single exact-goal terminal surfacing contract

Refactor the planner boundary so the rule “this grounded goal may synthesize its terminal root candidate from canonical goal binding” is declared once and consumed uniformly by search. The contract must own:
- which live goal families are eligible
- how authoritative targets are derived from either exact `GoalKind` identity or grounded evidence
- when payload overrides are derived from canonical goal binding rather than affordance payloads

### 2. Remove scattered operator-specific surfacing logic

Collapse the narrow branches in `crates/worldwake-ai/src/search/candidates.rs` into the explicit contract from section 1. Keep deferred goals explicitly deferred rather than letting them appear accidentally through generic synthesis.

### 3. Strengthen exact-goal conformance coverage

Extend planner conformance and focused search tests so all live non-deferred exact-goal families covered by the contract are asserted intentionally, not by incidental regression tests.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/search/candidates.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/tests/planner_conformance.rs` (modify)

## Out of Scope

- `GoalKind::PunishAccused` implementation or undefer work
- Planning snapshot completeness auditing beyond what is required to compile against the new contract
- Decision-trace schema expansion for omitted operators or missing prerequisites
- Any authoritative system changes in `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. Search can surface exact-goal terminal operators for all live non-deferred goal families covered by the contract without relying on incidental affordance payload enumeration.
2. Deferred goal families such as `GoalKind::PunishAccused` remain intentionally absent from the live terminal operator surface.
3. Planner conformance covers the live exact-goal terminal families rather than leaving them as noop coverage gaps.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Goal identity, not incidental affordance payload enumeration, is the canonical source of truth for exact-goal terminal surfacing.
2. Search continues to obey binding, blocker, and legality checks after the root candidate is surfaced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/planner_conformance.rs` — add conformance coverage for live exact-goal terminal families so the planner/handler contract is asserted directly.
2. `crates/worldwake-ai/src/search/tests.rs` — add focused root-candidate surfacing coverage proving exact-goal terminals still appear when affordance payload enumeration is empty, including at least one `GoalKind`-bound family (`tell` or `investigate`) and one grounded-evidence-bound `trade` case.
3. `crates/worldwake-ai/src/goal_model.rs` — strengthen payload/target synthesis tests so the exact-goal contract stays centralized and explicit across both `GoalKind`-bound and grounded-evidence-bound families.

### Commands

1. `cargo test -p worldwake-ai goal_model::tests::grounded_goal_synthesizes_tell_root_targets_from_goal_identity`
2. `cargo test -p worldwake-ai goal_model::tests::grounded_goal_synthesizes_accuse_root_targets_from_goal_identity`
3. `cargo test -p worldwake-ai goal_model::tests::grounded_goal_synthesizes_trade_root_targets_from_single_evidence_entity`
4. `cargo test -p worldwake-ai --test planner_conformance conformance_tell`
5. `cargo test -p worldwake-ai --test planner_conformance conformance_investigate`
6. `cargo test -p worldwake-ai --test planner_conformance conformance_accuse`
7. `cargo test -p worldwake-ai --test planner_conformance conformance_trade_exact_acquisition`
8. `cargo test -p worldwake-ai`
9. `cargo build --workspace`
10. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-26
- What actually changed:
  - Moved exact terminal root-candidate synthesis into the grounded-goal boundary by adding a single synthesis contract in `crates/worldwake-ai/src/goal_model.rs`, then routing `crates/worldwake-ai/src/search/candidates.rs` through that contract instead of a private ad hoc matcher.
  - Covered both exact `GoalKind`-bound families (`tell`, `investigate`, `accuse`, `press_force_claim`) and evidence-bound exact trade families that derive their terminal binding from `GroundedGoal.evidence_entities`.
  - Strengthened planner conformance so `tell`, `investigate`, `accuse`, and exact local `trade` are asserted directly instead of leaving trade as the only named exact-goal gap.
  - Updated the existing search regression to reflect the new canonical architecture: `Accuse` now surfaces its exact bound root candidate from goal identity once the action is live, while `PunishAccused` remains deferred.
- Deviations from original plan:
  - No new trace schema or planning-snapshot work was added; those follow-ups remain out of scope.
  - The implementation stayed entirely inside `worldwake-ai`; no authoritative systems changes were required.
- Verification results:
  - `cargo test -p worldwake-ai` ✅
  - `cargo build --workspace` ✅
  - `cargo clippy --workspace` ✅
