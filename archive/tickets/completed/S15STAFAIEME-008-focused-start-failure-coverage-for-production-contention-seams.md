# S15STAFAIEME-008: Focused Start-Failure Coverage For Production Contention Seams

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Focused production coverage only; no runtime architecture changes required
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
   - `golden_contested_harvest_start_failure_recovers_via_remote_fallback`
   - `golden_contested_harvest_start_failure_recovers_via_remote_fallback_replays_deterministically`
4. The missing layer is focused/runtime coverage that explicitly names the current architectural contract for contested harvest:
   - second contender loses at start, not at commit
   - source stock has not yet been consumed when that start failure is recorded
   - depletion happens only after the winner later commits
5. Ordering is mixed-layer. The contract is not "later tick number" by itself. The contract is action-lifecycle ordering first, then authoritative source mutation ordering.
6. The currently missing proof is narrower than the original ticket language implied. The repository already has:
   - mixed-layer golden coverage for the remote recovery branch
   - focused production coverage for reservation/start blocking plus abort-time source preservation
   - focused production coverage for ordinary happy-path depletion at commit
7. The actual remaining gap is one focused production-layer test that composes those facts into a single seam proof without relying on the golden or on cross-file mental stitching.
8. The cleanest layer is [crates/worldwake-systems/src/production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs), not `tick_step.rs`. The contract here is production-specific authoritative start/commit behavior on a real harvest action, not generic scheduler best-effort plumbing.

## Architecture Check

1. Adding one seam-specific focused production test is cleaner than pushing more assertions into the golden. Goldens should prove emergence; focused tests should prove the exact authoritative seam.
2. The clean architecture is still the current one: start-time contention is resolved through the shared start gate and reservation layer, while stock depletion remains a later production commit effect. A focused test should make that distinction easier to preserve, not change it.
3. No backward-compatibility or production-only exception path should be added. If a focused test exposes a bug, fix the shared runtime or authoritative seam directly.

## Verification Layers

1. Second contender is rejected at start through the real harvest start path -> focused authoritative/runtime test in `crates/worldwake-systems/src/production_actions.rs`.
2. Source quantity is unchanged at the moment of failed second start -> focused authoritative world-state assertion in the same production test.
3. Source quantity decreases only after winner commit -> focused authoritative world-state assertion after running the winning harvest to completion in that same production test.
4. Mixed-layer remote recovery remains covered separately -> existing golden in `crates/worldwake-ai/tests/golden_production.rs`.

## What to Change

### 1. Add one focused seam test for contested harvest start vs depletion timing

Add a test that sets up two lawful harvest contenders on the same ordinary orchard and proves:

- the winner can start
- the loser receives the recoverable start-time failure for the same workstation
- the orchard source quantity is still unchanged immediately after the loser fails to start
- only later, after the winner commits, does the orchard source decrease

This should live in [crates/worldwake-systems/src/production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs), because the clean proof uses direct `start_action` plus normal harvest completion and lets the test assert the authoritative source state at the exact seam without involving broader scheduler behavior.

### 2. Tighten existing test names or comments if needed

If existing nearby tests are kept, update their names/comments so contributors can quickly see which ones prove:

- reservation/start blocking
- source preservation before commit
- source depletion at commit

### 3. Keep the golden as the higher-layer proof

Do not move remote fallback assertions into the focused test. Reference the golden as the mixed-layer companion rather than duplicating it.

## Files to Touch

- `crates/worldwake-systems/src/production_actions.rs` (modify, required for the seam-local proof)
- `crates/worldwake-ai/tests/golden_production.rs` (modify only if a tiny comment or cross-reference improves clarity; not for new seam assertions)

## Out of Scope

- redesigning harvest reservations or source consumption timing
- changing AI planner behavior
- replacing the golden with focused-only proof

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems production_actions::tests::harvest_second_start_failure_preserves_source_until_winner_commit -- --exact`
2. `cargo test -p worldwake-systems production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source -- --exact`
3. `cargo test -p worldwake-systems production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot -- --exact`
4. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The authoritative seam remains: reservation/start failure first, source depletion later at commit.
2. Focused coverage names the correct causal layer instead of inferring start-time facts from later source mutation.
3. No production-only workaround or alternate runtime path is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production_actions.rs` — add `harvest_second_start_failure_preserves_source_until_winner_commit`, a single focused seam test proving failed second start precedes any source depletion and that depletion happens only after the winner commits.
2. `crates/worldwake-systems/src/production_actions.rs` — optional name/comment tightening on adjacent reservation/depletion tests if that improves architectural clarity.

### Commands

1. `cargo test -p worldwake-systems production_actions::tests::harvest_second_start_failure_preserves_source_until_winner_commit -- --exact`
2. `cargo test -p worldwake-systems production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source -- --exact`
3. `cargo test -p worldwake-systems production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot -- --exact`
4. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completion date**: 2026-03-19
- **What actually changed**: Reassessed the ticket against the current tree, narrowed the scope to the real remaining gap, and added one focused production-layer test in `crates/worldwake-systems/src/production_actions.rs`: `harvest_second_start_failure_preserves_source_until_winner_commit`.
- **Deviations from original plan**: No `tick_step.rs` or golden-test changes were needed. The current architecture was already correct; the repo only needed a single combined seam proof at the production layer rather than broader runtime or E2E expansion.
- **Verification results**:
  - `cargo test -p worldwake-systems production_actions::tests::harvest_second_start_failure_preserves_source_until_winner_commit -- --exact`
  - `cargo test -p worldwake-systems production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source -- --exact`
  - `cargo test -p worldwake-systems production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot -- --exact`
  - `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
