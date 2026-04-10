---
name: detect-architectural-debt
description: "Analyze engine code exercised by a test suite to find missing abstractions and architectural fractures — from single-concept scatter to cross-subsystem authority confusion. Two parallel detection lenses (structural scatter + architectural fractures) with cross-lens reinforcement. Outputs a report compatible with /assessment-to-specs."
user-invocable: true
arguments:
  - name: test_path
    description: "Path to a test file or test directory (e.g., crates/worldwake-ai/tests/golden_trade_acquisition.rs or crates/worldwake-systems/tests/)"
    required: true
---

# Detect Architectural Debt

Analyze engine code exercised by a test suite to find authority confusion at every scale — from single-concept scatter (repeated predicates, scattered match arms, missing lifecycle types) to cross-subsystem fractures (split protocols, authority leaks, boundary inversions). Two parallel detection lenses with cross-lens reinforcement produce findings that neither lens could generate alone.

## Invocation

```
/detect-architectural-debt <test-file-or-directory-path>
```

**Parameter**: Path to a test file or directory that exercises the engine area to analyze.

**Optional**: `--prior-reports` — paths to earlier `architectural-debt` reports. The skill builds on previous analysis rather than rediscovering known issues.

**Optional**: `--differential` — when the exercised module set has high overlap with a prior report (>80%), skip re-investigation of already-covered code paths and focus on uniquely exercised or uniquely stressed paths. Auto-detected when overlap exceeds 80%.

**Output**: Structured report at `reports/architectural-debt-<date>-<context>.md`, formatted for consumption by `/assessment-to-specs`. `<context>` is derived from the input: for a test file, strip the path prefix and `.rs` suffix; for a directory, use the directory name.

**Incremental mode** (optional): If a previous report exists for the same test path (check `reports/architectural-debt-*-<context>.md`), read it at the start. Focus measurement only on clusters/fractures whose file counts changed by >20% or that include newly added modules since the previous report. Carry forward unchanged "Acceptable" verdicts without re-measuring. Note "incremental — carried forward from <previous date>" for reused verdicts.

## Background

Authority confusion manifests at two scales:

**Structural scatter** (within a single concept): A semantic concept (e.g., "patrol lifecycle", "trade readiness") whose state transitions or readiness checks are scattered across many files with no unifying type, or with a type that lacks sufficient derived state — forcing callers to re-compute readiness/applicability from scratch. In a Rust/ECS codebase, symptoms include: the same enum matched in 3+ files with similar-but-not-identical logic; the same combination of component checks in multiple locations; a single concept requiring 3+ crates to implement correctly; functions in different modules computing the same derived value from the same inputs.

**Architectural fractures** (across subsystems): The boundary between subsystems is wrong, authority over shared truth is split, or the same concept lives under different names in neighboring crates. Symptoms include: fixing a bug in one crate requires compensating changes in another; the same eligibility predicate is computed from scratch in multiple crates; error handlers in one layer catch problems that another layer should prevent; files across crate boundaries repeatedly change together.

Both are **authority confusion** — they differ in scale, not in kind. This skill detects both through parallel lenses and synthesizes findings through cross-lens reinforcement.

## Pipeline Overview

```
Phase 1: GATHER       — Build exercised module set + git history
Phase 2: SCENARIO MAP — Cluster tests into behavioral families
                        (Phases 1 & 2 run in parallel)
Phase 3: TRACE        — Build test-to-code traceability
                        (Often collapses into Phase 1 via short-circuit)
Phase 4: DETECT       — Two parallel lenses:
  |-- Lens A: Structural Scatter
  |   Bottom-up: cluster files by shared concepts, measure
  |   structural signals (scattered guards, repeated predicates,
  |   derived state recomputation, clone-like redundancy, etc.)
  |
  \-- Lens B: Architectural Fractures
      Top-down: map scenario families to subsystem boundaries,
      detect fracture types (split protocols, authority leaks,
      projection drift, boundary inversions, etc.)

Phase 5: SYNTHESIZE   — Cross-lens reinforcement + severity ranking
Phase 6: VALIDATE     — Survival criteria + FOUNDATIONS alignment
```

## Methodology

**Execution Strategy**: Phases 1 and 2 run in parallel. Phase 3 often collapses into Phase 1 when the short-circuit applies. Lens A and Lens B within Phase 4 run in parallel. For large analyses (>30 modules), launch up to 3 Explore agents in parallel for Phase 4. Phase 5 requires results from both lenses. Phase 6 requires FOUNDATIONS context and runs last.

### Phase 1: GATHER — Build the Exercised Module Set

Starting from the test file(s), build a list of source modules that the tests exercise.

**Short-circuit for golden/integration tests**: If the test calls a top-level simulation step function (e.g., `step_once()`, `tick()`, or equivalent) in a loop, all source modules in the referenced crates are exercised. Skip per-symbol tracing and enumerate all `.rs` files in those crates' `src/` directories directly, excluding `lib.rs` barrel files and `mod.rs` files that only contain `mod` declarations.

**Otherwise, trace per-symbol**:

1. If the input is a directory, collect all `.rs` files in it (excluding `mod.rs` files that only contain `mod` declarations). If a single file, use that file.
2. Read the test file(s) and extract all `use` statements to identify which crates are referenced (e.g., `worldwake_core`, `worldwake_sim`, `worldwake_systems`, `worldwake_ai`).
3. Extract all type names, function names, struct names, and enum variant names actually used in the test code body (not just imported). Focus on symbols from the `worldwake_*` crates.
4. For each referenced crate, grep `crates/<crate-name>/src/` for the definitions of those symbols (`pub fn <name>`, `pub struct <name>`, `pub enum <name>`, `pub trait <name>`) to identify which source modules are exercised.
5. For each exercised source module, read its internal `use` and `mod` statements to add 1-2 levels of internal dependencies to the exercised set.
6. Produce a deduplicated list of all source modules exercised by the test suite.

**Important**: Rust crate `lib.rs` files re-export most public items. Do not count `lib.rs` as an exercised module — it is a barrel file. Trace through to the actual defining module.

**Git history analysis** (runs in parallel with symbol tracing): Run bounded git history: `git log --since="6 months ago" --name-only` on exercised files. Use recursive globs (e.g., `'crates/worldwake-*/src/**/*.rs'`). From the output, group files by commit. For each commit, enumerate all cross-crate file pairs that changed together. Count how many commits each pair co-appears in. Report the top 20 cross-crate pairs with 3+ co-changes, ordered by frequency. Also report the crate-to-crate coupling matrix (total co-changing commits per crate pair).

**Prior reports**: Read any `prior_reports` if provided. Also scan `reports/` for existing `architectural-debt-*` reports matching the same test context or related test contexts (same harness setup, same crate under test, same short-circuit scope). Two test files that both exercise the entire `worldwake-ai` crate via `step_once()` loops produce overlapping coverage — their reports are mutually relevant. If the exercised module set overlaps >80% with a prior report's scope, auto-activate `--differential` mode.

**Sub-agent delegation**: For large test suites (>20 direct `use` imports or barrel re-exports), delegate import tracing to 1-3 parallel Explore sub-agents. Also delegate git history analysis to a separate sub-agent if the file list exceeds 30 modules.

**Tool usage**: Read test files, Grep for `use worldwake_`, Grep for `pub (fn|struct|enum|trait)` definitions in crate source directories, Bash for `git log`.

### Phase 2: SCENARIO MAP — Cluster Tests into Behavioral Families

Treat tests as behavioral scenarios, not just import sources.

For each test or test family (a `mod tests` block, a `#[test]` function, or a golden test file), recover:

- **What behavior** is being exercised (e.g., "goal replanning after action failure")
- **Which setup path** it uses (e.g., scenario RON file, `TestHarness` builder, manual component registration)
- **Which assertions** define success/failure (e.g., `assert_eq!`, `assert!`, custom assertion helpers)
- **Which domain concepts** appear in names, helpers, and expected values (e.g., "goal", "belief", "action", "need", "trade")

For golden E2E tests specifically, also note:
- Which RON scenario file is loaded (if any)
- Which systems are exercised through the simulation loop
- What emergent behavior the test validates (cross-system interactions)

Then cluster tests into **scenario families** — named behavioral groups. Example shapes:

- "goal dispatch lifecycle"
- "belief propagation chain"
- "action validation pipeline"
- "need satisfaction cycle"
- "trade negotiation flow"
- "combat resolution sequence"
- "perception and observation"

Every later finding must be tied back to scenario families. A finding not grounded in test behavior is speculation.

**Soak/endurance tests**: When the test runs the simulation for many ticks and checks invariants, derive scenario families from the invariant categories and emergence assertions rather than from test function boundaries. Each per-tick invariant check (conservation, needs bounds, unique placement) and each emergence threshold check (death, trade, political events) becomes a scenario family.

**Resilience/chaos tests**: When the test injects disruptions (kills, deletions, workstation removal, teleportation) and validates invariants hold despite them, derive scenario families from the disruption categories and the invariant categories they stress. The disruption injection protocol itself is a scenario family if it exercises a distinct code path. Similarly, serialization roundtrip tests form their own scenario family around the serialization boundary.

**Determinism replay tests**: Tests that run the same scenario twice with the same seed and assert identical outcomes are determinism validators, not separate scenario families. Count them with their parent scenario but note determinism validation as a cross-cutting concern.

**Sub-agent delegation**: For large test directories (>30 test files), delegate scenario extraction to 2-3 parallel Explore sub-agents, each handling a subset. Merge and deduplicate scenario families.

### Phase 3: TRACE — Build Test-to-Code Traceability

Build test-to-code traceability using multiple strategies:

| Strategy | What it finds | Confidence |
|----------|--------------|------------|
| `use` statements | Direct dependencies | High |
| Static call graph (from `assert!`/`assert_eq!` back to production) | Functions actually exercised | High |
| Naming/lexical similarity (test helpers vs production functions) | Conceptual links | Medium |
| Temporal coupling from git history (files that co-change) | Hidden dependencies | Medium |

Each traceability link gets a confidence tag (high/medium/low) and a brief reason code.

The purpose of multi-strategy tracing is to catch hidden dependencies that `use` statements alone miss — trait dispatch, `SystemFn` registration, `register_action_handler` indirection, and temporal coupling are the most common sources of invisible links in this codebase.

**After short-circuit**: When Phase 1's short-circuit determined all modules are exercised, skip the `use` statement and call graph strategies. Focus on temporal coupling analysis and naming/lexical similarity for mapping modules to scenario families. The traceability table should focus on modules uniquely relevant to specific scenario families — modules that handle the test's distinctive code paths (e.g., save/load, disruption handling, invariant checking). A focused table of 10-15 key modules is more useful than an exhaustive listing of 200.

### Phase 4: DETECT — Two Parallel Lenses

Run Lens A and Lens B in parallel. For large analyses (>100 modules), delegate each lens to separate Explore sub-agents.

#### Lens A: Structural Scatter

Within the exercised modules, find cross-cutting concept clusters with structural debt signals.

**Step 1 — Find Concept Clusters**:

1. **Filename clustering** (primary method): Group exercised modules by dominant concept in their filename (e.g., `patrol.rs`, `patrol_actions.rs` → "patrol" cluster; `goal_policy.rs`, `goal_model.rs`, `goal_dispatch_key.rs` → "goal" cluster). Module filenames in this codebase reliably reflect concept boundaries.
2. **Import clustering** (supplement): For modules with generic names, scan their `use` statements and key `pub` exports to assign them to a concept cluster.
3. **Enum-centered clustering**: Grep for key enums (`pub enum`) in the exercised modules. Any enum whose variants are referenced in 3+ files is a cluster seed — name the cluster after the enum.
4. Name each cluster by its dominant concept fragment.
5. Filter to clusters exceeding the file-count threshold: >10% of analyzed files, or 5+ files, whichever is larger. For small analyses (<30 modules), use 5+ files as the floor.
6. Merge clusters with >50% module overlap.

**Step 2 — Early-Exit Check**:

If a cluster's symbols are predominantly single-component accessors (`get_component_*`, `effective_place`, `possessor`, `ground_location`, `commodity_quantity`, or similar read-only queries) and no enum in the cluster is being matched in multiple files, mark the cluster as "Acceptable — fundamental accessor" and skip full measurement. Fundamental accessors appear in many files by design.

**Step 3 — Measure Structural Signals**:

For clusters that did not early-exit, compute these metrics:

| Metric | How to measure | Signal strength |
|--------|---------------|-----------------|
| **File count** | Distinct source files containing the concept | Baseline — spread indicates cross-cutting concern |
| **Scattered match arms** | Grep for `match` expressions on the cluster's key enums; count files with similar-but-not-identical match logic on the same enum | **Strong** — callers re-deriving meaning the enum should carry |
| **Repeated predicate patterns** | Grep for recurring combinations of `has::<T>()`, `get::<T>()`, `.is_some()`, `.map()`, `matches!()`, `if let` chains that check the same component set in 2+ locations | **Strong** — missing derived/cached concept |
| **Cross-crate spread** | Count distinct crates (`worldwake-core`, `-sim`, `-systems`, `-ai`) containing the concept | **Moderate** — 3+ crates suggests boundary misplacement |
| **Derived state recomputation** | Functions that compute the same derived value from the same inputs in different modules (same parameter types, same return semantics, different call sites); same computation from same `&self` fields in multiple `impl` blocks | **Strong** — should be stored as state or a method on the type |
| **Clone-like redundancy** | Near-duplicate `impl` blocks or free functions with same structure, different concrete types | **Strong** — missing generic or trait abstraction |
| **Option lifecycle smell** | `Option<T>` fields modelling implicit phases; multiple `Option` fields that are correlated (if A is Some, B must be Some) without a state enum | **Strong** — missing lifecycle type |
| **Workaround indicators** | Grep for `// workaround`, `// hack`, `// TODO`, `// FIXME`, `// safety net`, `// fallback`, `// temporary`, `// HACK` (case-insensitive) | **Direct evidence** |

Three counts per cluster: defining files, consumer files, temporally-coupled files.

**Step 4 — Scenario Grounding**:

For each cluster, check whether it maps to at least one scenario family from Phase 2. A cluster that cannot explain any test scenario is demoted to "Needs Investigation" regardless of signal strength.

**Tool usage**: Glob for filenames, Grep for `pub enum` definitions, `match` expressions, predicate patterns, workaround comments. Read specific functions for manual comparison when grep finds potential matches.

#### Lens B: Architectural Fractures

Use the temporal coupling matrix from Phase 1 to prioritize which boundary files to read. Start with the top 5 cross-crate file pairs by co-change frequency.

Scan the exercised code for these 8 fracture types:

| # | Fracture Type | What to look for | Rust-Specific Signals |
|---|--------------|-----------------|----------------------|
| 1 | **Split protocol** | The legal sequence of interactions is spread across multiple modules/crates. Module A decides "what", module B decides "when", module C decides "whether". | Trait implementations scattered across modules; a trait's methods implemented in different crates/modules. |
| 2 | **Authority leak** | Multiple modules write the same truth. Two or more places create/mutate/invalidate the same piece of state. | Multiple modules constructing/modifying the same struct; `pub` fields allowing uncontrolled mutation from outside the owning module. |
| 3 | **Projection drift** | Derived summaries or cached computations are recomputed everywhere. No single module owns the projection. Detect at ALL scales (intra-subsystem and cross-subsystem). | Same derived value computed from same struct fields in different modules (no shared method or associated function). |
| 4 | **Boundary inversion** | Higher crates own rules that belong in lower crates. `worldwake-ai` enforces what `worldwake-core` or `worldwake-systems` should prevent. | Higher-level crate enforcing invariants that should be in a lower-level crate's type system. |
| 5 | **Concept aliasing** | The same domain concept exists under different names/types in neighboring crates. | Type aliases or newtypes for the same concept with different names in different modules. |
| 6 | **Hidden seam** | Files across nominal crate boundaries repeatedly change together in git history, suggesting they belong in the same module or crate. | Language-agnostic — git co-change analysis. |
| 7 | **Overloaded abstraction** | One type/module carries several lifecycle roles that should be separated. | Enum with too many variants serving different lifecycle purposes; struct with fields for multiple disjoint use cases. |
| 8 | **Orphan compatibility layer** | A shim, fallback path, or "safety net" handler exists only to mask a deeper missing abstraction. | Wrapper types, `From`/`Into` impls, or compatibility modules that exist only to bridge a missing abstraction. |

**Evidence rule**: A fracture is NOT reported in main findings unless supported by at least two independent signals (e.g., import analysis + temporal coupling, or naming similarity + assertion patterns). Single-signal fractures go in "Needs Investigation."

**Rust-specific considerations**:
- **Trait coherence and orphan rules** create forced boundaries that may mask or create fractures.
- **Ownership and borrowing patterns** — authority confusion may manifest as lifetime complexity or excessive `.clone()` calls.
- **Module visibility (`pub`, `pub(crate)`, `pub(super)`)** — the visibility system IS the boundary system; use it for boundary analysis.
- **Crate boundaries** — in this multi-crate workspace, crate boundaries are the primary subsystem boundaries.
- **Derive macros and procedural macros** may hide structural patterns from grep-based detection.

**Sub-agent delegation**: For large exercised sets (>100 modules), delegate to 2-3 parallel Explore sub-agents, each analyzing a different crate boundary surface (e.g., ai↔sim, ai↔core, sim↔systems).

**Tool usage**: Grep for shared type names across crates, Grep for duplicate predicate patterns, Grep for `pub enum` and `match` expressions crossing module boundaries, Read key functions at boundary points, Bash for `git log` co-change analysis.

### Phase 5: SYNTHESIZE — Cross-Lens Reinforcement

This phase is the core value of the unified skill. Compare findings from both lenses:

| Lens A Finding | Lens B Finding | Result |
|---------------|---------------|--------|
| Cluster with signals | Fracture in overlapping modules | **Merged finding** — confidence elevated automatically |
| Cluster with signals | No fracture | **Contained scatter** — lower severity (Medium or Low) |
| No cluster | Fracture detected | **Boundary-level fracture** — severity by fracture type |
| Single signal from either lens | — | **Needs Investigation** |

For each validated finding (from either lens or merged), produce a candidate abstraction:

- **title**: Descriptive name (e.g., "Goal Dispatch Protocol")
- **lens_source**: Lens A / Lens B / Merged (both lenses)
- **kind**: One of: Protocol | Authority boundary | Bounded context | Projection owner | Capability ledger | Workflow coordinator | Translation boundary | Lifecycle carrier
- **scope**: Which crates/modules it spans
- **owned_truth**: What state or invariant this abstraction would own (the single most important field — if you can't name this, the candidate is not ready)
- **invariants**: What must always be true when this abstraction is correctly implemented
- **owner_boundary**: Which crate/module should own it
- **modules_affected**: Existing modules that would be absorbed, constrained, or simplified
- **scenario_families_explained**: Which scenario families from Phase 2 this candidate accounts for
- **expected_simplification**: What gets simpler — fewer writers, fewer repeated predicates, fewer cross-boundary transitions, fewer co-change edges, clearer ownership
- **severity**: Critical / High / Medium / Low (see Severity Ranking below)
- **confidence**: High / Medium / Low (evidence certainty)
- **counter_evidence**: What would falsify this hypothesis. **MANDATORY** — every candidate must have this field populated.

### Severity Ranking

| Level | Definition |
|-------|-----------|
| **Critical** | Multiple subsystems write the same truth with no single owner. Fixing a bug requires synchronized cross-boundary changes. |
| **High** | Lifecycle transitions scattered across subsystem boundaries, or protocol split so "what"/"when"/"whether" live in different modules. |
| **Medium** | Intra-subsystem scatter with strong structural signals. Contained but substantial. |
| **Low** | Single-subsystem scatter with moderate signals, or boundary-level fracture with limited blast radius. |

Ranking rules (in priority order):
1. Cross-lens reinforced > single-lens at same signal strength
2. More scenario families explained > fewer
3. Temporal coupling evidence present > absent
4. More affected modules > fewer (tiebreaker within same severity)

### Phase 6: VALIDATE — Survival Criteria + FOUNDATIONS Alignment

**Prerequisite**: Read `docs/FOUNDATIONS.md` in full before this phase (skip if already read in this session).

Apply two validation filters, in this order:

**Filter 1 — Survival criteria.** Drop any candidate that fails ANY of these:

1. It explains at least two tests or one whole scenario family
2. It reduces at least one real architectural cost (not just "cleaner")
3. It can name the owned truth
4. It can name the rightful owner boundary
5. It does not merely wrap existing code with a facade

**Filter 2 — FOUNDATIONS alignment.** For surviving candidates only, check against `docs/FOUNDATIONS.md`.

#### Always-check principles (every finding):

| Principle | Check |
|-----------|-------|
| **P1** — Maximal Emergence Through Local Causality | Does the authority confusion prevent emergent composition? Would a first-class type enable new system interactions? |
| **P3** — Concrete State Over Abstract Scores | Is the concept represented as an abstract score or flag when it should be concrete state with identity? |
| **P7** — Locality of Motion, Interaction, and Communication | Does the scattering force modules to query non-local information to derive what should be locally available? |
| **P26** — Systems Interact Through State, Not Through Each Other | Are systems calling each other's functions instead of reading shared state? Does the scattered logic create hidden coupling? |
| **P27** — Derived Summaries Are Caches, Never Truth | Is derived state being recomputed from scratch instead of stored and maintained? |
| **P28** — No Backward Compatibility in Live Authority Paths | Are there shims, deprecated wrappers, or compatibility layers masking the need for a proper abstraction? |

#### Auto-selected principles (2-3 additional, based on domain):

- **Combat / needs / metabolism** → P8 (action cost/occupancy), P11 (feedback dampeners)
- **Belief / knowledge / perception** → P14 (world state is not belief state), P15 (knowledge locality), P16 (ignorance is first-class)
- **Agent decision / goal / planning** → P19 (agent symmetry), P20 (resource-bounded reasoning), P21 (revisable commitments)
- **Institutional / office / social** → P23 (roles/offices as world state), P24 (ownership/custody/access), P25/P25A (social artifacts)
- **Production / trade / economy** → P4 (persistent identity and explicit transfer), P5 (carriers of consequence)

For each relevant principle, note whether the candidate aligns, strains, or conflicts. Flag conflicts prominently — a candidate that violates FOUNDATIONS needs redesign before it becomes a spec.

**This ordering matters.** Recovery first, judgement second. Do not let FOUNDATIONS bias the fracture detection — detect what IS, then evaluate what SHOULD BE.

## Report Format

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

## Hard Rules

1. **Read-only.** Do not modify any source files. Do not run `cargo test` or any other test command. Static analysis and git history only.
2. **No spec/ticket writing.** Only write the report. Spec authoring is a separate step via `/assessment-to-specs`.
3. **Two-signal minimum** for all findings in the main Findings section. Single-signal observations go in "Needs Investigation."
4. **Every finding needs counter-evidence.** A finding without counter-evidence is an assertion, not an analysis.
5. **No pattern theater.** Never recommend a pattern name unless it corresponds to owned truth and a real boundary.
6. **No abstraction without authority.** If the proposal cannot name the owner, it is not ready. Move it to "Needs Investigation."
7. **No wrapper-only recommendations.** The question is always: what truth moves, and who gains authority?
8. **Recovery first, judgement second.** Build the scenario map and detect findings BEFORE applying FOUNDATIONS principles. Do not let architectural ideals bias what you observe.
9. **Do not invent problems.** "Acceptable architecture" must remain a valid and prominent outcome.
10. **No archived prior reports.** If a report already exists at the target path, overwrite it.
11. **Scenario grounding required** for Lens A clusters. A cluster that cannot explain any test scenario is demoted to "Needs Investigation."
12. **Findings must be complete.** All fields in the finding template must be populated.

## Workflow Context

Typically invoked after implementing a spec or after `/golden-gap-analysis` identifies coverage gaps. Output feeds into `/assessment-to-specs` for spec generation from proposals. The workflow is:

1. Implement spec
2. `/golden-gap-analysis` (coverage)
3. `/detect-architectural-debt` (structural debt + architectural fractures)
4. `/assessment-to-specs` (spec drafting from proposals)
