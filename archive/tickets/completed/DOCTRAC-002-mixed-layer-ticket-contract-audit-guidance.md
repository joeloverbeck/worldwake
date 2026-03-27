# DOCTRAC-002: Mixed-layer ticket contract-audit guidance

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md), [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md), [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md), [archive/tickets/E17CRITHEJUS-013.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E17CRITHEJUS-013.md)

## Problem

Mixed-layer tickets still too easily start from a scenario narrative instead of an explicit shared contract audit. That produces avoidable failures:

- the first visible contradiction is fixed while deeper contract splits remain
- planner/runtime mismatches are described as “test gaps”
- tickets name outcomes without naming the exact symbols that carry the fact across layers

That weakens architectural rigor and violates `FOUNDATIONS.md` Principles 3, 12, 24, 25, 27, and 28. In Worldwake, the ticket itself is part of the architecture-control surface: if the ticket collapses distinct boundaries into one vague story, implementation quality follows it downward.

## Assumption Reassessment (2026-03-27)

1. [`tickets/README.md`](/home/joeloverbeck/projects/worldwake/tickets/README.md) already requires the core mixed-layer contract this ticket originally claimed was missing: exact shared-boundary naming, intended-invariant restatement, verification-layer mapping, adjacent-contradiction classification, and traceability escalation.
2. [`tickets/_TEMPLATE.md`](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) already prompts for the same mixed-layer audit structure, including the exact shared boundary, first failure boundary, intended invariant, and strongest proof surfaces.
3. The earlier docs/process tickets [`archive/tickets/DOCTRAC-001-layered-verification-and-inventory-grounding.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/DOCTRAC-001-layered-verification-and-inventory-grounding.md) and [`archive/tickets/completed/E17CRITHEJUS-019.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/E17CRITHEJUS-019.md) already delivered most of this architecture. Repeating their checklist language would create duplicate guidance rather than a cleaner contract.
4. Shared abstraction boundary under audit: the canonical authoring entry point in `tickets/README.md` for mixed-layer tickets that cross planner, belief, runtime, action, record, or golden-proof boundaries.
5. Intended invariant: a contributor starting from `tickets/README.md` should be routed to the authoritative follow-on docs for planner contracts and golden proof-surface selection without reconstructing those rules from archived tickets.
6. This remains a docs-only ticket. Reassessment found no runtime contradiction and no missing engine behavior.
7. The real remaining gap is narrower than the original ticket described: `tickets/README.md` links `docs/planner-contracts.md` and generated golden inventories indirectly through its surrounding contract, but it does not explicitly point mixed-layer ticket authors to [`docs/golden-e2e-testing.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) for ordering and proof-surface choice when a golden scenario is part of the boundary under audit.
8. Adjacent contradiction classification:
   - in scope: correcting this ticket's stale assumptions and adding one canonical cross-reference in `tickets/README.md`
   - out of scope: reworking the template, duplicating rules already present in `AGENTS.md` / `docs/precision-rules.md`, or adding automation
9. Mismatch + correction: the original ticket overstated missing authoring structure. The corrected scope is not "add a new mixed-layer checklist"; it is "avoid duplicate checklist churn, add the one missing golden-guidance cross-reference, and archive the ticket with the corrected narrower outcome."

## Architecture Check

1. The cleanest architecture is to keep `tickets/README.md` as the canonical entry point and route outward to the authoritative deep-dive docs only where they add information that should not be duplicated.
2. That is better than restating a second mixed-layer checklist that already exists in `tickets/README.md`, `tickets/_TEMPLATE.md`, `docs/precision-rules.md`, and `AGENTS.md`. More duplicated wording would make the contract harder to maintain, not clearer.
3. No backward-compatible wording or optional alternate template path should be added. The existing contract should be tightened in place with one discoverability fix.

## Verification Layers

1. Canonical ticket entry point now routes mixed-layer authors to the right downstream golden-proof guidance -> docs diff review in `tickets/README.md`
2. Existing planner cross-reference remains intact and non-duplicative -> docs diff review in `tickets/README.md`
3. Existing runtime/tests remain the regression baseline; this ticket is documentation-only -> workspace test/lint commands
4. Additional layer mapping is not applicable beyond documentation because no engine behavior changes

## What to Change

### 1. Correct `DOCTRAC-002` scope

Update this ticket so it reflects the live repository state:

- note that the core mixed-layer checklist already exists
- cite the archived tickets that already delivered it
- narrow the implementation to the remaining missing cross-reference instead of re-adding checklist language

### 2. Add the missing golden-guidance cross-reference

Add a short canonical pointer from `tickets/README.md` to `docs/golden-e2e-testing.md` so mixed-layer ticket authors know where ordering and golden proof-surface choice live, without duplicating that document's content.

## Files to Touch

- `tickets/DOCTRAC-002-mixed-layer-ticket-contract-audit-guidance.md` (modify)
- `tickets/README.md` (modify)

## Out of Scope

- Code changes
- Template redesign
- Automated linting for ticket content
- Repeating checklist language already delivered by `DOCTRAC-001` and `E17CRITHEJUS-019`

## Acceptance Criteria

### Tests That Must Pass

1. `tickets/README.md` explicitly points mixed-layer golden-ticket authors to `docs/golden-e2e-testing.md` for ordering and proof-surface choice.
2. `tickets/README.md` continues to keep mixed-layer checklist rules centralized instead of duplicating downstream docs.
3. Existing suite remains the regression baseline: `cargo test --workspace`
4. `cargo clippy --workspace`

### Invariants

1. Mixed-layer tickets must still name shared contracts and failure boundaries concretely enough to preserve explainable emergence and debugability.
2. Ticket guidance must prefer a single canonical contract plus targeted cross-references over duplicated folklore.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `sed -n '1,260p' tickets/README.md`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-27
- What actually changed:
  - corrected `DOCTRAC-002` to match the live repository state after `DOCTRAC-001` and `E17CRITHEJUS-019`
  - added one canonical cross-reference in `tickets/README.md` pointing mixed-layer golden-ticket authors to `docs/golden-e2e-testing.md`
- Deviations from original plan:
  - the original ticket proposed a broader mixed-layer checklist/documentation tightening, but reassessment showed that architecture was already delivered
  - the implemented scope was intentionally narrower to avoid duplicating canonical rules across `tickets/README.md`, `tickets/_TEMPLATE.md`, `docs/precision-rules.md`, and `AGENTS.md`
- Verification results:
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
