# S73PLASNAENT-006: Reconcile S73 late-window proof with NA telemetry outcome

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S73PLASNAENT-002, S73PLASNAENT-004

## Problem

The active S73 spec still claims that seed-0 planning cost no longer grows superlinearly with tick count, but `S73PLASNAENT-004` landed the benchmark telemetry surface and measured an honest seed-0 result of `late_to_early_planning_avg_ratio=NA` because the configured late window (`9000..10000`) had zero planning-phase samples. That leaves the active proof contract misaligned: we now have a truthful benchmark surface, but not a numeric late/early proof for the claim the spec currently makes.

## Assumption Reassessment (2026-04-08)

1. `archive/tickets/S73PLASNAENT-004.md` records that the benchmark now emits explicit early/late planning telemetry from `plan_and_validate_next_step(...)`, and the verified seed-0 run produced `late_planning_sample_count=0` and `late_to_early_planning_avg_ratio=NA`.
2. `specs/S73-planning-snapshot-entity-relevance.md` Validation item 3 still says: "Soak seed 0 planning cost does not grow superlinearly with tick count (the late-game spike is eliminated)."
3. No active ticket currently owns reconciling that validation claim with the measured `NA` result. `S73PLASNAENT-005` only owns deferred soak-behavior validation, not the late-window proof contract.
4. The mismatch may be resolved either by redefining the benchmark windows/metric to capture meaningful late planning samples or by updating the active spec/validation contract to match the real measured planner behavior. This ticket owns that reassessment and the narrowest honest fix.
5. Reassessment conclusion: the live benchmark surface already behaves honestly. The narrowest fix is spec-proof alignment, not more benchmark code churn. Validation item 3 in `specs/S73-planning-snapshot-entity-relevance.md` should describe the `NA` late-window outcome as a real result when no late planning samples exist.

## Architecture Check

1. Updating the active spec validation contract is cleaner than perturbing a benchmark surface that already reports honest data.
2. This keeps the benchmark and spec aligned without changing production planner behavior or inventing a stronger measurement claim than the live runner supports.

## Verification Layers

1. Active validation contract matches the real benchmark surface -> updated `specs/S73-planning-snapshot-entity-relevance.md` plus archived `S73PLASNAENT-004` measurement evidence
2. Single-layer ticket: benchmark/spec proof alignment only.

## What to Change

### 1. Reassess the late-window proof contract

Determine whether the right fix is:
- a benchmark-window/metric adjustment that still measures the intended late-game planning behavior, or
- a factual narrowing of the active S73 validation claim to match the measured `NA` outcome.

Reassessment result: use the second option. The benchmark already emits the strongest honest surface we have for seed 0.

### 2. Update the owned proof surfaces

Apply the narrowest honest change across the active proof surfaces so they no longer imply a numeric late/early proof that was not actually observed.

## Files to Touch

- `tickets/S73PLASNAENT-006.md` (modify — completion notes)
- `specs/S73-planning-snapshot-entity-relevance.md` (modify — validation contract alignment)

## Out of Scope

- Changing production snapshot-filter behavior from `S73PLASNAENT-002`
- Deferred soak-behavior validation owned by `S73PLASNAENT-005`
- Multi-seed telemetry sweeps unless reassessment proves they are the narrowest honest proof surface
- Benchmark code changes — not needed if the live runner already reports the strongest honest outcome

## Acceptance Criteria

### Tests That Must Pass

1. The active S73 proof contract no longer claims a numeric late/early result that the benchmark does not actually produce
2. The active spec explicitly treats `NA` as the honest late-window outcome when no late planning samples exist

### Invariants

1. Measured `NA` outcomes are treated as real results, not silently coerced into numeric proof
2. No active spec or ticket implies stronger late-game proof than the live benchmark surface supports

## Test Plan

### New/Modified Tests

None — this ticket is documentation/proof-contract alignment based on already-recorded benchmark evidence.

### Commands

1. None — no code changes required; verification relies on the benchmark evidence already recorded in `archive/tickets/S73PLASNAENT-004.md`

## Outcome

Completed on 2026-04-08.

- Updated `specs/S73-planning-snapshot-entity-relevance.md` Validation item 3 so the active proof contract matches the live benchmark surface.
- Preserved the existing benchmark behavior from `S73PLASNAENT-004`; no code changes were needed because the runner already emitted the honest seed-0 result (`late_to_early_planning_avg_ratio=NA`).

## Deviations

- Reassessment chose spec-proof alignment instead of more benchmark changes. The live telemetry surface was already truthful, so changing it again would have added churn without improving honesty.

## Verification Result

- Verified by reassessment against recorded evidence in `archive/tickets/S73PLASNAENT-004.md`
- No new code verification commands were needed because this ticket landed no production or benchmark code changes
