# S34GENEPIACT-007: Ranking — verification motive and explicit provenance

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai`: complete `VerifyBelief` ranking with profile-driven motive and explicit ranking provenance
**Deps**: [archive/tickets/completed/S34GENEPIACT-006.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-006.md) (completed candidate emission), [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md)

## Problem

`VerifyBelief` still has an incomplete ranking surface. The family already has suppression policy and low-priority-class wiring, but `motive_score()` still hardcodes `GoalKind::VerifyBelief` to `0`. Candidate emission from ticket 006 now makes that gap live: emitted verification candidates are filtered out by the zero-motive path instead of participating in ranking through `VerificationDispositionProfile`.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is the ranking/provenance contract across [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), [`RankedGoalProvenance` in `goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), and the human-readable ranking summary in [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs). If verification ranking becomes explicit, that provenance must remain explainable through this whole boundary rather than existing only as an opaque `u32`.
2. `ranking.rs` contains `rank_candidates()`, `goal_ranking_provenance()`, and motive scoring functions. `GoalKind::VerifyBelief { .. }` is already classified as `GoalPriorityClass::Low`, but `motive_score()` still returns `0` and `goal_ranking_provenance()` still falls through to `None`.
3. The spec says `GoalPriorityClass::Low` for `VerifyBelief` — same as `InvestigateViolation`. That policy is already live in both [crates/worldwake-ai/src/goal_policy.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs) and [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), so this ticket must not re-add it.
4. `GoalFamilyPolicy` already includes `GoalKind::VerifyBelief { .. }` under the low-priority suppressed-under-stress family. The architectural gap is motive and provenance, not family registration.
5. The spec says motive score comes from `VerificationDispositionProfile::verification_motive_weight`. That weight is per-agent, not per-candidate. It governs whether verification participates relative to other low-priority goal families, not relative ordering among multiple `VerifyBelief` candidates for the same agent.
6. The zero-motive filter in `rank_candidates()` is still the right contract. After this ticket, `VerifyBelief` should participate only when the agent’s `VerificationDispositionProfile` supplies positive motive weight; lack of profile or zero weight should continue to suppress the goal, preserving P20 diversity in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md).
7. Mismatch + correction: earlier ticket wording still implied policy registration and priority assignment work. Those are already delivered. The remaining work is:
   - motive scoring
   - explicit verification provenance that does not alias drive provenance
   - focused coverage proving emitted `VerifyBelief` candidates are no longer silently zeroed out
8. Follow-on confirmation after ticket 006 completion: [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) now emits `VerifyBelief` candidates, but [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) still returns `0` for `GoalKind::VerifyBelief` in `motive_score()`. This ticket is now the active owner of making emitted verification candidates selectable through the normal ranking path.

## Architecture Check

1. The clean fix is to complete the existing ranking path rather than bolting on exceptions into candidate generation or planner search. `VerifyBelief` should rank the same way other goal families do: suppression/policy from family policy, motive from concrete per-agent profile state, and provenance from an explicit ranking input type.
2. Do not alias verification into `RankedGoalProvenance::Drive`. Verification is not a bodily drive, and overloading the drive provenance shape would make ranking traces less truthful. If explicit provenance is added, it should be a dedicated verification form.
3. No backwards-compatibility shims. This ticket extends existing ranking dispatch instead of adding a “verification goals bypass zero-motive filter” special case.

## Verification Layers

1. `VerifyBelief` receives non-zero motive from `VerificationDispositionProfile` instead of the current hardcoded zero -> focused ranking test
2. Zero-motive filter still suppresses `VerifyBelief` when profile weight is zero or absent -> focused ranking test
3. `VerifyBelief` remains `GoalPriorityClass::Low` when ranked and is suppressed under higher-priority survival pressure by the already-live family policy -> focused ranking test
4. Ranking provenance for `VerifyBelief` is explicit and profile-driven rather than falling through a generic or empty path -> focused ranking/summary test
5. Decision-trace summary remains explanatory after the provenance change -> focused decision-trace test
6. Single-layer ranking/provenance ticket. Planner closure belongs to ticket 005 and candidate emission belongs to ticket 006.

## What to Change

### 1. Add verification motive scoring

In `crates/worldwake-ai/src/ranking.rs`, add:

```rust
fn verification_motive(candidate: &GroundedGoal, context: &RankingContext<'_>) -> u32 {
    let Some(profile) = context.view.verification_disposition_profile(context.agent) else {
        return 0;
    };
    u32::from(profile.verification_motive_weight.value())
}
```

### 2. Add explicit verification provenance

Extend `RankedGoalProvenance` with a dedicated verification variant that records:

- final priority class
- verification motive weight used

and wire `GoalKind::VerifyBelief { .. }` through that explicit variant. Update decision-trace summary rendering so verification-ranked goals remain explainable.

### 3. Wire into motive_score()

Add the `VerifyBelief` dispatch in `motive_score()` to call `verification_motive()`.

### 4. Add focused coverage for the already-live policy contract

Do not change `GoalFamilyPolicy` unless reassessment finds a real bug. Instead, add focused tests proving that:

- `VerifyBelief` remains `GoalPriorityClass::Low`
- non-zero verification motive survives the ranking path
- zero or missing motive still suppresses the goal
- verification provenance is explicit rather than absent or misclassified as a drive

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — add verification motive/provenance, tests)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add explicit verification provenance type if reassessment still shows no truthful existing variant)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — keep ranked-goal summaries explanatory after provenance addition)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (test-only — update exhaustive provenance matches for the new explicit variant)
- `crates/worldwake-ai/tests/golden_combat.rs` (test-only — update exhaustive provenance matches for the new explicit variant)
- `crates/worldwake-ai/src/goal_policy.rs` (no change unless reassessment finds a real policy bug)

## Out of Scope

- Candidate generation — ticket 006
- Planner ops and goal model — ticket 005
- Action handlers — tickets 003/004
- Golden E2E tests — ticket 008
- Changes to `GoalPriorityClass` enum or `GoalFamilyPolicy` unless reassessment finds a live contradiction
- Interrupt/goal-switching logic for VerifyBelief (the low-priority family contract is already live)
- Feasibility sketching dispatch for VerifyBelief (already exists in `feasibility.rs`)

## Acceptance Criteria

### Tests That Must Pass

1. `verification_motive()` returns `verification_motive_weight` from the agent’s `VerificationDispositionProfile`
2. `verification_motive()` returns `0` when the profile is absent or weight is zero
3. `VerifyBelief` is suppressed by the zero-motive filter when motive is `0`
4. `VerifyBelief` remains `GoalPriorityClass::Low` when ranked and remains suppressed under critical stress per the existing family policy
5. Verification-ranked goals remain explainable through `DecisionOutcome::summary()` or equivalent trace output after the provenance change
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `GoalPriorityClass::Low` remains fixed for `VerifyBelief`
2. Ranking reads belief/profile state only
3. Zero-motive filter still applies to `VerifyBelief`
4. No candidate-generation or planner special case is needed to make verification goals rank
5. Verification provenance is explicit and truthful, not an alias of bodily-drive provenance

## Test Plan

### New/Modified Tests

1. `ranking::tests::verify_belief_uses_profile_driven_motive_and_explicit_provenance`
Rationale: proves emitted verification goals now participate in ranking through the per-agent profile and remain explainable through explicit provenance.

2. `ranking::tests::verify_belief_without_profile_is_zero_motive_and_filtered`
Rationale: proves the existing zero-motive contract still suppresses verification when the architecture says that agent type does not proactively verify.

3. `ranking::tests::verify_belief_is_suppressed_under_critical_survival_pressure`
Rationale: proves the ranking fix does not reopen priority inversion and that the already-live suppression contract still wins under critical self-care pressure.

4. `decision_trace::tests::summary_planning_includes_selected_verification_provenance`
Rationale: proves the trace/debugging surface stays truthful after adding explicit verification provenance.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `cargo build --workspace`

## Outcome

- Completed: 2026-03-28
- What changed:
  - Added `RankedVerificationGoalProvenance` plus `RankedGoalProvenance::Verification` so verification ranking is explicit instead of falling back to an opaque score or aliasing bodily-drive provenance.
  - Wired `GoalKind::VerifyBelief` through `verification_motive()` and explicit provenance in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), preserving the existing zero-motive filter and low-priority policy contract.
  - Updated [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) so ranked-goal summaries explain verification motive weight directly.
  - Added focused ranking and decision-trace tests, and updated existing exhaustive provenance matches in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) and [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs) for the new provenance variant.
- Deviations from original plan:
  - Reassessment narrowed the ticket away from policy/family registration and toward the ranking/provenance boundary only.
  - Focused coverage proved the stronger live invariant is “suppressed under critical survival pressure” rather than merely “ranks below survival after both survive suppression,” so the test plan was corrected to that architectural contract.
  - The explicit provenance variant required small follow-on test updates outside the original file list because several existing tests matched the provenance enum exhaustively.
- Verification results:
  - `cargo test -p worldwake-ai verify_belief --lib`
  - `cargo test -p worldwake-ai summary_planning_includes_selected_verification_provenance --lib`
  - `cargo test -p worldwake-ai`
  - `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
  - `cargo build --workspace`
