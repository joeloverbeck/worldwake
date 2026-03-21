# RANKGUIDE-001: Tighten Ticket And Golden Guidance For Live Ranking Arithmetic And Branch Symmetry

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md), [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md), [specs/S17-wound-lifecycle-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S17-wound-lifecycle-golden-suites.md), [archive/tickets/completed/S17WOULIFGOLSUI-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S17WOULIFGOLSUI-002.md)

## Problem

The current authoring guidance is strong on mixed-layer precision, but it still left room for one specific ranking-authoring mistake that just happened in S17: a ticket/spec treated "equal utility weights" as if that implied equal motive scores between competing branches. Under the live architecture, equal weights can still produce asymmetric motive scores because the pressures differ.

That mistake propagated into:

- a ticket claim that the branches were symmetric except for priority promotion,
- a spec line saying equal weights ensured priority class, not motive score, determined order,
- a weaker-than-ideal initial proof shape.

This is a documentation and ticket-authoring gap. The architecture is fine; the contract for writing about ranking arithmetic needs to be sharper.

## Assumption Reassessment (2026-03-21)

1. [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) already requires ticket authors to state whether compared branches are symmetric and whether divergence depends on priority class or motive score. That is the correct direction, but it does not explicitly warn that equal weights do not imply equal motive scores when pressure differs.
2. [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already says not to claim "same-state, weight-only divergence" unless the compared branches are driven by comparable ranking substrates. That is also correct, but it still does not explicitly require validating the live arithmetic when a ticket claims a tie, neutrality, or "priority-class only" comparison.
3. [specs/S17-wound-lifecycle-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S17-wound-lifecycle-golden-suites.md) currently contains the stale Scenario 30 statement "Default `UtilityProfile` with equal weights — ensures priority class (not motive score) determines action order". That statement diverges from the implemented reality documented in [archive/tickets/completed/S17WOULIFGOLSUI-002.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S17WOULIFGOLSUI-002.md).
4. [tickets/S17WOULIFGOLSUI-003.md](/home/joeloverbeck/projects/worldwake/tickets/S17WOULIFGOLSUI-003.md) also still references a nonexistent `golden-e2e-coverage.md`, so there is already active ticket drift in this documentation area.
5. This is not an AI-runtime behavior ticket. The intended verification layer is documentation/ticket review plus doc-validation commands.
6. No ordering-sensitive behavior, heuristics, stale-request boundaries, political closure boundaries, or `ControlSource` semantics are under change.
7. Mismatch corrected: the missing substrate is not more engine code. It is explicit authoring guidance that forces ticket/spec writers to validate the live formula inputs before claiming symmetry or neutrality in ranking scenarios.

## Architecture Check

1. The clean solution is to tighten the written contract where the mistake occurred: ticket authoring guidance, golden testing guidance, and the stale S17 spec text. That is better than adding compensating code or relying on reviewers to re-derive ranking arithmetic ad hoc.
2. The docs should point authors toward concrete live arithmetic, not abstract narrative claims. That aligns with Principle 3 (concrete state over abstract scores) and Principle 27 (debuggability).
3. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Ticket authoring contract explicitly requires validating full live ranking arithmetic before claiming branch symmetry/ties -> manual doc review in [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md)
2. Golden testing guidance explicitly distinguishes equal weights from equal motive scores and points to live arithmetic validation -> manual doc review in [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md)
3. S17 wound-lifecycle spec no longer states the incorrect "equal weights => motive neutrality" claim -> manual doc review in [specs/S17-wound-lifecycle-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S17-wound-lifecycle-golden-suites.md)
4. Active S17 docs catch-up ticket no longer references nonexistent files or stale assumptions -> manual ticket review in [tickets/S17WOULIFGOLSUI-003.md](/home/joeloverbeck/projects/worldwake/tickets/S17WOULIFGOLSUI-003.md)
5. Single-layer note: this is a documentation/ticket-authoring ticket; no additional runtime verification layers apply

## What to Change

### 1. Tighten the ticket authoring contract

Update [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) so ranking-sensitive tickets must explicitly validate the live arithmetic when they claim:

- equal motive scores,
- branch symmetry,
- "priority-class only" divergence,
- or neutrality from equal utility weights.

The contract should state that authors must check the actual formula inputs: pressure, weight, caps/promotions, and any other live ranking substrate that can break symmetry.

### 2. Tighten golden testing guidance

Update [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) with a short explicit rule:

- equal weights do not imply equal motive scores,
- validate live arithmetic before writing a symmetry claim,
- if the compared branches are asymmetric in motive even with equal weights, say so directly and explain what stronger substrate actually drives the outcome.

### 3. Correct the stale S17 wound-lifecycle planning material

Update [specs/S17-wound-lifecycle-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S17-wound-lifecycle-golden-suites.md) so Scenario 30 no longer claims equal weights neutralize motive score. It should describe the implemented and architecturally stronger reality: recovery-aware class promotion overrides a stronger competing wash motive.

### 4. Correct the active S17 docs catch-up ticket

Update [tickets/S17WOULIFGOLSUI-003.md](/home/joeloverbeck/projects/worldwake/tickets/S17WOULIFGOLSUI-003.md) so its assumptions and file references match the current repository.

## Files to Touch

- `tickets/README.md` (modify)
- `docs/golden-e2e-testing.md` (modify)
- `specs/S17-wound-lifecycle-golden-suites.md` (modify)
- `tickets/S17WOULIFGOLSUI-003.md` (modify)

## Out of Scope

- Any production code changes
- Any new trace payloads or ranking behavior changes
- Rewriting historical archived tickets beyond what is needed for cross-reference accuracy
- Reformatting unrelated docs sections

## Acceptance Criteria

### Tests That Must Pass

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `cargo test --workspace` (sanity check because active ticket/spec references affect future work)

### Invariants

1. The authoring contract explicitly prevents future "equal weights => equal motive scores" mistakes
2. Golden guidance explicitly requires validating live ranking arithmetic before claiming symmetry or neutrality
3. The active S17 spec/ticket text matches the implemented Scenario 30 architecture
4. No production or test code changes are introduced

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `cargo test --workspace`
