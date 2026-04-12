# CLI Evaluation Performance Campaign

## Objective

Minimize the wall-clock time of running 1440 ticks of `scenarios/cli-evaluation.ron` in debug mode.

The scenario exercises 4 agents across 5 places with full GOAP planning, perception, needs, production, trade, combat, and social systems. Recent specs (S88-S98) introduced always-on expansion-level trace recording in the planner, profile lookup amplification in planning snapshots, and heuristic computation overhead that collectively degraded debug-mode wall time from ~30s to ~330s. Release mode completes in ~21s; the 16x debug/release ratio (vs typical 3-5x) confirms that unoptimized allocation and cloning in the planner trace infrastructure dominates debug cost.

**Primary metric**: `duration_ms` — wall-clock time of the 1440-tick simulation in debug mode, measured by the harness wrapper around the observer binary (with dump output suppressed).

**Metric direction**: `lower-is-better`

## Measurement Surface

The harness builds the observer binary in debug mode (matching the simulation-observer skill workflow), then runs:

```bash
cargo run -p worldwake-cli --bin observer -- \
  scenarios/cli-evaluation.ron --ticks 1440 --output /dev/null
```

The `--output /dev/null` suppresses dump I/O to isolate simulation cost. The harness measures wall time externally via epoch milliseconds.

Once the campaign begins, this harness is immutable. Do not modify it during the loop.

## Mutable Files

Any production Rust source file in the 4 production crates:

```text
crates/worldwake-core/src/**/*.rs
crates/worldwake-sim/src/**/*.rs
crates/worldwake-systems/src/**/*.rs
crates/worldwake-ai/src/**/*.rs
```

## Immutable Files (NEVER modify)

- `scenarios/cli-evaluation.ron`
- `campaigns/cli-eval-perf/harness.sh`
- `campaigns/cli-eval-perf/checks.sh`
- `docs/FOUNDATIONS.md`

## Conditionally Mutable Files

- `crates/worldwake-ai/tests/**/*.rs`

These may be modified only when a lawful production-side optimization changes behavior and an assertion must be updated to preserve the original proof intent. Never weaken the intended contract just to keep an optimization.

- `crates/worldwake-cli/src/bin/observer.rs`

May be modified only if observer dump-generation code is proven to be in the hot path despite `--output /dev/null` (e.g., string formatting still executed before discard).

## FOUNDATIONS And Repo Constraints

All optimizations must preserve:

- Principle 12 / performance compression without causal cheating
- determinism (BTreeMap, ChaCha8Rng, no floats, no wall-clock time)
- conservation (`verify_conservation`)
- append-only event history
- belief-only planning
- system decoupling through state
- agent symmetry

Follow [CLAUDE.md](../../CLAUDE.md) in full.

## Profile-First Requirement

Do not assume the current bottleneck.

Before every optimization experiment:

1. Gather fresh profiling evidence from the current accepted state.
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

## Runtime Files

These campaign files are expected to evolve during the loop:

- `results.tsv`
- `musings.md`
- `lessons.jsonl`
- `program.md.backup` (auto-created by meta-loop before meta-reviews)

## Experiment Categories

### 1. `trace-overhead`
ExpansionCandidateTrace allocation, Vec cloning, search report structures. The primary suspect: always-on trace recording with no feature gate.

### 2. `snapshot-cost`
Planning snapshot construction and per-entity profile lookups. Each expansion rebuilds entity snapshots with growing profile sets (ArtifactPostingProfile added by S97).

### 3. `candidate-volume`
Affordance generation count, candidate filtering efficiency, and early termination. Candidate counts can reach 1000+ before filtering at each expansion depth.

### 4. `search-budget`
Planner search parameters, expansion limits, backoff tuning. Interacts with existing exhaustion caching and exponential backoff.

### 5. `heuristic-cost`
Landmark extraction and relaxed-plan heuristic computation per candidate. Added by S95.

### 6. `perception-opt`
Perception system cost, redundant observation reduction, observation frequency.

### 7. `event-log-scan`
Linear event log searches that degrade as O(total_events) per tick. Known issue from soak campaign lessons.

### 8. `system-skip`
Early-exit for systems with no relevant work on a given tick. Empty-input short-circuit.

### 9. `alloc-reduction`
Heap allocation reduction, buffer reuse, arena allocation patterns in hot paths.

## Root Causes To Seed

These are profiling prompts, not implementation permission. Verify them first.

1. **H1 (trace-overhead)**: `ExpansionCandidateTrace` Vec allocated at every search expansion (`search/mod.rs:401`) and cloned into `SearchExpansionSummary` (`search/mod.rs:636`). Always-on, no feature gate. Per-expansion cost: allocation + population + clone of 50-500+ trace structs.
2. **H2 (snapshot-cost)**: `ArtifactPostingProfile` and other profile lookups in `planning_snapshot.rs` amplified per-entity per-expansion. Added incrementally by S96-S97.
3. **H3 (candidate-volume)**: Candidate volume explosion (1000+ before filtering) at each expansion depth in `candidates.rs` (1182 lines). Filtering happens after full generation.
4. **H4 (heuristic-cost)**: S95 relaxed-plan heuristic computation cost per candidate in `heuristic.rs`. Landmark extraction and helpful action marking add per-node overhead.
5. **H5 (event-log-scan)**: Event log linear scans degrading as O(total_events) per tick. Known from soak campaign (lesson confidence 0.95).
6. **H6 (perception-opt)**: Perception system cost from observation frequency and entity count. Redundant perception of unchanged entities.
7. **H7**: Bottlenecks shift after each major optimization. Always re-profile after a meaningful accept.

## Accept / Reject Logic

- **ACCEPT**: `duration_ms` improves by more than 3% relative to best known duration, and all correctness checks pass.
- **ACCEPT (simplification)**: `duration_ms` is within 3% of best known duration, `lines_delta < 0`, and all correctness checks pass.
- **NEAR_MISS**: `duration_ms` is within 3% of best known duration, but the change is not a simplification.
- **REJECT**: `duration_ms` worsens by more than 3%, or a tiny improvement under 3% costs more than 30 added lines, or correctness checks fail.
- **CRASH**: harness or checks fail before producing a valid outcome.

The 3% threshold (wider than default 1%) accounts for single-run debug-mode measurement noise. The harness uses NOISE_TOLERANCE=0.03 and single runs. Each harness invocation takes ~5 minutes at the current baseline, so iteration speed matters.

## Configuration

```text
METRIC_DIRECTION = lower-is-better
PRIMARY_METRIC_KEY = duration_ms
HARNESS_RUNS = 1
NOISE_TOLERANCE = 0.03
ABORT_THRESHOLD = 0.10
PLATEAU_THRESHOLD = 5
UCB_EXPLORATION_C = 1.414
MAX_IMPROVEMENT_PCT = 40
REGRESSION_CHECK_INTERVAL = 5
PIVOT_CHECK_INTERVAL = 10
MAX_ITERATIONS = unlimited
CHECKS_TIMEOUT = 300
CEILING_THRESHOLD = 10
MIN_CONFIDENCE_RUNS = 2
meta_improvement = false
```
