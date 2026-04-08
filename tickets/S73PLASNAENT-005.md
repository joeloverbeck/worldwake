# S73PLASNAENT-005: Complete deferred soak-behavior validation for S73 cap/filter change

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S73PLASNAENT-002, S73PLASNAENT-003

## Problem

The active S73 spec still claims that the per-place entity cap and goal-aware filtering preserve soak behavioral outcomes, but `S73PLASNAENT-003` closed on the narrower aggregate seed-0 runtime proof after explicit user waiver of the longer `golden_soak` run. We need a bounded follow-up ticket that completes the deferred soak-behavior validation honestly instead of leaving the spec's broader validation claim ownerless.

## Assumption Reassessment (2026-04-08)

1. `tickets/S73PLASNAENT-003.md` now records a completed seed-0 aggregate runtime win (`194335` ms vs `341822` baseline) but explicitly states that `cargo test -p worldwake-ai --features soak --test golden_soak` was not completed by user choice.
2. The active spec `specs/S73-planning-snapshot-entity-relevance.md` still includes Validation item 4: "Per-place entity cap does not change soak behavioral outcomes when set to 50."
3. No active ticket currently owns that deferred soak-behavior proof. `tickets/S73PLASNAENT-004.md` only owns segmented benchmark telemetry for the early/late planning-cost claim.
4. The strongest existing behavior proof surface is `crates/worldwake-ai/tests/golden_soak.rs`, whose `t30_seven_day_soak` case exercises the seven-day soak contract behind the `soak` feature.

## Architecture Check

1. Keeping the deferred soak-behavior proof in its own verification ticket is cleaner than retroactively over-claiming `S73PLASNAENT-003`.
2. No production changes or compatibility paths are needed; this ticket is verification and handoff only.

## Verification Layers

1. Soak behavior remains valid under the S73 filter/cap change -> `cargo test -p worldwake-ai --features soak --test golden_soak`
2. Single-layer ticket: no additional layer mapping applies because this ticket owns verification only.

## What to Change

### 1. Run the deferred soak behavior proof

Execute `cargo test -p worldwake-ai --features soak --test golden_soak` at the intended proof scope and record whether the S73 filter/cap change preserves the soak invariants.

### 2. Update ticket handoff factually

Record the exact verification scope that completed, including any intentional sharding or narrower proof surface used, so the active S73 roadmap accurately reflects what was actually proved.

## Files to Touch

- `tickets/S73PLASNAENT-005.md` (modify — completion notes only)

## Out of Scope

- Benchmark telemetry instrumentation owned by `S73PLASNAENT-004`
- Updating aggregate performance baselines owned by `S73PLASNAENT-003`
- Production planner or cognitive-profile changes

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --features soak --test golden_soak`
2. Ticket outcome records the exact completed soak verification scope and result

### Invariants

1. No broader soak-behavior proof is claimed unless it was actually run
2. The deferred S73 behavior-validation slice has an active owner until completed

## Test Plan

### New/Modified Tests

None — verification is command-based and existing soak coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai --features soak --test golden_soak`
