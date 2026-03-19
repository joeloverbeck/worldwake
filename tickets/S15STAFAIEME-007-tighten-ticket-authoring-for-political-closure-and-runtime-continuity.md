# S15STAFAIEME-007: Tighten Ticket Authoring For Political Closure Boundaries And Runtime Continuity

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S15STAFAIEME-003, S15STAFAIEME-006

## Problem

The current ticket authoring contract in [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) is strong on generic mixed-layer precision, but it still leaves room for two political/test-authoring mistakes that surfaced during S15 implementation:

1. collapsing support-law office closure into vague language like "someone else got there first" without naming the exact closure boundary in the current office law
2. assuming harness control changes automatically clear retained AI runtime intent, even when the runtime can still continue an already-selected plan shape

Without tightening those authoring rules, future political tickets can still describe the wrong architecture and drive brittle or misleading tests.

## Assumption Reassessment (2026-03-19)

1. The current ticket contract already requires naming the first failure boundary and the exact verification layer split, but it does not explicitly require political tickets to name the office-claim closure boundary itself. Support-law office closure currently depends on office vacancy/holder state plus succession timing, not on `declare_support` alone. The relevant symbols are `office_is_visibly_vacant()` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) and support-law succession in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs).
2. The current testing guidance in [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already warns against proxying early ordering with later installation, but [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) does not yet tell ticket authors to name support declaration, succession resolution, office-holder mutation, and visible-vacancy loss as distinct possible boundaries.
3. Existing coverage already proves the political locality and start-failure behavior that exposed this authoring gap: `golden_tell_propagates_political_knowledge`, `golden_same_place_office_fact_still_requires_tell`, and `golden_remote_office_claim_start_failure_loses_gracefully` in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs). I confirmed the live test names today with `cargo test -p worldwake-ai -- --list`.
4. The current decision trace already exposes `SelectionTrace.selected_plan_source` and `PlanningPipelineTrace.plan_continued` in [decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs), which means retained-plan continuity is part of the current architecture and ticket authors should be required to account for it when a test manipulates control/runtime conditions.
5. This is a documentation-only ticket. It does not change AI, politics, or scheduler behavior. The gap is missing precision in ticket-authoring rules and template prompts.
6. Ordering is not the primary contract here. The issue is semantic boundary naming: tickets must say whether their proof depends on request resolution, authoritative start, visible vacancy, succession timer completion, or final office-holder installation.
7. Scenario-isolation guidance already exists for goldens, but the ticket contract should also require political tickets to disclose when they intentionally remove unrelated lawful branches such as self-care or social propagation in order to isolate a closure-boundary proof.
8. Scope correction: do not broaden this into general documentation cleanup. Limit the work to [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) and, if needed, [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) so future tickets are forced to state these boundaries explicitly.

## Architecture Check

1. Tightening the ticket contract is cleaner than repeatedly correcting the same architectural misunderstanding ticket-by-ticket after implementation begins.
2. No backwards-compatibility wording or alternate ticket path should preserve vague political closure language once the repo already has the more precise architecture and tests.

## Verification Layers

1. Ticket-authoring guidance explicitly distinguishes support declaration, visible-vacancy loss, succession resolution, and office-holder installation -> doc diff review in [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) and [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md).
2. Ticket-authoring guidance explicitly requires accounting for retained-plan/runtime continuity when control or harness conditions are manipulated -> doc diff review.
3. Existing political goldens remain the named reference examples for those rules -> existing golden coverage in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs).
4. Additional action/request/politics trace changes are not applicable because this ticket only changes authoring guidance.

## What to Change

### 1. Tighten the political-boundary rules in the ticket contract

Update [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) so political tickets must name the exact office-claim closure boundary they rely on:

- support declaration
- visible-vacancy loss
- succession resolution
- office-holder mutation

Also require political tickets to name the exact current symbols they checked when making that claim.

### 2. Tighten runtime-continuity rules in the ticket contract and template

Update [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) and, if necessary, [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) so tickets that manipulate `ControlSource`, queued inputs, or harness runtime conditions must state whether retained AI runtime intent can still lawfully continue and how the intended proof accounts for that.

## Files to Touch

- `tickets/README.md` (modify)
- `tickets/_TEMPLATE.md` (modify, only if needed to enforce the new rule)

## Out of Scope

- `docs/golden-e2e-testing.md`
- `docs/golden-e2e-scenarios.md`
- any `crates/` runtime or test code
- broad non-political ticket-authoring rewrites unrelated to closure boundaries or runtime continuity

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
2. `cargo test -p worldwake-ai golden_tell_propagates_political_knowledge -- --exact`
3. Inventory check: `cargo test -p worldwake-ai -- --list`

### Invariants

1. Future political tickets must not be able to describe support-law closure as vague "got there first" language when the actual closure boundary is different.
2. Future tickets that manipulate control/runtime state must account for retained-plan continuity rather than assuming control changes automatically clear AI runtime intent.
3. The ticket template and contract must remain precise without adding compatibility wording or parallel authoring paths.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
2. `cargo test -p worldwake-ai golden_tell_propagates_political_knowledge -- --exact`
3. `cargo test -p worldwake-ai -- --list`
