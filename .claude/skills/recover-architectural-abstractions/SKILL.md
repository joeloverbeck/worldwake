---
name: recover-architectural-abstractions
description: "Use when a complex test suite exercises cross-subsystem code and you suspect higher-level architectural fractures — split protocols, authority leaks, boundary inversions — that detect-missing-abstractions cannot see because it works within single concepts. Outputs a report compatible with /assessment-to-specs."
user-invocable: true
arguments:
  - name: test_path
    description: "Path to a test file or test directory (e.g., crates/worldwake-ai/tests/golden_trade_acquisition.rs or crates/worldwake-systems/tests/)"
    required: true
---

# Recover Architectural Abstractions

Given a complex test suite, recover the as-built architecture of the exercised area and propose higher-order abstractions that name owned truth, invariants, interaction protocols, and owner boundaries. Works at the cross-subsystem level, complementing `detect-missing-abstractions` which works within single concepts.

## Invocation

```
/recover-architectural-abstractions <test-file-or-directory>
```

**Parameter**: Path to a test file or directory that exercises the crate area to analyze.

**Optional**: `--prior-reports` — paths to earlier `missing-abstractions` or `architectural-abstractions` reports. The skill builds on previous analysis rather than rediscovering known issues.

**Output**: Structured report at `reports/architectural-abstractions-<date>-<context>.md`. `<context>` is derived from the input: for a test file, strip the path prefix and `.rs` suffix; for a directory, use the directory name.

## Background

`detect-missing-abstractions` finds scattered state machines within a single concept (e.g., "goal lifecycle spread across 15 files"). This skill operates one level higher: it finds architectural fractures that span multiple subsystems — where the boundary between subsystems is wrong, where authority over shared truth is split, or where the same concept lives under different names in neighboring modules.

These fractures manifest as: fixing a bug in one crate requires compensating changes in another crate; the same eligibility/readiness predicate is computed from scratch in multiple crates; error handlers in one layer catch problems that another layer should prevent; files across nominal crate boundaries repeatedly change together.

## Methodology

### Phase 1: GATHER

Starting from the test file(s), build a dependency graph of source modules:

1. Read the test file(s) and extract all `use` statements to identify which crates are referenced (e.g., `worldwake_core`, `worldwake_sim`, `worldwake_systems`, `worldwake_ai`).
2. For each referenced crate (`crates/worldwake-*/src/`), grep for the definitions of symbols used in the test (`pub fn`, `pub struct`, `pub enum`, `pub trait`) to identify which source modules are exercised.
3. For each exercised source module, read its internal `use` and `mod` statements to add 1-2 levels of internal dependencies to the exercised set.
4. Produce a deduplicated list of all source files exercised by the test suite. Exclude `lib.rs` barrel files and `mod.rs` files that only contain `mod` declarations.
5. Read `docs/FOUNDATIONS.md` — hold it for Phase 6 validation. Do NOT apply it yet.
6. Read any `prior_reports` if provided via `--prior-reports`. Also scan the `reports/` directory for existing `missing-abstractions-*` and `architectural-abstractions-*` reports matching the same test context. Treat discovered reports the same as explicitly provided ones — note already-identified issues to avoid rediscovery.
7. Check for existing coverage/trace artifacts in the repo. Use them if present.
8. Run bounded git history: `git log --since="6 months ago" --name-only` on exercised files to identify temporal coupling. From the output, group files by commit. For each commit, enumerate all cross-crate file pairs that changed together. Count how many commits each pair co-appears in. Report the top 20 cross-crate pairs with 3+ co-changes, ordered by frequency. Also report the crate-to-crate coupling matrix (total co-changing commits per crate pair).

**Short-circuit for golden/integration tests**: If the test calls a top-level simulation step function (e.g., `step_once()`, `tick()`, or equivalent) in a loop, all source modules in the referenced crates are exercised. Skip per-symbol tracing (steps 2-3) and enumerate all `.rs` files in those crates' `src/` directories directly, excluding `lib.rs` barrel files and `mod.rs` files that only contain `mod` declarations.

**Sub-agent delegation**: For large test suites (>20 direct `use` imports or barrel re-exports), delegate import tracing to 1-3 parallel Explore sub-agents. Each agent traces a subset of the import tree. Merge their deduplicated file lists. Also delegate git history analysis to a separate sub-agent if the file list exceeds 30 modules.

### Phase 2: SCENARIO MAP

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

Every later architectural inference must be tied back to scenario families. A finding not grounded in test behavior is speculation.

**Soak/endurance tests**: When the test is a single function that runs the simulation for many ticks and checks invariants, derive scenario families from the invariant categories and emergence assertions rather than from test function boundaries. Each per-tick invariant check (e.g., conservation, needs bounds, unique placement) and each emergence threshold check (e.g., death, trade, political events) becomes a scenario family.

**Sub-agent delegation**: For large test directories (>30 test files), delegate scenario extraction to 2-3 parallel Explore sub-agents, each handling a subset. Merge and deduplicate scenario families.

### Phase 3: TRACE

Build test-to-code traceability using multiple strategies — no single trick catches everything:

| Strategy | What it finds | Confidence |
|----------|--------------|------------|
| `use` statements | Direct dependencies | High |
| Static call graph (from `assert!`/`assert_eq!` back to production) | Functions actually exercised | High |
| Naming/lexical similarity (test helpers vs production functions) | Conceptual links | Medium |
| Temporal coupling from git history (files that co-change) | Hidden dependencies | Medium |

Each traceability link gets a confidence tag (high/medium/low) and a brief reason code.

The purpose of multi-strategy tracing is to catch hidden dependencies that `use` statements alone miss — trait dispatch, `SystemFn` registration, `register_action_handler` indirection, and temporal coupling are the most common sources of invisible links in this codebase.

**After short-circuit**: When Phase 1's short-circuit determined all modules are exercised, skip the `use` statement and call graph strategies — they add no value when the answer is "everything is exercised." Focus Phase 3 on temporal coupling analysis (files that co-change across commits) and the confidence/reason-code tagging of module-to-scenario links. The traceability table still provides value by mapping modules to scenario families with confidence levels.

### Phase 4: DETECT FRACTURES

Scan the exercised code for these 8 fracture types:

| # | Fracture Type | What to look for |
|---|--------------|-----------------|
| 1 | **Split protocol** | The legal sequence of interactions is spread across multiple modules/crates. Module A decides "what", module B decides "when", module C decides "whether". |
| 2 | **Authority leak** | Multiple modules write the same truth. Two or more places create/mutate/invalidate the same piece of state. |
| 3 | **Projection drift** | Derived summaries or cached computations are recomputed everywhere. No single module owns the projection. |
| 4 | **Boundary inversion** | Higher crates own rules that belong in lower crates. `worldwake-ai` enforces what `worldwake-core` or `worldwake-systems` should prevent. |
| 5 | **Concept aliasing** | The same domain concept exists under different names/types in neighboring crates (e.g., "goal kind" in `worldwake-ai`, "action type" in `worldwake-systems`, same semantic role). |
| 6 | **Hidden seam** | Files across nominal crate boundaries repeatedly change together in git history, suggesting they belong in the same module or crate. |
| 7 | **Overloaded abstraction** | One type/module carries several lifecycle roles that should be separated. A type that is "created, configured, activated, consumed, and cleaned up" but the type doesn't model these phases. |
| 8 | **Orphan compatibility layer** | A shim, fallback path, or "safety net" handler exists only to mask a deeper missing abstraction. |

**Evidence rule**: A fracture is NOT reported unless supported by at least two independent signals (e.g., import analysis + temporal coupling, or naming similarity + assertion patterns). Single-signal fractures go in a "Needs investigation" bucket, not in the main findings.

**Tool usage**: Grep for shared type names across crates, Grep for duplicate predicate patterns, Grep for `pub enum` and `match` expressions crossing module boundaries, Read key functions at boundary points, Bash for `git log` co-change analysis.

### Phase 5: SYNTHESIZE

For each validated fracture (two+ signals), produce a candidate abstraction:

- **title**: Descriptive name (e.g., "Goal Dispatch Protocol")
- **kind**: One of: Protocol | Authority boundary | Bounded context | Projection owner | Capability ledger | Workflow coordinator | Translation boundary | Lifecycle carrier
- **scope**: Which crates/modules it spans
- **owned_truth**: What state or invariant this abstraction would own (the single most important field — if you can't name this, the candidate is not ready)
- **invariants**: What must always be true when this abstraction is correctly implemented
- **owner_boundary**: Which crate/module should own it
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

**Filter 2 — FOUNDATIONS alignment.** For surviving candidates only, check against `docs/FOUNDATIONS.md`. The document defines principles in 5 categories (Causal Standard, World Dynamics, Knowledge/Belief/Evidence, Agents/Institutions/Social Order, System Architecture). For each relevant principle, note whether the candidate aligns, strains, or conflicts. Flag conflicts prominently — a candidate that violates FOUNDATIONS needs redesign before it becomes a spec.

This ordering matters. Recovery first, judgement second. Do not let FOUNDATIONS bias the fracture detection — detect what IS, then evaluate what SHOULD BE.

## Report Format

Write to `reports/architectural-abstractions-<date>-<context>.md`:

```markdown
# Architectural Abstraction Recovery: <context>

**Date**: <YYYY-MM-DD>
**Input**: <test path>
**Source modules analyzed**: <count>
**Crates touched**: <list>
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
| <file> | <families> | High/Med/Low | <use/naming/temporal/...> |

## Fracture Summary

| # | Fracture Type | Location | Evidence Sources | Severity |
|---|--------------|----------|-----------------|----------|
| 1 | <type> | <crates/modules involved> | <which signals> | HIGH/MEDIUM/LOW |

## Candidate Abstractions

### <Candidate Title>

**Kind**: <Protocol / Authority boundary / ...>
**Scope**: <crates spanned>
**Fractures addressed**: <which fracture(s) from the summary table>

**Owned truth**: <what this abstraction would own>
**Invariants**: <what must always hold>
**Owner boundary**: <which crate/module should own this>

**Modules affected**: <list of modules absorbed or constrained>
**Tests explained**: <which scenario families>
**Expected simplification**: <what gets cleaner>

**FOUNDATIONS alignment**:
- <Principle P<N>>: <aligned / strained / conflicts> — <brief explanation>

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
4. **Read-only.** Do not modify any source files. Do not run `cargo test` or any other test command. Static analysis and git history only.
5. **Do not invent problems.** "Acceptable complexity" must remain a valid and prominent outcome. If no fractures are found, say so clearly. An analysis that finds nothing wrong is a useful analysis.
6. **Every finding needs counter-evidence.** The report must say what would falsify each hypothesis. A finding without counter-evidence is an assertion, not an analysis.
7. **Recovery first, judgement second.** Build the scenario map and detect fractures BEFORE applying FOUNDATIONS principles. Do not let architectural ideals bias what you observe.
8. **Two-signal minimum.** No fracture is reported in the main findings unless supported by at least two independent evidence sources. Single-signal observations go in "Needs investigation."

## Important Rules

- This skill is READ-ONLY. Do not modify any source files.
- Do not run `cargo test` or any other test command. Static analysis and git history only.
- Do not write specs. Only write the report. Spec authoring is a separate step via `/assessment-to-specs`.
- Focus on cross-subsystem fractures. Single-concept scatter (e.g., "this function is duplicated in 5 files") is the domain of `detect-missing-abstractions`, not this skill.
- Always check against `docs/FOUNDATIONS.md` — but only in Phase 6, not earlier.
- The report should be actionable: each finding either needs a spec or doesn't.
- If a report already exists at the target path, overwrite it — each run produces a complete standalone report.
- If prior reports are provided, acknowledge already-known issues and focus analysis on NEW findings. Do not re-report what was already found.

## Workflow Context

Typically invoked after `/detect-missing-abstractions` when you suspect the issues run deeper than single-concept scatter. The full workflow is:

1. Implement spec
2. `/golden-gap-analysis` (coverage)
3. `/detect-missing-abstractions` (single-concept structural debt)
4. `/recover-architectural-abstractions` (cross-subsystem fractures)
5. `/assessment-to-specs` (spec drafting from proposals)

Output feeds into `/assessment-to-specs` for spec generation from proposals. Reports from `/detect-missing-abstractions` can be passed via `--prior-reports` to avoid rediscovery.
