# S156HTNAUTHON-006: HTN method drafting checklist + planner-contract docs

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: specs/S156-htn-authority-honesty.md (D6)

## Problem

The strip in S156HTNAUTHON-001..005 removes declared-but-unenforced HTN method semantics. Without
a drafting rule, a future contributor could reintroduce the same fossils (empty schema fields,
no-op preconditions, dead criteria) or silently make a goal method-required without proving
fallback is invalid. This ticket folds an HTN method drafting checklist into
`docs/spec-drafting-rules.md` and documents the method-trace fallback/rejection contract in
`docs/planner-contracts.md`, so the honesty guarantees are durable (FND-29).

## Assumption Reassessment (2026-05-20)

1. `docs/spec-drafting-rules.md` exists; it has a numbered FND-01 Section H rule list (1-5,
   including item 5 "Planner-formalism analysis") plus an "Agent Profile Scenario Contract"
   section. It has no dedicated HTN method checklist today.
2. `docs/planner-contracts.md` exists with sections "Why This Exists", "1. Exact-Goal Terminal
   Surfacing", "2. Planning Snapshot Completeness", and further numbered contract sections. It does
   not yet document a method-trace fallback/rejection contract.
3. This is a documentation-only ticket — no engine code changes. The trace fields it documents are
   delivered by S156HTNAUTHON-005; this ticket should be written after or alongside 005 so the
   documented field names match the delivered ones (soft ordering; no compile dependency).
4. Adjacent contradiction classification: none. The docs codify decisions already made in the
   spec; no code contradiction is exposed.

## Architecture Check

1. Codifying the drafting rule prevents recurrence of the exact fossil class this spec removed —
   the rule is the durable counterpart to the one-time strip. Placing it in the canonical
   `spec-drafting-rules.md` (rather than a new doc) keeps a single source of truth.
2. No backward-compatibility surface: documentation only.

## Verification Layers

1. Checklist presence and correctness -> manual review against the spec's D6 requirements; the doc
   names the required declarations (reusable pursuit pattern, why flat GOAP is insufficient,
   fallback policy, beliefs/records/observations read, golden tests for selection/rejection/
   fallback/trace) and states that any field expressing required artifacts/claims/failure modes
   must be enforced when declared.
2. Documentation-only ticket: no runtime invariant to map to a trace/event surface — verification
   is review-based, with the runtime contract it describes proven by S156HTNAUTHON-005's tests.

## What to Change

### 1. Add an HTN method drafting checklist to `docs/spec-drafting-rules.md`

Add a checklist requiring each proposed HTN method to declare: the reusable pursuit pattern it
encodes; why flat GOAP is insufficient; whether flat-GOAP fallback is allowed / forbidden /
allowed-after-traced-failure; every belief, record, and observation it reads; and the golden tests
proving selection, rejection, fallback, and trace. State that any field expressing required
artifacts/claims/failure modes must be *enforced* when declared (no re-creation of dead schema),
and that a method-required goal is invalid unless the schema proves fallback would satisfy the
wrong semantic condition.

### 2. Document the method-trace fallback/rejection contract in `docs/planner-contracts.md`

Add a section describing the post-S156 method-trace contract: the trace records the selected
method, the rejected candidate methods with their failing precondition, and the explicit fallback
reason when no method produces stages (matching the fields delivered by S156HTNAUTHON-005). Name
it as a transient debug read-model, not authoritative state.

## Files to Touch

- `docs/spec-drafting-rules.md` (modify)
- `docs/planner-contracts.md` (modify)

## Out of Scope

- Any code change (delivered by S156HTNAUTHON-001..005).
- Reintroducing the removed schema fields.

## Acceptance Criteria

### Tests That Must Pass

1. None — documentation-only ticket; verification is review-based and the runtime contract it
   describes is proven by S156HTNAUTHON-005's tests.

### Invariants

1. `docs/spec-drafting-rules.md` carries an HTN method checklist that forbids declared-but-
   unenforced method semantics and gates method-required goals on a fallback-invalidity proof.
2. `docs/planner-contracts.md` documents the method-trace fallback/rejection contract consistent
   with the delivered `MethodPlanAttemptTrace` fields.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `git diff --stat docs/spec-drafting-rules.md docs/planner-contracts.md` (confirm both files changed)
2. `./scripts/verify.sh` (before PR — confirms the doc edits do not break any doc-consuming check)
