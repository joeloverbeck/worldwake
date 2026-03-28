# S34GENEPIACT-005: Planner ops — classify epistemic actions and expose terminal planner operators

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai`: epistemic planner op kinds, planner semantics classification, `GoalKind::VerifyBelief` operator surface, epistemic payload validation/synthesis, binding semantics, barrier behavior, focused planner/search coverage
**Deps**: S34GENEPIACT-003 (completed: `verify_belief` action is live), S34GENEPIACT-004 (completed: `ask_witness` action is live), [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md)

## Problem

The AI planner still cannot close the live epistemic action loop. `GoalKind::VerifyBelief` already exists, but its planner operator set is empty, `build_semantics_table()` still leaves both live epistemic actions unclassified, and the current binding contract only understands place-bound verification, which rejects `ask_witness` affordances because their authoritative target is the witness agent rather than the place under verification. If ticket 006 starts emitting `VerifyBelief` candidates before this is fixed, the planner will generate a lawful goal family with no complete terminal planner path.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is the planner-facing epistemic contract across [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs), [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), and the search root-candidate seam in [crates/worldwake-ai/src/search/candidates.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs) plus [crates/worldwake-ai/src/search/transition.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/transition.rs).
2. `GoalKindTag::VerifyBelief` and the core `GoalKind::VerifyBelief` architecture are already live. In [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), `goal_kind_tag()`, `relevant_observed_commodities()`, `goal_relevant_places()`, and the surrounding planner extension structure already know about the goal family. The original ticket scope overstated this missing work.
3. The current planner gap is still real: `VERIFY_BELIEF_OPS` is `&[]` in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), so the goal family exposes no relevant planner operators even though the goal itself exists.
4. Both epistemic actions are now live, not just `verify_belief`. [crates/worldwake-systems/src/action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs) registers both `register_verify_belief_action()` and `register_ask_witness_action()`, and [crates/worldwake-systems/src/epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs) defines both actions under `ActionDomain::Epistemic`. This ticket should no longer treat `AskWitness` as optional or deferred-by-default.
5. `PlannerOpKind` in [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs) still lacks both `VerifyBelief` and `AskWitness`, and `classify_action_def()` has no `ActionDomain::Epistemic` mapping. The planner semantics table therefore still fails to classify live epistemic defs.
6. The existing planner semantics inventory test is also stale. The live test symbol is `planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs`, but the current ticket command `cargo test -p worldwake-ai build_semantics_table_classifies_registered_planner_action_defs` does not run that unit test because it omits the module path. The ticket’s verification commands must be corrected to real runnable filters.
7. The current `VerifyBelief` binding contract is incomplete for witness queries. In [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), `matches_binding()` treats `GoalKind::VerifyBelief` as place-bound for all terminal ops. That is correct for `verify_belief` but rejects `ask_witness` affordances because those candidates bind the witness agent as the authoritative target.
8. The current affordance-payload contract is also too loose for `VerifyBelief`. `payload_override_from_affordance()` in [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) validates payloads for `ShareBelief`, `InvestigateViolation`, and `Accuse`, but it has no `VerifyBelief` branch. If we merely relax `AskWitness` binding without adding payload validation, an unrelated witness-topic payload could satisfy the goal. The canonical subject must remain the single source of truth.
9. `build_payload_override()` already has the correct architectural hook for canonical payload handling. `VerifyBeliefPayload { subject }` can and should be synthesized directly from the goal’s canonical `VerificationSubject`. `AskWitness` should remain affordance-driven at the payload level because the witness target is inherently contingent on co-located available agents; the planner should validate affordance payloads against the goal subject, not invent witness targets out of thin air.
10. `is_progress_barrier()` already treats `Tell`, `Investigate`, `Accuse`, punishment ops, and selected political terminals as explicit barriers. Epistemic terminals belong in the same family because the planner cannot lawfully predict what direct observation or witness testimony will reveal.
11. `apply_planner_step()` currently leaves `GoalKind::VerifyBelief` unsatisfied and side-effect free, which is the correct architecture. Epistemic terminals should stay barriers rather than mutating hypothetical belief state into guessed post-observation knowledge.
12. There is currently no focused planner/search coverage mentioning `VerifyBelief` or `AskWitness` in `worldwake-ai`, and no `conformance_verify_belief` or `conformance_ask_witness` test in [crates/worldwake-ai/tests/planner_conformance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/planner_conformance.rs). This is a focused planner coverage gap, not just a missing golden scenario.

## Architecture Check

1. The clean architecture is to classify live epistemic actions as first-class planner ops and keep `VerificationSubject` as the canonical identity across goal model, payload synthesis, and payload validation. That gives one honest planner contract instead of a live runtime action registry plus planner-side escape hatches.
2. `VerifyBelief` should synthesize its own payload from the canonical subject, while `AskWitness` should remain affordance-selected and then validated against that same subject. This is cleaner than adding an alias planner-only witness target field or letting the planner fabricate socially contingent witness choices.
3. Epistemic terminals should mirror the existing barrier architecture used by `Investigate` and `Tell`: explicit terminal barriers with `GoalModelFallback`, not speculative belief mutation. That preserves locality, partial observability, and belief-vs-truth separation under Principles 7, 12, 13, 14, 18, and 25 of [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md).
4. No backwards-compatibility shims, no duplicate epistemic identity paths, and no permanent “live action may remain unclassified” exception.

## Verification Layers

1. Live epistemic action defs classify into planner semantics with no stale escape hatch -> focused `planner_ops` unit tests
2. `GoalKind::VerifyBelief` exposes the correct planner-op surface and barrier contract -> focused `goal_model` unit tests
3. `VerifyBelief` payload synthesis remains canonical and subject-driven -> focused `goal_model` unit tests
4. `AskWitness` is admitted only for witness payloads that match the goal’s canonical subject -> focused `goal_model` and search/root-candidate tests
5. Planner constructs `Travel -> VerifyBelief` for remote verification and can terminate on epistemic barriers -> focused search tests
6. Planner search/runtime must preserve the actor verification profile across the planning snapshot/state boundary so epistemic affordances and duration expressions remain visible inside search -> focused `planning_state` and duration-contract unit tests
7. Full regression safety for planner-side epistemic integration -> `cargo test -p worldwake-ai` and `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## What to Change

### 1. Classify live epistemic action defs into planner ops

In [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs):

- add `PlannerOpKind::VerifyBelief`
- add `PlannerOpKind::AskWitness`
- classify `ActionDomain::Epistemic` action defs by name:
  - `verify_belief` -> `PlannerOpKind::VerifyBelief`
  - `ask_witness` -> `PlannerOpKind::AskWitness`
- add semantics entries for both:
  - `may_appear_mid_plan = false`
  - `is_materialization_barrier = false`
  - `transition_kind = GoalModelFallback`
  - `relevant_goal_kinds = &[GoalKindTag::VerifyBelief]`
- remove the temporary unclassified-action expectation and prove the live registry classifies both epistemic actions

### 2. Fill the existing `VerifyBelief` goal family operator surface

In [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs):

- change `VERIFY_BELIEF_OPS` from `&[]` to:
  - `Travel`
  - `VerifyBelief`
  - `AskWitness`
- keep `Travel` as the place-reaching prerequisite op
- keep both epistemic terminals as leaf-only barrier operators for this goal family

### 3. Finish the remaining epistemic payload and binding contract

In [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs):

- extend `payload_override_from_affordance()` so `GoalKind::VerifyBelief` only accepts:
  - `ActionPayload::VerifyBelief` whose `subject` exactly matches the goal subject
  - `ActionPayload::AskWitness` whose topic matches the goal subject
- extend `build_payload_override()` so `PlannerOpKind::VerifyBelief` synthesizes `VerifyBeliefPayload { subject }` from the goal’s canonical `VerificationSubject`
- do not synthesize `AskWitnessPayload` without an affordance payload; witness identity must come from the live affordance surface, not planner guesswork
- update `matches_binding()` so `AskWitness` is not incorrectly rejected just because its authoritative target is the witness rather than the place; keep `verify_belief` itself place-bound
- extend `is_progress_barrier()` so both epistemic terminals are explicit barriers for `GoalKind::VerifyBelief`
- keep `apply_planner_step()` side-effect free for epistemic terminals

### 4. Add focused planner/search coverage

- prove the semantics table fully classifies both live epistemic actions
- prove `GoalKind::VerifyBelief` no longer reports an empty relevant-op set
- prove canonical `VerifyBeliefPayload` synthesis
- prove mismatched `ask_witness` payloads are rejected for a `VerifyBelief` goal
- prove planner search can build `Travel -> VerifyBelief` for a remote verification goal
- prove planner search can terminate on `AskWitness` for a co-located witness payload that matches the goal subject
- preserve actor verification disposition through `PlanningSnapshot` / `PlanningState` so epistemic affordances and duration estimation stay lawful inside search

## Files to Touch

- [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs)
- [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [crates/worldwake-ai/src/search/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/tests.rs) if dedicated search coverage is cleaner there
- [crates/worldwake-ai/tests/planner_conformance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/planner_conformance.rs) only if conformance-level proof becomes necessary after focused search coverage

## Out of Scope

- Candidate generation (`emit_verify_belief_goals`) — ticket 006
- Ranking and motive scoring — ticket 007
- Golden E2E tests — ticket 008
- Authoritative runtime handler semantics, belief mutation, action tracing, and ask-memory behavior — tickets 003, 004, and 009 already own those layers
- Any hypothetical post-observation or post-testimony belief mutation in `PlanningState`

## Acceptance Criteria

1. `build_semantics_table()` classifies both live epistemic actions instead of leaving them outside planner semantics
2. `GoalKind::VerifyBelief` no longer reports an empty relevant-op set
3. Planner constructs `Travel -> VerifyBelief` for a remote `VerifyBelief` goal
4. Planner can terminate on `AskWitness` for a co-located witness only when the witness payload matches the goal subject
5. `VerifyBelief` and `AskWitness` are explicit progress barriers for `GoalKind::VerifyBelief`
6. `VerifyBelief` remains planner-barrier-driven in this ticket; no speculative or satisfaction-based post-observation closure is added
7. No live epistemic action remains in a planner-classification escape hatch

## Invariants

1. Planner only reasons from belief-facing state and canonical goal identity, never authoritative omniscience, when handling `VerifyBelief`
2. Epistemic terminal ops remain explicit barriers; planner does not hallucinate post-observation or post-testimony outcomes
3. `VerificationSubject` remains the canonical cross-layer identity for epistemic verification
4. Determinism is preserved: no `HashMap`, no floats, no wall-clock time in planner paths

## Tests

### New/Modified Tests

1. `planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs`
Rationale: proves the live registry no longer leaves `verify_belief` or `ask_witness` outside planner semantics.

2. `goal_model::tests::verify_belief_goal_relevant_ops_include_epistemic_terminals`
Rationale: proves the goal family no longer exposes an empty operator surface.

3. `goal_model::tests::verify_belief_epistemic_terminals_are_progress_barriers`
Rationale: proves epistemic terminals close as explicit planner barriers instead of speculative state mutation.

4. `goal_model::tests::verify_belief_goal_builds_verify_belief_payload_override`
Rationale: proves canonical `VerificationSubject` identity is the source of truth for `VerifyBelief` payloads.

5. `goal_model::tests::verify_belief_goal_rejects_mismatched_ask_witness_affordance_payload`
Rationale: proves the goal model, not ad-hoc search behavior, owns the epistemic identity contract.

6. `goal_model::tests::search_verify_belief_returns_travel_then_verify_belief_barrier_for_remote_subject`
Rationale: proves the planner can close a remote epistemic plan without candidate-generation work from ticket 006.

7. `goal_model::tests::search_verify_belief_returns_ask_witness_barrier_for_matching_colocated_payload`
Rationale: proves the planner can use the live social epistemic terminal when the affordance payload lawfully matches the goal subject.

8. `planning_state::tests::planning_state_matches_runtime_duration_estimation_for_dynamic_duration_contract`
Rationale: proves planner search still sees the actor verification disposition after snapshot conversion, which epistemic affordances and duration estimation require.

9. `planner_duration_contract::tests::planner_duration_inventory_matches_live_non_fixed_planner_surface`
Rationale: proves the duration dependency inventory covers the live epistemic duration expressions added to planner-visible actions.

### Commands

1. `cargo test -p worldwake-ai planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs -- --exact`
2. `cargo test -p worldwake-ai verify_belief --lib`
3. `cargo test -p worldwake-ai planner_duration_contract::tests::planner_duration_inventory_matches_live_non_fixed_planner_surface -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-28
- What changed:
  - Classified both live epistemic actions as first-class planner ops.
  - Filled `GoalKind::VerifyBelief` with the real operator surface: `Travel`, `VerifyBelief`, and `AskWitness`.
  - Synthesized canonical `VerifyBeliefPayload` from `VerificationSubject`, validated `AskWitness` affordance payloads against that same subject, and fixed witness-target binding semantics.
  - Marked both epistemic terminals as explicit planner barriers and updated planner exhaustiveness sites that pattern-match `PlannerOpKind`.
  - Preserved `VerificationDispositionProfile` through `PlanningSnapshot` and `PlanningState`, which the reassessment showed was required for lawful epistemic affordances and duration estimation during search.
  - Extended the planner duration dependency inventory for the live epistemic duration expressions.
- Deviations from original plan:
  - The ticket originally assumed `VerifyBelief` satisfaction should remain generation-tick based. Live code does not currently satisfy `VerifyBelief` via `is_satisfied()`, and this ticket intentionally kept closure barrier-based rather than introducing speculative post-observation planner state.
  - The ticket originally scoped only planner-op classification and goal-model exposure. Reassessment showed the planner snapshot/state boundary also needed to carry the actor verification profile, so that boundary fix was included as part of this ticket’s planner-architecture closure.
- Verification results:
  - `cargo test -p worldwake-ai planner_ops::tests::build_semantics_table_classifies_registered_planner_action_defs -- --exact`
  - `cargo test -p worldwake-ai verify_belief --lib`
  - `cargo test -p worldwake-ai`
  - `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
