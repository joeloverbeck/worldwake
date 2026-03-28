# AITRACEPLAN-002: Expose epistemic root-omission provenance in planner traces

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` planner traceability surfaces and docs update; no planner-behavior change
**Deps**: [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md), [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md), [archive/tickets/completed/S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-010.md), [archive/tickets/completed/E17CRITHEJUS-022.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/E17CRITHEJUS-022.md)

## Problem

The current planner trace surfaces can prove which root candidates existed and which plan won, but they are too coarse when a plausible epistemic operator never materializes. During S34 `AskWitness` golden completion, the focused planner test proved the `AskWitness` barrier was lawful, while the end-to-end scenario initially showed only `Travel` root candidates. The trace surface exposed that `AskWitness` was absent, but not why:

- no stale epistemic subjects derived
- stale subjects derived but no matching `AskWitness` affordance existed
- an affordance existed but no payload variant matched the subject/topic

That gap matters architecturally because the epistemic contract is belief- and staleness-sensitive. If the trace does not explain omission at that boundary, engineers are pushed toward ad hoc source diving instead of using the intended debugging substrate.

## Assumption Reassessment (2026-03-28)

1. The live planner traceability contract is documented in [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md). It already names `RootOperatorOmissionTrace` and `RootCandidateTrace` as the canonical root-boundary explanation surfaces.
2. The live omission surface is still coarse. [decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) exposes `RootOperatorOmissionReason::{NoMatchingActionDef, NoAffordanceOrSynthesisPath, SynthesisUnsupportedGoalOp, SynthesisTargetDerivationFailed}` only. None of those reasons distinguish epistemic-subject derivation failure from later affordance/payload absence.
3. The exact shared abstraction boundary under audit is the grounded-goal epistemic barrier path inside `worldwake-ai`:
   - stale-subject derivation in `grounded_goal_epistemic_subjects()` in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
   - root candidate surfacing in `search_candidates()` and `record_root_operator_omissions()` in [search/candidates.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs)
   - trace serialization in [decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
4. The motivating invariant is traceability, not planner legality: when `AskWitness` is relevant to a grounded goal but omitted at the root, the trace should explain whether the missing branch was blocked by confidence/staleness derivation, missing witness affordance, or payload mismatch. This ticket must not change which plans are legal or selected.
5. The live `GoalKind` used in the motivating scenario remains `GoalKind::RestockCommodity`. The exact operator surface is `PlannerOpKind::AskWitness` as a grounded-goal epistemic progress barrier; this ticket should not widen scope to unrelated planner operators.
6. Existing focused coverage proves the behavior boundary but not the missing provenance:
   - `goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload`
   - `goal_model::tests::grounded_goal_epistemic_barrier_matches_only_matching_payloads`
   - `golden_stale_prerequisite_ask_witness_chain` in [golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)
7. This is an AI/runtime traceability ticket, not a candidate-generation or authoritative-world ticket. The intended verification layers are focused search/trace tests plus planner-contract doc updates. No `worldwake-sim` or `worldwake-systems` production change is required.
8. Scenario-isolation lesson from S34 does not itself warrant a separate process-doc ticket. Current repo docs already require branch isolation, lower-layer proof fallback, and follow-up traceability tickets when provenance is too coarse. The live missing substrate is the planner trace surface itself.
9. Adjacent contradiction classification:
   - required consequence of this ticket: expose epistemic omission provenance more precisely at the root-candidate boundary
   - not in scope: changing epistemic ranking, staleness arithmetic, or action-handler behavior
10. Mismatch + correction: current `RootOperatorOmissionReason::NoAffordanceOrSynthesisPath` is too broad for epistemic operators. For `AskWitness`, it collapses multiple architecturally distinct states into one opaque bucket, weakening the debugging contract described in `docs/planner-contracts.md`.
11. If implementation confirms that omission reasoning needs per-operator structured detail rather than a single enum expansion, prefer the smallest explicit structured trace type that keeps the root-boundary contract honest. Do not hide the new provenance behind formatted strings or comments.

## Architecture Check

1. The clean fix is to strengthen planner traceability at the exact omission boundary, not to add ad hoc debug output to specific tests. That preserves Principle 3 from [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): concrete structured state over vague summaries.
2. This is cleaner than widening golden assertions to later downstream consequences, because the missing information lives at the root-candidate boundary. The right place to explain it is the planner trace, not a later action/world side effect.
3. This is cleaner than adding a one-off `AskWitness` special case in tests, because the architectural lesson is generic: epistemic omission needs structured provenance just like other planner root omissions.
4. No backwards-compatibility aliasing or parallel trace representations. The existing root-omission surface should be extended or refined directly so there remains one canonical explanation contract.

## Verification Layers

1. stale-subject derivation failure vs success for epistemic barriers -> focused `goal_model` / search trace unit tests
2. omitted `AskWitness` root operator exposes the correct omission provenance -> focused planner trace tests in `worldwake-ai`
3. existing end-to-end `AskWitness` golden still passes under the strengthened trace contract -> `golden_stale_prerequisite_ask_witness_chain`
4. planner-contract documentation matches the live omission taxonomy -> doc review plus command-based verification
5. this is a single-layer AI traceability ticket; additional authoritative/action-layer mapping is not applicable because the behavior contract is unchanged

## What to Change

### 1. Refine epistemic root-omission tracing

Extend the planner root trace surface so epistemic omissions can distinguish at least:

- no stale epistemic subjects were derived
- stale subjects existed but no `AskWitness` affordance target was available
- an `AskWitness` affordance existed but no payload variant matched the derived subject/topic

The final representation can be either new `RootOperatorOmissionReason` variants or a dedicated structured detail payload for epistemic operators, but it must remain machine-readable and specific.

### 2. Thread the omission provenance through the existing search root boundary

Update [search/candidates.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs) so the new epistemic omission detail is emitted from the same root-candidate pass that currently records `RootCandidateTrace` and `RootOperatorOmissionTrace`. Do not derive it later from selected-plan symptoms.

### 3. Add focused tests for the new provenance

Add focused `worldwake-ai` tests that prove:

- a non-stale belief omits `AskWitness` with the explicit “no stale epistemic subjects” reason
- stale epistemic subjects with no co-located witness affordance omit `AskWitness` with the explicit “no witness affordance” reason
- stale subjects with a witness target but no matching payload/topic omit `AskWitness` with the explicit payload-mismatch reason, if that state is reachable in the live architecture

### 4. Update planner traceability documentation

Update [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) so the root-omission section documents the new epistemic omission contract and tells authors to use it instead of inferring the reason from missing root candidates.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — refine root omission trace surface)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — emit epistemic omission provenance at the root-candidate boundary)
- `crates/worldwake-ai/src/goal_model.rs` or `crates/worldwake-ai/src/search/tests.rs` (modify — add focused epistemic omission trace tests)
- `docs/planner-contracts.md` (modify — document the refined epistemic omission contract)

## Out of Scope

- planner ranking or search legality changes
- candidate-generation changes outside the traceability boundary
- authoritative belief, action-handler, or event-log behavior changes
- new golden scenarios beyond keeping existing S34 goldens passing

## Acceptance Criteria

### Tests That Must Pass

1. Focused planner trace coverage proves epistemic omission reasons distinguish stale-subject absence from missing witness/payload availability
2. Existing `AskWitness` focused planner search coverage still passes
3. Existing `AskWitness` golden still passes
4. `cargo test -p worldwake-ai`

### Invariants

1. Root-candidate traceability explains epistemic omission at the earliest planner boundary where the branch disappears.
2. The ticket does not change which epistemic plans are legal or selected; it only makes omission provenance explicit.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` or `crates/worldwake-ai/src/goal_model.rs` — focused root-omission trace test for “no stale epistemic subjects”
   Rationale: proves the trace distinguishes confidence/staleness gating from later affordance failures.
2. `crates/worldwake-ai/src/search/tests.rs` or `crates/worldwake-ai/src/goal_model.rs` — focused root-omission trace test for “no matching witness affordance/payload”
   Rationale: proves the trace distinguishes subject derivation from witness/payload availability.
3. `crates/worldwake-ai/tests/golden_supply_chain.rs` — keep `golden_stale_prerequisite_ask_witness_chain` passing without changing its behavior contract
   Rationale: ensures the refined trace surface does not regress the new S34 end-to-end path.

### Commands

1. `cargo test -p worldwake-ai goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload -- --exact`
2. `cargo test -p worldwake-ai golden_stale_prerequisite_ask_witness_chain -- --exact`
3. `cargo test -p worldwake-ai`
