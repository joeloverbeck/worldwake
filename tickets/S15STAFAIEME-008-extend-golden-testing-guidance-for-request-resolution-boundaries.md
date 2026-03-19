# S15STAFAIEME-008: Extend Golden Testing Guidance For Request-Resolution Boundaries

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S15STAFAIEME-006, S15STAFAIEME-007

## Problem

[docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already separates decision traces, action traces, and authoritative-state assertions, and S15STAFAIEME-006 already covers the specific S08 contract "lawful start rejection is recoverable." The remaining gap is earlier in the chain: the guide still does not explicitly teach authors to distinguish request-resolution failure from authoritative start failure, or to use a dedicated runtime request-resolution trace once that substrate exists.

Without that guidance, future tickets can still mis-specify a pre-start runtime rejection as a `StartFailed` action problem, or rely on weak indirect evidence such as "no action trace exists."

## Assumption Reassessment (2026-03-19)

1. The current guide in [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) is strong on action-lifecycle ordering, decision-trace reasoning, and scenario isolation, but it does not yet name the request-resolution boundary as its own assertion surface.
2. S15 exposed a concrete case where the missing boundary mattered: stale `BestEffort` requests in [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) could be rejected during affordance reproduction before any authoritative action start occurred. That confused ticket scope until the shared runtime layer was re-examined.
3. Existing golden/focused tests already cover adjacent examples that the guide should cite after the trace substrate exists: `golden_care_pre_start_wound_disappearance_records_blocker` in [crates/worldwake-ai/tests/golden_care.rs:760](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs#L760), `golden_local_trade_start_failure_recovers_via_production_fallback` in [crates/worldwake-ai/tests/golden_trade.rs:875](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs#L875), and `best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` in [crates/worldwake-sim/src/tick_step.rs:1474](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L1474).
4. This is a documentation-only ticket. The missing layer is authoring guidance, not executable behavior. Additional code changes belong in S15STAFAIEME-007.
5. The guide should stay concise. It should add the missing boundary naming and assertion rules, not duplicate the whole trace API reference or rewrite unrelated sections that are already correct.
6. `docs/FOUNDATIONS.md` makes traceability a first-class architecture goal. Golden guidance should therefore teach authors to assert the earliest truthful boundary directly instead of inferring it from later absence-of-commit symptoms.
7. Scope correction: this ticket should update `docs/golden-e2e-testing.md`, not invent a parallel guidance file for the same topic.

## Architecture Check

1. Extending the existing golden guide is cleaner than scattering request-resolution guidance across tickets or relying on reviewer memory. The assertion hierarchy should live in one canonical place.
2. Teaching authors to distinguish request resolution from authoritative start preserves the architectural separation of layers instead of collapsing them into a vague "trace" bucket.
3. No backwards-compatible loophole should remain that lets future tickets claim a start-failure proof from an action that may never have reached start.

## Verification Layers

1. The guide explicitly names request resolution as a distinct assertion boundary before authoritative start -> doc diff review.
2. The guide tells authors to use the earliest truthful trace surface and not infer pre-start rejection from missing action events -> doc diff review.
3. The guide anchors the new rule in existing repo examples and points at `docs/golden-e2e-testing.md` as the canonical location -> doc diff review.
4. Additional runtime verification layers are not applicable because this ticket only changes authoring guidance.

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

The guide should reject "no later action trace event happened" as sufficient proof of a pre-start rejection.

### 3. Anchor the rule in live repo examples

Reference the care/trade examples and the focused runtime stale-request test so future authors can see the intended split between runtime request traces, action traces, and next-tick decision traces.

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
3. `cargo test -p worldwake-sim best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
4. Existing suite: `cargo test -p worldwake-ai -- --list`

### Invariants

1. Golden guidance must distinguish request resolution, authoritative start, and post-start execution as separate proof boundaries.
2. Future tickets must not be able to claim pre-start or start-failure proof from missing action events alone when a lower-layer assertion surface exists.
3. The canonical golden testing guidance must remain centralized in `docs/golden-e2e-testing.md`.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-sim best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
2. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
3. `cargo test -p worldwake-ai --test golden_trade`
