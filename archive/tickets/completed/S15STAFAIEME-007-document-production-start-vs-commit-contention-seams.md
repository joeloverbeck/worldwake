# S15STAFAIEME-007: Document Production Start-Gate vs Commit-Time Contention Seams

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None; ticket correction and archival only
**Deps**: `docs/FOUNDATIONS.md`, `docs/golden-e2e-testing.md`, `specs/S15-start-failure-emergence-golden-suites.md`, `tickets/README.md`, archived `S15STAFAIEME-001`

## Problem

This ticket was written to clarify a real architectural seam in ordinary contested harvest:

1. shared start-gate / reservation failure at action start
2. later source depletion at harvest commit

After reassessing the current tree, that documentation gap is no longer an active implementation problem. The repo now already documents and tests the seam in the correct layers. The remaining work for this ticket is to correct its stale assumptions and archive it instead of duplicating already-landed guidance.

## Assumption Reassessment (2026-03-19)

1. The underlying architecture claim is correct. Ordinary contested harvest still splits across:
   - shared authoritative start handling in [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs)
   - later source consumption at commit in [production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs)
2. The focused/runtime proof surfaces named by this ticket already exist and still match current symbols:
   - `production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source` in [production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs)
   - `production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot` in [production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs)
   - `tick_step::tests::best_effort_request_drops_recoverable_start_failure_without_failing_tick` in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs)
3. The golden scenario this ticket treated as merely planned already exists in [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs): `golden_contested_harvest_start_failure_recovers_via_remote_fallback` plus deterministic replay coverage. That means the repo already has mixed-layer proof for the production branch.
4. The documentation gap described in the original ticket has largely already been closed:
   - [specs/S15-start-failure-emergence-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S15-start-failure-emergence-golden-suites.md) Scenario 26 now explicitly says both agents choose from the same snapshot, one fails at authoritative start, and the winner later commits the local harvest.
   - [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already teaches mixed-layer ordering, action-trace vs decision-trace separation, and warns against using later effects as a proxy for earlier divergence.
   - [tickets/README.md](/home/joeloverbeck/projects/worldwake/tickets/README.md) already requires mixed-layer tickets to map invariants to exact verification layers and to name ordering/substrate asymmetries explicitly.
5. Ordering here remains mixed-layer and not weight-only. Same-snapshot goal selection can still be symmetric while authoritative divergence happens first at start and only later at commit. That is the correct architecture and the current docs now describe it adequately.
6. The cleaner architecture remains the current one. A shared start-gate failure plus later commit-time depletion is more robust and extensible than collapsing both seams into a production-specific “opportunity vanished” story or inventing aliases for different failure phases.
7. Mismatch found and corrected: this ticket is stale. It no longer represents missing work in the current repo. The proper scope is ticket correction plus archival, not new doc/code changes.

## Architecture Check

1. The current architecture is preferable to any rewrite that would blur start-time reservation failure and commit-time source depletion into one path. Keeping those seams distinct preserves causal legibility and matches the shared action framework.
2. Opening new docs changes now would mostly duplicate existing guidance already split cleanly across the spec, golden testing conventions, focused tests, and the production golden.
3. No backward-compatibility aliasing or alternative explanatory path should be added just to keep this stale ticket “active.” The clean move is to retire it.

## Verification Layers

1. Start-time contested-harvest rejection uses the shared authoritative seam -> focused runtime coverage in [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) and [production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs)
2. Source quantity is consumed later at harvest commit -> focused authoritative world-state assertions in [production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs)
3. Mixed-layer production recovery after `StartFailed` exists end-to-end -> golden coverage in [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs)
4. Additional file changes are not applicable because the intended clarification work is already present elsewhere in the tree.

## What to Change

### 1. Correct the ticket scope

Update this ticket so it no longer claims missing documentation or missing production golden coverage that already exists in the repository.

### 2. Archive the ticket

Mark the ticket completed and archive it with an accurate outcome instead of creating duplicate doc churn.

## Files to Touch

- `tickets/S15STAFAIEME-007-document-production-start-vs-commit-contention-seams.md` (modify, then move to archive)

## Out of Scope

- changing harvest reservations, production commit semantics, or action framework behavior
- adding duplicate documentation for guidance that already exists
- adding tests for behavior already covered in the focused and golden layers

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
2. `cargo test -p worldwake-systems production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source -- --exact`
3. `cargo test -p worldwake-systems production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot -- --exact`
4. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_drops_recoverable_start_failure_without_failing_tick -- --exact`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The repo continues to describe contested harvest using the real layered architecture: start-gate failure first, source depletion later at commit.
2. No duplicate or conflicting explanation path is introduced just to preserve stale ticket wording.
3. Ticket archival accurately reflects that the implementation gap was already closed elsewhere in the tree.

## Test Plan

### New/Modified Tests

1. `None — no code or documentation behavior changes were required beyond correcting and archiving this stale ticket. Existing focused and golden coverage remains the verification surface.`

### Commands

1. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
2. `cargo test -p worldwake-systems production_actions::tests::harvest_reservation_blocks_second_actor_and_abort_preserves_source -- --exact`
3. `cargo test -p worldwake-systems production_actions::tests::harvest_happy_path_reduces_source_and_creates_output_lot -- --exact`
4. `cargo test -p worldwake-sim tick_step::tests::best_effort_request_drops_recoverable_start_failure_without_failing_tick -- --exact`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completion date**: 2026-03-19
- **What actually changed**: Reassessed the ticket against the current tree and corrected it to match reality. The claimed documentation gap had already been addressed by existing spec wording, golden authoring guidance, ticket authoring guidance, focused seam tests, and the production golden scenario.
- **Deviations from original plan**: No new doc or code edits were warranted. Compared with the original plan, the cleanest result was to retire this ticket instead of duplicating already-landed guidance.
- **Verification results**: Verified current test names and symbols in the repo; runtime and golden commands listed in this ticket were rerun during reassessment, along with workspace lint.
