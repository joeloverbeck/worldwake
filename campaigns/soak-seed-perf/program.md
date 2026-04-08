# Soak Seed Performance Campaign

## Objective

Minimize the wall-clock time of a single seven-day soak execution that follows the same execution shape as the soak CI workload:

- T30 world setup
- 20 agents across 10 places
- 10,080 ticks (7 in-game days)
- the per-run invariants currently enforced by the soak suite

This campaign intentionally optimizes one seed per experiment to keep iteration speed reasonable.

**Primary metric**: `duration_ms` — wall-clock time of one one-seed soak execution chosen from a fixed deterministic seed cycle.

**Metric direction**: `lower-is-better`

## Measurement Surface

The target behavior is the underlying one-seed soak execution shape, not the 10-seed aggregate.

The fixed campaign harness uses a dedicated standalone runner that executes the one-seed soak path directly:

```bash
cargo run --release -p worldwake-ai --bin soak_seed_perf -- <seed>
```

The runner reuses the same T30 world construction and the same one-seed per-run invariants, but it does not go through the Rust test harness or the 10-seed aggregate assertions. Once the campaign begins, this harness is immutable. Do not modify it during the loop.

## Seed Rotation And Comparison Rule

This campaign uses a fixed deterministic seed cycle:

```text
0, 1, 2, 3, 4
```

The harness rotates through that cycle based on the current experiment count and prints the `seed_id` it used.

**Critical rule**: accept/reject comparisons are made against the best prior result for the SAME seed only.

- Seed 0 experiments compare only against the best recorded seed 0 duration.
- Seed 1 experiments compare only against the best recorded seed 1 duration.
- And so on.

Do not compare a seed 3 run directly against the best seed 1 run. Cross-seed wall times are not comparable enough for acceptance decisions.

Track per-seed bests in `seed-baselines.tsv`.

A secondary campaign-wide progress view may be computed from the per-seed best table, but it is not the primary accept/reject gate.

## Mutable Files

Any production Rust source file in the 4 production crates:

```text
crates/worldwake-core/src/**/*.rs
crates/worldwake-sim/src/**/*.rs
crates/worldwake-systems/src/**/*.rs
crates/worldwake-ai/src/**/*.rs
```

## Immutable Files (NEVER modify)

- `campaigns/soak-seed-perf/harness.sh`
- `campaigns/soak-seed-perf/checks.sh`
- `docs/FOUNDATIONS.md`

## Conditionally Mutable Files

- `crates/worldwake-ai/tests/**/*.rs`

These may be modified only when a lawful production-side optimization changes behavior and an assertion must be updated to preserve the original proof intent. Never weaken the intended contract just to keep an optimization.

## FOUNDATIONS And Repo Constraints

All optimizations must preserve:

- Principle 12 / performance compression without causal cheating
- determinism
- conservation
- append-only event history
- belief-only planning
- system decoupling through state
- all per-run soak invariants currently exercised by the one-seed soak path

Follow [AGENTS.md](../../AGENTS.md) in full.

## Profile-First Requirement

Do not assume the current bottleneck.

Before every optimization experiment:

1. Gather fresh profiling evidence from the current accepted state or from the current in-progress branch if the campaign is intentionally investigating a candidate change.
2. Record the profiling finding in `musings.md` before editing production code.
3. Name the exact hot path, symbol, or repeated operation the experiment is targeting.

Acceptable profiling evidence includes:

- external sampling or tracing tools
- targeted timing instrumentation that is removed before the final harness measurement
- bounded diagnostic counters or logging runs that prove where time is going

Do not turn a root-cause guess into an experiment without profiling evidence. If no workable profiler or diagnostic path is available in the environment, stop and surface that blocker instead of guessing blindly.

## Correctness Gate

Every accepted experiment must pass:

1. Full workspace tests
2. Full workspace clippy with `--all-targets -- -D warnings`
3. A focused soak-path verification using `--features soak` on the one-seed soak test entrypoint

The focused soak verification exists because the soak path is feature-gated and may not be covered by the ordinary workspace test surface.

## Runtime Files

These campaign files are expected to evolve during the loop:

- `results.tsv`
- `seed-baselines.tsv`
- `musings.md`
- `lessons.jsonl`
- `program.md.backup` (auto-created by meta-loop before meta-reviews)

## Baseline Procedure

Before the first optimization experiment:

1. Run the harness once for each seed in the fixed cycle.
2. Record one baseline row per seed in `results.tsv`.
3. Initialize `seed-baselines.tsv` with the best-known baseline for each seed.
4. Only after all seeds have an initial baseline should normal experiments begin.

## What The One-Seed Harness Must Preserve

The optimization harness is allowed to drop the cross-seed aggregate assertions from `golden_soak`, because those only make sense over multiple runs.

It must continue to exercise the one-seed per-run contract:

- conservation checks
- needs bounds
- dead-agent inactivity
- unique placement
- tick monotonicity
- causal-link integrity
- world state changes over the seven-day run

## Experiment Categories

### 1. `tick-hot-path`
Per-tick scheduler, dispatch, and step-loop cost.

### 2. `ai-budget`
Planner and per-agent decision cost during long autonomous runs.

### 3. `affordance-pruning`
Affordance enumeration and binding explosion during soak ticks.

### 4. `perception-opt`
Perception/event observation cost during long runs.

### 5. `event-log-scan`
Linear or repeated scans over the append-only log that worsen over 10,080 ticks.

### 6. `ecs-lookup`
Hot authoritative storage lookups and repeated component access patterns.

### 7. `system-skip`
Systems that can prove they have no relevant work on many soak ticks.

### 8. `replay-hash-overhead`
Overhead caused by hashing, verification, or replay-adjacent work reached in the soak path.

## Root Causes To Seed

These are profiling prompts, not implementation permission. Verify them first.

1. Long-run cost may be dominated by work that scales with total event-log length rather than current tick-local activity.
2. Agents with active or effectively settled plans may still be paying too much full-planning cost every tick.
3. Perception and social observation may impose substantial steady-state cost even when little relevant information is changing.
4. Repeated authoritative component lookups inside inner loops may dominate long-run soak cost.
5. Some systems may be executing meaningful scan/setup work on ticks where they have no relevant entities or no relevant events.
6. Affordance generation may still be broader than the long-run soak path actually needs.
7. Bottlenecks may shift after each major optimization; re-profile after every meaningful accept.

## Accept / Reject Logic

Use the current experiment's `seed_id` to find the corresponding entry in `seed-baselines.tsv`.

- **ACCEPT**: `duration_ms` improves by more than 1% relative to that seed's best known duration, and all correctness checks pass.
- **ACCEPT (simplification)**: `duration_ms` is within 1% of that seed's best known duration, `lines_delta < 0`, and all correctness checks pass.
- **NEAR_MISS**: `duration_ms` is within 1% of that seed's best known duration, but the change is not a simplification.
- **REJECT**: `duration_ms` worsens by more than 1%, or a tiny improvement under 2% costs more than 20 added lines, or correctness checks fail.
- **CRASH**: harness or checks fail before producing a valid outcome.

On ACCEPT:
- update the matching seed row in `seed-baselines.tsv` if the new duration is the best for that seed
- append the full result to `results.tsv`
- commit the accepted state

On REJECT or NEAR_MISS:
- append the result to `results.tsv`
- roll back to the last accepted commit

## Configuration

```text
METRIC_DIRECTION = lower-is-better
PRIMARY_METRIC_KEY = duration_ms
SEED_KEY = seed_id
SEED_CYCLE = 0,1,2,3,4
HARNESS_RUNS = 1
ABORT_THRESHOLD = 0.05
PLATEAU_THRESHOLD = 5
UCB_EXPLORATION_C = 1.414
NOISE_TOLERANCE = 0.01
MAX_IMPROVEMENT_PCT = 30
REGRESSION_CHECK_INTERVAL = 5
PIVOT_CHECK_INTERVAL = 10
MAX_ITERATIONS = unlimited
CHECKS_TIMEOUT = 300
CEILING_THRESHOLD = 10
MIN_CONFIDENCE_RUNS = 2
meta_improvement = true
META_REVIEW_INTERVAL = 20
META_TRIAL_WINDOW = 10
```
