# S18TICKETDOC-001: Tighten ticket and golden authoring rules for live planner contracts

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `docs/FOUNDATIONS.md`, `docs/golden-e2e-testing.md`, `docs/precision-rules.md`, `tickets/README.md`, `archive/tickets/completed/S18PREAWAEME-003.md`

## Problem

The main failure in `S18PREAWAEME-003` was not coding first. It was ticket drift: the ticket asserted a stale-source recovery story under the wrong goal family, overstated a later failure boundary, and described a broader scenario than the architecture actually needed. The current ticket/doc contract requires reassessment, but it does not yet force authors to validate the live goal/operator surface before drafting a planner scenario.

That gap allows tickets to be precise in wording while still being architecturally wrong. It also leaves unclear when a missing causal explanation belongs in a follow-up traceability ticket instead of ad-hoc debugging inside an implementation ticket.

## Assumption Reassessment (2026-03-21)

1. `tickets/README.md` already requires reassessment, exact symbol naming, verification-layer mapping, and explicit correction when ticket assumptions diverge from current code.
2. `docs/golden-e2e-testing.md` already requires earliest-causal-boundary assertions, scenario-isolation disclosure, and avoidance of using broad downstream effects as a proxy for earlier planner behavior.
3. `docs/precision-rules.md` already requires phase distinction, layer precision, stale-request boundary naming, and divergence-first correction.
4. The current doc set does not yet explicitly require planner/golden tickets to validate the live goal family and operator surface they rely on before describing the scenario. The `S18PREAWAEME-003` confusion came from assuming `ProduceCommodity` covered a harvest-backed stale-source recovery when the live operator surface that lawfully supported that branch was `RestockCommodity`.
5. The current doc set also does not explicitly require a follow-up traceability ticket when decision traces expose the selected outcome but not the concrete planner provenance needed to explain that outcome architecturally.
6. This is a documentation/process ticket. Additional verification-layer mapping is not applicable beyond ensuring the guidance cites the correct existing architectural layers and trace surfaces.
7. Mismatch + correction: current documentation is directionally correct but missing two specific authoring guardrails that would have prevented the stale ticket model from surviving reassessment.
8. No arithmetic or start-failure contract is being changed here.

## Architecture Check

1. Strengthening the ticket/doc contract is cleaner than relying on reviewers to catch planner-surface mismatches case by case. It pushes correctness earlier, before code or golden shape starts drifting.
2. The rule change aligns with `docs/FOUNDATIONS.md`: decisions must stay belief-local, causally legible, and grounded in actual live systems rather than narrative expectations.
3. No backwards-compatibility shims are involved. The docs should state the preferred contract directly and remove ambiguity.

## Verification Layers

1. ticket authors must validate the actual goal family/operator surface before specifying a planner scenario -> documentation contract in `tickets/README.md` and `tickets/_TEMPLATE.md`
2. golden docs must state that missing planner provenance should trigger a traceability follow-up ticket when it matters architecturally -> `docs/golden-e2e-testing.md` and `docs/precision-rules.md`
3. documentation stays internally consistent with the current planner/testing guidance -> docs inventory / repository verification commands
4. single-layer documentation ticket; no additional runtime-layer mapping is applicable

## What to Change

### 1. Add goal-family/operator-surface validation to the ticket contract

Update ticket authoring guidance so planner- or golden-driven tickets must explicitly:

- name the live goal kind under test
- validate the current operator surface or planner support the scenario depends on
- correct scope immediately if the live goal family differs from the original ticket narrative

This should live both in `tickets/README.md` and in `tickets/_TEMPLATE.md` so the check is visible at creation time.

### 2. Add a traceability-escalation rule

Update `docs/golden-e2e-testing.md` and `docs/precision-rules.md` to state that when decision traces reveal the selected outcome but not the concrete planner provenance needed to explain it, authors should:

- drop to focused lower-layer tests for immediate implementation work
- open a follow-up traceability ticket if that missing provenance is architecturally important
- avoid papering over the gap with ad-hoc debug output or weaker downstream assertions

### 3. Add a focused example from S18PREAWAEME-003

Capture the corrected lesson with a short concrete example referencing the archived ticket:

- wrong initial assumption: `ProduceCommodity` stale-source branch
- corrected live contract: `RestockCommodity` stale-belief fallback with fresh replanning after local perception

Keep the example concise and architectural, not historical.

## Files to Touch

- `tickets/README.md` (modify)
- `tickets/_TEMPLATE.md` (modify)
- `docs/golden-e2e-testing.md` (modify)
- `docs/precision-rules.md` (modify)

## Out of Scope

- planner code or trace-schema changes
- new runtime or golden tests beyond whatever existing docs inventory validation already requires
- rewriting the broader foundations document

## Acceptance Criteria

### Tests That Must Pass

1. ticket guidance explicitly requires validation of the live goal family/operator surface for planner/golden tickets
2. golden/precision docs explicitly require follow-up traceability tickets for architecturally important missing provenance
3. Existing suite: `python3 scripts/golden_inventory.py --write --check-docs`

### Invariants

1. documentation steers authors toward the earliest lawful planner contract instead of narrative scenario expectations
2. missing trace provenance is handled by explicit architectural follow-up, not ad-hoc debug exceptions

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `python3 scripts/golden_inventory.py --write --check-docs`
2. `scripts/verify.sh`
