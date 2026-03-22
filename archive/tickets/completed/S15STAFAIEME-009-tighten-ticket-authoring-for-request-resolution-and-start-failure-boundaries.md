# S15STAFAIEME-009: Tighten Ticket Authoring For Request-Resolution And Start-Failure Boundaries

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None; ticket-authoring contract/template only
**Deps**: S15STAFAIEME-002, S15STAFAIEME-007, `tickets/README.md`, `tickets/_TEMPLATE.md`

## Problem

The current [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) already requires precise layer naming and explicit verification surfaces, but S15 still surfaced an avoidable ticket-authoring failure: a mixed AI/runtime/action scenario was initially written as a trade `StartFailed` issue even though the first live rejection happened earlier during shared request resolution in [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs).

The authoring contract needs one more explicit rule: tickets that claim start-failure or stale-request behavior must name the exact failure boundary and must check the shared runtime request layer before assigning scope to an action handler or AI helper.

## Assumption Reassessment (2026-03-19)

1. The current authoring contract in [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) already requires exact layer naming, invariant-to-layer verification mapping, and distinction among candidate generation, execution, and authoritative outcome, but it does not yet explicitly force start-failure/stale-request tickets to name the first failure boundary or to check the shared request-resolution seam before assigning domain scope.
2. The live authoritative boundary is the shared runtime path `apply_input` and `resolve_affordance` in [crates/worldwake-sim/src/tick_step.rs:227](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L227) and [crates/worldwake-sim/src/tick_step.rs:429](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L429), not a trade- or care-specific helper.
3. Existing focused coverage already proves the shared runtime seam directly, not just via downstream domain goldens: `best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` in [crates/worldwake-sim/src/tick_step.rs:1541](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L1541), `reproduced_request_records_request_resolution_trace_before_start` in [crates/worldwake-sim/src/tick_step.rs:1627](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L1627), and `resolve_affordance_uses_shared_request_binding_rule` in [crates/worldwake-sim/src/tick_step.rs:1817](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L1817).
4. Existing golden coverage still matters, but it now sits on top of that shared runtime substrate: `golden_care_pre_start_wound_disappearance_records_blocker` in [crates/worldwake-ai/tests/golden_care.rs:764](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs#L764) and `golden_local_trade_start_failure_recovers_via_production_fallback` in [crates/worldwake-ai/tests/golden_trade.rs:907](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs#L907).
5. This remains a documentation/process ticket. The executable architecture is already the cleaner shared architecture: request-resolution trace for pre-start binding facts, scheduler/action trace for authoritative start failure, and decision trace for AI reconciliation. The remaining gap is authoring guidance that still allows a ticket to skip the shared runtime seam during assumption reassessment.
6. Updating only [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) would leave the default ticket scaffold silent on the new rule. The cleaner, more durable scope is to update both the contract and [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) so future tickets are prompted to state the failure boundary explicitly.
7. `docs/FOUNDATIONS.md` requires legible, fully traceable chains of consequence. Ticket templates and authoring rules should enforce the same causal precision at the planning layer instead of relying on later ticket correction.
8. Scope correction: the canonical places for this change are [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) and [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md); do not scatter the rule across individual S15 tickets only.

## Architecture Check

1. Tightening the ticket authoring contract and template is cleaner than relying on future implementers to rediscover the same runtime seam during code investigation. The planning artifact should force the right question early.
2. Requiring an explicit failure-boundary statement makes tickets more extensible because it scales across domains without embedding trade-, care-, or politics-specific exceptions.
3. Updating the template is cleaner than relying on README-only prose because it turns the rule into a default authoring prompt instead of a rule that can still be forgotten.
4. No backwards-compatibility escape hatch should remain where a ticket can say "start failure" while actually targeting request-resolution failure, or vice versa.

## Verification Layers

1. The authoring contract explicitly requires start-failure and stale-request tickets to name the exact failure boundary -> doc diff review in [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md).
2. The authoring contract explicitly requires assumption reassessment to check the shared request/runtime layer before assigning action-handler scope -> doc diff review in [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md).
3. The default ticket scaffold prompts authors to record that boundary during ticket creation instead of relying on memory -> doc diff review in [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md).
4. Additional runtime verification layers are not applicable because this ticket only changes ticket-authoring guidance.

## What to Change

### 1. Add an explicit failure-boundary rule

Extend [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) so tickets involving stale requests, contested affordances, or start-failure recovery must name whether the first failure boundary is:

- request resolution / affordance reproduction
- authoritative start
- post-start abort / commit-time revalidation

### 2. Add a mandatory runtime-seam check

Add a pre-implementation check or assumption requirement stating that mixed AI/runtime/action tickets must verify the shared runtime request path before assigning scope to a domain action handler or AI failure-reconciliation helper.

### 3. Add a verification-layer reminder for these tickets

Document that tickets in this class should usually map invariants across:

- focused runtime request-resolution coverage
- action trace for actual start/abort lifecycle
- decision trace for AI reconciliation
- golden E2E only when the recovery chain itself is part of the contract

### 4. Prompt for the boundary in the ticket template

Update [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) so start-failure/stale-request tickets are prompted to record:

- the first failure boundary
- the exact shared runtime symbols checked during reassessment
- the verification layer that proves each boundary

## Files to Touch

- `tickets/README.md` (modify)
- `tickets/_TEMPLATE.md` (modify)

## Out of Scope

- changing the runtime behavior itself
- rewriting `AGENTS.md`
- modifying archived tickets retroactively beyond any minimal cross-reference updates if needed
- adding domain-specific ticket rules that do not generalize

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
2. `cargo test -p worldwake-sim tick_step::tests::reproduced_request_records_request_resolution_trace_before_start -- --exact`
3. `cargo test -p worldwake-sim tick_step::tests::resolve_affordance_uses_shared_request_binding_rule -- --exact`
4. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
5. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
6. Existing suite: `cargo test -p worldwake-ai -- --list`
7. Existing suite: `cargo test --workspace`
8. Existing suite: `cargo clippy --workspace`

### Invariants

1. Ticket authoring guidance must force writers to identify the first true failure boundary instead of collapsing request resolution, authoritative start, and later abort into one vague claim.
2. Mixed-layer tickets must explicitly account for the shared runtime request seam before assigning scope to a domain-specific handler.
3. The ticket template must prompt the same boundary analysis so the contract survives beyond this one README edit.
4. The authoring contract must remain domain-agnostic and reusable across future S08/S15-style tickets.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime/golden coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-sim tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
2. `cargo test -p worldwake-sim tick_step::tests::reproduced_request_records_request_resolution_trace_before_start -- --exact`
3. `cargo test -p worldwake-sim tick_step::tests::resolve_affordance_uses_shared_request_binding_rule -- --exact`
4. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
5. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
6. `cargo test -p worldwake-ai -- --list`
7. `cargo test --workspace`
8. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-19
- What actually changed:
  - tightened [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) so stale-request, contested-affordance, and start-failure tickets must name the first failure boundary explicitly
  - added contract language requiring reassessment of the shared runtime request seam before assigning domain-specific scope
  - updated [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) so new tickets are prompted to record the boundary and the boundary-specific verification layers
- Deviations from original plan:
  - scope widened slightly from `tickets/README.md` only to include [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md), because README-only guidance would still leave the default ticket scaffold silent on the new requirement
  - no runtime or AI code changes were needed because the current architecture already cleanly separates request-resolution trace, authoritative start failure, and AI reconciliation
- Verification results:
  - `cargo test -p worldwake-sim tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
  - `cargo test -p worldwake-sim tick_step::tests::reproduced_request_records_request_resolution_trace_before_start -- --exact`
  - `cargo test -p worldwake-sim tick_step::tests::resolve_affordance_uses_shared_request_binding_rule -- --exact`
  - `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
  - `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
  - `cargo test -p worldwake-ai -- --list`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
