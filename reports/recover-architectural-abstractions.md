---
name: recover-architectural-abstractions
description: Use when a complex test suite exercises cross-subsystem code and you suspect higher-level architectural fractures — split protocols, authority leaks, boundary inversions — that detect-missing-abstractions cannot see because it works within single concepts.
---

# Recover Architectural Abstractions

Given a complex test suite, recover the as-built architecture of the exercised area and propose higher-order abstractions that name owned truth, invariants, interaction protocols, and owner boundaries. Works at the cross-subsystem level, complementing `detect-missing-abstractions` which works within single concepts.

## Invocation

```
/recover-architectural-abstractions <test-file-or-directory> [--prior-reports path1 path2 ...]
```

**Parameter**: Path to a test file or directory that exercises the engine area to analyze.

**Optional**: `--prior-reports` — paths to earlier `missing-abstractions` or `architectural-abstractions` reports. The skill builds on previous analysis rather than rediscovering known issues.

**Output**: Structured report at `reports/architectural-abstractions-<date>-<context>.md`. `<context>` is derived from the input: for a test file, strip the path prefix and `.test.ts`/`.test.js` suffix; for a directory, use the directory name.

## Background

`detect-missing-abstractions` finds scattered state machines within a single concept (e.g., "grant lifecycle spread across 15 files"). This skill operates one level higher: it finds architectural fractures that span multiple subsystems — where the boundary between subsystems is wrong, where authority over shared truth is split, or where the same concept lives under different names in neighboring modules.

These fractures manifest as: fixing a bug in subsystem A requires compensating changes in subsystem B; the same eligibility/readiness predicate is computed from scratch in multiple subsystems; error handlers in one layer catch problems that another layer should prevent; files across nominal module boundaries repeatedly change together.

## Methodology

### Phase 1: GATHER

Starting from the test file(s), build a dependency graph of engine source modules:

1. Read the test file(s) and extract all `import` statements
2. For each imported engine module (`packages/engine/src/**`), read it and extract ITS imports
3. Continue 2-3 levels deep until reaching leaf modules or modules outside `packages/engine/src/`. Barrel/index re-export files count as zero depth.
4. Produce a list of all engine source files exercised by the test suite
5. Read `docs/FOUNDATIONS.md` — hold it for Phase 6 validation. Do NOT apply it yet.
6. Read any `prior_reports` if provided — note already-identified issues to avoid rediscovery.
7. Check for existing coverage/trace artifacts in the repo. Use them if present.
8. Run bounded git history: `git log --since="6 months ago" --name-only` on exercised files to identify temporal coupling (files that frequently change together across commits).

**Sub-agent delegation**: For large test suites (>20 direct imports or barrel re-exports), delegate import tracing to 1-3 parallel Explore sub-agents. Each agent traces a subset of the import tree. Merge their deduplicated file lists. Also delegate git history analysis to a separate sub-agent if the file list exceeds 30 modules.

### Phase 2: SCENARIO MAP

Treat tests as behavioral scenarios, not just import sources.

For each test or test family (a `describe` block or test file), recover:

- **What behavior** is being exercised (e.g., "free action continuation after grant expires")
- **Which fixture/setup path** it uses (e.g., `makeIsolatedInitialState` with specific overrides)
- **Which assertions** define success/failure (e.g., "state has no remaining grants")
- **Which domain concepts** appear in names, helpers, and expected values (e.g., "grant", "window", "turn flow")

Then cluster tests into **scenario families** — named behavioral groups. Example shapes:

- "free action continuation"
- "grant lifecycle management"
- "turn interruption and resumption"
- "ownership transfer"
- "decision resolution and override"
- "capability cost enforcement"

Every later architectural inference must be tied back to scenario families. A finding not grounded in test behavior is speculation.

**Sub-agent delegation**: For large test directories (>30 test files), delegate scenario extraction to 2-3 parallel Explore sub-agents, each handling a subset. Merge and deduplicate scenario families.

### Phase 3: TRACE

Build test-to-code traceability using multiple strategies — no single trick catches everything:

| Strategy | What it finds | Confidence |
|----------|--------------|------------|
| Import statements | Direct dependencies | High |
| Static call graph (from assertions back to production) | Functions actually exercised | High |
| Naming/lexical similarity (test helpers vs production functions) | Conceptual links | Medium |
| Temporal coupling from git history (files that co-change) | Hidden dependencies | Medium |

Each traceability link gets a confidence tag (high/medium/low) and a brief reason code.

The purpose of multi-strategy tracing is to catch hidden dependencies that imports alone miss — registry/dispatch patterns, builder indirection, and temporal coupling are the most common sources of invisible links in this codebase.

### Phase 4: DETECT FRACTURES

Scan the exercised code for these 8 fracture types:

| # | Fracture Type | What to look for |
|---|--------------|-----------------|
| 1 | **Split protocol** | The legal sequence of interactions is spread across multiple modules/layers. Module A decides "what", module B decides "when", module C decides "whether". |
| 2 | **Authority leak** | Multiple modules write the same truth. Two or more places create/mutate/invalidate the same piece of state. |
| 3 | **Projection drift** | Derived summaries or cached computations are recomputed everywhere. No single module owns the projection. |
| 4 | **Boundary inversion** | Higher layers own rules that belong in lower layers. The simulator enforces what the kernel should prevent. |
| 5 | **Concept aliasing** | The same domain concept exists under different names/types in neighboring subsystems (e.g., "grant" in one module, "capability window" in another, same semantic role). |
| 6 | **Hidden seam** | Files across nominal module boundaries repeatedly change together in git history, suggesting they belong in the same module. |
| 7 | **Overloaded abstraction** | One type/module carries several lifecycle roles that should be separated. A type that is "created, configured, activated, consumed, and cleaned up" but the type doesn't model these phases. |
| 8 | **Orphan compatibility layer** | A shim, fallback path, or "safety net" handler exists only to mask a deeper missing abstraction. |

**Evidence rule**: A fracture is NOT reported unless supported by at least two independent signals (e.g., import analysis + temporal coupling, or naming similarity + assertion patterns). Single-signal fractures go in a "Needs investigation" bucket, not in the main findings.

**Tool usage**: Grep for shared type names across modules, Grep for duplicate predicate patterns, Read key functions at boundary points, Bash for git log co-change analysis.

### Phase 5: SYNTHESIZE

For each validated fracture (two+ signals), produce a candidate abstraction:

- **title**: Descriptive name (e.g., "Turn Continuation Protocol")
- **kind**: One of: Protocol | Authority boundary | Bounded context | Projection owner | Capability ledger | Workflow coordinator | Translation boundary | Lifecycle carrier
- **scope**: Which subsystems/modules it spans
- **owned_truth**: What state or invariant this abstraction would own (the single most important field — if you can't name this, the candidate is not ready)
- **invariants**: What must always be true when this abstraction is correctly implemented
- **owner_boundary**: Which module/package should own it
- **modules_affected**: Existing modules that would be absorbed, constrained, or simplified
- **tests_explained**: Which scenario families from Phase 2 this candidate accounts for
- **expected_simplification**: What gets simpler — fewer writers, fewer repeated predicates, fewer cross-boundary transitions, fewer co-change edges, clearer ownership
- **confidence**: High / Medium / Low
- **counter_evidence**: What would falsify this hypothesis. Every candidate MUST have this field populated.

### Phase 6: VALIDATE

Apply two validation filters, in this order:

**Filter 1 — Survival criteria.** Drop any candidate that fails ANY of these:

1. It explains at least two tests or one whole scenario family
2. It reduces at least one real architectural cost (not just "cleaner")
3. It can name the owned truth
4. It can name the rightful owner boundary
5. It does not merely wrap existing code with a facade

**Filter 2 — FOUNDATIONS alignment.** For surviving candidates only, check against `docs/FOUNDATIONS.md`. For each relevant foundation principle, note whether the candidate aligns, strains, or conflicts. Flag conflicts prominently — a candidate that violates FOUNDATIONS needs redesign before it becomes a spec.

This ordering matters. Recovery first, judgement second. Do not let FOUNDATIONS bias the fracture detection — detect what IS, then evaluate what SHOULD BE.

## Report Format

Write to `reports/architectural-abstractions-<date>-<context>.md`:

```markdown
# Architectural Abstraction Recovery: <context>

**Date**: <YYYY-MM-DD>
**Input**: <test path>
**Engine modules analyzed**: <count>
**Prior reports consulted**: <list or "none">

## Executive Summary

<2-4 sentences: were cross-subsystem fractures found? How severe?
How many candidates survived validation?>

## Scenario Families

| Family | Tests | Domain Concepts | Key Assertions |
|--------|-------|----------------|----------------|
| <name> | <count> | <concepts> | <what they verify> |

## Traceability Summary

| Module | Scenario Families | Confidence | Strategy |
|--------|------------------|------------|----------|
| <file> | <families> | High/Med/Low | <import/naming/temporal/...> |

## Fracture Summary

| # | Fracture Type | Location | Evidence Sources | Severity |
|---|--------------|----------|-----------------|----------|
| 1 | <type> | <modules involved> | <which signals> | HIGH/MEDIUM/LOW |

## Candidate Abstractions

### <Candidate Title>

**Kind**: <Protocol / Authority boundary / ...>
**Scope**: <subsystems spanned>
**Fractures addressed**: <which fracture(s) from the summary table>

**Owned truth**: <what this abstraction would own>
**Invariants**: <what must always hold>
**Owner boundary**: <which module/package should own this>

**Modules affected**: <list of modules absorbed or constrained>
**Tests explained**: <which scenario families>
**Expected simplification**: <what gets cleaner>

**FOUNDATIONS alignment**:
- <Foundation #N>: <aligned / strained / conflicts> — <brief explanation>

**Confidence**: High / Medium / Low
**Counter-evidence**: <what would falsify this>

## Acceptable Architecture

<Areas analyzed that are complex but correctly architected.
Name them explicitly — "acceptable complexity" is a valid and important finding.
Brief explanation of why they don't need intervention.>

## Needs Investigation

<Single-signal fractures that didn't meet the two-signal minimum.
List them with the one signal found and what second signal to look for.>

## Recommendations

- **Spec-worthy**: <candidate names that warrant a spec>
- **Acceptable**: <areas that are fine as-is>
- **Needs investigation**: <areas where more context is needed>
```

## Hard Rules

1. **No pattern theater.** Never recommend a pattern name unless it corresponds to owned truth and a real boundary. "Strategy pattern" or "Observer pattern" without naming what truth is owned is not a finding.
2. **No abstraction without authority.** If the proposal cannot say who owns the truth, it is not ready. Move it to "Needs investigation."
3. **No wrapper-only recommendations.** "Create a helper/service/interface" is not sufficient unless it relocates invariant ownership. The question is always: what truth moves, and who gains authority?
4. **Read-only.** Do not modify any source files. Do not run tests. Static analysis and git history only.
5. **Do not invent problems.** "Acceptable complexity" must remain a valid and prominent outcome. If no fractures are found, say so clearly. An analysis that finds nothing wrong is a useful analysis.
6. **Every finding needs counter-evidence.** The report must say what would falsify each hypothesis. A finding without counter-evidence is an assertion, not an analysis.
7. **Recovery first, judgement second.** Build the scenario map and detect fractures BEFORE applying FOUNDATIONS principles. Do not let architectural ideals bias what you observe.
8. **Two-signal minimum.** No fracture is reported in the main findings unless supported by at least two independent evidence sources. Single-signal observations go in "Needs investigation."

## Important Rules

- This skill is READ-ONLY. Do not modify any source files.
- Do not run tests. Static analysis and git history only.
- Do not write specs. Only write the report. Spec authoring is a separate step.
- Focus on cross-subsystem fractures. Single-concept scatter (e.g., "this function is duplicated in 5 files") is the domain of `detect-missing-abstractions`, not this skill.
- Always check against `docs/FOUNDATIONS.md` — but only in Phase 6, not earlier.
- The report should be actionable: each finding either needs a spec or doesn't.
- If a report already exists at the target path, overwrite it — each run produces a complete standalone report.
- If prior reports are provided, acknowledge already-known issues and focus analysis on NEW findings. Do not re-report what was already found.
