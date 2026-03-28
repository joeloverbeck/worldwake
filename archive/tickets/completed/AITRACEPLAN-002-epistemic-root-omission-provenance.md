# AITRACEPLAN-002: Expose epistemic root-omission provenance in planner traces

**Status**: ✅ COMPLETED
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
2. Mismatch + correction: for the motivating `GoalKind::RestockCommodity` path, `PlannerOpKind::AskWitness` is not part of `GoalKindPlannerExt::relevant_op_kinds()`. `RESTOCK_OPS` in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) contains `Travel`, `Trade`, `QueueForFacilityUse`, `Harvest`, `Craft`, and `MoveCargo` only. `AskWitness` is instead a conditional epistemic barrier candidate injected inside `search_candidates()` in [search/candidates.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs) when `grounded_goal_epistemic_subjects()` returns non-empty stale subjects.
3. The live omission surface is still coarse for that conditional barrier path. [decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) exposes `RootOperatorOmissionReason::{NoMatchingActionDef, NoAffordanceOrSynthesisPath, SynthesisUnsupportedGoalOp, SynthesisTargetDerivationFailed}` only, and [search/candidates.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs) records omissions only for `goal.key.kind.relevant_op_kinds()`. As a result, the current root trace cannot explain why an `AskWitness` barrier never surfaced for a stale-evidence goal.
4. The exact shared abstraction boundary under audit is the grounded-goal epistemic barrier path inside `worldwake-ai`:
   - stale-subject derivation in `grounded_goal_epistemic_subjects()` in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
   - root candidate surfacing in `search_candidates()` and `record_root_operator_omissions()` in [search/candidates.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs)
   - trace serialization in [decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
5. The motivating invariant is traceability, not planner legality: when `AskWitness` is a lawful grounded-goal epistemic barrier candidate but omitted at the root, the trace should explain whether the missing branch was blocked by confidence/staleness derivation or missing witness affordance. This ticket must not change which plans are legal or selected.
6. The live `GoalKind` used in the motivating scenario remains `GoalKind::RestockCommodity`. The exact operator surface under audit is `PlannerOpKind::AskWitness` as a conditional grounded-goal epistemic progress barrier injected during root candidate search; this ticket should not widen scope to unrelated planner operators.
7. Existing focused coverage proves the behavior boundary but not the missing provenance:
   - `goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload`
   - `goal_model::tests::grounded_goal_epistemic_barrier_matches_only_matching_payloads`
   - `golden_stale_prerequisite_ask_witness_chain` in [golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)
8. This is an AI/runtime traceability ticket, not a candidate-generation or authoritative-world ticket. The intended verification layers are focused search/trace tests plus planner-contract doc updates. No `worldwake-sim` or `worldwake-systems` production change is required.
9. Scenario-isolation lesson from S34 does not itself warrant a separate process-doc ticket. Current repo docs already require branch isolation, lower-layer proof fallback, and follow-up traceability tickets when provenance is too coarse. The live missing substrate is the planner trace surface itself.
10. Adjacent contradiction classification:
   - required consequence of this ticket: expose epistemic omission provenance more precisely at the root-candidate boundary
   - not in scope: changing epistemic ranking, staleness arithmetic, or action-handler behavior
11. Mismatch + correction: current `RootOperatorOmissionReason::NoAffordanceOrSynthesisPath` is too broad for epistemic operators, and the current omission recorder does not even emit `AskWitness` omissions for `RestockCommodity`. The gap is therefore both taxonomy and coverage: the planner needs a canonical root-boundary omission surface for conditional epistemic barriers.
12. Additional mismatch + correction from live code review: the planner snapshot/state path does not currently preserve `ask_witness_memory`, so a planner-search-visible "matching payload variant suppressed while unrelated witness payloads remain" case is not live-reachable at this boundary. Do not add speculative omission taxonomy for unreachable states; if planner-visible ask-witness memory becomes part of the snapshot contract later, open a follow-up ticket to extend the omission detail honestly.
13. If implementation confirms that omission reasoning needs per-operator structured detail rather than a single enum expansion, prefer the smallest explicit structured trace type that keeps the root-boundary contract honest. Do not hide the new provenance behind formatted strings or comments.

## Architecture Check

1. The clean fix is to strengthen planner traceability at the exact omission boundary for conditional epistemic barriers, not to add ad hoc debug output to specific tests. That preserves Principle 3 from [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): concrete structured state over vague summaries.
2. This is cleaner than widening golden assertions to later downstream consequences, because the missing information lives at the root-candidate boundary. The right place to explain it is the planner trace, not a later action/world side effect.
3. This is cleaner than adding a one-off `AskWitness` special case in tests, because the architectural lesson is generic: epistemic omission needs structured provenance just like other planner root omissions.
4. No backwards-compatibility aliasing or parallel trace representations. The existing root-boundary omission surface should be extended directly so there remains one canonical explanation contract for both relevant operators and conditional epistemic barriers.

## Verification Layers

1. stale-subject derivation failure vs success for epistemic barriers -> focused `goal_model` / search trace unit tests
2. omitted conditional `AskWitness` root barrier exposes the correct omission provenance at the search root boundary -> focused planner trace tests in `worldwake-ai`
3. existing end-to-end `AskWitness` golden still passes under the strengthened trace contract -> `golden_stale_prerequisite_ask_witness_chain`
4. planner-contract documentation matches the live omission taxonomy -> doc review plus command-based verification
5. this is a single-layer AI traceability ticket; additional authoritative/action-layer mapping is not applicable because the behavior contract is unchanged

## What to Change

### 1. Refine epistemic root-omission tracing

Extend the planner root trace surface so conditional epistemic-barrier omissions can distinguish at least:

- no stale epistemic subjects were derived
- stale subjects existed but no `AskWitness` affordance target was available

The final representation can be either new `RootOperatorOmissionReason` variants plus structured `AskWitness` detail or a dedicated structured omission payload for conditional epistemic barriers, but it must remain machine-readable and specific.

### 2. Thread the omission provenance through the existing search root boundary

Update [search/candidates.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs) so the new epistemic omission detail is emitted from the same root-candidate pass that currently records `RootCandidateTrace` and `RootOperatorOmissionTrace`. That root-boundary pass must record omission provenance for conditional `AskWitness` barriers even though `AskWitness` is not part of `GoalKind::relevant_op_kinds()`. Do not derive it later from selected-plan symptoms.

### 3. Add focused tests for the new provenance

Add focused `worldwake-ai` tests that prove:

- a non-stale belief omits the conditional `AskWitness` barrier with the explicit “no stale epistemic subjects” reason
- stale epistemic subjects with no co-located witness affordance omit `AskWitness` with the explicit “no witness affordance” reason

### 4. Update planner traceability documentation

Update [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) so the root-omission section documents the new conditional epistemic-barrier omission contract and tells authors to use it instead of inferring the reason from missing root candidates.

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

1. Focused planner trace coverage proves conditional epistemic-barrier omission reasons distinguish stale-subject absence from missing witness availability
2. Existing `AskWitness` focused planner search coverage still passes
3. Existing `AskWitness` golden still passes
4. `cargo test -p worldwake-ai`

### Invariants

1. Root-candidate traceability explains conditional epistemic-barrier omission at the earliest planner boundary where the branch disappears.
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

## Outcome

- Completion date: 2026-03-29
- What changed:
  - extended the planner root omission contract to cover conditional `AskWitness` epistemic barriers through structured omission detail
  - added focused `worldwake-ai` search trace coverage for the two live omission causes: no stale epistemic subjects and no witness affordance
  - updated `docs/planner-contracts.md` to document the conditional barrier omission contract explicitly
- Deviations from original plan:
  - did not implement a payload-mismatch omission reason because reassessment confirmed that planner search does not currently preserve `ask_witness_memory`, so that distinction is not live-reachable at this boundary
  - narrowed scope to the two architecturally reachable omission causes instead of adding speculative trace taxonomy
- Verification results:
  - `cargo test -p worldwake-ai goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload -- --exact`
  - `cargo test -p worldwake-ai goal_model::tests::grounded_goal_epistemic_barrier_matches_only_matching_payloads -- --exact`
  - `cargo test -p worldwake-ai golden_stale_prerequisite_ask_witness_chain -- --exact`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
