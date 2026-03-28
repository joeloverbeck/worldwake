# S34GENEPIACT-006: Candidate generation — emit_verify_belief_goals()

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai`: second-pass `VerifyBelief` candidate emission in [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) plus focused candidate-generation coverage
**Deps**: [archive/tickets/S34GENEPIACT-005.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S34GENEPIACT-005.md) (completed planner/operator surface for `VerifyBelief`), [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md)

## Problem

The live planner and runtime action surface for `GoalKind::VerifyBelief` exists, but candidate generation still never emits `VerifyBelief` goals. That leaves the epistemic action family unreachable from the AI pipeline: agents can only discover stale beliefs reactively through passive observation or violation detection, not proactively because a currently generated goal depends on low-confidence evidence.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is the second-pass candidate-evidence contract across [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), [`GroundedGoal.evidence_entities` / `evidence_places`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), and the belief-confidence read surface on [`GoalBeliefView`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) plus [`worldwake_core::belief_confidence()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs).
2. The current candidate-generation file already follows the intended structural pattern: focused `emit_*` helpers guarded by profile/state and emitting `GroundedGoal`s through `emit_candidate_with_trace()`. `generate_candidates_with_travel_horizon()` currently ends with `emit_recorded_violation_candidates()` and `emit_expectation_violation_candidates()`, then filters blocked candidates. There is no `emit_verify_belief_goals()` call yet.
3. The spec is still correct that `emit_verify_belief_goals()` should be a second-pass scan over already-emitted goals rather than a broad scan of the whole belief store. That preserves goal relevance under Principles 5 and 18 in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md).
4. The belief-confidence substrate is already live. [`worldwake_core::belief_confidence()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) derives confidence from provenance and staleness, and [`GoalBeliefView::belief_confidence_policy()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) already exposes the acting agent's policy.
5. Reassessment initially suggested the goal-formation boundary already exposed `verification_disposition_profile()`, but the live split is narrower: [`RuntimeBeliefView`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) exposes it while [`GoalBeliefView`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) does not. Because candidate generation runs on `GoalBeliefView`, this ticket must extend that belief-facing boundary cleanly rather than reaching around it. `ask_witness_memory()` remains out of scope for candidate generation and already lives on the existing boundary surface.
6. `GoalKind::VerifyBelief`, `GoalKindTag::VerifyBelief`, `VERIFY_BELIEF_OPS`, and the planner barrier/payload surface are already live from [archive/tickets/S34GENEPIACT-005.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S34GENEPIACT-005.md). This ticket must only make candidate generation produce lawful grounded opportunities for that existing goal family.
7. `GroundedGoal` already stores `evidence_entities` and `evidence_places`, so there is no separate evidence-extraction infrastructure gap. The missing work is selecting low-confidence belief dependencies from existing grounded candidates, mapping them to `VerificationSubject`, deduplicating by subject, and emitting the verification goal with the correct anchor/evidence.
8. Ask-memory suppression for `AskWitness` is already owned by the live affordance/runtime surface in [crates/worldwake-systems/src/epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs). Candidate generation should emit `VerifyBelief` goals and let the planner/affordance layer decide whether the available terminal is `verify_belief` or `ask_witness`.
9. The live ranking phase still hardcodes `GoalKind::VerifyBelief` motive to zero in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs); that is ticket 007's gap. This ticket is therefore candidate-generation scoped plus the minimal belief-view boundary extension required to read the verification profile lawfully. It should prove raw candidate emission, not full end-to-end selection inside ranking.
10. Coverage reassessment: `cargo test -p worldwake-ai -- --list` shows extensive focused tests under `candidate_generation::tests`, but no test names covering `VerifyBelief` candidate emission. The gap is missing focused candidate-generation coverage, not missing golden coverage.

## Architecture Check

1. The cleaner architecture is to derive proactive verification from already-grounded opportunities instead of inventing a parallel "belief refresh sweep" subsystem. That keeps one canonical trigger for epistemic work: a currently lawful goal depends on stale or low-confidence evidence, so the AI emits a goal to refresh that exact dependency.
2. The second-pass design scales better than goal-specific special cases. It reuses `GroundedGoal` evidence as the shared dependency language across candidate generation, planning, and later trace/debugging surfaces.
3. `VerificationSubject` must remain the canonical identity for deduplication and downstream planner/runtime handling. Candidate generation should map low-confidence evidence into that subject once, not introduce alias keys or goal-family-specific duplicate bookkeeping.
4. The minimal `GoalBeliefView` extension is architecturally preferable to giving candidate generation privileged access to runtime-only surfaces. Goal formation needs the acting agent's verification profile, so that profile belongs on the goal-formation boundary.
5. No backwards-compatibility shims and no fallback broad scan of every belief on every tick.

## Verification Layers

1. Low-confidence evidence dependency produces a `VerifyBelief` candidate -> focused `candidate_generation` unit coverage
2. Missing `VerificationDispositionProfile` suppresses verification candidate generation -> focused `candidate_generation` unit coverage
3. Candidate generation uses already-emitted grounded-goal evidence rather than a global belief sweep -> focused `candidate_generation` unit coverage
4. Duplicate low-confidence dependencies collapse to one `VerificationSubject` -> focused `candidate_generation` unit coverage
5. Resource-source evidence emits `VerificationSubject::SupplyAvailability` rather than a generic location refresh -> focused `candidate_generation` unit coverage
6. Fresh or high-confidence evidence does not emit unnecessary verification work -> focused `candidate_generation` unit coverage
7. Single-layer ticket: ranking, planner search, action execution, and goldens are owned by tickets 007-009, so no extra mixed-layer proof mapping is required here

## What to Change

### 1. Add `emit_verify_belief_goals()` second-pass emission

In [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), add:

```rust
pub(crate) fn emit_verify_belief_goals(
    ctx: &GenerationContext<'_>,
    candidates: &mut Vec<GroundedGoal>,
    diagnostics: &mut CandidateGenerationDiagnostics,
) {
    // 1. Guard: return if agent lacks VerificationDispositionProfile
    // 2. Scan already-emitted candidates for low-confidence evidence entities
    // 3. For each evidence entity, check belief_confidence < threshold
    // 4. Map stale belief to canonical VerificationSubject
    // 5. Deduplicate by VerificationSubject
    // 6. Emit VerifyBelief { subject, generation_tick: ctx.current_tick }
}
```

The implementation should also emit an evidence/knowledge-path trace for the verification goal. The strongest local provenance is the stale belief that justified verification, so the candidate trace should record that belief rather than inventing new contributor categories.

### 2. Call from `generate_candidates_with_travel_horizon()`

Call `emit_verify_belief_goals()` after the existing domain emitters in `generate_candidates_with_travel_horizon()` and before blocked-candidate filtering. This preserves the intended second-pass scan over the live candidate set while still letting blocker filtering suppress the derived verification opportunity if appropriate.

### 3. Map grounded evidence to `VerificationSubject`

Use the existing `GroundedGoal.evidence_entities` set as the candidate-dependency surface. For each evidence entity with a known belief:

- if the belief has `resource_source: Some(resource)` and `last_known_place: Some(place)`, emit `VerificationSubject::SupplyAvailability { commodity: resource.commodity, source: entity, place }`
- otherwise, if the belief has `last_known_place: Some(place)`, emit `VerificationSubject::EntityLocation { entity, place }`

This keeps supply verification more specific than a generic location refresh when the belief dependency is a concrete source.

### 4. Extend `GoalBeliefView` with the verification profile

Because candidate generation already runs on `GoalBeliefView`, add `verification_disposition_profile()` to that trait and delegate it through the existing `impl_goal_belief_view!` bridge from `RuntimeBeliefView`. This is the smallest clean boundary change that keeps candidate generation belief-facing.

## Files to Touch

- [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
- [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs)

## Out of Scope

- Ask-memory suppression and `ask_witness` affordance enumeration details; those already live in [crates/worldwake-systems/src/epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs)
- Ranking and motive scoring (`GoalKind::VerifyBelief` is still zero-motive until ticket 007)
- Golden E2E scenarios (ticket 008)
- Any additional belief-view expansion beyond exposing `verification_disposition_profile()` on `GoalBeliefView`
- Belief-confidence arithmetic or `BeliefConfidencePolicy`
- Planner search / planner-op closure, already completed in [archive/tickets/S34GENEPIACT-005.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S34GENEPIACT-005.md)

## Acceptance Criteria

### Tests That Must Pass

1. `VerifyBelief` candidate is emitted only when a currently grounded goal depends on evidence whose confidence falls below `VerificationDispositionProfile::belief_verification_threshold`
2. No verification candidate is emitted when the agent lacks `VerificationDispositionProfile`
3. Verification emission is driven by already-emitted grounded-goal evidence, not by scanning unrelated beliefs
4. Duplicate low-confidence references to the same `VerificationSubject` emit only one verification candidate per generation pass
5. Resource-source beliefs emit `VerificationSubject::SupplyAvailability`
6. High-confidence or fresh beliefs do not emit `VerifyBelief`
7. Focused candidate-generation tests and `cargo test -p worldwake-ai` pass

### Invariants

1. Candidate generation reads the belief-facing AI boundary only, never authoritative world state, when deriving verification goals
2. Verification candidates are goal-relevant because they are derived from already-grounded evidence dependencies
3. Deduplication happens on canonical `VerificationSubject`, not on lossy ad-hoc tuples
4. Determinism is preserved: use sorted iteration / `BTreeSet`, never `HashMap` / `HashSet`

## Tests

### New/Modified Tests

1. `candidate_generation::tests::verify_belief_emits_for_low_confidence_evidence_dependency`
Rationale: proves the second-pass scan derives epistemic work from an already-grounded dependency instead of a global sweep.

2. `candidate_generation::tests::verify_belief_requires_verification_profile`
Rationale: proves proactive verification remains profile-driven and optional per agent.

3. `candidate_generation::tests::verify_belief_deduplicates_same_subject_across_multiple_candidates`
Rationale: proves duplicate grounded dependencies do not proliferate identical verification work.

4. `candidate_generation::tests::verify_belief_emits_supply_subject_for_stale_resource_source`
Rationale: proves resource-source evidence maps to the more specific supply-verification contract.

5. `candidate_generation::tests::verify_belief_skips_high_confidence_evidence`
Rationale: proves the ticket does not turn verification into a noisy default behavior.

6. `candidate_generation::tests::verify_belief_adds_belief_provenance_trace`
Rationale: proves the emitted candidate remains explainable through the stale belief that justified it.

7. `worldwake-sim` belief-view trait delegation compiles with the new verification-profile accessor
Rationale: proves the candidate-generation boundary change stays explicit and centralized rather than creating a one-off escape hatch.

### Commands

1. `cargo test -p worldwake-ai verify_belief --lib`
2. `cargo test -p worldwake-ai candidate_generation::tests::verify_belief_emits_for_low_confidence_evidence_dependency -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
5. `cargo build --workspace`

## Outcome

- Completed: 2026-03-28
- What changed:
  - Added second-pass `emit_verify_belief_goals()` emission in [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), driven by already-grounded evidence entities and canonical `VerificationSubject` deduplication.
  - Added belief-provenance traces for emitted verification candidates so the stale belief that justified verification remains explainable in diagnostics.
  - Extended [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) so `GoalBeliefView` lawfully exposes `verification_disposition_profile()` through the existing runtime-to-goal delegation boundary.
  - Added focused `candidate_generation` tests for low-confidence emission, profile gating, deduplication, supply-subject mapping, high-confidence suppression, and provenance tracing.
- Deviations from original plan:
  - Reassessment during implementation showed `GoalBeliefView` did not actually expose `verification_disposition_profile()`, so the ticket grew to include that minimal boundary fix. The earlier reassessment text was corrected accordingly.
  - The implementation emits verification candidates with diagnostics provenance immediately rather than adding a later follow-up just for traceability.
- Verification results:
  - `cargo test -p worldwake-ai verify_belief --lib`
  - `cargo test -p worldwake-ai candidate_generation::tests::verify_belief_emits_for_low_confidence_evidence_dependency -- --exact`
  - `cargo test -p worldwake-ai`
  - `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
  - `cargo build --workspace`
