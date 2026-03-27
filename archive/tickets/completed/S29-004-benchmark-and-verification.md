# S29-004: Benchmarking and Full Verification

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — benchmark coverage, spec/ticket archival, implementation-order docs
**Deps**: S29-002, S29-003

## Problem

The spec requires before/after benchmarks confirming that the structural-sharing optimization delivers measurable end-to-end planning improvement without changing behavior. This ticket closes out S29 by adding benchmark coverage at the existing golden-scenario ownership boundary, collecting baseline/optimized measurements, running the relevant verification suite, and updating/archiving the planning docs.

## Assumption Reassessment (2026-03-27)

1. `S29-001`, `S29-002`, and `S29-003` are already implemented on `main` (`edc9802`, `b8f2b75`, `1bc062c`). The live code already uses `SharedMap`/`SharedSet`/`SharedVec` in [`crates/worldwake-ai/src/shared_collections.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/shared_collections.rs), [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs), and [`crates/worldwake-ai/src/search/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/mod.rs). This ticket is closeout/verification, not a speculative benchmark placeholder.
2. Rust nightly `#[bench]` is still unavailable on this stable workspace, and `criterion` is not present in any `Cargo.toml`. The correct fit remains `#[test] #[ignore]` wall-clock measurements run explicitly via `cargo test -- --ignored`, with no new dependency surface.
3. The current golden layout already contains the scenario ownership boundaries this ticket needs: `run_world_runs_without_observers()` in [`crates/worldwake-ai/tests/golden_determinism.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs), `run_stale_prerequisite_belief_discovery_replan()` in [`crates/worldwake-ai/tests/golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs), and the branch-heavy `golden_bribe_support_coalition()` scenario in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs). A new integration test file would duplicate private golden setup and drift from the canonical scenario ownership boundary.
4. The benchmark contract is end-to-end planning/runtime scenario timing, not isolated `PlanningState::clone()` microbenching. That is the stronger architectural proof here because it measures the optimization at the real `search_plan` call sites the golden scenarios exercise.
5. This ticket is not code-free. Beyond benchmark tests, completion requires updating the active planning spec and implementation-order docs, then archiving the completed ticket/spec per [`docs/archival-workflow.md`](/home/joeloverbeck/projects/worldwake/docs/archival-workflow.md).
6. Baseline measurement must come from the parent of `edc9802` (the pre-S29 implementation commit), because that is the last commit before any structural-sharing code exists. The benchmark harness should therefore be cherry-picked or replicated into a temporary baseline worktree for apples-to-apples measurement instead of inferring “before” performance from memory.
7. Mismatch + correction: `Engine Changes: None` was inaccurate because this ticket necessarily adds ignored benchmark tests and updates planning/archive docs. Scope corrected accordingly.

## Architecture Check

1. Benchmark wrappers should live beside the canonical golden scenario runners they exercise, not in a new duplicate integration harness. That keeps the performance proof attached to the real scenario ownership boundary, avoids divergent setup logic, and is more extensible if those scenarios evolve.
2. `#[test] #[ignore]` functions remain the lightest viable mechanism: no new crate deps, no nightly, explicit opt-in via `cargo test -- --ignored`.
3. No backwards-compatibility shims or aliasing. The benchmark helpers are additive, and the doc/archive updates should mark S29 complete rather than layering a second “pending” path on top of the delivered code.

## Verification Layers

1. Measurable planning/runtime improvement -> ignored benchmark tests printing wall-clock timing for existing golden scenario runners
2. Default-budget deterministic scenario stability -> existing golden determinism assertions in [`crates/worldwake-ai/tests/golden_determinism.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs)
3. High-budget planner-chain stability -> existing golden supply-chain and offices assertions plus deterministic replay in [`crates/worldwake-ai/tests/golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) and [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs)
4. Workspace health -> `cargo test --workspace` and `cargo clippy --workspace`
5. Single-code-layer ticket with documentation closeout; no separate action-trace/event-log invariant is being changed by this ticket itself.

## What to Change

### 1. Add benchmark tests

Add `#[test] #[ignore]` benchmark wrappers in the existing golden test modules that already own the scenario builders/runners. Each wrapper should:
- reuse the canonical scenario runner directly
- measure wall-clock time via `std::time::Instant`
- print elapsed time via `eprintln!`
- assert the runner still completes successfully so the ignored test doubles as a real verification surface

Scenarios:
- Default-budget: `golden_world_runs_without_observers` (default `PlanningBudget`)
- High-budget: use both the `max_node_expansions=1024` stale-prerequisite supply-chain scenario and the `beam_width=16` branchy office coalition benchmark so the closeout records whether the optimization helps different higher-budget search shapes equally

### 2. Run and document results

Run the ignored benchmark tests on:
- baseline: the parent of `edc9802`
- optimized: current `main`

Record the measured timings and the comparison in the archived ticket/spec `Outcome` sections. Also update `specs/IMPLEMENTATION-ORDER.md` so S29 is no longer listed as pending.

## Files to Touch

- `crates/worldwake-ai/tests/golden_determinism.rs` (modify — ignored default-budget benchmark wrapper)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — ignored high-budget benchmark wrapper)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify — ignored branch-heavy office benchmark wrapper)
- `specs/S29-planning-state-structural-sharing.md` (modify, then archive)
- `specs/IMPLEMENTATION-ORDER.md` (modify)
- `tickets/S29-004-benchmark-and-verification.md` (modify, then archive)

## Out of Scope

- Adding `criterion` or any other benchmark crate as a dependency.
- Changing the structural-sharing production implementation in `planning_state.rs`, `search/*.rs`, or `shared_collections.rs` unless verification exposes a real defect.
- CI integration of benchmarks (these are manual, opt-in).
- Creating duplicate benchmark-only scenario harnesses when existing golden runners already own the canonical setup.
- Performance thresholds stricter than the spec’s “measurable reduction” requirement.

## Acceptance Criteria

### Tests That Must Pass

1. Ignored benchmark wrappers compile and run on current `main`.
2. Relevant existing golden suites pass unchanged on optimized code.
3. Full workspace: `cargo test --workspace`
4. `cargo clippy --workspace` reports no new warnings/errors.

### Invariants

1. Existing golden scenario behavior remains unchanged; this ticket must not alter the live planning/search semantics.
2. Benchmark coverage reuses canonical golden scenario setup rather than introducing a second benchmark-specific harness for the same scenarios.
3. Golden determinism remains intact across the optimized code path.
4. No new external dependencies introduced.
5. Benchmark tests do not run in the normal `cargo test` suite (they are `#[ignore]`).

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-ai/tests/golden_determinism.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs) — add an ignored benchmark wrapper for `run_world_runs_without_observers()` so the default-budget measurement stays attached to the canonical determinism scenario.
2. [`crates/worldwake-ai/tests/golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) — add an ignored benchmark wrapper for the `max_node_expansions=1024` stale-prerequisite scenario so the closeout includes a deeper prerequisite-aware search surface.
3. [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs) — add an ignored benchmark wrapper for the `beam_width=16` office coalition scenario so the closeout also measures a branch-heavy political search surface.

### Commands

1. `cargo test -p worldwake-ai --test golden_determinism bench_world_runs_without_observers -- --ignored --nocapture`
2. `cargo test -p worldwake-ai --test golden_supply_chain bench_high_budget_prerequisite_replan -- --ignored --nocapture`
3. `cargo test -p worldwake-ai --test golden_offices bench_branchy_office_coalition -- --ignored --nocapture`
4. `cargo test -p worldwake-ai --test golden_determinism golden_world_runs_without_observers`
5. `cargo test -p worldwake-ai --test golden_supply_chain golden_merchant_restocks_via_prerequisite_aware_craft`
6. `cargo test -p worldwake-ai --test golden_supply_chain golden_stale_prerequisite_belief_discovery_replan`
7. `cargo test -p worldwake-ai --test golden_offices golden_bribe_support_coalition`
8. `cargo test --workspace`
9. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-27
- What actually changed: added ignored benchmark wrappers in the existing golden determinism, supply-chain, and offices modules; collected pre-S29 baseline numbers from temporary worktree `1791bf0`; updated `specs/S29-planning-state-structural-sharing.md` and `specs/IMPLEMENTATION-ORDER.md`; archived the completed S29 planning material.
- Deviations from original plan: the original ticket assumed a new benchmark integration test file and expected clear improvement on the chosen higher-budget scenarios. The final implementation kept perf checks at the canonical golden ownership boundaries instead, and the benchmark results were mixed rather than uniformly positive.
- Benchmark results:
  - `bench_world_runs_without_observers`: `3.188s` average on pre-S29 `1791bf0` vs `1.908s` average on current `main` (about 40% faster)
  - `bench_high_budget_prerequisite_replan`: `425.9ms` average on pre-S29 vs `419.5ms` on current `main` (effectively flat)
  - `bench_branchy_office_coalition`: `160.0ms` average on pre-S29 vs `187.0ms` on current `main` (modest regression on this branch-heavy surface)
- Verification results: `cargo test -p worldwake-ai --test golden_determinism bench_world_runs_without_observers -- --ignored --nocapture`, `cargo test -p worldwake-ai --test golden_supply_chain bench_high_budget_prerequisite_replan -- --ignored --nocapture`, `cargo test -p worldwake-ai --test golden_offices bench_branchy_office_coalition -- --ignored --nocapture`, `cargo test --workspace`, and `cargo clippy --workspace` all passed.
