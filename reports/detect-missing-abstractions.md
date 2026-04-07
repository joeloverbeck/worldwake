---
name: detect-missing-abstractions
description: Analyze engine code exercised by a test suite to find cross-cutting concepts with implicit state machines spread across many files — the signature of a missing or incomplete first-class abstraction.
---

# Detect Missing Abstractions

Analyze engine code exercised by a test suite to find implicit state machines and cross-cutting concepts that indicate a missing or incomplete first-class abstraction.

## Invocation

```
/detect-missing-abstractions <test-file-or-directory-path>
```

**Parameter**: Path to a test file or directory that exercises the engine area to analyze.

**Output**: Structured report at `reports/missing-abstractions-<date>-<context>.md`.

## Background

Missing or incomplete abstractions manifest as: a single semantic concept (e.g., "grant lifecycle") whose state transitions are scattered across many files with no unifying type, or with a type that lacks sufficient state — forcing callers to re-compute readiness/applicability from scratch. Symptoms:
- Fixing one file breaks another
- The same predicate appears in 5+ locations with slight variations
- Error handlers catch-and-recover from problems the kernel should prevent
- The simulator needs special handling for what should be a kernel concern
- A lifecycle type exists but callers still scatter readiness checks across many files

## Methodology

### Phase 1: TRACE

Starting from the test file(s), build a dependency graph of engine source modules:

1. Read the test file(s) and extract all `import` statements
2. For each imported engine module (`packages/engine/src/**`), read it and extract ITS imports
3. Continue 2-3 levels deep until reaching leaf modules or modules outside `packages/engine/src/`. Barrel/index re-export files count as zero depth — they add no information, only indirection.
4. Produce a list of all engine source files exercised by the test suite

**Tool usage**: Read test files, Grep for imports, Read imported modules.

### Phase 2: IDENTIFY

Within the exercised modules, find cross-cutting concept clusters:

1. Extract all exported function names, type names, and constant names from each module
2. Search for recurring concept-name fragments (e.g., `freeOperation`, `turnFlow`, `grant`) across exported symbols using Grep. Identify fragments that appear in 3+ files.
3. Group functions by shared concept fragments
4. Name each cluster by its dominant fragment (e.g., "freeOperation" cluster, "turnFlow" cluster)
5. Filter to clusters exceeding the file-count threshold: >10% of analyzed files, or 8+ files, whichever is larger. For small analyses (<50 modules), use 5+ files as the floor.

**Tool usage**: Grep for `export (const|function|type|interface)` across exercised files.

### Phase 3: MEASURE

For each concept cluster with 5+ files, compute:

| Metric | How to measure |
|--------|---------------|
| **File count** | Distinct files containing the concept |
| **Function count** | Exported functions matching the concept |
| **Workaround count** | `if` branches with comments containing "workaround", "hack", "safety net", "fallback", "broadened", or error catch blocks that swallow/recover |
| **Predicate broadening** | Conditions using `\|\|` that grew over time (check inline comments/annotations first, then git blame for multi-commit additions if comments are absent) |
| **Redundant checks** | Same semantic check (e.g., "is this grant ready?") computed in 2+ locations |
| **Simulator special cases** | Error handlers in `sim/` that compensate for kernel gaps. Zero `sim/` compensation handlers is a positive signal — the kernel/sim boundary is clean (FOUNDATIONS §5 satisfied). Report this finding either way. |

**Tool usage**: Grep for patterns, Read specific functions, Bash for git blame on key predicates.

### Phase 4: DIAGNOSE

For clusters exceeding thresholds (3+ workarounds AND meeting the file-count threshold from Phase 2), apply diagnostic questions. Workaround density (workarounds / files) is a stronger signal than raw file count — a cluster with 14 workarounds across 5 files is more likely a missing abstraction than one with 0 workarounds across 40 files.

1. **Implicit state machine?** Does the concept have identifiable phases (created → ready → active → consumed → removed) with no explicit lifecycle type? Check if transitions happen in different files.

2. **Incomplete abstraction?** Does a lifecycle type exist but lack derived/cached state that callers need, forcing them to re-compute readiness/applicability from scratch in multiple locations? This is distinct from a missing abstraction — the type exists but doesn't carry enough information.

3. **Redundant computation?** Is the same "readiness" or "eligibility" check computed from scratch in multiple locations instead of being stored as state?

4. **Boundary violation?** Does the simulator need special error handling for what should be a kernel concern? (FOUNDATIONS §5: One Rules Protocol)

5. **Architectural completeness?** Are there workarounds that address symptoms rather than root causes? (FOUNDATIONS §15)

6. **Determinism risk?** Could scattered state transitions produce different results depending on execution order? (FOUNDATIONS §8)

### Phase 5: REPORT

Write to `reports/missing-abstractions-<date>-<context>.md`:

```markdown
# Missing Abstraction Analysis: <context>

**Date**: <YYYY-MM-DD>
**Input**: <test path>
**Engine modules analyzed**: <count>

## Executive Summary

<1-3 sentences: were missing abstractions found? How severe?>

## Cluster Summary

| Cluster | Files | Workarounds | Verdict |
|---------|-------|-------------|---------|
| <name> | N | N | Missing / Incomplete / Acceptable / Needs investigation |

## Concept Clusters

### <Cluster Name> (Files: N, Functions: N, Workarounds: N)

**Modules**: <list of files>

**Key functions**: <list of exported functions>

**State machine phases** (if implicit):
```
phase1 → phase2 → phase3 → ...
  handled by: file1.ts    file2.ts    file3.ts
```

**Workarounds**:
- <file:line> — <description of workaround>

**FOUNDATIONS alignment**:
- <principle>: <violated/strained/satisfied>

**Diagnosis**: <Missing abstraction / Incomplete abstraction / Acceptable complexity / Needs investigation>

## Recommendations

- **Spec-worthy**: <cluster names that need a spec>
- **Acceptable**: <cluster names that are complex but correctly architected>
- **Needs investigation**: <clusters where more context is needed>
```

## Important Rules

- This skill is READ-ONLY. Do not modify any source files.
- Do not run tests. Static analysis only.
- Do not write specs. Only write the report. Spec authoring is a separate step.
- Focus on implicit state machines. Do not check for general code smells.
- Always check against `docs/FOUNDATIONS.md` — the architectural commandments.
- The report should be actionable: each finding either needs a spec or doesn't.
