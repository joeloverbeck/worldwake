# GOLTICDOC-001: Tighten Golden Ticket Reassessment and Runtime-Trace Granularity Guidance

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — documentation and ticket-contract updates only
**Deps**: `tickets/README.md`; `tickets/_TEMPLATE.md`; `docs/precision-rules.md`; `docs/golden-e2e-testing.md`; archived example [`archive/tickets/completed/S19INSRECCON-002.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S19INSRECCON-002.md)

## Problem

Recent golden-ticket reassessment exposed a recurring authoring gap: stale ticket narratives can drift from the live spec, helper surface, topology math, and runtime trace granularity even when the production architecture is correct. The current documentation tells authors to reassess assumptions, but it does not state several checks explicitly enough:

- verify ticket-to-spec scenario mapping
- verify whether claimed harness gaps already exist
- verify live topology and arithmetic when the scenario depends on travel or duration math
- distinguish focused planner/operator-family tests from runtime selected-plan traces, which may lawfully expose a more concrete multi-leg path

The result is avoidable ticket churn and mis-scoped “tests only” narratives. The clean fix is to strengthen the documentation contract, not to change production code.

## Assumption Reassessment (2026-03-22)

1. `tickets/README.md` already requires reassessment and divergence correction, but it does not explicitly require scenario-number/spec mapping checks or a helper-surface audit before implementation. This gap is visible from the S19 remote-record reassessment archived in [`archive/tickets/completed/S19INSRECCON-002.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S19INSRECCON-002.md).
2. `tickets/_TEMPLATE.md` already asks planner/golden tickets to name the live `GoalKind` and operator surface, but it does not explicitly ask whether the focused planner proof surface is more abstract than the live runtime trace surface.
3. `docs/precision-rules.md` already covers divergence protocol, cumulative arithmetic, and traceability escalation, but it does not yet spell out that route math and helper availability must be rechecked against the live code when a golden scenario depends on them.
4. `docs/golden-e2e-testing.md` already prefers decision traces for AI reasoning, but it does not explicitly warn authors that runtime `selected_plan` traces may expand a route into multiple travel legs even when a focused planner test summarizes the same contract as a single `Travel` step.
5. This is a documentation-only ticket. No authoritative engine contradiction was found in the underlying S19 work; the failure was stale planning material above the engine, so `Engine Changes: None` remains correct here.
6. Verification is single-layer documentation review. Additional mixed-layer mapping is not applicable because this ticket changes authoring guidance rather than runtime behavior.
7. Mismatch + correction: the repo already has the right production architecture; the missing substrate is explicit written guidance for how to reassess golden tickets against that architecture before implementation.

## Architecture Check

1. Documentation is the right layer for this fix. The production systems, planner, and golden harness behaved correctly; the problem was that stale ticket assumptions were not forced through a sufficiently explicit reassessment checklist. Tightening the docs keeps the architecture honest without adding test-only shims or compensating code.
2. No backward-compatibility aliasing or dual rules should be introduced. Update the canonical docs in place so there is one current reassessment contract.

## Verification Layers

1. Golden-ticket reassessment contract explicitly requires scenario/spec mapping, helper audit, and live route/arithmetic validation -> `tickets/README.md` and `tickets/_TEMPLATE.md`.
2. Precision rules explicitly require checking whether runtime selected-plan traces are more concrete than focused planner/operator-family tests -> `docs/precision-rules.md`.
3. Golden guidance explicitly explains selected-plan granularity and warns against asserting an abstract route shape when the runtime trace lawfully emits concrete travel legs -> `docs/golden-e2e-testing.md`.
4. Single-layer ticket: no runtime/action/event-log verification layer is applicable because no code changes are proposed.

## What to Change

### 1. Strengthen the ticket-authoring contract

Update [`tickets/README.md`](/home/joeloverbeck/projects/worldwake/tickets/README.md) and [`tickets/_TEMPLATE.md`](/home/joeloverbeck/projects/worldwake/tickets/_TEMPLATE.md) so golden/planner tickets must explicitly reassess:

- ticket-to-spec scenario numbering and naming
- whether claimed harness/helper gaps already exist
- live topology and duration arithmetic when scenario reachability depends on them
- whether focused coverage proves an abstract operator family while the runtime trace exposes a more concrete selected-plan shape

### 2. Tighten precision and golden-testing guidance

Update [`docs/precision-rules.md`](/home/joeloverbeck/projects/worldwake/docs/precision-rules.md) and [`docs/golden-e2e-testing.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) to clarify:

- route and duration math are part of reassessment, not optional narrative details
- selected-plan traces are the authoritative runtime surface for route shape
- focused planner tests may remain valid while runtime traces lawfully expose finer-grained travel steps
- tickets should be corrected first when this granularity differs, rather than weakening the runtime assertion or forcing production code to match an old narrative

## Files to Touch

- `tickets/README.md` (modify)
- `tickets/_TEMPLATE.md` (modify)
- `docs/precision-rules.md` (modify)
- `docs/golden-e2e-testing.md` (modify)

## Out of Scope

- Any production code or trace-sink changes
- Rewriting existing archived tickets beyond using them as examples
- Golden scenario implementations themselves

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`
2. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action_replays_deterministically`
3. `cargo test -p worldwake-ai`

### Invariants

1. The canonical reassessment contract must explicitly require live-code validation of scenario numbering, helper availability, and travel/duration math for golden tickets.
2. The docs must make a clean distinction between focused planner/operator-family proofs and runtime selected-plan trace granularity.
3. The guidance must continue to prefer correcting stale tickets/docs over introducing compensating code or weaker assertions.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `rg -n "scenario|helper|topology|selected-plan|granularity" tickets/README.md tickets/_TEMPLATE.md docs/precision-rules.md docs/golden-e2e-testing.md`
2. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`
3. `cargo test -p worldwake-ai`
