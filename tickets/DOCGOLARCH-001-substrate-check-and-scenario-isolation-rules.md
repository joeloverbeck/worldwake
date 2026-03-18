# DOCGOLARCH-001: Add Heuristic-Substrate and Scenario-Isolation Rules to Ticket and Golden Docs

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `tickets/README.md`, `docs/golden-e2e-testing.md`, existing AGENTS.md ticket discipline

## Problem

S13POLEMEGOLSUI-002 exposed two recurring authoring failures that the current docs do not state explicitly enough:

1. a heuristic can look like an arbitrary blocker when it is actually standing in for missing architecture
2. a cross-system golden can look correct on paper while still allowing lawful competing affordances that obscure the intended invariant

Current ticket and golden-doc guidance already covers trace surfaces and precision, but it does not yet explicitly require:

1. identifying what missing substrate a heuristic is currently substituting for before removing it
2. documenting scenario isolation choices when a golden is intended to prove one specific causal chain

Without those rules, future tickets can mis-scope architecture work and future goldens can become noisy or misleading while still "passing."

## Assumption Reassessment (2026-03-18)

1. `tickets/README.md` already requires assumption reassessment, exact symbol naming, coverage-gap precision, and verification-layer mapping. It does **not** explicitly ask authors to identify when an existing heuristic is compensating for a missing architectural substrate.
2. `docs/golden-e2e-testing.md` already defines assertion hierarchy and trace guidance. It does **not** explicitly tell authors to document scenario-isolation choices when lawful competing affordances exist and the golden is intended to prove one branch.
3. `AGENTS.md` already tells contributors to reassess tickets against current code/tests and update the ticket first when assumptions diverge, but the repo docs that ticket authors are expected to follow still leave the two failure modes above implicit.
4. The gap exposed by S13POLEMEGOLSUI-002 was documentation/process, not missing runtime test infrastructure. The existing tests and traces were sufficient to discover the problem; the missing rule was how to scope and describe the work before implementation.

## Architecture Check

1. The clean solution is to strengthen the ticket and golden authoring rules, not to rely on tribal knowledge or a postmortem memory of one bad ticket.
2. This is architecturally important because heuristic removal without substrate identification tends to create hidden regressions, which is the opposite of robust/extensible architecture.
3. Scenario isolation guidance belongs in the golden-testing doc, not buried in one completed ticket, because it is a reusable testing design rule across domains.
4. No compatibility layer is needed. The docs should directly replace the weaker guidance with stronger requirements.

## Verification Layers

1. Ticket authoring rules explicitly mention heuristic-substrate analysis -> doc content check in `tickets/README.md`
2. Golden authoring rules explicitly mention scenario isolation and lawful competing affordances -> doc content check in `docs/golden-e2e-testing.md`
3. Updated guidance remains consistent with existing AGENTS.md expectations -> manual cross-doc review

## What to Change

### 1. Strengthen `tickets/README.md`

Add an explicit rule that when a ticket proposes removing, weakening, or bypassing an AI heuristic or filter, it must state:

1. what concrete architectural substrate that heuristic is currently standing in for
2. whether the ticket is replacing that substrate or merely removing the heuristic
3. why removal does not open regressions in unrelated scenarios

This makes heuristic-removal work prove architectural replacement rather than just behavioral preference.

### 2. Strengthen `docs/golden-e2e-testing.md`

Add explicit guidance that cross-system golden scenarios must document scenario-isolation choices when:

1. the intended invariant is one specific causal chain
2. the current architecture lawfully permits competing branches
3. local perception, ranking, or planner branching could obscure the intended proof

The rule should tell authors to remove unrelated lawful affordances when they are not part of the contract under test, and to say so explicitly in the ticket/spec.

### 3. Align terminology across both docs

Use one consistent vocabulary for:

1. heuristic
2. missing substrate
3. scenario isolation
4. lawful competing affordance
5. intended branch/invariant

That keeps future tickets from drifting into vague phrases like "the AI is blocked by this check" when the real issue is architectural incompleteness.

## Files to Touch

- `tickets/README.md` (modify)
- `docs/golden-e2e-testing.md` (modify)

## Out of Scope

- implementing E15c or any runtime social-memory code
- changing current golden scenarios
- adding new AI traces or debug fields
- rewriting AGENTS.md unless a contradiction is discovered

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace`

### Invariants

1. Ticket docs must require heuristic-removal proposals to identify the missing replacement substrate.
2. Golden docs must require scenario-isolation notes when lawful competing affordances exist.
3. The guidance must remain aligned with `docs/FOUNDATIONS.md` emphasis on explicit local causality and state-mediated behavior.

## Test Plan

### New/Modified Tests

1. `tickets/README.md` — add heuristic-substrate authoring rule. Rationale: prevents future tickets from treating architecture-carrying heuristics as disposable checks.
2. `docs/golden-e2e-testing.md` — add scenario-isolation authoring rule. Rationale: prevents cross-system goldens from becoming noisy or accidentally proving the wrong thing.

### Commands

1. `rg -n "heuristic|substrate|scenario isolation|lawful competing affordance" tickets/README.md docs/golden-e2e-testing.md`
2. `cargo test --workspace`
