# E16DPOLPLAN-032: Tighten ticket authoring rules for existing coverage checks and runtime-test scope

**Status**: COMPLETED
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

1. [`tickets/README.md`](/home/joeloverbeck/projects/worldwake/tickets/README.md) already requires assumption reassessment, exact symbol naming, testing-gap distinctions, and dry-run verification of test commands, but it still does not explicitly require searching for existing focused/runtime/golden coverage before claiming a gap — corrected scope.
2. [`tickets/README.md`](/home/joeloverbeck/projects/worldwake/tickets/README.md) currently distinguishes broad AI phases such as candidate generation, ranking, plan search, and authoritative outcome, but it does not yet force ticket authors to name the intended regression layer as candidate generation vs runtime `agent_tick` decision-trace vs golden E2E — confirmed gap.
3. [`tickets/_TEMPLATE.md`](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) does not currently prompt authors to record existing focused/unit coverage, existing runtime trace coverage, existing golden/E2E coverage, or the runtime harness boundary when an `agent_tick` regression depends on non-needs affordances — confirmed.
4. The Force-law correction in archived [`archive/tickets/completed/E16DPOLPLAN-029.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/E16DPOLPLAN-029.md), read against [`specs/E16d-political-planning-and-golden-coverage.md`](/home/joeloverbeck/projects/worldwake/specs/E16d-political-planning-and-golden-coverage.md), shows the concrete failure mode: the invariant was already covered at candidate-generation and golden scope, but the requested runtime test boundary and harness requirements were not stated explicitly enough in the ticket.
5. This remains a docs/process ticket. Production code, runtime test semantics, and political-planning architecture are out of scope unless the reassessment reveals a real code defect, which it did not.

## Architecture Check

1. Tightening the ticket contract is cleaner than repeatedly fixing stale or ambiguous tickets during implementation.
2. The missing rule should be additive, not a rewrite of the existing contract. `tickets/README.md` already carries the broad architectural discipline; this ticket should add the narrower coverage-discovery and runtime-boundary prompts that are currently absent.
3. The rules should force authors to distinguish three AI-test scopes explicitly when relevant:
   - candidate-generation focused/unit coverage
   - runtime `agent_tick` trace/integration coverage
   - golden E2E coverage
4. No backwards-compatibility shims are involved. This is a documentation/process correction that improves ticket fidelity without adding a parallel process.

## What to Change

### 1. Update `tickets/README.md`

- Add an explicit pre-implementation requirement to search for and name existing focused/unit, runtime trace/integration, and golden/E2E coverage before claiming a testing gap.
- Add a precision rule that AI-regression tickets must state which layer they target:
  - candidate generation
  - runtime `agent_tick` / decision-trace integration
  - golden E2E
- Add a rule that if a runtime `agent_tick` regression depends on non-needs affordances, the ticket must say whether it uses the local needs-only harness or full action registries.
- Keep the existing broader phase distinctions intact instead of replacing them.

### 2. Update `tickets/_TEMPLATE.md`

- Add prompts that force authors to record:
  - existing focused/unit coverage already present
  - existing runtime trace/integration coverage already present
  - existing golden/E2E coverage already present
  - how those existing tests were verified when a coverage gap is claimed
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
2. The updated ticket contract explicitly requires checking for existing runtime trace and golden coverage before claiming a gap.
3. The template explicitly prompts for AI-test layer selection and runtime harness scope when relevant.
4. Verification command: `cargo test -p worldwake-ai -- --list` remains the recommended dry-run tool for confirming real test names/targets during ticket drafting.

### Invariants

1. New tickets are less likely to misstate coverage gaps that already have focused tests.
2. AI-regression tickets distinguish candidate generation, runtime `agent_tick`, and golden E2E instead of collapsing them into one claim.
3. Runtime `agent_tick` tickets that depend on non-needs affordances name the harness boundary instead of assuming it implicitly.
4. Doc/process updates remain concise enough to be followed consistently.

## Test Plan

### New/Modified Tests

1. `tickets/README.md` — add authoring rules for explicit coverage discovery, AI-test layer naming, and exact runtime harness scope.
2. `tickets/_TEMPLATE.md` — add prompts that force authors to record existing coverage and the intended runtime boundary when relevant.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `rg -n "focused/unit|runtime trace|golden/E2E|agent_tick|full action registr" tickets/README.md tickets/_TEMPLATE.md`
3. `git diff -- tickets/README.md tickets/_TEMPLATE.md`

## Outcome

- **Completion date**: 2026-03-18
- **What actually changed**:
  - Reassessed the ticket and corrected its scope to acknowledge that [`tickets/README.md`](/home/joeloverbeck/projects/worldwake/tickets/README.md) already enforced assumption reassessment, layer distinctions, and dry-run test-command verification before this work.
  - Tightened [`tickets/README.md`](/home/joeloverbeck/projects/worldwake/tickets/README.md) with the missing narrow rules: explicit coverage discovery across focused/unit, runtime trace/integration, and golden/E2E layers; explicit AI regression layer naming; explicit `agent_tick` harness-boundary naming when non-needs affordances matter; and explicit use of `cargo test -p worldwake-ai -- --list` for AI-ticket verification.
  - Tightened [`tickets/_TEMPLATE.md`](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) so new tickets must record existing coverage and the intended runtime boundary when the ticket targets AI regressions.
- **Deviations from original plan**:
  - No production-code or Rust-test changes were needed because the reassessment confirmed this was a ticket-authoring contract gap, not a runtime defect.
  - The final ticket scope is narrower and more accurate than the original wording: it extends the existing contract instead of implying that the current README lacked all testing-gap and command-verification guidance.
- **Verification results**:
  - `cargo test -p worldwake-ai -- --list` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace` ✅
