---
name: detect-architectural-debt
description: "Analyze engine code exercised by a test suite to find missing abstractions and architectural fractures — from single-concept scatter to cross-subsystem authority confusion. Two parallel detection lenses (structural scatter + architectural fractures) with cross-lens reinforcement. Outputs a report compatible with /assessment-to-specs."
user-invocable: true
arguments:
  - name: test_path
    description: "Path to a test file or test directory (e.g., crates/worldwake-ai/tests/scenarios/trade_acquisition.rs or crates/worldwake-systems/tests/)"
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

Phase 4.5: VERIFY     — Spot-check agent claims against source code
Phase 5: SYNTHESIZE   — Cross-lens reinforcement + severity ranking
Phase 6: VALIDATE     — Survival criteria + FOUNDATIONS alignment
```

## Methodology

**Execution Strategy**: Phases 1 and 2 run in parallel. Phase 3 often collapses into Phase 1 when the short-circuit applies. Lens A and Lens B within Phase 4 run in parallel. For large analyses (>30 modules), launch up to 3 Explore agents in parallel for Phase 4. Phase 5 requires results from both lenses. Phase 6 requires FOUNDATIONS context and runs last.

1. **Phases 1-3: Gather, Map, Trace.** Load `references/phases-1-3-gather-map-trace.md`. Build the exercised module set, cluster tests into scenario families, and establish test-to-code traceability.

2. **Phase 4: Detect.** Load `references/phase-4-detection-lenses.md`. Run Lens A (structural scatter) and Lens B (architectural fractures) in parallel across the exercised modules.

3. **Phase 4.5: Verify.** Before synthesis, spot-check agent claims against actual source code. For each finding with severity >= Medium: (a) for "duplicated logic" claims, read at least one cited site to confirm the code is genuinely copy-pasted rather than trait implementations serving different data sources; (b) for enum variant counts or "overloaded abstraction" claims, verify the actual variant count at the `pub enum` definition site — do not trust grep-count estimates; (c) for "boundary inversion" claims, read the cited upper-layer code to confirm it accesses authoritative state rather than belief state. Demote findings whose evidence does not survive source-level verification to "Needs Investigation."

4. **Phases 5-6: Synthesize and Validate.** Load `references/phases-5-6-synthesis-validation.md`. Cross-reference both lenses, produce candidate abstractions, apply survival criteria, and check FOUNDATIONS alignment.

5. **Write Report.** Load `references/report-format.md`. Write the structured report to `reports/architectural-debt-<date>-<context>.md`.

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
