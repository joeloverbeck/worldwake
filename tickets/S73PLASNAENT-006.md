# S73PLASNAENT-006: Reconcile S73 late-window proof with NA telemetry outcome

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — benchmark telemetry or spec-proof contract alignment
**Deps**: S73PLASNAENT-002, S73PLASNAENT-004

## Problem

The active S73 spec still claims that seed-0 planning cost no longer grows superlinearly with tick count, but `S73PLASNAENT-004` landed the benchmark telemetry surface and measured an honest seed-0 result of `late_to_early_planning_avg_ratio=NA` because the configured late window (`9000..10000`) had zero planning-phase samples. That leaves the active proof contract misaligned: we now have a truthful benchmark surface, but not a numeric late/early proof for the claim the spec currently makes.

## Assumption Reassessment (2026-04-08)

1. `archive/tickets/S73PLASNAENT-004.md` records that the benchmark now emits explicit early/late planning telemetry from `plan_and_validate_next_step(...)`, and the verified seed-0 run produced `late_planning_sample_count=0` and `late_to_early_planning_avg_ratio=NA`.
2. `specs/S73-planning-snapshot-entity-relevance.md` Validation item 3 still says: "Soak seed 0 planning cost does not grow superlinearly with tick count (the late-game spike is eliminated)."
3. No active ticket currently owns reconciling that validation claim with the measured `NA` result. `S73PLASNAENT-005` only owns deferred soak-behavior validation, not the late-window proof contract.
4. The mismatch may be resolved either by redefining the benchmark windows/metric to capture meaningful late planning samples or by updating the active spec/validation contract to match the real measured planner behavior. This ticket owns that reassessment and the narrowest honest fix.

## Architecture Check

1. A dedicated follow-up is cleaner than silently treating `NA` as equivalent to a numeric late/early proof.
2. The ticket keeps the benchmark and spec aligned without changing production planner behavior unless reassessment proves a telemetry surface adjustment is required.

## Verification Layers

1. Active validation contract matches the real benchmark surface -> updated ticket/spec/benchmark references plus focused `soak_seed_perf` output when needed
2. Any adjusted telemetry contract still emits honest deterministic output -> `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
3. Single-layer ticket: benchmark/spec proof alignment only.

## What to Change

### 1. Reassess the late-window proof contract

Determine whether the right fix is:
- a benchmark-window/metric adjustment that still measures the intended late-game planning behavior, or
- a factual narrowing of the active S73 validation claim to match the measured `NA` outcome.

### 2. Update the owned proof surfaces

Apply the narrowest honest change across the active ticket/spec/benchmark handoff surfaces so they no longer imply a numeric late/early proof that was not actually observed.

## Files to Touch

- `tickets/S73PLASNAENT-006.md` (modify — completion notes)
- active S73 proof-surface docs/tickets only if reassessment confirms they are stale
- `crates/worldwake-ai/src/bin/soak_seed_perf.rs` or adjacent benchmark telemetry only if a narrower measurement adjustment is the honest fix

## Out of Scope

- Changing production snapshot-filter behavior from `S73PLASNAENT-002`
- Deferred soak-behavior validation owned by `S73PLASNAENT-005`
- Multi-seed telemetry sweeps unless reassessment proves they are the narrowest honest proof surface

## Acceptance Criteria

### Tests That Must Pass

1. The active S73 proof contract no longer claims a numeric late/early result that the benchmark does not actually produce
2. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0` still emits deterministic telemetry if benchmark changes are made

### Invariants

1. Measured `NA` outcomes are treated as real results, not silently coerced into numeric proof
2. No active spec or ticket implies stronger late-game proof than the live benchmark surface supports

## Test Plan

### New/Modified Tests

None by default — verification is benchmark-output-based and documentation/ticket handoff based unless reassessment adds a bounded helper test.

### Commands

1. `cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0`
2. `cargo clippy --workspace --all-targets -- -D warnings` (only if benchmark code changes)
