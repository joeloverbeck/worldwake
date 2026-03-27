# Golden Test Performance Campaign

## Objective

Minimize the combined wall-clock time of the 5 slowest golden test suites in `crates/worldwake-ai/tests/`:

| Suite | Baseline (ms) | Tests |
|-------|--------------|-------|
| `golden_determinism` | 88270 | 23 |
| `golden_trade` | 17250 | 21 |
| `golden_care` | 16020 | 32 |
| `golden_supply_chain` | 10820 | 22 |
| `golden_production` | 9980 | 35 |
| **Total** | **142340** | **133** |

**Metric**: `combined_duration_ms` — sum of wall times for the 5 suites, each run with `--test-threads=1`.

**Metric direction**: `lower-is-better`

## Mutable Files

All production Rust source files across the 4 production crates:

```
crates/worldwake-core/src/**/*.rs
crates/worldwake-sim/src/**/*.rs
crates/worldwake-systems/src/**/*.rs
crates/worldwake-ai/src/**/*.rs
```

If profiling reveals bottlenecks in code not yet listed here (e.g., build scripts, Cargo features), update this list before modifying those files.

## Immutable Files (NEVER modify)

- `campaigns/golden-perf/harness.sh`
- `campaigns/golden-perf/checks.sh`
- `docs/FOUNDATIONS.md`

## Conditionally Mutable Files

- `crates/worldwake-ai/tests/**/*.rs` (golden test files): May be modified ONLY when a production-side optimization changes behavior in a way that causes test failures. The fix must preserve the **behavioral intent** of the test (what it proves), even if specific assertion values change (e.g., hash values, tick counts, exact commodity quantities). Never weaken assertions or remove test coverage — adapt assertions to the new correct behavior.

## FOUNDATIONS.md Constraints

All optimizations MUST align with these non-negotiable principles from `docs/FOUNDATIONS.md`:

- **Principle 11 (Performance Compresses Computation, Never Causality)**: Optimization allowed only if causally equivalent. Cannot skip travel time, inventory depletion, perception propagation, or any causal step.
- **Determinism invariant**: Must preserve `BTreeMap`/`BTreeSet` for all authoritative state. No `HashMap`/`HashSet`. No floats. No wall-clock time.
- **Conservation invariant**: `verify_live_lot_conservation()` and `verify_authoritative_conservation()` must continue to pass.
- **Append-only event log**: Events cannot be mutated or deleted.
- **Belief-only planning**: Agents must never read world state directly.

Radical architectural changes are allowed (e.g., replacing BTreeMap with a deterministic arena, restructuring the tick loop, caching affordance results) as long as they satisfy the above constraints.

## Profile-First Mandate

**NEVER assume what the bottlenecks or hot paths are.** Every hypothesis MUST include profiling evidence before implementation.

Allowed profiling approaches:
1. **Opt-in timing instrumentation**: Add `#[cfg(feature = "bench-profiling")]` gated timing code to production modules. Use `std::time::Instant` behind the feature flag.
2. **Per-suite timing**: The harness already reports per-suite times as intermediate metrics. Use partial signals to identify which suite benefits from a change.
3. **Tick-level instrumentation**: Add timing around `step_tick`, system dispatch, AI input production to identify per-tick cost distribution.
4. **Flamegraph / perf**: Use `cargo flamegraph` or `perf` on individual suites to identify hot functions.

The `bench-profiling` Cargo feature does NOT exist yet. Creating it (adding to `Cargo.toml` files) is an allowed mutable-scope change. It must be opt-in and zero-cost when disabled.

## Experiment Categories

UCB1-tracked categories for hypothesis selection:

### 1. `tick-hot-path`
Optimize per-tick cost in `step_tick`, system dispatch, action processing. Examples: batch system calls, reduce per-system overhead, optimize action state machine transitions.

### 2. `ai-budget`
Reduce per-agent-per-tick AI computation cost. Examples: skip full planning for agents with active non-interruptible actions, tune `PlanningBudget` parameters (beam width, expansion limit, candidate count), cache planner op semantics.

### 3. `affordance-pruning`
Reduce combinatorial cost of `get_affordances()` and `enumerate_bindings()`. Examples: use filtered `get_affordances_for_defs()` when planner only needs specific action types, pre-filter entity lists before binding enumeration, cache affordance results within a tick.

### 4. `hash-serial`
Optimize canonical hashing (`blake3`) and serialization (`bincode`). Examples: incremental/delta hashing instead of full re-serialization, lazy event log hashing (only new events), reduce number of `hash_world()` calls in determinism tests via production-side caching.

### 5. `replay-dedup`
Eliminate redundant work in replay and save/load paths. Examples: share scenario setup between base test and replay variant (production-side replay infrastructure), memoize seed-identical scenario state, optimize `replay_and_verify` to avoid full re-execution when possible.

### 6. `system-skip`
Short-circuit systems irrelevant to a scenario. Examples: systems detect empty input (no entities with relevant components) and return immediately, perception system skips when no events emitted this tick, politics system skips when no offices exist.

### 7. `ecs-lookup`
Optimize BTreeMap-based component storage access patterns. Examples: add component-presence bitflags per entity, cache frequently-accessed components within a tick, batch component reads, use arena-allocated storage for hot-path components.

### 8. `perception-opt`
Optimize the perception system specifically. Examples: index events by place to avoid iterating all events per witness, batch belief store updates, short-circuit `resolve_witnesses` for hidden events, skip perception when no `PerceptionProfile` exists.

## Root Causes to Seed

Initial hypotheses for profiling (VERIFY BEFORE IMPLEMENTING):

1. **H1 — Determinism tests run full simulations 3-5x unnecessarily**: The 88s golden_determinism suite may be dominated by sheer repetition (build scenario + run + replay + save/load). Profile to find how much time is scenario construction vs tick execution vs hashing.

2. **H2 — GOAP search runs full budget per agent per tick even when no replan needed**: `produce_agent_input` may run the full planning pipeline for agents with active actions. Profile to see what fraction of `agent_tick` time is spent on agents that take no new action.

3. **H3 — Perception system has non-trivial overhead even with few observers**: Many golden tests seed agents without `PerceptionProfile`, yet perception runs every tick. Profile perception system cost per tick in the 5 target suites.

4. **H4 — Affordance enumeration is combinatorial**: `get_affordances` iterates 17+ action definitions x all entities at a place. With multiple agents/items, binding enumeration could be quadratic. Profile `enumerate_bindings` call counts and time.

5. **H5 — `hash_world` re-serializes entire World on each call**: In determinism tests, `hash_world()` may be called 5+ times per test. Each call serializes the full World. Profile serialization time vs hash time.

6. **H6 — Supply chain tests run 280-500 ticks mostly idle**: Tests may complete meaningful behavior by tick 100 but run to 500. Profile per-tick event counts to find the "idle tail."

7. **H7 — BTreeMap component lookups dominate hot paths**: ECS uses `BTreeMap<EntityId, T>` which is O(log n). With many entities, repeated lookups in inner loops could add up. Profile component table access patterns in `step_tick`.

## Accept/Reject Logic

- **ACCEPT**: `combined_duration_ms` improved >1% (relative to `best_ms`), AND `checks.sh` passes.
- **ACCEPT (simplification)**: Metric within 1% AND `lines_delta < 0` (net code removed), AND `checks.sh` passes.
- **REJECT**: Metric worsened >1%, or tiny improvement (<2%) with >20 lines added.
- **NEAR_MISS**: Metric within 1% AND `lines_delta >= 0`.
- **CRASH**: Harness or checks failed — fix trivially (up to 3 retries) or reject.

## Configuration

```
METRIC_DIRECTION = lower-is-better
HARNESS_RUNS = 3
ABORT_THRESHOLD = 0.05
PLATEAU_THRESHOLD = 5
UCB_EXPLORATION_C = 1.414
NOISE_TOLERANCE = 0.01
MAX_IMPROVEMENT_PCT = 30
REGRESSION_CHECK_INTERVAL = 5
HARNESS_SEEDS = 1
META_IMPROVEMENT = false
PIVOT_CHECK_INTERVAL = 10
MAX_ITERATIONS = unlimited
CHECKS_TIMEOUT = 300
```
