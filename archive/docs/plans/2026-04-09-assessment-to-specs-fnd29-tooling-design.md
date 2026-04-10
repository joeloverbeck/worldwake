# Design: Assessment-to-Specs FND-29 Tooling Recognition

## Brainstorm Context

- **Original request**: The assessment-to-specs skill rejects tooling proposals as YAGNI because they don't introduce new simulation components. The user observed that tooling improvements contribute to architectural improvements downstream and wanted the skill to recognize this.
- **Reference file**: `.claude/skills/assessment-to-specs/SKILL.md`
- **Key interview insight**: FND-29 (Debuggability Is a Product Feature) is a FOUNDATIONS principle on equal footing with all others. Tooling that serves FND-29 IS creating meaningful downstream consequences — the skill's YAGNI filter had a blind spot for this.
- **Final confidence**: 95%
- **Approach chosen**: Extend YAGNI guardrail with FND-29 exception (Approach A — minimal, targeted)

## Overview

Two targeted additions to the assessment-to-specs skill:

1. **Step 5 (Classification)**: Add a note that tooling proposals serving FND-29 have valid downstream-consequence justification.
2. **YAGNI Guardrail**: Extend to explicitly recognize FND-29 as a source of meaningful downstream consequences.

No other steps, formats, or outputs change. Tooling proposals that survive triage become full specs (same format as architectural specs).

## Change 1 — Step 5 (Classify Each Proposal)

Add after the three classification definitions (Accept/Reject/Scope-Down), before Step 6:

> **Tooling and debuggability proposals**: Proposals that improve diagnostic capability (observer enhancements, trace enrichment, dump format improvements) should not be rejected as YAGNI solely because they don't introduce new simulation components or systems. FND-29 (Debuggability Is a Product Feature) makes diagnostic capability a first-class architectural concern. If a tooling proposal concretely improves the ability to diagnose an identified architectural gap or behavioral pathology, it has meaningful downstream consequences and should be classified as Accept. The proposal must still cite a specific diagnostic gap it addresses — "generally useful" is not sufficient.

## Change 2 — YAGNI Guardrail

Replace:

> **YAGNI**: Reject proposals that do not create meaningful downstream consequences (Principle 5). "It would be nice" or "it feels more complete" is not sufficient justification.

With:

> **YAGNI**: Reject proposals that do not create meaningful downstream consequences (Principle 5). "It would be nice" or "it feels more complete" is not sufficient justification. **Exception**: FND-29 (Debuggability) makes diagnostic capability a first-class concern. Tooling proposals that address a specific identified diagnostic gap (e.g., "the observer cannot answer 'why did this agent not eat?'") have meaningful downstream consequences and should not be rejected as YAGNI. The proposal must name the specific diagnostic question it enables — vague debuggability claims do not qualify.

## Edge Cases

1. **Vague debuggability claims**: "Better logging would be nice" fails. The proposal must name the specific diagnostic question the tooling enables.
2. **No identified gap**: If no behavioral pathology was identified that requires the tooling improvement, the FND-29 exception does not apply.
3. **Partially implemented tooling**: If underlying data exists but the dump format doesn't expose it, the proposal survives triage IF it cites a diagnostic gap. The spec describes the dump format change as its deliverable.
