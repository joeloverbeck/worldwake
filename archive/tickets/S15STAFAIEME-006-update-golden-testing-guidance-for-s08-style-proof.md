# S15STAFAIEME-006: Update Golden Testing Guidance For S08-Style Start-Failure Proof

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S15STAFAIEME-001, S15STAFAIEME-002, S15STAFAIEME-003

## Problem

[docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already describes action traces and decision traces separately, but S15 requires explicit guidance for the specific contract "lawful start rejection is recoverable." Without that addition, future tickets can still under-specify start-failure coverage as mere absence of a later commit.

## Assumption Reassessment (2026-03-19)

1. [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already contains most of the needed layering guidance. It distinguishes request-resolution rejection from authoritative `StartFailed`, says action traces prove lifecycle facts, says decision traces prove AI reasoning, and already names [crates/worldwake-ai/tests/golden_care.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs) plus [crates/worldwake-ai/tests/golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs) as examples of the later start-failure boundary.
2. The repo already contains all four golden examples this ticket originally treated as future work: [crates/worldwake-ai/tests/golden_care.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs), [crates/worldwake-ai/tests/golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs), [crates/worldwake-ai/tests/golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs), and [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs). The ticket must not describe them as merely planned follow-ups.
3. The shared runtime path is already covered in focused code and in goldens. The relevant authoritative symbols are `worldwake_sim::tick_step`, `Scheduler::record_action_start_failure`, `ActionTraceKind::StartFailed`, and AI-side `agent_tick` consumption into `planning.action_start_failures`. This ticket therefore remains documentation-only; it does not expose a production architectural contradiction.
4. A real documentation gap still remains: the guide does not yet present recoverable lawful start rejection as one short, explicit authoring rule that bundles the required proof surfaces together. Today the rule is inferable from adjacent sections, but not stated compactly.
5. That explicit rule should forbid the weak proxy "no later commit happened." Missing later commits can also arise from request-resolution rejection before start, candidate omission, ranking loss, plan search failure, or unrelated execution failure. For this ticket's contract, the first live failure boundary is authoritative start, not request resolution.
6. The rule should name the exact split required for this class of scenario: action trace `StartFailed` for the authoritative start boundary, then next-tick decision-trace `planning.action_start_failures` plus stale-plan clearing / replanning evidence for AI reconciliation.
7. Scope correction: keep the change surgical. Do not rewrite the guide; add one concise S08-style authoring rule anchored in the existing care/production/trade/politics examples.

## Architecture Check

1. A focused addition to the testing guide is cleaner than relying on reviewers to infer the expected proof shape from one care-domain example.
2. The better architecture is not a new helper or a new trace abstraction. The current architecture already has the right layers; the benefit here is making their contract explicit so future tickets do not weaken proof quality.
3. No backwards-compatible loophole should remain that allows future tickets to claim S08 coverage from missing commits alone.

## Verification Layers

1. Request-resolution boundary remains distinct from this contract -> existing focused runtime coverage in [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) and doc diff review.
2. Authoritative start failure is proven with action-trace `StartFailed` -> existing goldens plus doc diff review.
3. AI reconciliation is proven with next-tick decision-trace `planning.action_start_failures` and stale-plan clearing / replanning evidence -> existing goldens plus doc diff review.
4. Weak absence-of-commit proxies are explicitly rejected for this contract -> doc diff review.

## What to Change

### 1. Add explicit S08-style guidance

Extend [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) with one short rule for recoverable authoritative start-failure coverage:

- use action traces to prove `StartFailed`
- use the next AI tick's decision trace to prove consumption of `action_start_failures`
- do not infer reconciliation solely from missing later commits

### 2. Anchor the guidance in existing repo examples

Reference the existing care, production, trade, and political goldens as repo examples and phrase the rule so it stays general-purpose rather than S15-spec prose.

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

1. `None — documentation-only ticket; verification relies on existing focused runtime coverage and existing S15 goldens already present in the repo.`

### Rationale

1. `No new or modified tests are justified because the runtime and golden coverage this ticket documents already exists; the remaining gap is authoring guidance, not behavior or coverage.`

### Commands

1. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
2. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
3. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
4. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
5. `cargo test -p worldwake-ai --test golden_care`
6. `cargo test -p worldwake-ai --test golden_production`
7. `cargo test -p worldwake-ai --test golden_trade`
8. `cargo test -p worldwake-ai --test golden_emergent`
9. `cargo test --workspace`
10. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-20
- What actually changed: updated [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) with a dedicated "Recoverable Authoritative Start Failure" rule that explicitly requires action-trace `StartFailed` plus next-tick decision-trace reconciliation and rejects missing later commits as sufficient proof.
- Deviations from original plan: the reassessment found that the repo already contained the S15 production, trade, and political golden examples and already had most of the underlying testing guidance. The delivered change was therefore narrower than the original ticket implied: a surgical clarification, not a broad new documentation block and not any runtime/test implementation work.
- Verification results: `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`, `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`, `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`, `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`, `cargo test -p worldwake-ai --test golden_care`, `cargo test -p worldwake-ai --test golden_production`, `cargo test -p worldwake-ai --test golden_trade`, `cargo test -p worldwake-ai --test golden_emergent`, `cargo test --workspace`, and `cargo clippy --workspace` all passed.
