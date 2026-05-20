# S156HTNAUTHON-006: HTN method drafting checklist + planner-contract docs

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: archive/specs/S156-htn-authority-honesty.md (D6), archive/tickets/S156HTNAUTHON-005.md (completed D5 trace contract)

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
   delivered by completed `archive/tickets/S156HTNAUTHON-005.md`, so the documented field names should
   match the delivered `RejectedMethodTrace` and `StrategicFallbackReason` contract.
4. Adjacent contradiction classification: none. The docs codify decisions already made in the
   spec; no code contradiction is exposed.

## Architecture Check

1. Codifying the drafting rule prevents recurrence of the exact fossil class this spec removed —
   the rule is the durable counterpart to the one-time strip. Placing it in the canonical
   `spec-drafting-rules.md` (rather than a new doc) keeps a single source of truth.
2. No backward-compatibility surface: documentation only.

## Verified Layers

1. Checklist presence and correctness -> manual review against the spec's D6 requirements; the doc
   names the required declarations (reusable pursuit pattern, why flat GOAP is insufficient,
   fallback policy, beliefs/records/observations read, golden tests for selection/rejection/
   fallback/trace) and states that any field expressing required artifacts/claims/failure modes
   must be enforced when declared.
2. Documentation-only ticket: no runtime invariant to map to a trace/event surface — verification
   is review-based, with the runtime contract it describes proven by S156HTNAUTHON-005's tests.

## Landed Changes

### 1. Added an HTN method drafting checklist to `docs/spec-drafting-rules.md`

The checklist now requires each proposed HTN method to declare: the reusable pursuit pattern it
encodes; why flat GOAP is insufficient; whether flat-GOAP fallback is allowed / forbidden /
allowed-after-traced-failure; every belief, memory, record, observation, evidence, motive, and
profile value it reads; and the focused/golden tests proving selection, rejection, fallback, and
trace. It states that required artifacts, claims, records, roles, failure modes, locations, and
capabilities must be enforced when declared, and that a method-required goal is invalid unless
the schema proves fallback would satisfy the wrong semantic condition.

### 2. Documented the method-trace fallback/rejection contract in `docs/planner-contracts.md`

The planner contract now documents the post-S156 method-trace contract: the trace records the
selected method, rejected candidate methods with their failing precondition, and explicit
fallback reasons (`NoViableMethod` or `MethodProducedNoStages`) when flat-GOAP fallback is used.
It names the trace as a transient debug read-model, not authoritative state.

## Landed Files

- `docs/spec-drafting-rules.md` (modify)
- `docs/planner-contracts.md` (modify)

## Out of Scope

- Any code change (delivered by S156HTNAUTHON-001..005).
- Reintroducing the removed schema fields.

## Acceptance Result

### Verification

1. Passed: documentation review confirmed both docs contain the D6 checklist and planner-contract
   content.
2. Passed: `git diff --stat docs/spec-drafting-rules.md docs/planner-contracts.md` confirmed both
   files changed.
3. Passed: scoped Markdown/diff hygiene checks covered the doc-only diff.
4. Waived: `./scripts/verify.sh` for this ticket iteration because the `implement-spec-tickets`
   harness owns the final pre-push verification gate after the S156 family lands.

### Invariants

1. Passed: `docs/spec-drafting-rules.md` carries an HTN method checklist that forbids declared-but-
   unenforced method semantics and gates method-required goals on a fallback-invalidity proof.
2. Passed: `docs/planner-contracts.md` documents the method-trace fallback/rejection contract consistent
   with the delivered `MethodPlanAttemptTrace` fields.

## Test Plan Result

### Added/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands Run

1. Passed `git diff --stat docs/spec-drafting-rules.md docs/planner-contracts.md`
2. Passed `git diff --check -- docs/spec-drafting-rules.md docs/planner-contracts.md archive/tickets/S156HTNAUTHON-006.md`
3. Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py archive/tickets/S156HTNAUTHON-006.md`
4. Waived `./scripts/verify.sh` for this ticket iteration because the harness runs it before push after final spec archival.

## Outcome

Completed on 2026-05-20.

- Added the HTN method drafting checklist to `docs/spec-drafting-rules.md`.
- Added the HTN method trace fallback/rejection contract to `docs/planner-contracts.md`.
- No production code, tests, generated docs, or runtime behavior changed.

## Deviations

- The ticket remained documentation-only as drafted.
- The final `./scripts/verify.sh` gate is deferred to the harness-level pre-push verification after
  all S156 tickets and final spec archival are complete.

## Verification Result

- Passed `git diff --stat docs/spec-drafting-rules.md docs/planner-contracts.md`
- Passed `git diff --check -- docs/spec-drafting-rules.md docs/planner-contracts.md archive/tickets/S156HTNAUTHON-006.md`
- Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py archive/tickets/S156HTNAUTHON-006.md`
- Waived `./scripts/verify.sh` for this ticket iteration because the harness owns the final pre-push verification gate after the S156 family lands.
