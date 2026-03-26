# DOCTRAC-002: Mixed-layer ticket contract-audit guidance

**Status**: PENDING
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

1. [`tickets/README.md`](/home/joeloverbeck/projects/worldwake/tickets/README.md) is already strong, but it does not yet force a short explicit “shared contract audit” for mixed-layer tickets that touch both AI and authoritative runtime.
2. The E17 justice work exposed a repeatable pattern: the real contract spanned `GoalKind`, affordances, candidate generation, plan revalidation, authoritative validation, and trace proof surfaces. The original narrative was too narrow.
3. Shared abstraction boundary under audit: mixed-layer ticket authoring for changes that cross planner/belief/runtime/action/record boundaries.
4. Intended invariant: before implementation begins, a mixed-layer ticket should name the exact symbols that transport the relevant fact across layers, the first failure boundary, and the strongest proof surface for each invariant.
5. This is not a runtime code ticket. The live gap is documentation/authoring precision, not missing engine behavior.
6. The current repo already points in this direction:
   - `tickets/README.md` requires reassessment and shared-boundary naming
   - `docs/planner-contracts.md` names planner contracts
   - `docs/golden-e2e-testing.md` names assertion layers
   But the guidance is still spread across files and does not yet force a concise mixed-layer audit pattern at ticket-authoring time.
7. Adjacent contradiction classification:
   - in scope: tightening ticket-authoring guidance and explicit mixed-layer checklist language
   - out of scope: changing implementation workflow tooling or adding new automation
8. This aligns with Principle 28 directly. A ticket/spec should declare causal hooks, failure boundaries, and knowledge flow clearly enough that implementation cannot hide behind a vague scenario.

## Architecture Check

1. The cleaner architecture is to require each mixed-layer ticket to start from a named shared contract and failure boundary, rather than allowing implementation to discover the true contract accidentally.
2. This is cleaner than adding more ad hoc repo folklore because it centralizes the rule in the existing ticket contract.
3. No backward-compatible wording or optional alternate template path should be added. The authoring contract should be corrected in place.

## Verification Layers

1. Mixed-layer authoring guidance explicitly requires shared-contract audit, first failure boundary, and proof-surface mapping -> docs diff review in `tickets/README.md`
2. Planner/golden cross-references are explicit when a ticket touches those surfaces -> docs diff review in `tickets/README.md` and, if necessary, one linked sentence in `docs/planner-contracts.md`
3. Existing runtime/tests remain the implementation baseline; this ticket is documentation-only
4. Additional layer mapping is not applicable beyond documentation because no engine behavior changes

## What to Change

### 1. Tighten `tickets/README.md` for mixed-layer tickets

Add a required mini-checklist for tickets crossing AI and authoritative runtime:

- exact shared data contract under audit
- exact symbols at each layer
- first failure boundary
- lawful competing branches excluded from setup
- strongest proof surface per invariant

### 2. Add traceability-escalation guidance

Document that if reassessment shows traces prove the outcome but not enough provenance to explain the architecture, the ticket must:

- prove the immediate behavior at the strongest lower layer
- explicitly name the traceability gap
- spawn a dedicated follow-up traceability ticket when that gap matters to debugability

### 3. Cross-link planner/golden guidance

Add short links from `tickets/README.md` to:

- `docs/planner-contracts.md` for planner-root/snapshot/traceability tickets
- `docs/golden-e2e-testing.md` for ordering and proof-surface choice

## Files to Touch

- `tickets/README.md` (modify)
- `docs/planner-contracts.md` (modify only if a small cross-reference helps)

## Out of Scope

- Code changes
- Template redesign beyond the needed mixed-layer checklist clarifications
- Automated linting for ticket content

## Acceptance Criteria

### Tests That Must Pass

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.
2. Existing suite remains the behavioral baseline: `cargo test --workspace`

### Invariants

1. Mixed-layer tickets must name shared contracts and failure boundaries concretely enough to preserve explainable emergence and debugability
2. Ticket guidance must continue to prefer concrete world-state/data-contract language over vague scenario narratives

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace`
