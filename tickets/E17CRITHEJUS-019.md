# E17CRITHEJUS-019: Tighten cross-layer ticketing rules for information-path refactors

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: E17CRITHEJUS-017, E17CRITHEJUS-018

## Problem

`E17CRITHEJUS-017` showed that the repository’s ticketing rules are close, but still leave room for a specific failure mode: an interrupted cross-layer refactor can proceed with two lawful-looking transport paths for the same fact, and the ticket can stay narratively correct while being architecturally under-specified.

In this case, institutional claims existed both as first-class `TellTopic::InstitutionalClaim { .. }` artifacts and as legacy sidecars hanging off `TellTopic::EntityBelief { subject: office_or_record }`. The existing ticket rules required reassessment, layer naming, and traceability escalation, but they did not explicitly force the document to state:

1. whether one fact currently has multiple transport paths
2. which path is canonical after the change
3. whether the trace surface itself must be upgraded because the new contract would otherwise be hard to debug

This is a documentation/process gap, not a foundations gap. `docs/FOUNDATIONS.md` already requires one concrete path for social artifacts and explainable information flow.

## Assumption Reassessment (2026-03-26)

1. `tickets/README.md` already requires assumption reassessment, exact shared-boundary naming, live goal/operator identification, ranking-sensitive checks, and follow-up traceability tickets when traces are insufficient.
2. `docs/precision-rules.md` already distinguishes layers, ordering contracts, coverage classes, heuristic removal, divergence protocol, and traceability escalation. The missing specificity is not “tickets ignore traces”; it is “tickets do not yet force explicit one-fact/one-transport-path statements for information-bearing artifacts.”
3. The exact document boundary under audit is ticket authoring for mixed-layer social-information refactors, especially changes that touch `TellTopic`, `SharedTellState`, memory lanes, social observations, records, institutional claims, or other information-carrying artifacts.
4. The intended invariant, consistent with `docs/FOUNDATIONS.md`, is that beliefs and social artifacts travel through explicit concrete carriers with one canonical architectural path after a refactor. Duplicate transport paths are sometimes a temporary implementation state, but the ticket must call them out explicitly and state whether this ticket removes them or leaves a separate cleanup ticket.
5. The motivating live goal family from `E17CRITHEJUS-017` was `GoalKind::ShareBelief { .. }`, but this ticket is documentation-only. No planner or runtime code changes are proposed here.
6. Existing repository guidance in the root [AGENTS.md](/home/joeloverbeck/projects/worldwake/AGENTS.md) already says mixed-layer tickets must name the exact shared abstraction boundary and that outcome traces may require a follow-up traceability ticket. That guidance should be harmonized with `tickets/README.md` and `docs/precision-rules.md` rather than left partially implicit.
7. No ordering contract change is proposed. This ticket clarifies authoring obligations for future tickets whose claims depend on ranking order, action lifecycle order, or authoritative mutation order.
8. No heuristic removal is proposed. The ticket strengthens documentation so future tickets must say when a heuristic/filter is papering over a missing substrate and whether the ticket actually installs that substrate.
9. This is not a stale-request or start-failure ticket. The gap is document precision for mixed-layer information-path refactors.
10. Foundations alignment is direct:
    - Principle 7: communication must have an explicit path
    - Principle 13: information path must be explainable
    - Principle 16: memories and records are world state
    - Principle 23: social artifacts are first-class
    - Principle 24: systems interact through state, not hidden coupling
11. Adjacent contradictions exposed during reassessment:
    - required consequence of this ticket: ticket docs should explicitly require canonical-path declarations and trace-surface reassessment for information-path changes
    - separate future cleanup: broader process changes outside ticket/spec drafting do not belong here
12. Mismatch + correction: this is not a request to change `docs/FOUNDATIONS.md`. Foundations already says the right thing. The needed change is in the ticketing/precision contract that operationalizes those principles during implementation planning.

## Architecture Check

1. Tightening the ticketing contract is cleaner than relying on contributor memory or post-hoc review comments. Mixed-layer social refactors should fail fast in the ticket if they leave duplicate transport paths or an underpowered explanation surface.
2. This is better than adding more generic process text. The new rules should target the exact architecture hazard exposed here: one fact flowing through more than one transport path after the refactor narrative claims a cleanup.
3. No backwards-compatibility aliasing or additional process branches are introduced. The ticket only sharpens the existing single-source authoring contract.

## Verification Layers

1. Ticket contract explicitly requires “one fact, one canonical transport path” declarations for information-bearing refactors -> `tickets/README.md`
2. Precision rules explicitly require canonical-path and trace-surface reassessment when information-path abstractions change -> `docs/precision-rules.md`
3. Root repository guidance remains aligned with the ticket contract for mixed-layer traceability escalation -> `AGENTS.md`
4. Single-layer documentation ticket; no additional runtime verification mapping is applicable

## What to Change

### 1. Update the ticket authoring contract

- Amend `tickets/README.md` so mixed-layer tickets involving beliefs, tells, records, institutional claims, rumors, or other information carriers must explicitly state:
  - whether multiple transport paths currently exist for the same fact
  - which path is canonical after the change
  - whether duplicate paths are removed in-scope or deferred to a named follow-up ticket

### 2. Extend precision rules for information-path refactors

- Amend `docs/precision-rules.md` with a focused rule for information-bearing abstraction changes:
  - require “one fact, one transport path” analysis
  - require explicit classification of temporary mixed-state coexistence versus intended end-state architecture
  - require trace-surface reassessment when the new canonical path would otherwise be harder to debug than the old one

### 3. Harmonize root agent guidance

- Update the repository `AGENTS.md` ticket expectations or debugging sections only as needed so they reference the same canonical-path and traceability-escalation obligations as `tickets/README.md` and `docs/precision-rules.md`.
- Keep the changes narrow; do not duplicate the full precision-rules document into multiple locations.

## Files to Touch

- `tickets/README.md` (modify)
- `docs/precision-rules.md` (modify)
- `AGENTS.md` (modify, only if needed for alignment)

## Out of Scope

- Any production code change in `worldwake-core`, `worldwake-sim`, `worldwake-systems`, or `worldwake-ai`
- Reopening the architectural decisions from `E17CRITHEJUS-017`
- General documentation cleanup unrelated to mixed-layer information-path refactors
- Changes to `docs/FOUNDATIONS.md`

## Acceptance Criteria

### Tests That Must Pass

1. `tickets/README.md` explicitly requires canonical-path declarations for mixed-layer information-path refactors
2. `docs/precision-rules.md` explicitly requires trace-surface reassessment when new information paths become architecturally canonical
3. Repository guidance remains consistent across the updated docs with no conflicting instructions

### Invariants

1. Future tickets for social-information or record/institution refactors must fail early if they leave two transport paths for the same fact without saying so explicitly
2. The documentation changes must reinforce Principles 7, 13, 16, 23, and 24 without redefining or weakening `docs/FOUNDATIONS.md`

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `rg -n "canonical|transport path|trace-surface|traceability escalation|mixed-layer" tickets/README.md docs/precision-rules.md AGENTS.md`
2. `git diff -- tickets/README.md docs/precision-rules.md AGENTS.md`
