# E16DPOLPLAN-032: Tighten ticket authoring rules for existing coverage checks and runtime-test scope

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — ticket/doc contract updates only
**Deps**: E16DPOLPLAN-029

## Problem

The Force-law regression ticket drifted in two avoidable ways before implementation:
- it claimed a focused coverage gap even though `candidate_generation::tests::political_candidates_skip_force_law_offices` already existed
- it asked for an `agent_tick` regression without naming that the runtime test needed the full action registry rather than the default needs-only local harness

That kind of drift wastes time during implementation and makes it easier for future tickets to misdescribe the current architecture. The project already has strong ticket rules, but this specific failure mode is not spelled out explicitly enough in `tickets/README.md` and `tickets/_TEMPLATE.md`.

## Assumption Reassessment (2026-03-18)

1. [`tickets/README.md`](/home/joeloverbeck/projects/worldwake/tickets/README.md) already requires assumption reassessment, exact symbol naming, and layer distinctions, but it does not explicitly require searching for existing focused tests before claiming a gap — confirmed.
2. [`tickets/_TEMPLATE.md`](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) does not currently prompt authors to state whether a requested AI regression belongs at candidate generation, runtime `agent_tick`, or golden E2E scope — confirmed.
3. The Force-law ticket correction in archived [`archive/tickets/completed/E16DPOLPLAN-029.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/E16DPOLPLAN-029.md) shows both failure modes concretely: stale coverage claim and ambiguous runtime test scope — confirmed.
4. This is a docs/process ticket, not a request to alter production code or test semantics — corrected scope.

## Architecture Check

1. Tightening the ticket contract is cleaner than repeatedly fixing stale or ambiguous tickets during implementation.
2. The rules should force authors to distinguish three AI-test scopes explicitly when relevant:
   - candidate-generation focused/unit coverage
   - runtime `agent_tick` trace/integration coverage
   - golden E2E coverage
3. No backwards-compatibility shims are involved. This is a documentation/process correction that improves spec fidelity.

## What to Change

### 1. Update `tickets/README.md`

- Add an explicit pre-implementation requirement to search for existing focused tests before claiming a testing gap.
- Add a precision rule that AI-regression tickets must state which layer they target:
  - candidate generation
  - runtime `agent_tick` / decision-trace integration
  - golden E2E
- Add a rule that if a runtime `agent_tick` regression depends on non-needs affordances, the ticket must say whether it uses the local needs-only harness or full action registries.

### 2. Update `tickets/_TEMPLATE.md`

- Add prompts that force authors to record:
  - existing focused/unit coverage already present
  - existing runtime trace coverage already present
  - existing golden/E2E coverage already present
  - exact runtime test boundary if `agent_tick` is the intended layer

### 3. Keep the changes narrow and enforceable

- Do not expand the template into a long checklist wall.
- Prefer short, specific prompts tied directly to the failure modes observed in E16DPOLPLAN-029.

## Files to Touch

- `tickets/README.md` (modify)
- `tickets/_TEMPLATE.md` (modify)

## Out of Scope

- Rewriting old tickets in bulk
- Production code changes
- Trace-schema changes
- Golden harness documentation unrelated to ticket authoring

## Acceptance Criteria

### Tests That Must Pass

1. The updated ticket contract explicitly requires checking for existing focused coverage before claiming a gap.
2. The template explicitly prompts for AI-test layer selection and runtime harness scope when relevant.
3. Verification command: `cargo test -p worldwake-ai -- --list` remains the recommended dry-run tool for confirming real test names/targets during ticket drafting.

### Invariants

1. New tickets are less likely to misstate coverage gaps that already have focused tests.
2. AI-regression tickets distinguish candidate generation, runtime `agent_tick`, and golden E2E instead of collapsing them into one claim.
3. Doc/process updates remain concise enough to be followed consistently.

## Test Plan

### New/Modified Tests

1. `tickets/README.md` — add authoring rules for existing-coverage discovery and exact runtime-test scope.
2. `tickets/_TEMPLATE.md` — add prompts that force those checks into new tickets.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `rg -n "existing focused|agent_tick|golden E2E|full action registries" tickets/README.md tickets/_TEMPLATE.md`
3. `git diff -- tickets/README.md tickets/_TEMPLATE.md`

