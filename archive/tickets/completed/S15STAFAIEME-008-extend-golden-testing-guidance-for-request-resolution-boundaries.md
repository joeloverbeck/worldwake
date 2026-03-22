# S15STAFAIEME-008: Extend Golden Testing Guidance For Request-Resolution Boundaries

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S15STAFAIEME-006, S15STAFAIEME-007

## Problem

[docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already separates decision traces, action traces, and authoritative-state assertions, and S15STAFAIEME-006 already covers the specific S08 contract "lawful start rejection is recoverable." The remaining gap is earlier in the chain: the guide still does not explicitly teach authors to distinguish request-resolution failure from authoritative start failure, or to use the dedicated runtime request-resolution trace that now exists.

Without that guidance, future tickets can still mis-specify a pre-start runtime rejection as a `StartFailed` action problem, or rely on weak indirect evidence such as "no action trace exists."

## Assumption Reassessment (2026-03-19)

1. The current guide in [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) is strong on action-lifecycle ordering, decision-trace reasoning, and scenario isolation, but it does not yet name the request-resolution boundary as its own assertion surface.
2. The runtime request-resolution substrate now already exists in [crates/worldwake-sim/src/request_resolution_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/request_resolution_trace.rs) and is wired through [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs). The documentation gap is no longer "once that substrate exists"; it is that the guide has not been updated to teach authors when to use it.
3. The earlier ticket wording overstated one runtime example. `best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` in [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) does **not** prove a request-resolution rejection. It proves a request that binds through `RequestResolutionOutcome::Bound { start_attempted: true }` and then lawfully reaches authoritative `StartFailed`. The true focused pre-start rejection coverage is `strict_request_records_resolution_rejection_without_start_attempt` in [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs), which records `RequestResolutionOutcome::RejectedBeforeStart`.
4. Existing golden/focused tests already cover both sides of the split that the guide should now name explicitly: `golden_care_pre_start_wound_disappearance_records_blocker` in [crates/worldwake-ai/tests/golden_care.rs:764](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs#L764), `golden_local_trade_start_failure_recovers_via_production_fallback` in [crates/worldwake-ai/tests/golden_trade.rs:907](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs#L907), `best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` in [crates/worldwake-sim/src/tick_step.rs:1541](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L1541), and `strict_request_records_resolution_rejection_without_start_attempt` in [crates/worldwake-sim/src/tick_step.rs:1684](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L1684).
5. This remains a documentation-only ticket. The missing layer is canonical authoring guidance, not executable behavior. No production architecture change is required here.
6. The guide should stay concise. It should add the missing boundary naming and assertion rules, not duplicate the whole trace API reference or rewrite unrelated sections that are already correct.
7. `docs/FOUNDATIONS.md` makes traceability a first-class architecture goal. Golden guidance should therefore teach authors to assert the earliest truthful boundary directly instead of inferring it from later absence-of-commit symptoms.
8. Scope correction: this ticket should update `docs/golden-e2e-testing.md`, not invent a parallel guidance file for the same topic.

## Architecture Check

1. Extending the existing golden guide is cleaner than scattering request-resolution guidance across tickets or relying on reviewer memory. The assertion hierarchy should live in one canonical place.
2. Teaching authors to distinguish request resolution from authoritative start preserves the architectural separation of layers instead of collapsing them into a vague "trace" bucket.
3. No backwards-compatible loophole should remain that lets future tickets claim a start-failure proof from an action that may never have reached start.

## Verification Layers

1. The guide explicitly names request resolution as a distinct assertion boundary before authoritative start -> doc diff review.
2. The guide distinguishes `RequestResolutionOutcome::RejectedBeforeStart` from `ActionTraceKind::StartFailed` and tells authors to assert the earliest truthful trace surface -> doc diff review.
3. The guide rejects "no later action trace event happened" as sufficient proof of request-resolution rejection when request-resolution tracing exists -> doc diff review.
4. The guide anchors the new rule in existing repo examples and points at `docs/golden-e2e-testing.md` as the canonical location -> doc diff review.
5. Additional runtime verification layers are not applicable because this ticket only changes authoring guidance.

## What to Change

### 1. Add request-resolution guidance to the assertion hierarchy

Update [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) so the document explicitly distinguishes:

- request-resolution / affordance-reproduction facts
- authoritative action-start facts
- later commit/abort facts

The guidance should tell authors to assert the earliest truthful boundary directly.

### 2. Add a short rule for stale concrete requests

Document that when a scenario involves stale or retained requests, the owning ticket and golden rationale must state whether the contract is:

- request-resolution failure before start
- `StartFailed` at authoritative start
- abort after lawful start

The guide should reject "no later action trace event happened" as sufficient proof of a pre-start rejection, and it should point authors at `RequestResolutionOutcome::RejectedBeforeStart` as the direct proof surface for that boundary.

### 3. Anchor the rule in live repo examples

Reference the care/trade examples plus both focused runtime request-resolution tests so future authors can see the intended split between runtime request traces, action traces, and next-tick decision traces.

## Files to Touch

- `docs/golden-e2e-testing.md` (modify)

## Out of Scope

- `docs/golden-e2e-coverage.md`
- `docs/golden-e2e-scenarios.md`
- runtime or AI source changes
- duplicating the ticket authoring rules from `tickets/README.md`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
2. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
3. `cargo test -p worldwake-sim 'tick_step::tests::strict_request_records_resolution_rejection_without_start_attempt' -- --exact`
4. `cargo test -p worldwake-sim 'tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches' -- --exact`
5. Existing suite: `cargo test -p worldwake-ai -- --list`

### Invariants

1. Golden guidance must distinguish request resolution, authoritative start, and post-start execution as separate proof boundaries.
2. Future tickets must not be able to claim pre-start or start-failure proof from missing action events alone when a lower-layer assertion surface exists.
3. The canonical golden testing guidance must remain centralized in `docs/golden-e2e-testing.md`.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-sim 'tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches' -- --exact`
2. `cargo test -p worldwake-sim 'tick_step::tests::strict_request_records_resolution_rejection_without_start_attempt' -- --exact`
3. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
4. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
5. `cargo test -p worldwake-ai -- --list`

## Outcome

- Completion date: 2026-03-19
- What actually changed: corrected the ticket's assumptions and verification commands to match the current runtime architecture, then updated [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) to add request-resolution traces as their own assertion boundary, document the stale-request boundary rule, and anchor the distinction with live focused and golden examples.
- Deviations from original plan: no code or test files changed because the runtime request-resolution substrate and focused coverage already existed; the necessary work was narrower and cleaner than the original ticket implied. The ticket itself also needed correction because its original `worldwake-sim` command examples were not exact runnable selectors under `--exact`.
- Verification results:
  - `cargo test -p worldwake-sim 'tick_step::tests::strict_request_records_resolution_rejection_without_start_attempt' -- --exact`
  - `cargo test -p worldwake-sim 'tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches' -- --exact`
  - `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
  - `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
  - `cargo test -p worldwake-ai --test golden_care`
  - `cargo test -p worldwake-ai --test golden_trade`
  - `cargo test -p worldwake-ai -- --list`
  - `cargo clippy --workspace --all-targets -- -D warnings`
