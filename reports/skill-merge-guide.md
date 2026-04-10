# Skill Merge Guide: detect-missing-abstractions + recover-architectural-abstractions

**Date**: 2026-04-10
**Purpose**: Reference document for merging two abstraction-detection skills into one unified skill in a different repository. Feed this to a brainstorm skill in the target repo to guide the merge while preserving language/architecture-specific adaptations.

## Why the Merge

### The Problem

Two skills existed with an intended pipeline relationship:
- **detect-missing-abstractions**: Finds single-concept scatter (e.g., a lifecycle spread across many files with duplicated logic, scattered discriminant checks, repeated predicates)
- **recover-architectural-abstractions**: Finds cross-subsystem fractures (split protocols, authority leaks, boundary inversions, concept aliasing)

The workflow was: run `detect` first for structural debt, then optionally run `recover` for higher-level fractures.

### What Happened in Practice

The user only ever ran `recover`. Reasons:
1. If a higher-level architectural fracture exists, fixing single-concept scatter underneath it is treating symptoms. You want to find the top-level problem first.
2. `recover`'s "acceptable architecture" verdict was trusted — if cross-subsystem boundaries are clean, intra-subsystem scatter is manageable tech debt.
3. The one major success story (`detect` finding a grant lifecycle state manager across ~30 files) would likely have been caught by `recover` too, because the problem crossed functional subsystem boundaries.
4. Decision fatigue: unclear when one skill benefits over the other, leading to always choosing the more comprehensive one.

### The Core Insight

Both skills detect **authority confusion** — they just operate at different scales. The distinction between "intra-subsystem scatter" and "cross-subsystem fractures" is a spectrum, not a binary. A unified skill with two parallel detection lenses covers the full spectrum without requiring the user to choose.

## The Unified Design

### Architecture: One Pipeline, Two Lenses

```
Phase 1: GATHER          — Build exercised module set + git history
Phase 2: SCENARIO MAP    — Cluster tests into behavioral families
                           (Phases 1 & 2 run in parallel)
Phase 3: TRACE           — Build test-to-code traceability
                           (Often collapses into Phase 1)
Phase 4: DETECT           — Two parallel lenses:
  |-- Lens A: Structural Scatter   (from old detect skill)
  |   Bottom-up: cluster files by shared concepts, measure
  |   structural signals (scattered guards, repeated predicates,
  |   derived state recomputation, clone-like redundancy, etc.)
  |
  \-- Lens B: Architectural Fractures   (from old recover skill)
      Top-down: map scenario families to subsystem boundaries,
      detect fracture types (split protocols, authority leaks,
      projection drift, boundary inversions, etc.)

Phase 5: SYNTHESIZE       — Merge findings from both lenses,
                           rank by authority confusion severity
Phase 6: VALIDATE         — Survival criteria + foundations alignment
```

### Key Design Decisions (Language-Agnostic)

These decisions should be preserved regardless of the target language:

1. **Unified evidence threshold: two-signal minimum for ALL findings.** The old `detect` flagged clusters on any single strong signal. The old `recover` required two signals. The merged skill uses the stricter rule universally. Single-signal observations go in "Needs Investigation." This eliminates noise.

2. **Projection drift detection at all scales.** The old `recover` explicitly excluded intra-subsystem projection drift, deferring it to `detect`. Since there's only one skill now, detect it everywhere. Severity ranking handles the distinction — cross-subsystem ranks higher.

3. **Scenario families become the backbone for both lenses.** The old `detect` clustered by filename/type patterns (bottom-up only). The merged skill grounds everything in test behavior first (scenario families from Phase 2), then Lens A adds structural metrics on top. A cluster that can't explain any test scenario gets demoted to "Needs Investigation." This prevents file-counting noise.

4. **Cross-lens reinforcement is the core value.** After both lenses complete, compare findings:
   - Overlapping modules across Lens A + Lens B → **merged finding**, confidence elevated automatically
   - Lens A cluster with no Lens B fracture → **contained scatter**, lower severity
   - Lens B fracture with no Lens A cluster → **boundary-level fracture**
   - Either lens, single signal → "Needs Investigation"
   
   This is something neither old skill could do alone.

5. **Severity and confidence are orthogonal axes.** Old `detect` used Priority (Critical/High/Medium). Old `recover` used Confidence (High/Medium/Low). The merged skill has both: severity = impact (Critical/High/Medium/Low based on authority confusion scope), confidence = certainty (High/Medium/Low based on evidence strength). A finding can be high-severity but medium-confidence.

6. **Counter-evidence is mandatory for every finding.** Carried from `recover`. Every finding must state what would falsify it.

7. **"Acceptable architecture" is a valid and prominent outcome.** Do not invent problems. An analysis that finds nothing wrong is useful.

8. **Recovery first, judgement second.** Detect what IS before applying architectural principles. Do not let ideals bias observation. Foundations/principles validation happens in the final phase only.

### Severity Ranking (Language-Agnostic)

| Level | Definition |
|-------|-----------|
| **Critical** | Multiple subsystems write the same truth with no single owner. Fixing a bug requires synchronized cross-boundary changes. |
| **High** | Lifecycle transitions scattered across subsystem boundaries, or protocol split so "what"/"when"/"whether" live in different modules. |
| **Medium** | Intra-subsystem scatter with strong structural signals. Contained but substantial. |
| **Low** | Single-subsystem scatter with moderate signals, or boundary-level fracture with limited blast radius. |

Ranking rules:
1. Cross-lens reinforced > single-lens at same signal strength
2. More scenario families explained > fewer
3. Temporal coupling evidence present > absent
4. More affected modules > fewer (tiebreaker within same severity)

### What to Preserve from Each Old Skill

**From detect-missing-abstractions (Lens A mechanics):**
- Concept clustering by naming patterns, discriminant types, state-carrier types, exported symbols
- File-count thresholds with scaling rules for small/large analyses
- Three counts per cluster: defining, consumer, temporally-coupled files
- Early-exit for fundamental accessors (prevents noise)
- Full metric measurement table (scattered guards, repeated predicates, recomputation, clone-like redundancy, lifecycle smells, fan-in, workaround indicators, etc.)
- Cluster merging when >50% overlap

**From recover-architectural-abstractions (Lens B mechanics):**
- Scenario family extraction from test behavior
- 8 fracture types (split protocol, authority leak, projection drift, boundary inversion, concept aliasing, hidden seam, overloaded abstraction, orphan compatibility layer)
- Two-signal evidence rule
- Candidate abstraction structure (owned truth, invariants, owner boundary, modules affected, expected simplification, counter-evidence)
- Temporal coupling analysis via git history
- Phase dependency graph with parallelization guidance

**NEW in the merged skill (not in either old skill):**
- Cross-lens reinforcement step after Phase 4
- Scenario grounding requirement for Lens A clusters (demote to "Needs Investigation" if no scenario family match)
- Unified severity ranking across both lenses
- Single report format merging both old formats

### What to Drop

- The old `detect`'s single-signal flagging rule (replaced by two-signal minimum)
- The old `recover`'s explicit exclusion of intra-subsystem projection drift
- The old `detect`'s standalone IDENTIFY phase (absorbed into Lens A Step 1 within Phase 4)
- The old `detect`'s Cluster Summary table in report (absorbed into Findings entries)
- The old `detect`'s Cross-Cutting Findings section (cross-lens reinforcement handles this natively)
- The pipeline relationship between the two skills (no more "run detect then recover")
- Any description text that says one skill "complements" or "feeds into" the other

### Hard Rules (Language-Agnostic)

These rules should be preserved in every language adaptation:

1. **Read-only.** Do not modify source files. Do not run tests. Static analysis and git history only.
2. **No spec/ticket writing.** Only write the report.
3. **Two-signal minimum** for all findings in the main Findings section.
4. **Every finding needs counter-evidence.**
5. **No pattern theater.** Never recommend a pattern name without naming owned truth and a real boundary.
6. **No abstraction without authority.** Can't name the owner → "Needs Investigation."
7. **No wrapper-only recommendations.**
8. **Recovery first, judgement second.** Detect before applying architectural principles.
9. **Do not invent problems.** "Acceptable architecture" is valid.
10. **No archived prior reports.**
11. **Scenario grounding required** for Lens A clusters.
12. **Findings must be complete** (all fields populated).

## Adaptation Notes for Rust

The merged skill in the TypeScript repo has language-specific detection mechanisms that need Rust equivalents. Here's what to watch for when adapting:

### Lens A Metric Equivalents

| TypeScript Mechanism | Rust Equivalent to Look For |
|---------------------|---------------------------|
| Scattered discriminant guards (`switch`/`if` on `kind`/`type` fields) | Scattered `match` arms on enums across multiple files; partial match patterns in different modules |
| Repeated predicate patterns (`.filter()`, boolean combos) | Repeated `.iter().filter()` closures, repeated `if let` / `matches!()` chains with same structure |
| Optional-property lifecycle smell (`pending?`, `done?`) | `Option<T>` fields modelling implicit phases; multiple `Option` fields that are correlated (if A is Some, B must be Some) without a state enum |
| Clone-like redundancy | Near-duplicate `impl` blocks or free functions with same structure, different concrete types |
| Derived state recomputation | Same computation from same `&self` fields in multiple `impl` blocks or free functions across modules |
| Discriminant clustering (`kind:`, `type:` fields) | Enum variant clustering — which enums are matched in 3+ files |
| State-carrier clustering | Struct types passed as parameters across 3+ modules without a single owning module |

### Lens B Fracture Equivalents

| Fracture Type | Rust-Specific Signals |
|--------------|----------------------|
| Split protocol | Trait implementations scattered across modules; a trait's methods implemented in different crates/modules |
| Authority leak | Multiple modules constructing/modifying the same struct; `pub` fields allowing uncontrolled mutation from outside the owning module |
| Projection drift | Same derived value computed from same struct fields in different modules (no shared method or associated function) |
| Boundary inversion | Higher-level crate enforcing invariants that should be in a lower-level crate's type system |
| Concept aliasing | Type aliases or newtypes for the same concept with different names in different modules |
| Hidden seam | Same as TypeScript — git co-change analysis is language-agnostic |
| Overloaded abstraction | Enum with too many variants serving different lifecycle purposes; struct with fields for multiple disjoint use cases |
| Orphan compatibility layer | Wrapper types, `From`/`Into` impls, or compatibility modules that exist only to bridge a missing abstraction |

### Rust-Specific Considerations

The Rust skills likely have language-specific detection mechanisms around:
- **Trait coherence and orphan rules** — these create forced boundaries that may mask or create fractures
- **Ownership and borrowing patterns** — authority confusion may manifest as lifetime complexity or excessive `.clone()` calls
- **Module visibility (`pub`, `pub(crate)`, `pub(super)`)** — the visibility system IS the boundary system; Lens B should use it
- **Crate boundaries** — in multi-crate workspaces, crate boundaries are the primary subsystem boundaries (equivalent to TypeScript's `packages/`)
- **Derive macros and procedural macros** — may hide structural patterns from grep-based detection

**Preserve all Rust-specific detection mechanisms from both old skills.** The merge is about unifying the pipeline and adding cross-lens reinforcement, not about removing language-specific heuristics.

## How to Use This Document

Feed this report to a brainstorm skill in the Rust repository with a prompt like:

> "We have two skills: detect-missing-abstractions and recover-architectural-abstractions. I've already merged them in another repository. Here's the merge guide with the rationale, unified design, and adaptation notes. Apply the same merge to our Rust-specific skills, preserving all Rust-specific detection mechanisms while adopting the unified pipeline, two-lens architecture, cross-lens reinforcement, and severity ranking system."

The brainstorm skill should:
1. Read both existing Rust skills
2. Use this document as the design reference
3. Identify which Rust-specific mechanisms map to Lens A vs Lens B
4. Preserve all Rust-specific heuristics while restructuring the pipeline
5. Produce a unified skill with the same quality bar
