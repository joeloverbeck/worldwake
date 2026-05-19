# S154TESBINCON-002: Source moves + entry-binary plumbing

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None (test layout only; no `src/` changes)
**Deps**: 001

## Problem

54 separate `cargo test --test golden_*` integration-test binaries are the dominant on-disk-cost driver in `target/debug/deps/`. Consolidating them into a single `golden_ai` binary plus a single `integration_ai` binary recovers an expected 25–40 GiB but requires a coordinated rename: 54 + 9 source-file moves, 2 harness-directory renames, 2 new entry-binary `.rs` files, and ~51 internal harness-import edits.

The move must preserve every existing `#[test]` function, test name, and feature-gating (e.g., `soak_profiler.rs`'s `#![cfg(feature = "soak")]`).

## Assumption Reassessment (2026-05-19)

1. Reassessment confirmed: 54 `golden_*.rs` files, 9 non-golden top-level `.rs` files, 3 harness directories (`golden_harness/`, `golden_planner_pathology_harness/`, `golden_scenario_diagnostics_harness/`), and `fixtures/` exist in `crates/worldwake-ai/tests/`. 194 `#[test]` function names across the goldens are globally unique. 51 of 54 golden files declare `mod golden_harness;`; 2 declare `mod golden_planner_pathology_harness;` (`golden_planner_pathology.rs`, `golden_planner_pathology_degenerate.rs`); 2 declare `mod golden_scenario_diagnostics_harness;` (`golden_scenario_diagnostics_fixture.rs`, `golden_scenario_diagnostics_replay.rs`).
2. `soak_profiler.rs` carries `#![cfg(feature = "soak")]` as an inner attribute (line 9). This attribute remains valid when the file becomes `tests/integration/soak_profiler.rs` because inner cfg attributes on module files conditionally compile the module body — `mod soak_profiler;` in `integration/mod.rs` references an empty module when the feature is off, which compiles cleanly.
3. Shared boundary under audit: the test-binary layout that cargo discovers. Cargo treats every top-level `.rs` file in `tests/` as an integration-test binary; nested module declarations within those files (`mod scenarios; mod golden_harness;`) compose into a single binary's compilation unit. The post-move layout depends on this discovery contract — confirmed valid by Rust's integration-test conventions and exercised today by the existing `mod golden_harness;` pattern in 51 of 54 goldens.

## Architecture Check

1. Two binaries (not one) preserves the conceptual separation between "golden snapshot scenario tests" and "forensic/soak/conformance/policy tests" so failures route to the right cognitive bucket on test output. Both binaries still statically link the full `worldwake-{core,sim,systems,ai}` dep tree — two large binaries are vastly cheaper than 63 medium-sized ones (the deps are amortized once per binary, not 63 times).
2. The de-prefixing rename (`golden_foo.rs` → `scenarios/foo.rs`) eliminates the workflow muscle-memory cost of remembering that `golden_` is decorative; the post-move module path `scenarios::foo::*` already encodes the categorization.
3. The asymmetric harness rename (`golden_harness/` keeps its prefix; the two niche harnesses drop theirs) is justified in the spec: `golden_harness/` embodies the golden-snapshot harness contract used by 51 of 54 scenarios and reads naturally with the prefix; the two niche harnesses describe specific testing patterns (planner pathology, scenario diagnostics) that read more naturally without the prefix once scenario files no longer carry it.

## Verification Layers

1. Full test-count preservation → `cargo test -p worldwake-ai` reports the same total `#[test]` count after the move as before (194 in goldens + N in integrations)
2. Per-scenario filter still functions → `cargo test -p worldwake-ai --test golden_ai place_dirtiness` matches all tests in `scenarios::place_dirtiness::*` (substring filter against module path)
3. Feature-gated test compilation → `cargo test -p worldwake-ai --features soak` builds and runs the soak-gated tests inside the consolidated `integration_ai` binary
4. Binary inventory → after a clean build, `target/debug/deps/` contains exactly two per-spec binaries: `golden_ai-<hash>` and `integration_ai-<hash>`, plus zero `golden_<scenario>-<hash>` binaries
5. Disk-size measurement → `du -sh target/` after `./scripts/verify.sh` lands in the expected 8–15 GiB window (recorded in this ticket's Outcome section)
6. Single-layer ticket: pure test-layout restructuring with no engine effects. Action-trace / event-log / decision-trace verification surfaces are not applicable.

## What to Change

### 1. Create `scenarios/` submodule directory

Create `crates/worldwake-ai/tests/scenarios/` with `mod.rs` listing every scenario submodule (`pub mod merchant_selling; pub mod epistemic_sensing; …` — one line per de-prefixed scenario name).

### 2. Move 54 golden source files

Move each `tests/golden_<name>.rs` → `tests/scenarios/<name>.rs`. The `golden_` prefix is dropped from both the filename and the module name. `#[test]` function names inside each file are preserved unchanged.

### 3. Update internal harness imports per moved file

For each moved file:

- Drop the top-level `mod golden_harness;` declaration (it is now declared at the `golden_ai.rs` binary root; a sibling `mod` declaration here would shadow it).
- Change `use golden_harness::*;` (and `use golden_harness::<symbol>` and similar) to `use crate::golden_harness::*;` (and matching `use crate::golden_harness::<symbol>`). Same treatment for the planner-pathology and scenario-diagnostics harness imports in the 4 files that use them.

### 4. Create `integration/` submodule directory and move non-golden files

Create `crates/worldwake-ai/tests/integration/` with `mod.rs` listing every non-golden submodule (`pub mod forensic_determinism; pub mod soak_profiler; …`). Move the 9 non-golden top-level files into it. `soak_profiler.rs`'s `#![cfg(feature = "soak")]` inner attribute is preserved unchanged.

### 5. Create the two entry-binary `.rs` files

- `crates/worldwake-ai/tests/golden_ai.rs` containing exactly:

  ```rust
  mod golden_harness;
  mod planner_pathology_harness;
  mod scenario_diagnostics_harness;
  mod scenarios;
  ```

- `crates/worldwake-ai/tests/integration_ai.rs` containing exactly:

  ```rust
  mod integration;
  ```

### 6. Rename the two niche harness directories

- `golden_planner_pathology_harness/` → `planner_pathology_harness/`
- `golden_scenario_diagnostics_harness/` → `scenario_diagnostics_harness/`

`golden_harness/` keeps its current name and contents per spec rationale.

### 7. Audit for test-name collisions

Pre-move, scan the 194 golden test names plus the integration-binary test names for collisions across the formerly-separate binaries. Existing global uniqueness (194 names, 194 unique) covers the goldens, but integration tests must be checked too. Collisions are renamed with a `<scenario>_` prefix.

### 8. Audit for process-global shared state

Pre-move, grep each moved file for `static mut`, `std::env::set_var`, and `lazy_static!` with mutation. Per the spec's risk surface, zero such cases are expected; any found must be refactored to per-test setup/teardown before the merge.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/mod.rs` (new)
- `crates/worldwake-ai/tests/scenarios/<name>.rs` × 54 (new — moved from `tests/golden_<name>.rs`; old files removed)
- `crates/worldwake-ai/tests/integration/mod.rs` (new)
- `crates/worldwake-ai/tests/integration/<name>.rs` × 9 (new — moved from `tests/<name>.rs`; old files removed)
- `crates/worldwake-ai/tests/golden_ai.rs` (new)
- `crates/worldwake-ai/tests/integration_ai.rs` (new)
- `crates/worldwake-ai/tests/planner_pathology_harness/` (renamed from `golden_planner_pathology_harness/`)
- `crates/worldwake-ai/tests/scenario_diagnostics_harness/` (renamed from `golden_scenario_diagnostics_harness/`)
- `crates/worldwake-ai/tests/golden_harness/` (unchanged)
- `crates/worldwake-ai/tests/fixtures/` (unchanged)

## Out of Scope

- `scripts/golden_inventory.py` and `scripts/test_golden_inventory.py` dual-layout updates — already landed in ticket 001
- Regeneration of `docs/generated/*` — ticket 003's scope
- Retirement of ticket 001's dual-glob fallback — ticket 003's scope
- Doc edits to `docs/golden-e2e-testing.md`, `docs/debugging-traces.md`, `.claude/skills/*.md`, `CLAUDE.md` — ticket 005's scope
- `campaigns/golden-perf/harness.sh` rework — ticket 004's scope
- `worldwake-systems/tests/`, `worldwake-cli/tests/`, `worldwake-core/tests/` consolidation (per spec Non-Goals)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` passes with identical total `#[test]` count to pre-T2 (full goldens + integrations)
2. `cargo test -p worldwake-ai --test golden_ai place_dirtiness` filters to exactly the tests in `scenarios::place_dirtiness::*` (per-scenario substring filter works on the consolidated binary)
3. `cargo test -p worldwake-ai --features soak` builds and runs the soak-gated tests inside the consolidated `integration_ai` binary
4. After `cargo clean` and a clean rebuild, `ls target/debug/deps/ | grep -E '^golden_ai-|^integration_ai-' | wc -l` returns 2, and `ls target/debug/deps/ | grep -E '^golden_[a-z]' | grep -v '^golden_ai-' | wc -l` returns 0
5. Existing suite: `scripts/verify.sh` passes (full workspace lint + clippy + test)

### Invariants

1. Every pre-T2 `#[test]` function exists in the post-T2 layout with the same function name; the test count is conserved
2. Cargo's integration-test discovery sees exactly 2 entry-point binaries in `crates/worldwake-ai/tests/` (`golden_ai` + `integration_ai`); no other top-level `.rs` files remain
3. `soak_profiler.rs`'s feature gate (`#![cfg(feature = "soak")]`) continues to gate compilation — the module body is empty when the feature is off

## Test Plan

### New/Modified Tests

1. None — this is a structural move, not a behavioral change. The 194+ existing tests are preserved unchanged inside their new module paths and provide the regression coverage.

### Commands

1. `cargo test -p worldwake-ai` (verify full test count match)
2. `cargo test -p worldwake-ai --test golden_ai place_dirtiness` (verify per-scenario filter)
3. `cargo test -p worldwake-ai --features soak --test integration_ai soak_profiler` (verify feature-gated soak path)
4. `cargo clean && cargo test -p worldwake-ai` then `ls target/debug/deps/ | grep -E '^(golden_ai|integration_ai)-' | wc -l` returns 2
5. `du -sh target/` after `./scripts/verify.sh` — record in Outcome
6. `scripts/verify.sh` — full gate
