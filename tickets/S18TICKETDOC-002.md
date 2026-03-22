# S18TICKETDOC-002: Document live-architecture reassessment and planner-boundary debugging workflow

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — documentation/process ticket
**Deps**: `docs/FOUNDATIONS.md`, `tickets/README.md`, `docs/golden-e2e-testing.md`, `archive/tickets/completed/E16BFORLEGJURCON-009.md`

## Problem

The recent force-legitimacy work lost time to stale ticket assumptions and a planner/runtime boundary bug that was only diagnosable by dropping below existing traces. The repository already has strong ticket and trace guidance, but it does not yet explicitly tell authors and implementers how to handle live-architecture divergence plus planner-boundary debugging when decision traces stop short of the needed provenance.

## Assumption Reassessment (2026-03-22)

1. Current ticket authoring rules in `tickets/README.md` and `docs/precision-rules.md` already require reassessment and trace preference, but they do not explicitly call out the live-vs-planning-snapshot boundary as a common divergence source.
2. `docs/golden-e2e-testing.md` already guides decision traces and action traces, but it does not yet describe what to do when the decision trace proves the outcome but omits the planner provenance needed to explain missing search candidates.
3. The motivating ticket is `archive/tickets/completed/E16BFORLEGJURCON-009.md`, where current code and the ticket narrative diverged materially, and the missing explanation sat at the `PerAgentBeliefView` -> `PlanningSnapshot` -> `PlanningState` -> `search_plan()` boundary.
4. This is a documentation/process ticket; no runtime harness choice is involved.
5. No ordering contract is central here. The contract is authoring/debugging discipline: update stale tickets first, then use the earliest causal boundary and open a follow-up traceability ticket when traces do not expose enough provenance.
6. No heuristic removal is proposed.
7. The first failure boundary named in the motivating regression was planner search-root provenance, not authoritative action start or AI recovery. The docs should teach contributors to name that explicitly.
8. Political-office precision still matters here because the motivating case was `GoalKind::ClaimOffice`, but the guidance should remain generic across planner families.
9. No `ControlSource` runtime behavior is in scope.
10. Scenario isolation is not relevant; this is a documentation ticket.
11. Mismatch corrected: the repo already has solid generic trace guidance, so this ticket should refine and connect existing docs rather than invent a new parallel workflow.
12. No cumulative arithmetic is relevant.

## Architecture Check

1. The clean approach is to tighten existing shared docs instead of creating ad-hoc local notes in individual tickets. That keeps the debugging/authoring contract centralized and reusable.
2. The new guidance should explicitly align with Principles 1, 7, 10, 13, 24, and 25: explainable emergence, locality of knowledge, belief-only planning, provenance-bearing knowledge, state-mediated system interaction, and caches not replacing truth.
3. No backwards-compatibility aliasing or duplicate ticket workflow should be introduced.

## Verification Layers

1. Ticket authoring requirements reflect live-architecture reassessment discipline -> doc review against `tickets/README.md`
2. Golden/AI debugging guidance explains planner-boundary escalation when decision traces are insufficient -> doc review against `docs/golden-e2e-testing.md`
3. No additional lower-layer verification mapping is applicable because this is a documentation/process ticket
4. The ticket should name why action trace or event-log guidance is not being expanded unless those layers are the actual debugging contract
5. Single-layer doc ticket; additional architectural proof surfaces are not applicable

## What to Change

### 1. Tighten ticket authoring guidance around live architecture divergence

Update `tickets/README.md` so authors explicitly:

- compare the current ticket narrative against live code before implementation
- name when a bug actually exposes a planner/runtime boundary contradiction rather than a "tests only" gap
- document when a ticket started as `Engine Changes: None` but reassessment discovered a production contradiction

### 2. Add planner-boundary debugging guidance to golden/AI trace docs

Update `docs/golden-e2e-testing.md` to describe a clean escalation path when decision traces stop short:

- use the earliest causal boundary available
- compare live belief-view affordances against planner-snapshot affordances when relevant
- open a dedicated traceability ticket if the missing provenance matters architecturally

### 3. Add one short worked example reference

Reference the force-law `ClaimOffice` / `PressForceClaim` regression as an example of:

- stale ticket assumptions
- live-vs-snapshot divergence
- correct follow-up traceability ticketing

Keep the example short and principle-driven, not a long postmortem.

## Files to Touch

- `tickets/README.md` (modify)
- `docs/golden-e2e-testing.md` (modify)
- `docs/precision-rules.md` (modify only if a small clarifying addition is truly needed)

## Out of Scope

- New runtime trace features
- New ticket templates or a parallel workflow document
- Retrospective edits to already-archived tickets beyond cited examples

## Acceptance Criteria

### Tests That Must Pass

1. Documentation names live-architecture reassessment as mandatory before implementation when ticket assumptions and code may diverge
2. Documentation names planner-boundary escalation when decision traces prove outcome but not missing candidate provenance
3. Existing suite: `python3 scripts/golden_inventory.py --write --check-docs`

### Invariants

1. Shared docs must reinforce a single debugging/authoring workflow, not create a second competing process
2. Guidance must point contributors to earliest causal boundaries and explicit follow-up traceability tickets instead of ad-hoc instrumentation

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `cargo test -p worldwake-ai -- --list`
