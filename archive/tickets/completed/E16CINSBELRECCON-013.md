# E16CINSBELRECCON-013: Reassess Institutional-Belief Ranking/Failure Scope

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — reassessment corrected the scope to existing architecture plus focused verification
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-009, E16CINSBELRECCON-011, E16CINSBELRECCON-012, `specs/E16c-institutional-beliefs-and-record-consultation.md`

## Problem

This ticket was drafted from an earlier E16c Phase B2 narrative that no longer matches the live AI architecture. If implemented as written, it would add redundant ranking and blocker state on top of an architecture that already resolves conflicted institutional beliefs at candidate generation and already clears stale political starts through the shared S08 start-failure recovery path.

## Assumption Reassessment (2026-03-22)

1. `GoalKind` has political goals `ClaimOffice` and `SupportCandidateForOffice`, but there is no `GoalKind::ConsultRecord` in [goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs). `ConsultRecord` is a `PlannerOpKind` prerequisite step in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), not a standalone ranked goal. The original ticket's "ConsultRecord needs its own priority assignment" assumption is false.
2. Political unknown/certain/conflicted institutional reads are already handled in candidate generation, not deferred to ranking. [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) suppresses political candidates on `InstitutionalBeliefRead::Conflicted(_)`, requires consultable record evidence for `Unknown`, and emits ordinary political goals only when the belief surface allows commitment. Focused coverage already exists in `political_candidates_use_institutional_beliefs_for_unknown_certain_and_conflicted_reads`, `political_candidates_unknown_belief_require_consultable_record_evidence`, and `political_candidates_suppress_conflicted_support_beliefs`.
3. Current ranking only sees the surviving political goals. [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) ranks `ClaimOffice` as `GoalPriorityClass::Medium` with enterprise-weight motive and `SupportCandidateForOffice` as `GoalPriorityClass::Low` with social/loyalty motive. Existing focused coverage already asserts the live ranking substrate in `claim_office_uses_enterprise_weight_and_medium_priority`. There is no current ranking path where conflicted political beliefs remain candidates and need an extra motive reduction.
4. `failure_handling.rs` still records generic `BlockingFact` values in [blocked_intent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs). There are no `InstitutionalBeliefStale` or `InstitutionalBeliefConflicted` variants today, and adding them would duplicate information that already lives in the institutional belief store and candidate omission diagnostics.
5. The current stale political failure boundary is authoritative action start, not ranking. In [office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs), `validate_declare_support_context_in_world()` rejects a closed office with `ActionError::PreconditionFailed("office ... is not vacant")`. In [failure_handling.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs), that precondition-failure detail currently falls through to the shared `BlockingFact::Unknown` path.
6. The recovery contract is already covered end-to-end by `golden_remote_office_claim_start_failure_loses_gracefully` in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs): request resolution binds the stale `declare_support`, authoritative start fails once the office closes, shared S08 reconciliation clears the stale plan, and next-tick candidate generation omits `ClaimOffice` with `OfficeNotVisiblyVacant`.
7. Ordering for the stale political branch is not a ranking tie-breaker. The golden contract is a mixed-layer sequence: request resolution binds the user request, action lifecycle records `StartFailed`, authoritative politics closes the office before the failure, and next-tick candidate generation omits the political branch. The original ticket's ranking-centric framing was imprecise.
8. `contradiction_tolerance` exists on `PerceptionProfile`, but the live political architecture uses it for information acquisition/trust policy, not for "rank conflicted political candidates lower." The spec line suggesting motive reduction for conflicted beliefs is stale relative to the current earlier-boundary suppression design.
9. Mismatch + correction: this ticket should not add `BlockingFact` variants or ranking logic. The corrected scope is to document the divergence, preserve the cleaner architecture, and add focused coverage that the shared failure path intentionally remains generic while political retry suppression comes from updated beliefs and candidate omission.
10. The relevant live goal family is `ClaimOffice` / `SupportCandidateForOffice`, and the relevant prerequisite surface is `PlannerOpKind::ConsultRecord` in political search/planning, not a standalone consult-record goal family.

## Architecture Check

1. The current architecture is cleaner than the original proposal because conflicted institutional knowledge is resolved at the earliest causal boundary: candidate generation refuses to commit to institution-sensitive goals when the belief read is conflicted, and unknown beliefs route through consultable-record prerequisites. Adding ranking penalties for conflicted political goals would be redundant because those goals should not survive candidate generation in the first place.
2. Keeping stale political start failures on the shared `BlockingFact::Unknown` path is also cleaner than introducing institution-specific blocker variants today. The institutional reason already exists in the belief layer and in political candidate omission diagnostics on the next tick. Duplicating it in blocked-intent memory would create another partially overlapping epistemic taxonomy without improving the actual recovery behavior.
3. No backward-compatibility aliasing or shims are introduced. The reassessment preserves the current belief-first political design rather than reviving an outdated alternative.

## Verification Layers

1. Conflicted/unknown institutional beliefs gate political candidates at the AI boundary -> focused candidate-generation tests and political omission diagnostics
2. Remote consult-record prerequisite insertion for unknown office-holder beliefs -> focused goal-model/search tests
3. Stale political request reaches authoritative start failure only after office closure -> request-resolution trace + action trace + politics trace in `golden_remote_office_claim_start_failure_loses_gracefully`
4. AI recovery after the stale political start failure -> next-tick decision trace in `golden_remote_office_claim_start_failure_loses_gracefully`
5. Shared blocker classification for political precondition failures remains generic/transient -> focused `failure_handling.rs` unit coverage

## What to Change

### 1. Correct the ticket scope

Rewrite this ticket to match the live architecture instead of the stale Phase B2 narrative.

### 2. Add focused verification for the shared political start-failure path

Strengthen `failure_handling.rs` tests so the current contract is explicit: a stale `declare_support` start failure is still classified through the shared generic blocker path, while political retry suppression comes from refreshed belief/candidate generation on the following tick.

## Files to Touch

- `tickets/E16CINSBELRECCON-013.md` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — tests only)

## Out of Scope

- Adding `BlockingFact::InstitutionalBeliefStale`
- Adding `BlockingFact::InstitutionalBeliefConflicted`
- Introducing rank-time motive reductions for conflicted political beliefs
- Creating a standalone consult-record goal kind or ranking surface
- Updating the E16c spec in this ticket

## Acceptance Criteria

### Tests That Must Pass

1. Existing focused political candidate-generation coverage still passes unchanged
2. Existing golden stale-political-start-failure coverage still passes unchanged
3. `failure_handling.rs` has focused coverage proving stale political `declare_support` start failures remain on the shared generic blocker path
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Conflicted institutional political branches are suppressed before ranking rather than ranked with a reduced motive
2. Unknown political institutional beliefs route through consultable-record prerequisites, not a standalone ranked consult goal
3. Shared start-failure recovery remains generic; political retry suppression comes from the updated belief/candidate pipeline on the next tick

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/failure_handling.rs` — document that stale political `declare_support` precondition failures currently record a generic transient blocker through the shared failure path

### Commands

1. `cargo test -p worldwake-ai failure_handling`
2. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-22
- What actually changed:
  - Reassessed the ticket against live code/tests and corrected the scope to the current architecture.
  - Added focused coverage in `crates/worldwake-ai/src/failure_handling.rs` proving stale political `declare_support` start failures stay on the shared generic blocker path.
- Deviations from original plan:
  - Did not add `BlockingFact::InstitutionalBeliefStale` or `BlockingFact::InstitutionalBeliefConflicted`.
  - Did not add any ranking changes for conflicted institutional beliefs.
  - Did not create or rank a standalone consult-record goal, because the live architecture uses `PlannerOpKind::ConsultRecord` as a prerequisite step under political goals instead.
- Verification results:
  - `cargo test -p worldwake-ai failure_handling` ✅
  - `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace` ✅
