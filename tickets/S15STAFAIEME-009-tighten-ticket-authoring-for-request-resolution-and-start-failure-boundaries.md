# S15STAFAIEME-009: Tighten Ticket Authoring For Request-Resolution And Start-Failure Boundaries

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S15STAFAIEME-002, S15STAFAIEME-007

## Problem

The current [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) already requires precise layer naming and explicit verification surfaces, but S15 still surfaced an avoidable ticket-authoring failure: a mixed AI/runtime/action scenario was initially written as a trade `StartFailed` issue even though the first live rejection happened earlier during request resolution in [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs).

The authoring contract needs one more explicit rule: tickets that claim start-failure or stale-request behavior must name the exact failure boundary and must check the shared runtime request layer before assigning scope to an action handler or AI helper.

## Assumption Reassessment (2026-03-19)

1. The current authoring contract in [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) already requires exact layer naming, invariant-to-layer verification mapping, and distinction among candidate generation, execution, and authoritative outcome.
2. Even with those rules, S15STAFAIEME-002 initially had to be corrected because the first rejection boundary was not in trade handler code but in the shared runtime path `apply_input` / `resolve_affordance` inside [crates/worldwake-sim/src/tick_step.rs:215](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L215) and [crates/worldwake-sim/src/tick_step.rs:378](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L378).
3. Existing focused/golden coverage now makes that distinction concrete: `best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` in [crates/worldwake-sim/src/tick_step.rs:1474](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L1474), `golden_care_pre_start_wound_disappearance_records_blocker` in [crates/worldwake-ai/tests/golden_care.rs:760](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs#L760), and `golden_local_trade_start_failure_recovers_via_production_fallback` in [crates/worldwake-ai/tests/golden_trade.rs:875](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs#L875).
4. This is a documentation/process ticket. The gap is not executable behavior; it is an authoring rule that still allows a ticket to skip the runtime request-resolution seam during assumption reassessment.
5. The contract should remain narrow and actionable. It should add a specific checklist item and wording requirement for start-failure/stale-request tickets rather than expanding into a generic style guide rewrite.
6. `docs/FOUNDATIONS.md` requires legible, fully traceable chains of consequence. Ticket templates and authoring rules should enforce that same causal precision at the planning layer.
7. Scope correction: the canonical place for this change is `tickets/README.md`; do not scatter the rule across individual S15 tickets only.

## Architecture Check

1. Tightening the ticket authoring contract is cleaner than relying on future implementers to rediscover the same runtime seam during code investigation. The planning artifact should force the right question early.
2. Requiring an explicit failure-boundary statement makes tickets more extensible because it scales across domains without embedding trade-, care-, or politics-specific exceptions.
3. No backwards-compatibility escape hatch should remain where a ticket can say "start failure" while actually targeting request-resolution failure, or vice versa.

## Verification Layers

1. The authoring contract explicitly requires start-failure and stale-request tickets to name the exact failure boundary -> doc diff review.
2. The authoring contract explicitly requires assumption reassessment to check the shared request/runtime layer before assigning action-handler scope -> doc diff review.
3. The authoring contract explicitly maps mixed-layer stale-request scenarios to the correct verification surfaces -> doc diff review.
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

## Files to Touch

- `tickets/README.md` (modify)

## Out of Scope

- changing the runtime behavior itself
- rewriting `AGENTS.md`
- modifying archived tickets retroactively beyond any minimal cross-reference updates if needed
- adding domain-specific ticket rules that do not generalize

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
2. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
3. `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
4. Existing suite: `cargo test -p worldwake-ai -- --list`

### Invariants

1. Ticket authoring guidance must force writers to identify the first true failure boundary instead of collapsing request resolution, authoritative start, and later abort into one vague claim.
2. Mixed-layer tickets must explicitly account for the shared runtime request seam before assigning scope to a domain-specific handler.
3. The authoring contract must remain domain-agnostic and reusable across future S08/S15-style tickets.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-sim best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
2. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
3. `cargo test -p worldwake-ai -- --list`
