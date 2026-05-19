# S154: Test-Binary Consolidation for Build-Artifact Reduction

## Summary

Collapse the per-file integration-test fan-out in `crates/worldwake-ai/tests/`
from 63 top-level `.rs` files (and thus 63 separately-built debug binaries)
down to two entry-point binaries — `golden_ai.rs` (covering 54 scenarios) and
`integration_ai.rs` (covering 9 forensic/soak/conformance/policy tests) — while
preserving the one-source-file-per-scenario authoring layout under
`tests/scenarios/` and `tests/integration/` submodule directories.

This is a tooling-boundary change. It introduces no simulation behavior change,
no new components, no new actions, no new systems. It exists to recover disk
budget on space-constrained developer environments (WSL2, VMs).

## Phase

Developer Tooling (not phase-gated; independent of engine phase work).

## Status

COMPLETED.

## Crates

- **Modified**: `worldwake-ai` (test layout only; no `src/` changes).
- **No new crates.**
- **No external dependency changes.**

## Dependencies

No spec-level dependencies. Builds on the immediate Option-1 defaults landed
alongside this spec's drafting:

- Workspace `Cargo.toml` `[profile.dev]` / `[profile.test]` `debug =
  "line-tables-only"`.
- `scripts/verify.sh` exporting `CARGO_INCREMENTAL=0`.

These defaults cap `target/` at roughly half of the pre-change footprint
(~95 GiB → ~45–55 GiB after a full broad gate). This spec exists because the
structural cause of the remaining ~50 GiB is the 63-binary fan-out, and
defaults alone cannot fix it.

## Problem Statement

`cargo clean` after every spec was reclaiming **94,065 files / 95.2 GiB**.
Root-cause analysis identified three drivers, in descending order of
contribution:

1. **63-binary fan-out**: `crates/worldwake-ai/tests/` contains 54
   `golden_*.rs` and 9 non-golden top-level files. Each becomes its own debug
   integration-test binary that statically links the full
   `worldwake-{core,sim,systems,ai}` dep tree plus dev-deps. At hundreds of MB
   per binary, this alone produces 25–40 GiB.
2. **Full debug info in every binary** — addressed by the immediate defaults
   (line-tables-only).
3. **Per-crate × per-profile incremental caches** — addressed by the immediate
   defaults (`CARGO_INCREMENTAL=0` in `verify.sh`).

The defaults eliminate (2) and (3) but leave (1). This spec eliminates (1).

## Context

Existing test layout (`crates/worldwake-ai/tests/`):

- 54 top-level `golden_*.rs` source files, each generating its own integration
  binary.
- 9 top-level non-golden source files: three `forensic_*.rs`
  (`forensic_determinism.rs`, `forensic_sleep_progress_barrier.rs`,
  `forensic_wash_vs_water_competition.rs`), two `conformance_*.rs`
  (`conformance_execution_budget.rs`, `conformance_motive_sources.rs`), plus
  `planner_conformance.rs`, `goal_schema_methods.rs`,
  `htn_registry_validation.rs`, and `soak_profiler.rs`.
- 3 shared harness directories already using Rust's
  non-test-target convention: `golden_harness/`,
  `golden_planner_pathology_harness/`,
  `golden_scenario_diagnostics_harness/`. These are statically duplicated into
  every binary that does `mod golden_harness;`.
- 1 `fixtures/` directory (RON scenarios + expected JSON), used by reference,
  not built.

`crates/worldwake-systems/tests/` (4 files) and `crates/worldwake-cli/tests/`
(4 files) also fan out but at single-digit cost; this spec leaves them
as-is, with a note in §Non-Goals.

## Target Architecture

```
crates/worldwake-ai/tests/
├── golden_ai.rs                      ← single binary entry for all 54 goldens
│   contents:
│     mod golden_harness;
│     mod planner_pathology_harness;
│     mod scenario_diagnostics_harness;
│     mod scenarios;
├── integration_ai.rs                 ← single binary entry for 9 non-goldens
│   contents:
│     mod golden_harness;
│     mod integration;
├── scenarios/
│   ├── mod.rs                        ← `pub mod merchant_selling; pub mod epistemic_sensing; ...`
│   ├── merchant_selling.rs           ← moved from golden_merchant_selling.rs; #[test]s unchanged
│   ├── epistemic_sensing.rs
│   └── ... (52 more)
├── integration/
│   ├── mod.rs                        ← lists forensic_determinism, soak_profiler, etc.
│   ├── forensic_determinism.rs
│   └── ... (8 more)
├── golden_harness/                   ← unchanged
├── planner_pathology_harness/        ← renamed from golden_planner_pathology_harness/
├── scenario_diagnostics_harness/     ← renamed from golden_scenario_diagnostics_harness/
└── fixtures/                         ← unchanged
```

Cargo discovers `#[test]` functions anywhere in the binary regardless of
nesting depth. Per-scenario files retain their existing `#[test] fn name()`
form. Test names remain globally unique within each binary (they are already
de-facto unique today via descriptive naming).

Why two binaries, not one: keeps the conceptual separation between "golden
snapshot scenario tests" and "forensic/soak/conformance/policy tests" so
failures route to the right cognitive bucket. Both binaries still statically
link the full dep tree — two large binaries are vastly cheaper than 63
medium-sized ones.

## Non-Goals

- **`worldwake-systems/tests/` consolidation** (4 files): small fan-out;
  marginal disk win; deferred. A future ticket may revisit.
- **`worldwake-cli/tests/` consolidation** (4 files): same reasoning;
  deferred.
- **`worldwake-core/tests/` consolidation** (3 files): not worth touching.
- **`worldwake-visualizer` workspace surgery**: separate concern, distinct
  spec when prioritized.
- **`cargo-sweep` integration**: documented in
  `docs/cargo-artifact-hygiene.md` as user-installable tooling; not enforced.

## Blast-Radius Inventory

The per-file `golden_*.rs` layout is consumed by the following tooling and
docs. All require coordinated update; the spec is decomposed into tickets to
land them in safe order.

| Consumer | Coupling | Breakage on consolidation | Owner |
|---|---|---|---|
| `scripts/golden_inventory.py` | Globs `tests/golden_*.rs`; runs `cargo test --test <stem>` per file; parses `Running tests/golden_*.rs (target/debug/deps/...)`; maps filename → per-file detail page via `_file_stem_to_detail_name()` | Breaks. Rewrite glob to `tests/scenarios/*.rs`; replace 54 per-file cargo invocations with one `cargo test --test golden_ai -- --list`; reshape per-binary parsing to per-source-file grouping. | T1 |
| `scripts/test_golden_inventory.py` | Unit tests for the above (test fixtures embed the `Running tests/golden_*.rs` cargo-output format). | Breaks. Rewrite fixtures + expected outputs. | T1 |
| `campaigns/golden-perf/harness.sh` | `find tests/ -name 'golden_*.rs'` then `cargo test --test <suite>` per file; ranks the 5 slowest test **binaries**. | Conceptually broken after consolidation (single binary). T4 retired the dormant campaign rather than introducing a nightly-only per-scenario timing replacement. | T4 |
| `docs/generated/golden-e2e-inventory.md` | Auto-generated; per-file table; "Golden test files: N" summary line. | Regenerates fine once T1 lands; schema shifts (no per-file count — only scenarios). | T3 |
| `docs/generated/golden-scenario-index.md` | Scenario blocks across files; `Source: golden_foo.rs:NNN`. | Regenerates fine; source paths shift to `scenarios/foo.rs:NNN`. | T3 |
| `docs/generated/golden-scenario-details/*.md` | One markdown file per source file, named via `_file_stem_to_detail_name()`. | Regenerates fine with updated naming derivation. | T3 |
| `docs/generated/golden-coverage-matrix.md` | Scenario-keyed; no file-name coupling. | Regenerates unchanged. | T3 |
| `docs/golden-e2e-testing.md` | Multiple prose references to `tests/golden_*.rs` and the inventory workflow. | Doc edits — non-trivial. | T5 |
| `docs/debugging-traces.md` | References `cargo test --test golden_*` command patterns. | Doc edits — instruction examples shift. | T5 |
| `docs/plans/*` (5 active plan docs) | Read-only references to the convention. | No edit required; references describe past state at time of writing. | — |
| `.claude/skills/*` (5 skills: `detect-architectural-debt`, `reassess-spec`, `simulation-remediation`, `goap-architecture-report`, `implement-ticket`) | Reference `tests/golden_*.rs` as a location pattern for AI agents. | Light edits to agent guidance — not enforcement. | T5 |
| Workflow muscle memory: `cargo test -p worldwake-ai --test golden_foo` | Every developer + every AI-agent invocation pattern. | Becomes `cargo test -p worldwake-ai --test golden_ai golden_foo`. Accepted cost; documented in T5. | T5 |

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-29 (Debuggability is a product feature) | Test binaries ARE the debuggability surface. This spec keeps every test, every assertion, every backtrace, every `--report-time` measurement intact. It only changes the on-disk packaging. Failure-attribution improves (the two-binary split makes the golden/forensic distinction explicit in test output). |
| FND-26 (System decoupling) | N/A for simulation systems. The dependency graph between `worldwake-{core,sim,systems,ai}` is unchanged. |
| FND-28 (No backward compatibility) | The old `cargo test --test golden_<scenario>` invocation form is *not* preserved via a shim. Users adopt `cargo test --test golden_ai <scenario>` (the `golden_` prefix is dropped during T2's file rename — the new substring filter matches the module path `scenarios::<scenario>::*`) after this lands. T5 sweeps every doc and skill so the new form is the only documented form. T3 additionally retires T1's transitional dual-glob in `golden_inventory.py` so no fossilized fallback path survives the migration. |
| All 28 simulation principles | N/A — no simulation code is touched. |

## Phased Decomposition (Tickets)

Each ticket is independently landable and individually verifiable. Order is
chosen to de-risk: tooling first (so the inventory script keeps reporting
correctly through the move), then the move itself, then the doc sweep.

```text
T1 (tooling rewrite)  ->  T2 (source moves)  ->  T3 (regenerate docs/generated/
                                                  + retire dual-glob fallback)
                                            ->  T4 (campaigns/golden-perf retirement)
                                            ->  T5 (hand-authored doc + skill sweep)
```

### T1: Tooling Rewrite (`scripts/golden_inventory.py` + `scripts/test_golden_inventory.py`)

**Deliverable**: Updated `golden_inventory.py` that:

- Discovers scenarios by globbing `crates/worldwake-ai/tests/scenarios/*.rs`
  AND (transitional, retired in T3) `crates/worldwake-ai/tests/golden_*.rs`.
  Both sources are merged into the per-source-file inventory while T2 is
  in flight.
- Runs a single `cargo test -p worldwake-ai --test golden_ai -- --list`
  invocation (when the new binary exists) instead of 54 per-file invocations.
  Falls back to the per-file invocation path while pre-T2 layout is still in
  place. Both transitional branches are removed in T3.
- Renders per-source-file detail pages identical in structure to today's
  output (one markdown file per scenario source, derived from
  `_file_stem_to_detail_name()` extended to accept both `golden_foo.rs` and
  `foo.rs` (post-move) → `foo.md`).
- `test_golden_inventory.py` test fixtures updated to cover both layouts and
  both cargo-output forms.

**Verification**: `python3 scripts/golden_inventory.py --write --check-docs`
exits 0 against the current (unchanged) test layout. Generated artifacts in
`docs/generated/` remain stable unless the live source tree already contains
pre-existing generated-doc drift; in that case T1 may refresh that drift while
leaving post-T2 regeneration to T3. Unit tests pass.

This ticket lands first because it converts the tooling from a layout-coupled
implementation to a layout-agnostic one. After T1, T2 can land without
breaking docs/generated.

### T2: Source Moves + Entry-Binary Plumbing

**Deliverable**:

- Create `crates/worldwake-ai/tests/scenarios/` directory; add
  `scenarios/mod.rs` listing every scenario submodule.
- Move each of the 54 `tests/golden_*.rs` files into `tests/scenarios/<name>.rs`
  (drop the `golden_` prefix; the module name becomes the de-prefixed stem).
- Create `crates/worldwake-ai/tests/integration/` analogously; move the 9
  non-golden top-level files into it.
- Create `crates/worldwake-ai/tests/golden_ai.rs` with
  `mod golden_harness; mod planner_pathology_harness; mod scenario_diagnostics_harness; mod scenarios;`.
- Create `crates/worldwake-ai/tests/integration_ai.rs` with
  `mod golden_harness; mod integration;` because several non-golden
  integration tests use the shared golden helper module.
- Rename `golden_planner_pathology_harness/` →
  `planner_pathology_harness/`; rename `golden_scenario_diagnostics_harness/`
  → `scenario_diagnostics_harness/`. `golden_harness/` keeps its prefix
  because it embodies the golden-snapshot harness contract (used by 51 of
  54 scenarios) and reads naturally with the prefix; the two niche
  harnesses describe specific testing patterns (planner pathology,
  scenario diagnostics) that read more naturally without the prefix once
  test files themselves no longer carry it.
- Per moved file: drop `mod golden_harness;` (now declared at the binary
  root); change `use golden_harness::*;` to `use crate::golden_harness::*;`.
  Same treatment for pathology and diagnostics harness imports.

**Verification**:

- `cargo test -p worldwake-ai` passes with all authored scenario and
  integration tests reachable under `golden_ai` and `integration_ai`. Raw
  helper self-test execution count is not identical because consolidation
  intentionally removes repeated helper-module self-test execution that came
  from declaring the same helper module in many top-level binaries.
- `cargo test -p worldwake-ai --test golden_ai <scenario>` works as a
  per-scenario filter on each pre-existing scenario name. The bare
  `<scenario>` is the de-prefixed module name (e.g.,
  `cargo test --test golden_ai place_dirtiness` for the file previously
  named `golden_place_dirtiness.rs`, now at
  `tests/scenarios/place_dirtiness.rs`); cargo passes it as a substring
  filter against the test path `scenarios::place_dirtiness::*`. Use
  `scenarios::<scenario>` if a more precise path filter is needed.
- `cargo test -p worldwake-ai --features soak --test integration_ai soak_profiler`
  still passes for the soak-gated profiler test.
- `target/debug/deps/` contains `golden_ai-<hash>` and
  `integration_ai-<hash>` and **no** `golden_<scenario>-<hash>` binaries
  after a clean build.
- Disk-size measurement: `du -sh target/` after a full `./scripts/verify.sh`
  recorded in the ticket's Outcome section. Expected: ~8–15 GiB.

**Risk surface**:

- Process-global shared state inside the consolidated binary. Audit each
  moved file for `static mut`, `std::env::set_var` calls, or `lazy_static!`
  with mutation; isolate or refactor as needed (expected zero such cases
  given the codebase's deterministic-by-construction discipline).
- Test name collisions across previously-separate binaries: a scan during
  T2 confirms uniqueness; collisions are renamed with a `<scenario>_`
  prefix.

### T3: Generated Doc Regeneration + Transitional-Fallback Removal

**Deliverable**:

1. Run `python3 scripts/golden_inventory.py --write --check-docs` against the
   new layout. Commit the refreshed:
   - `docs/generated/golden-e2e-inventory.md`
   - `docs/generated/golden-scenario-index.md`
   - `docs/generated/golden-scenario-details/*.md`
   - `docs/generated/golden-coverage-matrix.md`
2. Retire T1's transitional dual-glob from `golden_inventory.py`: drop the
   `tests/golden_*.rs` glob branch and the per-file
   `cargo test --test <stem>` invocation path; the script now globs only
   `tests/scenarios/*.rs` and invokes the single
   `cargo test -p worldwake-ai --test golden_ai -- --list`. Update
   `test_golden_inventory.py` fixtures to cover only the new layout
   (drop the pre-T2 cargo-output format fixtures introduced in T1). Per
   FND-28 (no backward compatibility in live authority paths), the
   transitional fallback exists only to bridge T1 → T2; once T2 lands,
   the dead branch must be removed rather than fossilized.

**Verification**: (1) re-run `golden_inventory.py --check-docs` exits 0
(drift-free). (2) `grep -n "golden_\\*\\.rs" scripts/golden_inventory.py
scripts/test_golden_inventory.py` returns matches only in docstrings or
generated-output strings, never in glob/invocation code paths. (3)
`scripts/test_golden_inventory.py` passes.

### T4: `campaigns/golden-perf/harness.sh` Retirement

**Deliverable**: One of:

1. **Repurpose**: rewrite the harness to drive `cargo test -p worldwake-ai
   --test golden_ai -- -Z unstable-options --report-time --test-threads=1`,
   parse the per-test duration output, rank the 5 slowest scenarios. (Requires
   nightly-only flag or alternative timing surface — investigated during ticket.)
2. **Retire**: if the campaign infrastructure is no longer actively used,
   delete `campaigns/golden-perf/` entirely and remove any references.

T4 outcome: retired. Live implementation found no CI/workflow invocation, no
live doc/script references outside this spec and the active T4 ticket, only
March 2026 campaign commits, and a header-only `results.tsv`. The stale
per-binary timing campaign was removed instead of keeping a dead or nightly-only
replacement surface.

**Verification**: removal is clean (no dangling references).

### T5: Hand-Authored Doc + Skill Sweep

**Deliverable**: edit:

- `docs/golden-e2e-testing.md`: replace `tests/golden_*.rs` with
  `tests/scenarios/*.rs`; update workflow commands.
- `docs/debugging-traces.md`: update `cargo test --test golden_*` example
  patterns to the new filter form.
- `docs/scenario-roadmap.md`: update active backing-golden links and auxiliary
  golden references to `tests/scenarios/*.rs`.
- `docs/cargo-artifact-hygiene.md`: remove now-stale pre-S154 bloat wording
  and update one-off golden command examples to `golden_ai` filters.
- `.claude/skills/detect-architectural-debt/SKILL.md`: update location
  references.
- `.claude/skills/reassess-spec/SKILL.md`: same.
- `.claude/skills/simulation-remediation/SKILL.md`: same.
- `.claude/skills/goap-architecture-report/SKILL.md`: same.
- `.claude/skills/implement-ticket/SKILL.md`: same.
- `.claude/skills/fix-ci-failures/SKILL.md`: update golden CI repro and
  fixture-regeneration commands to `golden_ai` filters.
- Other active `.claude/skills/` guidance that still treats
  `golden_*.rs` as the source-file layout: update to
  `tests/scenarios/*.rs` while preserving valid `golden_harness/` helper
  paths and `fn golden_*` test-name conventions.
- `CLAUDE.md`: ensure no stale references remain.

**Verification**: grep across repo for `tests/golden_\*\.rs` and
`--test golden_[^a]` invocations returns only intentional historical
references (commits, archived specs, prior plan docs) plus valid
`tests/golden_harness/` helper paths and `fn golden_*` test names.

## Explicit Opt-Outs from `docs/spec-drafting-rules.md`

This is a tooling-boundary spec: it introduces no new simulation state, laws,
or agent behavior. The following spec-drafting rules are documented as N/A
with explicit reasoning:

| Rule | Status | Reason |
|------|--------|--------|
| FND-01 Section H causal-hooks analysis | N/A | Introduces zero new simulation entities, relations, actions, information paths, conserved quantities, scarce capacities, feedback loops, lifecycle states, or boundary conditions. Section H analyzes a proposed *world-system*; there is no world-system here. |
| `Permille` for [0,1] or [0,1000] range values | N/A | No numeric range values introduced. All changes are file-layout, build-config, and tooling. |
| Profile-driven parameters | N/A | No per-agent behavior tunables introduced. |
| SystemFn integration | N/A | Not a simulation system. Test binaries register no `SystemFn`. |
| Component registration | N/A | No ECS components defined or modified. |
| Cross-system interactions (via FND-26) | N/A | No cross-system interactions. Test binaries depend on the same `worldwake-*` crate graph as today. |

## Rollout

Single tooling wave; no phase gate.

1. **T1** (tooling rewrite): land first. After this, `docs/generated/*` is
   regenerable from either layout — de-risks T2.
2. **T2** (source moves): land second. The big diff. After this, on-disk test
   layout matches the target.
3. **T3** (regenerate generated docs + retire dual-glob fallback): land
   alongside or immediately after T2. The fallback removal completes the
   FND-28 migration discipline — no fossilized dead branch in
   `golden_inventory.py` survives.
4. **T4** (campaigns harness): independent; can land anytime after T2.
5. **T5** (hand-authored doc + skill sweep): land last. The new commands and
   paths are stable by this point.

Each ticket is reviewable and revertable on its own. If T2 reveals an
unexpected blocker (e.g., a test that depends on process-global state across
formerly-isolated binaries), it can be paused without leaving T1 or the
defaults in a broken state.

## Outcome

Completed: 2026-05-19

What changed:

- The `worldwake-ai` test fan-out was consolidated from 63 top-level
  integration-test files into `golden_ai.rs` and `integration_ai.rs`, while
  preserving per-scenario authoring under `tests/scenarios/` and
  per-integration authoring under `tests/integration/`.
- `scripts/golden_inventory.py` and `scripts/test_golden_inventory.py` now use
  the consolidated `golden_ai` list surface and no longer keep transitional
  pre-S154 source-layout fallbacks.
- Generated golden docs were regenerated against the new layout.
- The obsolete `campaigns/golden-perf/` harness was retired after live review
  found no active consumer.
- Hand-authored docs, `CLAUDE.md`, and active `.claude/skills/*` guidance now
  document `crates/worldwake-ai/tests/scenarios/*.rs` and
  `cargo test -p worldwake-ai --test golden_ai <scenario>` as the active golden
  source and invocation forms, while preserving valid `golden_harness/` helper
  paths and `fn golden_*` test-name conventions.

Deviations from original plan:

- T4 retired the dormant `campaigns/golden-perf/` campaign instead of replacing
  it with a nightly-only per-scenario timing harness.
- T5 expanded beyond the original five `.claude/skills/*` entries after live
  reassessment found additional active guidance and roadmap docs still carrying
  old golden path or command contracts.
- Historical `docs/plans/*`, `archive/specs/*`, and `archive/tickets/*` records
  were left unchanged unless they were direct S154 handoff surfaces.

Verification:

- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `python3 scripts/test_golden_inventory.py`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo test -p worldwake-ai --test golden_ai place_dirtiness`.
- Passed `cargo test -p worldwake-ai --features soak --test integration_ai soak_profiler`.
- Passed `git diff --check` for each ticket closeout/archive step.
- Passed `rg -n 'tests/golden_|golden_[a-z0-9_]+\.rs|golden_\*\.rs|--test golden_[^a]' docs .claude/skills CLAUDE.md`, with only historical `docs/plans/*`, valid `tests/golden_harness/`, and valid `fn golden_*` references remaining.
- Passed `rg -n 'cargo test.*--test golden_[^a]' docs .claude/skills CLAUDE.md`, with only historical `docs/plans/*` matches remaining.
- Passed `./scripts/verify.sh`.
