# S15STAFAIEME-007: Tighten Ticket Authoring For Political Closure Boundaries And Runtime Continuity

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S15STAFAIEME-003, S15STAFAIEME-006

## Problem

The current ticket authoring contract in [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) is strong on generic mixed-layer precision, but it still leaves room for two political/test-authoring mistakes that surfaced during S15 implementation:

1. collapsing support-law office closure into vague language like "someone else got there first" without naming the exact closure boundary in the current office law
2. assuming harness control changes automatically clear retained AI runtime intent, even when the runtime can still continue an already-selected plan shape

Without tightening those authoring rules, future political tickets can still describe the wrong architecture and drive brittle or misleading tests.

## Assumption Reassessment (2026-03-20)

1. Reassessment correction: the repo already contains the S15 political start-failure goldens this ticket references, so this ticket is not introducing new behavioral coverage. I confirmed the live names with `cargo test -p worldwake-ai -- --list`, including `golden_remote_office_claim_start_failure_loses_gracefully`, `golden_tell_propagates_political_knowledge`, and `golden_same_place_office_fact_still_requires_tell` in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs).
2. The current ticket contract already requires naming the first failure boundary and the exact verification layer split, but it still does not explicitly require political tickets to name the office-claim closure boundary itself. In the current architecture, the AI political candidate boundary is `office_is_visibly_vacant()` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), while authoritative support-law closure is enforced through `validate_declare_support_context_in_world()` in [office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) and succession / holder mutation in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs).
3. The current testing guidance in [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already warns against proxying early ordering with later installation, but [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) and [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) do not yet force ticket authors to name support declaration, visible-vacancy loss, succession resolution, and office-holder mutation as distinct possible political boundaries.
4. Retained runtime continuity is part of the current architecture. The decision-trace surface exposes `SelectionTrace.selected_plan_source` and `PlanningPipelineTrace.plan_continued` in [decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs), and the live political/trade goldens explicitly manipulate `ControlSource` while asserting reconciliation behavior in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) and [golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs).
5. This remains a documentation-only ticket. It does not change AI, political law, scheduler behavior, or runtime request handling. The architectural gap is missing precision in ticket-authoring rules and template prompts, not missing production substrate.
6. Ordering is not the primary contract here. The ticket needs semantic boundary naming: future tickets must say whether their proof depends on request resolution, authoritative start, visible-vacancy loss, succession resolution, or final office-holder mutation/installation.
7. Scenario-isolation guidance already exists for goldens, but the ticket contract should also require political tickets to disclose when they intentionally remove unrelated lawful branches such as self-care or social propagation in order to isolate a closure-boundary proof.
8. Scope correction: keep this limited to [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) and [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md). Do not broaden it into runtime/docs cleanup elsewhere because the current architecture and tests already provide the needed behavioral substrate.

## Architecture Check

1. Tightening the ticket contract and template is more beneficial than changing runtime architecture here because the current architecture already cleanly separates AI-visible vacancy, authoritative support validation, succession resolution, and retained-plan continuity. The actual defect is documentation precision, not a missing engine abstraction.
2. Requiring exact boundary names and runtime-continuity checks at ticket-authoring time is cleaner than repeatedly correcting the same misunderstanding during implementation review. It preserves the existing architecture instead of encouraging tests that lean on vague closure language or false assumptions about plan clearing.
3. No backwards-compatibility wording or alternate authoring path should preserve vague political closure language once the repo already has the more precise architecture and tests.

## Verification Layers

1. Ticket-authoring guidance explicitly distinguishes support declaration, visible-vacancy loss, succession resolution, and office-holder installation -> doc diff review in [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) and [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md).
2. Ticket-authoring guidance explicitly requires accounting for retained-plan/runtime continuity when control or harness conditions are manipulated -> doc diff review.
3. Existing political and retained-runtime goldens remain the named reference examples for those rules -> existing golden coverage in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) and [golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs).
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
3. `cargo test -p worldwake-ai golden_same_place_office_fact_still_requires_tell -- --exact`
4. Inventory check: `cargo test -p worldwake-ai -- --list`

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
3. `cargo test -p worldwake-ai golden_same_place_office_fact_still_requires_tell -- --exact`
4. `cargo test -p worldwake-ai -- --list`

## Outcome

- Completion date: 2026-03-20
- What actually changed: tightened [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) and [tickets/_TEMPLATE.md](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) so political office-claim tickets must name the exact closure boundary and symbols checked, and tickets that manipulate `ControlSource` or harness/runtime conditions must account for retained runtime intent with exact runtime/trace symbols.
- Deviations from original plan: no production or test code changed because the current architecture already cleanly separates AI-visible vacancy, authoritative support validation, succession resolution, and retained-plan continuity; the necessary correction was ticket-contract precision only.
- Verification results: `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`, `cargo test -p worldwake-ai golden_tell_propagates_political_knowledge -- --exact`, `cargo test -p worldwake-ai golden_same_place_office_fact_still_requires_tell -- --exact`, `cargo test -p worldwake-ai -- --list`, `cargo test --workspace`, and `cargo clippy --workspace` all passed.
