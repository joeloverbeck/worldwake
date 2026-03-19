# S15STAFAIEME-006: Update Golden Testing Guidance For S08-Style Start-Failure Proof

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S15STAFAIEME-001, S15STAFAIEME-002, S15STAFAIEME-003

## Problem

[docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already describes action traces and decision traces separately, but S15 requires explicit guidance for the specific contract "lawful start rejection is recoverable." Without that addition, future tickets can still under-specify start-failure coverage as mere absence of a later commit.

## Assumption Reassessment (2026-03-19)

1. The current testing guide already says action traces prove lifecycle facts and decision traces prove AI reasoning, and it warns against using later authoritative outcomes as proxies for earlier ordering. It does not yet call out the exact S08 start-failure pattern as its own guidance block.
2. The care golden in [crates/worldwake-ai/tests/golden_care.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs) is the current concrete repo example of the intended proof shape: `ActionTraceKind::StartFailed` plus next-tick `planning.action_start_failures`.
3. After S15, there will be three more cross-domain examples of the same proof pattern in production, trade, and politics. The guide should generalize that pattern instead of leaving it implicit in individual tests.
4. This is a documentation-only ticket. The gap is not missing engine behavior; it is missing explicit test-authoring guidance for mixed-layer start-failure scenarios.
5. The guidance should forbid a weak proxy: "no later commit happened" is not enough to prove recoverable lawful start rejection, because the same symptom could arise from candidate omission, ranking loss, planner failure, or unrelated execution issues.
6. The guidance should name the required split clearly: action trace for `StartFailed`, then next-tick decision trace for AI failure handling and stale-plan clearing.
7. Scope correction: this ticket should not restate all golden conventions. Add only the missing S08-specific rule and keep the document concise.

## Architecture Check

1. A focused addition to the testing guide is cleaner than relying on reviewers to infer the expected proof shape from one care-domain example.
2. No backwards-compatible loophole should remain that allows future tickets to claim S08 coverage from missing commits alone.

## Verification Layers

1. The guide explicitly requires action-trace `StartFailed` for lawful start rejection -> doc diff review.
2. The guide explicitly requires next-tick decision-trace handling for AI reconciliation -> doc diff review.
3. The guide explicitly rejects "no later commit" as sufficient evidence -> doc diff review.
4. Additional runtime verification layers are not applicable because this ticket only changes authoring guidance.

## What to Change

### 1. Add explicit S08-style guidance

Extend [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) with a short rule for recoverable start-failure coverage:

- use action traces to prove `StartFailed`
- use the next AI tick's decision trace to prove consumption of `action_start_failures`
- do not infer reconciliation solely from missing later commits

### 2. Anchor the guidance in existing repo examples

Reference the care-domain golden as the existing example and phrase the rule so it also covers the new S15 production, trade, and political goldens without becoming spec-specific prose.

## Files to Touch

- `docs/golden-e2e-testing.md` (modify)

## Out of Scope

- `docs/golden-e2e-coverage.md`
- `docs/golden-e2e-scenarios.md`
- any `crates/` source or test file
- rewriting unrelated guidance sections that are already correct

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
2. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
3. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
4. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`

### Invariants

1. Golden authoring guidance must preserve the layer split between action lifecycle facts and AI reasoning facts.
2. Future tickets must not be able to claim S08-style recovery proof from indirect absence-of-commit evidence alone.
3. The guidance must remain general-purpose and apply across care, production, trade, and politics without introducing domain-specific test hacks.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
2. `cargo test -p worldwake-ai --test golden_care`
3. `cargo test -p worldwake-ai -- --list`
