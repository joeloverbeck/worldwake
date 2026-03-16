# Golden AI Test Performance — Improve-Loop Program

## Objective

Reduce the combined wall time of 4 golden end-to-end tests in `worldwake-ai`. These tests exercise the full GOAP decision architecture: goal ranking, plan search, action execution, and replanning across hundreds of simulation ticks.

**Baseline**: ~169,000ms combined
**Target**: <100,000ms (>40% reduction)

## Thresholds

- `ABORT_THRESHOLD`: 0.15 — kill harness if running total exceeds best × 1.15
- `PLATEAU_THRESHOLD`: 5 — stop after 5 consecutive experiments with no improvement
- `HARNESS_RUNS`: 1 — deterministic simulation, no statistical noise

## Pre-Harness Validation

Before running the harness, the following must pass. Failure = CRASH (revert and fix):

```bash
cargo test --workspace
cargo clippy --workspace
```

## Mutable Files

Only these 11 files may be modified:

| File | Role |
|------|------|
| `crates/worldwake-ai/src/search.rs` | GOAP best-first search (hottest function) |
| `crates/worldwake-ai/src/agent_tick.rs` | Per-agent tick driver, snapshot construction orchestration |
| `crates/worldwake-ai/src/planning_snapshot.rs` | Belief snapshot BFS construction |
| `crates/worldwake-ai/src/planning_state.rs` | Mutable planning state (cloned per search node) |
| `crates/worldwake-ai/src/budget.rs` | PlanningBudget defaults |
| `crates/worldwake-ai/src/candidate_generation.rs` | Goal candidate enumeration |
| `crates/worldwake-ai/src/ranking.rs` | Goal ranking |
| `crates/worldwake-ai/src/plan_revalidation.rs` | Plan revalidation |
| `crates/worldwake-ai/src/planner_ops.rs` | PlannedStep types |
| `crates/worldwake-ai/src/interrupts.rs` | Interrupt evaluation |
| `crates/worldwake-sim/src/affordance_query.rs` | `get_affordances()` |

## Immutable Files

- All test files (golden tests, unit tests, integration tests)
- `campaigns/golden-ai-perf/harness.sh`
- `campaigns/golden-ai-perf/program.md`

## Experiment Categories

- `search-pruning` — reduce search space via better pruning or early termination
- `caching` — memoize expensive computations (affordances, snapshots, hashes)
- `snapshot-optimization` — reduce snapshot construction cost or frequency
- `affordance-optimization` — reduce per-node affordance generation cost
- `budget-tuning` — adjust PlanningBudget parameters
- `clone-reduction` — reduce allocation/cloning in hot paths
- `candidate-reduction` — generate fewer or smarter goal candidates
- `replan-reduction` — reduce unnecessary replanning triggers
- `other` — anything not covered above

## Root Cause Hypotheses

Ordered by estimated impact (highest first):

### 1. Redundant snapshot construction
The planning cycle constructs up to 4 separate `SnapshotEntity` collections per agent per tick. Each snapshot runs a BFS over the place graph to collect visible entities. Most of these share the same underlying world state and could be unified or cached within a tick.

### 2. Per-node affordance recomputation
`get_affordances()` is called for each search node expansion (up to 512 per search). It queries the world for available actions, checking preconditions against the full entity set. Caching affordances per unique planning state fingerprint could eliminate most of this work.

### 3. Excessive replanning
The dirty flag in `AgentDecisionRuntime` is set nearly every tick (any event observation triggers it), causing a full plan search even when the current plan remains valid. More selective dirty-flag criteria or plan revalidation before search could reduce search invocations dramatically.

### 4. Vec<PlannedStep> cloning per successor
Each search node expansion clones the full plan prefix (up to depth 6) for each successor (up to 8 per node). At beam width 8 and max expansions 512, this creates thousands of Vec allocations. An arena or shared-prefix structure could eliminate most clones.

### 5. Budget over-provisioning
Default budget: beam_width=8, max_expansions=512, max_depth=6. For many goal types (eat, drink, sleep), optimal plans are 1-2 steps. Adaptive budgets per goal type could reduce search effort by 10-50x for simple goals.

### 6. PlanningState clone cost
`PlanningState` contains 13 BTreeMap/BTreeSet fields that are cloned for each successor node. Most successors modify only 1-2 fields. Copy-on-write or diff-based state tracking could reduce clone overhead.

## Critical Invariants

These must NEVER be violated. Any experiment that breaks these is an immediate CRASH:

- **Determinism**: `BTreeMap`/`BTreeSet` only in authoritative state, no `HashMap`/`HashSet`, no floats, no wall-clock time
- **Conservation**: Items cannot be created/destroyed except through explicit actions
- **Belief-only planning**: Agents never read world state directly
- **All existing tests must pass**: `cargo test --workspace` is the gate
