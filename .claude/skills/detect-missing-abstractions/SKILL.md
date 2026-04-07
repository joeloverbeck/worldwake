---
name: detect-missing-abstractions
description: "Analyze engine code exercised by a test suite to find cross-cutting concepts with implicit state machines or scattered logic across many files — the signature of a missing or incomplete first-class abstraction. Outputs a report compatible with /assessment-to-specs."
user-invocable: true
arguments:
  - name: test_path
    description: "Path to a test file or test directory (e.g., crates/worldwake-ai/tests/golden_trade_acquisition.rs or crates/worldwake-systems/tests/)"
    required: true
---

# Detect Missing Abstractions

Analyze engine code exercised by a test suite to find implicit state machines, scattered logic, and cross-cutting concepts that indicate a missing or incomplete first-class abstraction.

## Invocation

```
/detect-missing-abstractions <test-file-or-directory-path>
```

**Parameter**: Path to a test file or directory that exercises the engine area to analyze.

**Output**: Structured report at `reports/missing-abstractions-<date>-<context>.md`, formatted for consumption by `/assessment-to-specs`.

## Background

Missing or incomplete abstractions manifest as: a single semantic concept (e.g., "patrol lifecycle", "trade readiness") whose state transitions or readiness checks are scattered across many files with no unifying type, or with a type that lacks sufficient derived state — forcing callers to re-compute readiness/applicability from scratch. In a Rust/ECS codebase, symptoms include:

- The same enum is matched in 5+ files with similar-but-not-identical match logic
- The same combination of component checks (`has::<T>()`, `get::<T>()`) appears in multiple locations
- A single concept requires touching 3+ crates to implement correctly
- Functions in different modules compute the same derived value from the same inputs
- A lifecycle type exists but callers still scatter readiness checks across many files
- Fixing one file breaks another because they share implicit assumptions about state

## Methodology

### Phase 1: TRACE — Build the Exercised Module Set

Starting from the test file(s), build a list of source modules that the tests exercise:

1. If the input is a directory, collect all `.rs` files in it (excluding `mod.rs` files that only contain `mod` declarations). If a single file, use that file.
2. Read the test file(s) and extract all `use` statements to identify which crates are referenced (e.g., `worldwake_core`, `worldwake_sim`, `worldwake_systems`, `worldwake_ai`).
3. Extract all type names, function names, struct names, and enum variant names actually used in the test code body (not just imported). Focus on symbols from the `worldwake_*` crates.
4. For each referenced crate, grep `crates/<crate-name>/src/` for the definitions of those symbols (`pub fn <name>`, `pub struct <name>`, `pub enum <name>`, `pub trait <name>`) to identify which source modules are exercised.
5. For each exercised source module, read its internal `use` and `mod` statements to add 1 level of internal dependencies to the exercised set.
6. Produce a deduplicated list of all source modules exercised by the test suite.

**Important**: Rust crate `lib.rs` files re-export most public items. Do not count `lib.rs` as an exercised module — it is a barrel file. Trace through to the actual defining module.

**Tool usage**: Read test files, Grep for `use worldwake_`, Grep for `pub (fn|struct|enum|trait)` definitions in crate source directories.

### Phase 2: IDENTIFY — Find Concept Clusters

Within the exercised modules, find cross-cutting concept clusters:

1. Extract all `pub fn`, `pub struct`, `pub enum`, `pub type`, `pub const`, and `pub trait` names from each exercised module.
2. Tokenize each symbol name by splitting on `_` (snake_case) and CamelCase boundaries.
3. Identify recurring name-fragment tokens that appear in exported symbols across 3+ files (e.g., `patrol`, `trade`, `belief`, `combat`, `need`, `affordance`, `goal`, `plan`).
4. Group symbols by shared concept fragments.
5. Name each cluster by its dominant fragment (e.g., "patrol" cluster, "trade" cluster).
6. Filter to clusters exceeding the file-count threshold: >10% of analyzed files, or 5+ files, whichever is larger. For small analyses (<30 modules), use 5+ files as the floor.

**Tool usage**: Grep for `pub (fn|struct|enum|type|const|trait)` across exercised files.

### Phase 3: MEASURE — Quantify Structural Signals

For each concept cluster meeting the file-count threshold, compute the following metrics. These are the primary detection signals — not comment scanning.

| Metric | How to measure | Signal strength |
|--------|---------------|-----------------|
| **File count** | Distinct source files containing the concept | Baseline — spread indicates cross-cutting concern |
| **Scattered match arms** | Grep for `match` expressions on the cluster's key enums; count files with similar-but-not-identical match logic on the same enum | **Strong** — callers re-deriving meaning the enum should carry |
| **Repeated predicate patterns** | Grep for recurring combinations of `has::<T>()`, `get::<T>()`, `.is_some()`, `.map()` that check the same component set in 2+ locations | **Strong** — missing derived/cached concept |
| **Cross-crate spread** | Count distinct crates (`worldwake-core`, `-sim`, `-systems`, `-ai`) containing the concept | **Moderate** — 3+ crates suggests boundary misplacement |
| **Derived state recomputation** | Functions that compute the same derived value from the same inputs in different modules (same parameter types, same return semantics, different call sites) | **Strong** — should be stored as state or a method on the type |
| **Workaround indicators** | Grep for `// workaround`, `// hack`, `// TODO`, `// FIXME`, `// safety net`, `// fallback`, `// temporary`, `// HACK` in exercised modules (case-insensitive) | **Direct evidence** — rare in this codebase but should not be ignored |

**Scoring**: A cluster is flagged for diagnosis if it meets ANY of:
- Scattered match arms in 3+ files on the same enum
- Repeated predicate patterns in 3+ locations
- Derived state recomputation in 2+ locations
- Any workaround indicators

A cluster that meets only the file-count threshold but none of the above is reported as "Acceptable — spread without structural debt."

**Tool usage**: Grep for patterns, Read specific functions for manual comparison when grep finds potential matches.

### Phase 4: DIAGNOSE — Check Against FOUNDATIONS.md

**Prerequisite**: Read `docs/FOUNDATIONS.md` in full before this phase (skip if already read in this session).

For each cluster flagged in Phase 3, apply two layers of FOUNDATIONS analysis:

#### Always-check principles (every flagged cluster):

| Principle | Check |
|-----------|-------|
| **P1** — Maximal Emergence Through Local Causality | Does scattering this concept across files prevent emergent composition? Would a first-class type enable new system interactions? |
| **P3** — Concrete State Over Abstract Scores | Is the concept represented as an abstract score or flag when it should be concrete state with identity? |
| **P7** — Locality of Motion, Interaction, and Communication | Does the scattering force modules to query non-local information to derive what should be locally available? |
| **P26** — Systems Interact Through State, Not Through Each Other | Are systems calling each other's functions instead of reading shared state? Does the scattered logic create hidden coupling? |
| **P27** — Derived Summaries Are Caches, Never Truth | Is derived state being recomputed from scratch instead of stored and maintained? |
| **P28** — No Backward Compatibility in Live Authority Paths | Are there shims, deprecated wrappers, or compatibility layers masking the need for a proper abstraction? |

#### Auto-selected principles (2-3 additional, based on domain):

Determine the cluster's domain from its symbols and select additional principles:

- **Combat / needs / metabolism clusters** → P8 (action cost/occupancy), P11 (feedback dampeners)
- **Belief / knowledge / perception clusters** → P14 (world state is not belief state), P15 (knowledge locality), P16 (ignorance is first-class)
- **Agent decision / goal / planning clusters** → P19 (agent symmetry), P20 (resource-bounded reasoning), P21 (revisable commitments)
- **Institutional / office / social clusters** → P23 (roles/offices as world state), P24 (ownership/custody/access), P25/P25A (social artifacts)
- **Production / trade / economy clusters** → P4 (persistent identity and explicit transfer), P5 (carriers of consequence)

#### Diagnostic questions:

For each flagged cluster, answer these questions:

1. **Implicit state machine?** Does the concept have identifiable phases (e.g., created → ready → active → consumed → removed) with no explicit lifecycle type? Do transitions happen in different files? If yes, this is a **missing abstraction**.

2. **Incomplete abstraction?** Does a type exist for this concept but lack derived/cached state that callers need, forcing them to re-compute readiness/applicability from scratch in multiple locations? If yes, this is an **incomplete abstraction**.

3. **Redundant computation?** Is the same "readiness" or "eligibility" check computed from scratch in multiple locations instead of being stored as component state? If yes, this supports a missing or incomplete abstraction diagnosis.

4. **Boundary violation?** Does a higher-level crate (e.g., `worldwake-ai`) handle what should be a lower-level concern (e.g., `worldwake-core` or `worldwake-systems`)? Does the concept's logic leak upward? If yes, this is a **boundary misplacement**.

### Phase 5: REPORT — Write Assessment-Compatible Output

Write to `reports/missing-abstractions-<date>-<context>.md` where `<date>` is YYYY-MM-DD and `<context>` is derived from the test path (e.g., `golden-trade`, `e09-needs`, `ai-tests`).

Use this template:

```markdown
# Missing Abstraction Analysis: <context>

**Date**: <YYYY-MM-DD>
**Input**: <test path>
**Source modules analyzed**: <count>
**Crates touched**: <list>

## Executive Summary

<1-3 sentences: were missing abstractions found? How severe? How many clusters flagged vs acceptable?>

## Cluster Summary

| Cluster | Files | Crates | Scattered Matches | Repeated Predicates | Recomputation | Verdict |
|---------|-------|--------|-------------------|--------------------:|--------------:|---------|
| <name>  | N     | N      | N files            | N locations         | N locations   | Missing / Incomplete / Boundary / Acceptable |

## Concept Clusters

### <Cluster Name> (Files: N, Crates: N)

**Modules**: <bulleted list of source file paths>

**Key symbols**: <bulleted list of pub items most relevant to the concept>

**Scattered match arms** (if found):
- `<file:line>` — matches `EnumName` with <description of logic>
- `<file:line>` — matches `EnumName` with <similar but different logic>

**Repeated predicates** (if found):
- Pattern: `<description of repeated check>` appears in:
  - `<file1:line>`
  - `<file2:line>`

**Derived state recomputation** (if found):
- `<file1:fn_name>` and `<file2:fn_name>` both compute <what> from <same inputs>

**FOUNDATIONS alignment**:
- P<N> (<short name>): violated / strained / satisfied — <one-line explanation>

**Diagnosis**: Missing abstraction / Incomplete abstraction / Boundary violation / Acceptable complexity

**Rationale**: <2-3 sentences explaining the diagnosis>

---

## Proposals

For each cluster diagnosed as Missing, Incomplete, or Boundary violation, write a proposal. Number proposals sequentially (P1, P2, ...).

### P<N>: <Title>

**Claim**: <What is missing, incomplete, or misplaced — stated as a factual observation>
**Evidence**:
- <file:line> — <what was found>
- <file:line> — <what was found>
**FOUNDATIONS references**: P<N> (<name>), P<N> (<name>)
**Proposed change**: <What a spec should address — e.g., "Introduce a `PatrolPhase` enum in worldwake-core that carries readiness state, eliminating scattered match arms in worldwake-systems and worldwake-ai">
**Priority**: <Critical / High / Medium — based on file count, crate spread, and FOUNDATIONS severity>

---

## Acceptable Clusters

For each cluster that met the file-count threshold but showed no structural debt:

### <Cluster Name>

<1-2 sentences explaining why the spread is architecturally correct — e.g., "Trade touches 4 crates because trade is inherently cross-cutting: core defines the types, systems handles execution, ai generates trade goals. Each crate owns its own concern cleanly.">
```

## Important Rules

1. **READ-ONLY** — Do not modify any source files.
2. **No test execution** — Static analysis only. Do not run `cargo test` or any other test command.
3. **No spec writing** — Only write the report. Spec authoring is a separate step via `/assessment-to-specs`.
4. **Always read `docs/FOUNDATIONS.md`** before the DIAGNOSE phase.
5. **Focus on structural signals** — scattered match arms, repeated predicates, cross-crate spread, derived state recomputation. Do not check for general code smells, style issues, or naming conventions.
6. **Report must be actionable** — each finding either needs a spec proposal or is explicitly marked acceptable.
7. **Proposals section must be `/assessment-to-specs`-compatible** — each proposal has: ID, claim, evidence with file:line references, FOUNDATIONS references, proposed change, and priority.
8. **Do not invent problems** — if no missing abstractions are found, say so. A clean result is a valid and valuable finding. Report it with the same rigor as a problematic result.
9. **Workaround density matters more than file count** — a cluster with repeated predicates in 3 files is a stronger signal than a concept that simply appears in 15 files with clean boundaries.
