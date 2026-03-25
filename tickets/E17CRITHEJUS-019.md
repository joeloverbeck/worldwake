# E17CRITHEJUS-019: Strengthen mixed-layer ticket and golden authoring for abstraction audits and traceability gaps

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: None

## Problem

Recent E17 work exposed a repeated documentation/process failure mode: a ticket could be formally “well structured” yet still under-specify the abstraction boundary that actually needed reassessment. That allowed stale narratives like “feature X is missing” to survive long enough to drive implementation churn, when the real issue was a broader shared-boundary contradiction. Existing docs also do not make the traceability-gap response explicit enough: when traces prove the outcome but not the provenance, contributors need a defined rule to open a follow-up traceability ticket instead of broadening assertions or falling back to ad-hoc debugging.

This is a documentation/process gap, but it matters architecturally because weak ticket framing encourages workaround changes that violate the foundations mandate against patches and fossilized abstractions.

## Assumption Reassessment (2026-03-26)

1. `tickets/README.md` already requires `Assumption Reassessment`, `Architecture Check`, `Verification Layers`, and exact runnable commands. It also links `docs/precision-rules.md`.
2. `docs/precision-rules.md` already includes strong guidance on phase distinction, layer precision, scenario isolation, and `Traceability Escalation`, but it does not explicitly require a mixed-layer ticket to name the shared abstraction boundary being audited before implementation begins.
3. `AGENTS.md` already instructs contributors to update stale tickets before implementation and to use decision/action traces first for AI and action debugging, but it does not currently spell out the “abstraction-boundary audit first” rule or the requirement to state the intended invariant before trusting a failing golden.
4. Existing ticket templates in `tickets/_TEMPLATE.md` have room for these requirements, but the template itself does not prompt for:
   - the exact shared boundary or data contract under audit
   - whether newly discovered adjacent contradictions are in-scope consequences, separate bugs, or follow-up tickets
   - what trace surface is expected to explain a failure before authoring new downstream assertions
5. This is a docs/process ticket, not an engine ticket. Reassessment does not show a direct production contradiction that this ticket itself must patch.
6. The changes align with `docs/FOUNDATIONS.md`: the project explicitly rejects workaround architecture and requires causally legible, traceable systems. Tightening the authoring contract is therefore a direct support measure for P1, P3, P7, P13, P16, P24, and the opening architectural mandate in `FOUNDATIONS.md`.
7. Coverage gap classification:
   - no engine/runtime gap is being claimed here
   - the gap is repository guidance and ticket/spec authoring discipline
   - verification is doc-content inspection, not runtime behavior
8. Mismatch: the current docs are not “missing structure”; they are missing a few specific prompts that would have surfaced the recent failure mode earlier. The clean scope is to tighten those prompts rather than rewrite the process wholesale.

## Architecture Check

1. The clean fix is to strengthen the repository’s authoring contract so mixed-layer changes must explicitly audit the shared abstraction boundary, classify adjacent contradictions, and state the trace surface expected to explain the behavior.
2. This is better than adding ad-hoc lessons to one archived ticket because the failure mode is procedural and likely to recur across future epics.
3. No backwards-compatibility aliasing applies here; the docs should be updated in place so there is one authoritative contract.

## Verification Layers

1. Mixed-layer abstraction audit requirement is stated in the canonical ticket authoring docs -> doc content inspection
2. Golden invariant-restatement and competing-branch isolation rule is stated in the canonical authoring docs -> doc content inspection
3. Traceability-gap escalation rule points contributors toward follow-up tickets instead of weaker downstream assertions -> doc content inspection
4. Additional runtime verification is not applicable because this ticket only changes repository guidance

## What to Change

### 1. Tighten `tickets/README.md`

Add explicit rules for mixed-layer tickets:
- identify the shared abstraction boundary or data contract under audit before implementation
- classify newly exposed issues as:
  - required consequences of the intended change
  - separate bugs uncovered during reassessment
  - future cleanup that must become its own ticket
- if traceability proves an outcome but not enough provenance, require a follow-up traceability ticket instead of broadening downstream assertions

### 2. Update `tickets/_TEMPLATE.md`

Add prompts that force authors to state:
- the exact abstraction boundary under reassessment
- the intended invariant of any golden scenario
- the lawful competing behaviors intentionally excluded from setup
- the trace surface expected to explain the behavior, and what to do if it proves insufficient

### 3. Update `AGENTS.md`

Add concise contributor guidance:
- mixed-layer implementation work begins with an abstraction-boundary audit, not code changes
- do not trust a failing golden until the invariant under test and the excluded lawful branches are restated explicitly
- when traces are one layer too coarse, prove the immediate behavior at the strongest available lower layer and open a follow-up traceability ticket if the missing provenance matters architecturally

## Files to Touch

- `tickets/README.md` (modify)
- `tickets/_TEMPLATE.md` (modify)
- `AGENTS.md` (modify)

## Out of Scope

- Any runtime/engine traceability implementation
- Rewriting existing tickets en masse
- Changing the archival workflow

## Acceptance Criteria

### Tests That Must Pass

1. `tickets/README.md` explicitly requires mixed-layer tickets to name the abstraction boundary under audit
2. `tickets/_TEMPLATE.md` prompts authors for invariant-restatement, competing-branch isolation, and traceability-gap handling
3. `AGENTS.md` explicitly states the abstraction-audit-first and traceability-escalation rules
4. No contradictory guidance remains between the updated docs

### Invariants

1. Repository guidance continues to steer contributors toward root-cause architectural fixes rather than local workaround patches, consistent with `docs/FOUNDATIONS.md`
2. Documentation remains single-source and non-duplicative where possible; updates strengthen the canonical contracts instead of forking them

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is not the contract here.`

### Commands

1. `sed -n '1,260p' tickets/README.md`
2. `sed -n '1,260p' tickets/_TEMPLATE.md`
3. `sed -n '1,260p' AGENTS.md`
4. `sed -n '1,260p' docs/precision-rules.md`
