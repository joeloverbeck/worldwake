# Report Format

Write to `reports/architectural-debt-<date>-<context>.md`:

```markdown
# Architectural Debt Analysis: <context>

**Date**: <YYYY-MM-DD>
**Input**: <test path>
**Source modules analyzed**: <count>
**Crates touched**: <list>
**Prior reports consulted**: <list or "none">

## Executive Summary

<2-4 sentences: were findings discovered? How severe? How many findings
vs acceptable clusters? Did cross-lens reinforcement elevate any findings?>

## Scenario Families

| Family | Tests | Domain Concepts | Key Assertions |
|--------|-------|----------------|----------------|
| <name> | <count> | <concepts> | <what they verify> |

## Traceability Summary

| Module | Scenario Families | Confidence | Strategy |
|--------|------------------|------------|----------|
| <file> | <families> | High/Med/Low | <use/naming/temporal/...> |

(Focus on uniquely relevant modules, not an exhaustive listing.)

## Findings

### F<N>: <Title>

**Lens Source**: Lens A / Lens B / Merged
**Kind**: Protocol | Authority boundary | Bounded context | Projection owner | Capability ledger | Workflow coordinator | Translation boundary | Lifecycle carrier
**Fracture Type** (if Lens B): <type from the 8 fracture types>
**Severity**: Critical / High / Medium / Low
**Confidence**: High / Medium / Low
**Scope**: <crates/modules spanned>

**Owned truth**: <what this abstraction would own>
**Invariants**: <what must always hold>
**Owner boundary**: <which crate/module should own this>

**Evidence**:
- <file:line> — <what was found>
- <file:line> — <what was found>

**Modules affected**: <list of modules absorbed or constrained>
**Scenario families explained**: <which scenario families>
**Expected simplification**: <what gets cleaner>

**FOUNDATIONS alignment**:
- P<N> (<short name>): aligned / strained / conflicts — <brief explanation>

**Counter-evidence**: <what would falsify this>

---

## Acceptable Architecture

<Areas analyzed that are complex but correctly architected.
Name them explicitly — "acceptable complexity" is a valid and important finding.
Brief explanation of why they don't need intervention.>

## Needs Investigation

| Signal | Type Suspected | One Signal Found | Second Signal to Look For |
|--------|---------------|-----------------|--------------------------|
| <description> | <type> | <what was found> | <what to check next> |

(Single-signal observations from either lens, and Lens A clusters without scenario grounding.)

## Proposals

For each finding with severity Critical or High, write a proposal. Number proposals sequentially (P1, P2, ...).

### P<N>: <Title>

**Claim**: <What is missing, incomplete, or misplaced — stated as a factual observation>
**Evidence**:
- <file:line> — <what was found>
- <file:line> — <what was found>
**FOUNDATIONS references**: P<N> (<name>), P<N> (<name>)
**Proposed change**: <What a spec should address>
**Priority**: Critical / High / Medium

## Codebase Health Observations (optional)

<Notable architectural strengths discovered during analysis — effective centralization patterns, clean crate boundaries, low workaround density. Highlights what is working well.>
```

If no findings are found, state this explicitly in the Executive Summary. An analysis that finds nothing wrong is a useful analysis. Report it with the same rigor as a problematic result.
