# S139EPISENSUB-007: Apply witness recency to AskWitness satisfaction

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` (`goal_model.rs`)
**Deps**: archive/tickets/S139EPISENSUB-001.md, archive/tickets/S139EPISENSUB-002.md

## Problem

Before this ticket, ticket 001 had added `GoalKind::AskWitness` satisfaction with a temporary confidence-only gate and left a stale `S139EPISENSUB-002` marker in `ask_witness_satisfied`. Ticket 002 correctly landed only the profile substrate (`EpistemicDispositionProfile.witness_recency_preference`) and kept consumers out of scope. The active S139 spec still required the satisfaction predicate to accept a fresh report from the target witness, so the remaining source marker pointed at an archived substrate ticket and no active ticket owned the satisfaction freshness branch.

This ticket closed that gap before the end-to-end golden ticket. It replaced the stale marker with a concrete, profile-driven freshness branch in `GoalKindPlannerExt::is_satisfied` for `GoalKind::AskWitness`.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Before this ticket, `ask_witness_satisfied` in `crates/worldwake-ai/src/goal_model.rs` read `state.entity_beliefs_sourced_from_witness(actor, witness)`, filtered to the requested `TellTopic::EntityBelief` subject and `PerceptionSource::Report { from: witness, .. }`, derived `staleness_ticks` from `belief.last_observed_tick()`, and returned `true` only when `belief_confidence(...) >= profile.stale_evidence_barrier_threshold`.
2. Before this ticket, the helper still carried `TODO(S139EPISENSUB-002): add the witness_recency_preference freshness refinement once that profile field lands`. Ticket 002 had landed the field and was archived; leaving the TODO attached to 002 would misstate ownership for the remaining behavior.
3. `EpistemicDispositionProfile.witness_recency_preference: Permille` now exists on the universal profile, defaults to `pm(500)`, and is accessible through the existing `state.epistemic_disposition_profile(actor)` call in `ask_witness_satisfied`.
4. Shared abstraction boundary under audit: `GoalKindPlannerExt::is_satisfied` for `GoalKind::AskWitness`. This is a planner-satisfaction boundary, not candidate emission or ranking. Ticket 004 owns candidate emission, ticket 005 owns motive scoring, and ticket 006 owns E2E golden proof.
5. The freshness branch must remain belief-local (FND-14/FND-15): use the report-sourced `BelievedEntityState` already returned by `entity_beliefs_sourced_from_witness`, its `last_observed_tick()`, the actor's `BeliefConfidencePolicy`, and the actor's `EpistemicDispositionProfile`. Do not query authoritative witness/world state.
6. Concrete formula: compute a freshness budget from existing profile/policy state without a new magic constant. A report counts as fresh when `staleness_ticks * belief_confidence_policy.staleness_penalty_per_tick <= witness_recency_preference`. This is equivalent to converting the preference into a staleness window through the live confidence-decay policy. Saturate arithmetic deterministically and handle zero staleness penalty without division.
7. Precision boundary: this ticket changes the satisfaction predicate only. It does not calibrate emitter salience, ranking motive score, or suppression behavior.

## Architecture Check

1. The change uses the existing universal `EpistemicDispositionProfile`, preserving one epistemic disposition source of truth (FND-28) and avoiding a second recency policy path.
2. The freshness window is derived from concrete belief age and the live belief-confidence decay policy, not a hidden global constant. This keeps the branch explainable as "this agent treats this report as fresh enough given their profile and confidence policy."
3. No backwards-compatibility shim or alias is introduced. The stale TODO is removed and the final predicate is implemented in the existing `AskWitness` satisfaction helper.

## Verified Layers

1. Fresh report can satisfy even when confidence is below `stale_evidence_barrier_threshold` -> focused unit test in `goal_model.rs` against a report-sourced belief whose age is within the `witness_recency_preference` freshness budget.
2. Stale report below threshold does not satisfy -> focused unit test with the same report source and subject but `staleness_ticks` beyond the derived freshness budget.
3. Confidence gate remains valid -> focused unit test proving a report with confidence at or above `stale_evidence_barrier_threshold` still satisfies even when outside the freshness budget.
4. Stale TODO ownership cleanup passed -> grep for `TODO(S139EPISENSUB-002)` in `crates/worldwake-ai/src/goal_model.rs` returned zero matches.

## Landed Changes

### 1. Added a module-private freshness helper

In `crates/worldwake-ai/src/goal_model.rs`, near `ask_witness_satisfied`, added a small helper with the live by-reference signature:

```rust
fn report_is_fresh_enough_for_witness_preference(
    staleness_ticks: u64,
    profile: &EpistemicDispositionProfile,
    confidence_policy: &BeliefConfidencePolicy,
) -> bool {
    let staleness_penalty = u64::from(confidence_policy.staleness_penalty_per_tick.value());
    let freshness_budget = u64::from(profile.witness_recency_preference.value());
    staleness_ticks.saturating_mul(staleness_penalty) <= freshness_budget
}
```

The arithmetic stayed integer-only and saturating.

### 2. Replaced the temporary confidence-only satisfaction gate

In `ask_witness_satisfied`, replaced the stale TODO comment and confidence-only return with:

```rust
let confidence_satisfies =
    belief_confidence(&belief.source, staleness_ticks, &confidence_policy)
        >= profile.stale_evidence_barrier_threshold;
let freshness_satisfies = report_is_fresh_enough_for_witness_preference(
    staleness_ticks,
    &profile,
    &confidence_policy,
);
confidence_satisfies || freshness_satisfies
```

The branch still runs only after the existing subject and `PerceptionSource::Report { from: witness, .. }` checks pass.

### 3. Extended focused tests

Extended `goal_model.rs` tests around the existing `AskWitness` satisfaction fixture. The tests use profile values that make the freshness boundary explicit:

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

## Acceptance Result

### Tests Passed

1. Added focused unit test: below-threshold report within the derived freshness budget satisfies `GoalKind::AskWitness`.
2. Added focused unit test: below-threshold report outside the derived freshness budget does not satisfy.
3. Added focused unit test: above-threshold report still satisfies through the confidence branch.
4. Grep for `TODO(S139EPISENSUB-002)` in `crates/worldwake-ai/src/goal_model.rs` returned zero matches.
5. Existing suite passed: `cargo test -p worldwake-ai --lib goal_model::tests`.

### Invariants

1. Satisfaction reads belief state only; no authoritative world or witness lookup is added.
2. Freshness uses `Permille`/integer arithmetic only; no floats, wall-clock time, or nondeterministic state.
3. `witness_recency_preference` affects satisfaction only for report-sourced beliefs from the requested witness and requested topic subject.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — extended the `GoalKind::AskWitness` satisfaction tests with the three freshness/confidence cases above.

### Commands

1. `cargo test -p worldwake-ai --lib goal_model::tests`
2. `! rg -n "TODO\\(S139EPISENSUB-002\\)" crates/worldwake-ai/src/goal_model.rs`
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-13.

- Added `report_is_fresh_enough_for_witness_preference` in `goal_model.rs`, deriving the freshness branch from report age, `BeliefConfidencePolicy.staleness_penalty_per_tick`, and `EpistemicDispositionProfile.witness_recency_preference`.
- Replaced the stale `TODO(S139EPISENSUB-002)` confidence-only gate in `ask_witness_goal_satisfied` with `confidence_satisfies || freshness_satisfies`, still scoped to matching report-sourced entity beliefs from the requested witness.
- Extended `goal_model.rs` unit coverage for a recent below-threshold report satisfying through freshness, a stale below-threshold report remaining unsatisfied, and a stale report satisfying through the confidence branch.
- Updated `archive/specs/S139-epistemic-sensing-subgoals.md` so the spec no longer describes ticket 007 as future TODO ownership.

## Deviations

- The implementation used a by-reference helper signature for `EpistemicDispositionProfile` and `BeliefConfidencePolicy` because the live profile type is `Clone` rather than `Copy`; the arithmetic and predicate match the ticket's intended formula.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib goal_model::tests`.
- Passed `rg -n 'TODO\\(S139EPISENSUB-002\\)' crates/worldwake-ai/src/goal_model.rs` with zero matches.
- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-ai`.
- Passed `./scripts/verify.sh` (live gates: `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `bash scripts/check_no_artifact_state.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p worldwake-cli --bin scenario-coverage -- --check`).
