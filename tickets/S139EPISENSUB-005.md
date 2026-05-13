# S139EPISENSUB-005: Ranking integration for GoalKind::AskWitness

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` (`ranking.rs`)
**Deps**: archive/tickets/S139EPISENSUB-001.md, archive/tickets/S139EPISENSUB-002.md, archive/tickets/S139EPISENSUB-003.md

## Problem

Ticket 001 added placeholder priority (`GoalPriorityClass::Background`) and `motive_score` (`Permille::ZERO`) arms for `GoalKind::AskWitness` in `ranking.rs` to keep the workspace compiling. This ticket replaces those placeholders with the real ranking contract: priority class `Low`, and a `motive_score` formula combining (a) the confidence gap below `stale_evidence_barrier_threshold`, (b) a recency bonus weighted by `witness_recency_preference` (added in ticket 002), and (c) `LearnedOpportunityMemory` damping for repeated fruitless asks.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalPriorityClass` enum is defined at `crates/worldwake-ai/src/ranking.rs:1989-1995` with variants `Background, Low, Medium, High, Critical`. The placeholder for `AskWitness` (ticket 001) is `Background`; ticket 005 replaces it with `Low` because epistemic detours rank below productive economic goals but above pure background polling.
2. `motive_score` computation flows through `score_ranked_goal_with_trace` at `ranking.rs:206`; the function threads through `LearnedOpportunityMemory` parameter (line 192, 206, 554, 737, 782, 826-828). The damping pathway is already wired — the new variant must read from the same parameter.
3. Shared abstraction boundary under audit: the priority-class match and `motive_score` match in `ranking.rs`. Both are exhaustive matches over `GoalKind`. The placeholder arms from ticket 001 are TODO-marked with `S139EPISENSUB-005`.
4. Live `GoalKind` under test: `GoalKind::AskWitness` (added by ticket 001). The structural analog for `motive_score` is `GoalKind::ShareBelief` (also a `EPISTEMIC_SENSING_POLICY` / social-class family); verify the formula shape against `ShareBelief`'s `motive_score` arm during implementation.
5. `EpistemicDispositionProfile.witness_recency_preference` (added by ticket 002) is read via `ctx.view.epistemic_disposition_profile(ctx.agent).witness_recency_preference`. The `Permille` arithmetic constrains the formula to integer-percentile math (no floats per CLAUDE.md determinism invariant).
6. Existing inline tests in `ranking.rs` exercising the priority-class and motive_score machinery (named per precision-rules Rule 3): the tests are spread across the 10269-line file; specific test names are best identified during implementation by grep on `goal_priority_class` and `motive_score` test patterns. Note in this ticket's reassessment what existing test the new arm extends.
7. Ranking-sensitive ticket (precision-rules Rule 7 from `tickets/README.md`): branch symmetry. `AskWitness` and `ShareBelief` share the `EpistemicSensing`-family suppression rule, but ticket 005 must verify that the actual motive_score arithmetic does not silently tie the two — the validation Scenario 4 (critical-survival suppression) in ticket 006 depends on the suppression being decisive, not a coincidence of equal weights.

## Architecture Check

1. The motive_score formula respects determinism (CLAUDE.md): all arithmetic stays in `Permille` (integer percentile, no floats). The recency bonus is computed from `current_tick - last_observed_tick` (integer ticks) multiplied by `witness_recency_preference / pm(1000)` (Permille fraction).
2. Damping through `LearnedOpportunityMemory` reuses the existing pathway — no new memory type, no parallel damping state. FND-28 single-source-of-truth for damping.
3. The placeholder-replace pattern (per spec-to-tickets skill rule): ticket 001 names the placeholder explicitly with `// TODO(S139EPISENSUB-005):` comments at the priority and motive_score arm sites. This ticket searches for those markers and replaces them in one pass; the TODO comments are removed in the same patch.

## Verification Layers

1. `AskWitness` priority class is `Low` → focused unit test in `ranking.rs`'s `#[cfg(test)]` block asserting `goal_priority_class_for(GoalKind::AskWitness { .. }) == GoalPriorityClass::Low`.
2. `motive_score` for `AskWitness` rises with confidence-gap below threshold → focused unit test comparing two scoring inputs (high gap vs. low gap), asserting the high-gap input scores higher.
3. `motive_score` damping through `LearnedOpportunityMemory` → focused unit test injecting a `LearnedOpportunityMemory` with prior fruitless `AskWitness` attempts; asserting the damped score is strictly less than the undamped score for the same input.
4. Ranking branch symmetry vs. ShareBelief (precision-rules Rule 7) → focused unit test comparing `AskWitness` and `ShareBelief` scores under identical pressure / staleness inputs; document the expected divergence (or symmetry) explicitly. This proves the suppression-vs-arithmetic separation per Rule 4 ordering claim.
5. Authoritative ranking ordering → action trace surface is not relevant here; the contract is decision-trace at the ranking layer.

## What to Change

### 1. Replace placeholder priority class arm

In `crates/worldwake-ai/src/ranking.rs` (priority-class exhaustive match), find the `GoalKind::AskWitness { .. }` arm marked with `// TODO(S139EPISENSUB-005):` (placed by ticket 001) and replace `GoalPriorityClass::Background` with `GoalPriorityClass::Low`. Remove the TODO comment.

### 2. Replace placeholder motive_score arm

In `ranking.rs`'s `motive_score` (or `score_ranked_goal_with_trace`) exhaustive match, find the `GoalKind::AskWitness { .. }` arm marked with `// TODO(S139EPISENSUB-005):` and replace the `Permille::ZERO` placeholder with the real formula:

```rust
GoalKind::AskWitness { witness, topic } => {
    let profile = ctx.view
        .epistemic_disposition_profile(agent)
        .expect("agent universal profile");
    let recency_pref = profile.witness_recency_preference;
    let threshold = profile.stale_evidence_barrier_threshold;

    let subject = match topic {
        TellTopic::EntityBelief { subject } => *subject,
        _ => return Permille::ZERO,  // unsupported topics rank at zero per build_payload_override
    };

    let confidence = match ctx.view.entity_belief_confidence(agent, subject, *witness) {
        Some(c) => c,
        None => Permille::ZERO,  // cold-start case
    };
    let gap = threshold.saturating_sub(confidence);

    let staleness_ticks = ctx.view
        .entity_belief_last_observed_tick(agent, subject)
        .map(|t| ctx.current_tick.0.saturating_sub(t.0))
        .unwrap_or(0);
    let recency_bonus = Permille::from_ratio_clamped(
        staleness_ticks,
        STALENESS_NORMALIZATION_TICKS,
    ).scale_by(recency_pref);

    let base = gap.scale_by(GAP_WEIGHT).saturating_add(recency_bonus);

    // Apply LearnedOpportunityMemory damping
    let damping = ctx.learned_opportunity_memory
        .damping_for(GoalKey { kind: GoalKind::AskWitness { witness: *witness, topic: *topic }, .. });
    base.scale_by(damping)
}
```

`GAP_WEIGHT`, `STALENESS_NORMALIZATION_TICKS`, and the saturating arithmetic helpers may need module-private constants added at the top of `ranking.rs`. Choose initial values that match the analog `ShareBelief` arm's magnitudes — calibration is a follow-up if ticket 006 goldens reveal imbalance.

The `entity_belief_confidence` and `entity_belief_last_observed_tick` accessors may not exist on `GoalBeliefView` today — if absent, add them in this ticket (scope-extending). Both are pure read-only reductions over `BelievedEntityState`.

### 3. Remove the two TODO markers placed by ticket 001

Search for `TODO(S139EPISENSUB-005)` in `ranking.rs` and confirm zero matches remain after the edits.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — replace 2 placeholder arms + add module-private formula constants)
- Likely: `crates/worldwake-sim/src/belief_view.rs` (modify — add `entity_belief_confidence` / `entity_belief_last_observed_tick` accessors if absent; scope-extending)
- Likely: `crates/worldwake-core/src/numerics.rs` (modify — add `Permille::from_ratio_clamped` or equivalent if absent)

## Out of Scope

- Calibration of `GAP_WEIGHT` / `STALENESS_NORMALIZATION_TICKS` against multi-scenario goldens — initial values chosen by analog to `ShareBelief`; calibration is a follow-up if ticket 006 surfaces imbalance.
- Cross-witness topic-disagreement scoring — deferred until `TellTopic::SocialObservation` / `InstitutionalClaim` variants are supported.
- Goldens — ticket 006.

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit test in `ranking.rs`'s `#[cfg(test)]` block asserting `goal_priority_class_for(GoalKind::AskWitness { .. }) == GoalPriorityClass::Low`.
2. New focused unit test asserting `motive_score` for `AskWitness` rises with confidence-gap below threshold (compare two fixture inputs).
3. New focused unit test asserting `motive_score` damping through `LearnedOpportunityMemory` strictly reduces the score for a previously-fruitless ask.
4. New focused unit test comparing `AskWitness` vs. `ShareBelief` under identical pressure inputs (precision-rules Rule 7 branch-symmetry): the test documents whether the formulas tie or diverge, locking the expected behavior.
5. Existing suite: `cargo test -p worldwake-ai` passes.
6. Grep for `TODO(S139EPISENSUB-005)` in `ranking.rs` returns zero matches.

### Invariants

1. `motive_score` for `GoalKind::AskWitness` is expressible in `Permille` arithmetic — no floats, no wall-clock time (CLAUDE.md Critical Invariants).
2. `motive_score` is monotonically non-decreasing in confidence-gap (holding all other inputs constant) — verified by focused unit test.
3. `LearnedOpportunityMemory` damping is the single damping path for repeated asks — no parallel damping state introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (extend `#[cfg(test)]` block at line 2962) — add 4 new focused unit tests per Acceptance Criteria.

### Commands

1. `cargo test -p worldwake-ai -- ranking::tests` — targeted test run.
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
3. `./scripts/verify.sh` — full pre-PR gate.
