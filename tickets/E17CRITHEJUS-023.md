# E17CRITHEJUS-023: Document the planner exact-goal, snapshot, and traceability contracts

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: E17CRITHEJUS-020, E17CRITHEJUS-021, E17CRITHEJUS-022

## Problem

The architectural lessons from `E17CRITHEJUS-008` are now clearer than the current docs. The repository explains mixed-layer ticket drafting and traceability escalation well, but it does not yet document the specific planner contracts that matter here:

- when a live goal may surface a terminal operator from goal identity
- which planner-visible runtime belief data must survive into `PlanningSnapshot`
- what decision traces should explain when a relevant operator is absent or a planner prerequisite is missing

Without those contracts written down, future tickets are likely to repeat the same stale-narrative failure mode: a narrow symptom gets treated as the bug while the shared planner boundary remains implicit.

## Assumption Reassessment (2026-03-26)

1. `AGENTS.md`, `tickets/README.md`, and `docs/precision-rules.md` already require assumption reassessment, layer naming, verification-layer mapping, and traceability escalation. The current gap is planner-specific contract documentation, not a missing general authoring framework.
2. `AGENTS.md` already includes the Authoritative-To-AI impact rule and decision-trace debugging guidance, but it does not state the exact-goal terminal surfacing contract or a planning-snapshot completeness checklist.
3. No single current doc explains the planner path end to end: `GroundedGoal` / relevant operators -> root candidate surfacing -> payload/target synthesis -> snapshot-backed duration estimation -> decision-trace omission/prerequisite diagnostics.
4. The intended invariant is documentation clarity, not behavior change. This ticket should document the final architecture after `E17CRITHEJUS-020` through `E17CRITHEJUS-022`, not invent an alternate contract in prose.
5. This is a documentation-only ticket. Verification is doc-content inspection plus regression safety that the workspace still builds/tests after doc edits.
6. `docs/FOUNDATIONS.md` is the governing design source and should not be diluted or duplicated. The new docs should align to it by naming how these planner contracts support explainable emergence, locality of information, concrete state, and no workaround architecture.
7. Adjacent process guidance already exists in `archive/tickets/completed/E17CRITHEJUS-019.md`. This ticket should extend planner-specific documentation, not re-open the broader ticket-authoring contract work.
8. Mismatch + correction: the missing artifact is not “more general process text.” The missing artifact is planner-specific contract documentation that points back to the existing foundational/process docs instead of forking them.

## Architecture Check

1. The clean fix is to document the exact planner contracts that future work must preserve, in one authoritative planner-facing doc plus concise cross-references from the existing contributor docs.
2. That is cleaner than scattering this knowledge across archived tickets or relying on oral history from one implementation.
3. This aligns with `docs/FOUNDATIONS.md`: the project should explain its causal contracts explicitly and reject workaround lore.
4. No backwards-compatibility aliasing applies. Update the authoritative docs in place rather than creating parallel guidance sets.

## Verification Layers

1. Exact-goal terminal surfacing contract is documented against the final live planner architecture -> doc content inspection
2. Planning-snapshot completeness/audit contract is documented against the final live planner architecture -> doc content inspection
3. Traceability expectations for omitted operators and missing prerequisites are documented without duplicating `docs/precision-rules.md` -> doc content inspection
4. Single-layer ticket; additional runtime proof mapping is not applicable beyond regression safety checks

## What to Change

### 1. Add planner contract documentation

Create or update one planner-focused doc that explains:
- exact-goal terminal operator surfacing
- the snapshot completeness contract for planner-visible runtime belief data
- traceability expectations for omitted relevant operators and named missing prerequisites

### 2. Add concise contributor cross-references

Update the smallest useful set of contributor docs so future ticket authors and implementers know where the planner contract lives and when to consult it.

### 3. Keep documentation aligned with the foundations

Tie the planner contract back to the relevant foundational principles instead of restating them wholesale.

## Files to Touch

- `AGENTS.md` (modify)
- `docs/precision-rules.md` (modify only if a planner-specific cross-reference is needed)
- `docs/` planner-facing contract doc (new or modify; exact path to be chosen during implementation)

## Out of Scope

- Any planner behavior changes
- Rewriting `docs/FOUNDATIONS.md`
- Broad process-doc rewrites unrelated to planner/search/snapshot/traceability contracts

## Acceptance Criteria

### Tests That Must Pass

1. The repository contains one authoritative planner-facing doc covering exact-goal terminal surfacing, snapshot completeness, and traceability expectations.
2. `AGENTS.md` points contributors to that planner contract without duplicating the full content.
3. The new/updated docs remain consistent with `docs/FOUNDATIONS.md`, `tickets/README.md`, and `docs/precision-rules.md`.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Future planner tickets can cite one explicit contract instead of relying on stale narratives from prior bugfixes.
2. Documentation remains non-duplicative: foundations stay foundational, precision rules stay canonical for claim hygiene, and planner docs capture planner-specific architecture only.

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `sed -n '1,260p' AGENTS.md`
2. `sed -n '1,260p' docs/precision-rules.md`
3. `cargo test -p worldwake-ai`
