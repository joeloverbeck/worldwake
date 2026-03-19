# S15STAFAIEME-008: Focused Start-Failure Coverage For Production Contention Seams

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None expected; focused/runtime coverage only unless a hidden defect is exposed
**Deps**: `docs/FOUNDATIONS.md`, `specs/S15-start-failure-emergence-golden-suites.md`, archived `S15STAFAIEME-001`

## Problem

The new production golden now proves the mixed-layer emergent chain for contested harvest start failure and remote recovery. What it does not do is isolate the seam distinction in a focused test surface:

1. same-workstation start-time contention fails through the shared start gate
2. source quantity is still unchanged at that failure moment
3. source depletion occurs only later when a lawful harvest commits

We already have nearby focused coverage, but not one explicit seam-oriented proof that ties those facts together in the language future contributors need.

## Assumption Reassessment (2026-03-19)

1. Existing focused production coverage in [crates/worldwake-systems/src/production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs) already includes:
   - `production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source`
   - `production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot`
2. Existing shared start-failure plumbing coverage in [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) includes:
   - `tick_step::tests::best_effort_request_drops_recoverable_start_failure_without_failing_tick`
   - `tick_step::tests::action_trace_assigns_explicit_order_across_started_failed_and_committed_events`
3. Existing golden coverage in [crates/worldwake-ai/tests/golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs) now proves the end-to-end contested-harvest recovery branch, but that is intentionally broader than a seam-localized regression.
4. The missing layer is focused/runtime coverage that explicitly names the current architectural contract for contested harvest:
   - second contender loses at start, not at commit
   - source stock has not yet been consumed when that start failure is recorded
   - depletion happens only after the winner later commits
5. Ordering is mixed-layer. The contract is not "later tick number" by itself. The contract is action-lifecycle ordering first, then authoritative source mutation ordering.
6. This ticket should not duplicate the golden or replace it. It should provide the narrowest seam proof that future contributors can use before escalating to mixed-layer debugging.
7. Mismatch avoided: because nearby focused tests already exist, this ticket should refine and compose them rather than claim the repo has no focused coverage at all.

## Architecture Check

1. Adding seam-specific focused coverage is cleaner than pushing more assertions into the golden. Goldens should prove emergence; focused tests should prove the exact authoritative seam.
2. The recommended tests strengthen understanding of the current architecture rather than trying to force a new one. That is more robust and extensible than broadening golden assertions until they become brittle.
3. No backward-compatibility or production-only exception path should be added. If a focused test exposes a bug, fix the shared runtime or authoritative seam directly.

## Verification Layers

1. Second contender is rejected at start through the real shared seam -> focused runtime test in `worldwake-sim` or `worldwake-systems`.
2. Source quantity is unchanged at the moment of failed second start -> focused authoritative world-state assertion in the same test.
3. Source quantity decreases only after winner commit -> focused authoritative world-state assertion after ticking/commit progression.
4. Mixed-layer remote recovery remains covered separately -> existing golden in `crates/worldwake-ai/tests/golden_production.rs`.

## What to Change

### 1. Add one focused seam test for contested harvest start vs depletion timing

Add a test that sets up two lawful harvest contenders on the same ordinary orchard and proves:

- the winner can start
- the loser receives the recoverable start-time failure for the same workstation
- the orchard source quantity is still unchanged immediately after the loser fails to start
- only later, after the winner commits, does the orchard source decrease

This can live in [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) if the cleanest proof uses `BestEffort` input drain plus scheduler/action-trace surfaces, or in [crates/worldwake-systems/src/production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs) if the cleanest proof uses direct `start_action` / `tick_action` / `commit` progression. Choose the narrower layer that expresses the contract most directly.

### 2. Tighten existing test names or comments if needed

If existing nearby tests are kept, update their names/comments so contributors can quickly see which ones prove:

- reservation/start blocking
- source preservation before commit
- source depletion at commit

### 3. Keep the golden as the higher-layer proof

Do not move remote fallback assertions into the focused test. Reference the golden as the mixed-layer companion rather than duplicating it.

## Files to Touch

- `crates/worldwake-sim/src/tick_step.rs` (modify, if this is the cleanest seam-local proof)
- `crates/worldwake-systems/src/production_actions.rs` (modify, if this is the cleanest seam-local proof)
- `crates/worldwake-ai/tests/golden_production.rs` (modify only if a tiny comment or cross-reference improves clarity; not for new seam assertions)

## Out of Scope

- redesigning harvest reservations or source consumption timing
- changing AI planner behavior
- replacing the golden with focused-only proof

## Acceptance Criteria

### Tests That Must Pass

1. New focused test proving second-start failure occurs before source depletion in ordinary contested harvest
2. `cargo test -p worldwake-systems production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source -- --exact`
3. `cargo test -p worldwake-systems production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot -- --exact`
4. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`

### Invariants

1. The authoritative seam remains: reservation/start failure first, source depletion later at commit.
2. Focused coverage names the correct causal layer instead of inferring start-time facts from later source mutation.
3. No production-only workaround or alternate runtime path is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_step.rs` or `crates/worldwake-systems/src/production_actions.rs` — add a focused seam test proving failed second start precedes any source depletion.
2. `crates/worldwake-systems/src/production_actions.rs` — optional name/comment tightening on adjacent reservation/depletion tests if that improves architectural clarity.

### Commands

1. `cargo test -p worldwake-sim -- --list`
2. `cargo test -p worldwake-systems -- --list`
3. `cargo test -p worldwake-systems production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source -- --exact`
4. `cargo test -p worldwake-systems production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot -- --exact`
5. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
