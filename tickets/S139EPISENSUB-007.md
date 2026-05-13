# S139EPISENSUB-007: Apply witness recency to AskWitness satisfaction

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` (`goal_model.rs`)
**Deps**: archive/tickets/S139EPISENSUB-001.md, archive/tickets/S139EPISENSUB-002.md

## Problem

Ticket 001 added `GoalKind::AskWitness` satisfaction with a temporary confidence-only gate and left `TODO(S139EPISENSUB-002)` in `ask_witness_satisfied`. Ticket 002 correctly landed only the profile substrate (`EpistemicDispositionProfile.witness_recency_preference`) and kept consumers out of scope. The active S139 spec still requires the satisfaction predicate to accept a fresh report from the target witness, so the remaining source TODO now points at an archived substrate ticket and no active ticket owns the satisfaction freshness branch.

This ticket closes that gap before the end-to-end golden ticket. It replaces the stale TODO with a concrete, profile-driven freshness branch in `GoalKindPlannerExt::is_satisfied` for `GoalKind::AskWitness`.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ask_witness_satisfied` in `crates/worldwake-ai/src/goal_model.rs` currently reads `state.entity_beliefs_sourced_from_witness(actor, witness)`, filters to the requested `TellTopic::EntityBelief` subject and `PerceptionSource::Report { from: witness, .. }`, derives `staleness_ticks` from `belief.last_observed_tick()`, and returns `true` only when `belief_confidence(...) >= profile.stale_evidence_barrier_threshold`.
2. The helper still carries `TODO(S139EPISENSUB-002): add the witness_recency_preference freshness refinement once that profile field lands`. Ticket 002 has landed the field and is archived; leaving the TODO attached to 002 would misstate ownership for the remaining behavior.
3. `EpistemicDispositionProfile.witness_recency_preference: Permille` now exists on the universal profile, defaults to `pm(500)`, and is accessible through the existing `state.epistemic_disposition_profile(actor)` call in `ask_witness_satisfied`.
4. Shared abstraction boundary under audit: `GoalKindPlannerExt::is_satisfied` for `GoalKind::AskWitness`. This is a planner-satisfaction boundary, not candidate emission or ranking. Ticket 004 owns candidate emission, ticket 005 owns motive scoring, and ticket 006 owns E2E golden proof.
5. The freshness branch must remain belief-local (FND-14/FND-15): use the report-sourced `BelievedEntityState` already returned by `entity_beliefs_sourced_from_witness`, its `last_observed_tick()`, the actor's `BeliefConfidencePolicy`, and the actor's `EpistemicDispositionProfile`. Do not query authoritative witness/world state.
6. Concrete formula: compute a freshness budget from existing profile/policy state without a new magic constant. A report counts as fresh when `staleness_ticks * belief_confidence_policy.staleness_penalty_per_tick <= witness_recency_preference`. This is equivalent to converting the preference into a staleness window through the live confidence-decay policy. Saturate arithmetic deterministically and handle zero staleness penalty without division.
7. Precision boundary: this ticket changes the satisfaction predicate only. It does not calibrate emitter salience, ranking motive score, or suppression behavior.

## Architecture Check

1. The change uses the existing universal `EpistemicDispositionProfile`, preserving one epistemic disposition source of truth (FND-28) and avoiding a second recency policy path.
2. The freshness window is derived from concrete belief age and the live belief-confidence decay policy, not a hidden global constant. This keeps the branch explainable as "this agent treats this report as fresh enough given their profile and confidence policy."
3. No backwards-compatibility shim or alias is introduced. The stale TODO is removed and the final predicate is implemented in the existing `AskWitness` satisfaction helper.

## Verification Layers

1. Fresh report can satisfy even when confidence is below `stale_evidence_barrier_threshold` -> focused unit test in `goal_model.rs` against a report-sourced belief whose age is within the `witness_recency_preference` freshness budget.
2. Stale report below threshold does not satisfy -> focused unit test with the same report source and subject but `staleness_ticks` beyond the derived freshness budget.
3. Confidence gate remains valid -> focused unit test or extension of the existing satisfaction test proving a report with confidence at or above `stale_evidence_barrier_threshold` still satisfies even when outside the freshness budget.
4. TODO ownership cleanup -> grep for `TODO(S139EPISENSUB-002)` in `crates/worldwake-ai/src/goal_model.rs` returns zero matches.

## What to Change

### 1. Add a module-private freshness helper

In `crates/worldwake-ai/src/goal_model.rs`, near `ask_witness_satisfied`, add a small helper such as:

```rust
fn report_is_fresh_enough_for_witness_preference(
    staleness_ticks: u64,
    profile: EpistemicDispositionProfile,
    confidence_policy: BeliefConfidencePolicy,
) -> bool {
    let staleness_penalty = u64::from(confidence_policy.staleness_penalty_per_tick.value());
    let freshness_budget = u64::from(profile.witness_recency_preference.value());
    staleness_ticks.saturating_mul(staleness_penalty) <= freshness_budget
}
```

Adjust the signature/imports to match the live module. Keep the arithmetic integer-only and saturating.

### 2. Replace the temporary confidence-only satisfaction gate

In `ask_witness_satisfied`, replace the `TODO(S139EPISENSUB-002)` comment and confidence-only return with:

```rust
let confidence_satisfies =
    belief_confidence(&belief.source, staleness_ticks, &confidence_policy)
        >= profile.stale_evidence_barrier_threshold;
let freshness_satisfies = report_is_fresh_enough_for_witness_preference(
    staleness_ticks,
    profile,
    confidence_policy,
);
confidence_satisfies || freshness_satisfies
```

The branch must still run only after the existing subject and `PerceptionSource::Report { from: witness, .. }` checks pass.

### 3. Extend focused tests

Add or extend `goal_model.rs` tests around the existing `AskWitness` satisfaction fixture. Use profile values that make the freshness boundary explicit:

- preference high enough that a below-threshold recent report satisfies
- same report beyond the derived freshness budget does not satisfy
- confidence-above-threshold report still satisfies independently of freshness

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — satisfaction helper + focused tests + remove stale TODO)

## Out of Scope

- Candidate emission and per-tick cap — ticket 004.
- Ranking formula, priority class, and learned-opportunity damping — ticket 005.
- Golden E2E proof and observer audit — ticket 006.
- Changing `EpistemicDispositionProfile` fields or save format — already completed by ticket 002.

## Acceptance Criteria

### Tests That Must Pass

1. New/modified focused unit test: below-threshold report within the derived freshness budget satisfies `GoalKind::AskWitness`.
2. New/modified focused unit test: below-threshold report outside the derived freshness budget does not satisfy.
3. New/modified focused unit test: above-threshold report still satisfies through the confidence branch.
4. Grep for `TODO(S139EPISENSUB-002)` in `crates/worldwake-ai/src/goal_model.rs` returns zero matches.
5. Existing suite: `cargo test -p worldwake-ai --lib goal_model::tests`.

### Invariants

1. Satisfaction reads belief state only; no authoritative world or witness lookup is added.
2. Freshness uses `Permille`/integer arithmetic only; no floats, wall-clock time, or nondeterministic state.
3. `witness_recency_preference` affects satisfaction only for report-sourced beliefs from the requested witness and requested topic subject.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — extend the `GoalKind::AskWitness` satisfaction tests with the three freshness/confidence cases above.

### Commands

1. `cargo test -p worldwake-ai --lib goal_model::tests`
2. `! rg -n "TODO\\(S139EPISENSUB-002\\)" crates/worldwake-ai/src/goal_model.rs`
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh`
