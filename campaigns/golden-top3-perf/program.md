# Campaign: golden-top3-perf

## Objective

Reduce combined wall time of the 3 slowest golden test suites in `worldwake-ai` from ~43,200ms baseline. **Target: <32,000ms** (>26% reduction).

| Suite | Baseline (measured) | Share |
|-------|---------------------|-------|
| `golden_determinism` | 34,272ms | 79% |
| `golden_production` | 5,901ms | 14% |
| `golden_combat` | 3,053ms | 7% |

`golden_determinism` runs the simulation twice (normal + replay verification), so per-tick optimizations get doubled impact there.

## Thresholds

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| `ABORT_THRESHOLD` | 0.05 | 5% regression triggers abort (tighter than previous campaign since baseline is smaller) |
| `PLATEAU_THRESHOLD` | 5 | 5 consecutive experiments with <1% cumulative gain triggers plateau |
| `HARNESS_RUNS` | 1 | Deterministic simulation — no statistical averaging needed |

## Pre-Harness Validation

Before every harness run, the improve-loop MUST pass:

```bash
cargo test --workspace
cargo clippy --workspace
```

If either fails, the experiment is rejected without running the harness.

## Mutable Files (14)

### AI crate (`crates/worldwake-ai/src/`)

| # | File | Domain |
|---|------|--------|
| 1 | `search.rs` | GOAP best-first plan search |
| 2 | `agent_tick.rs` | Per-agent per-tick decision runtime |
| 3 | `planning_snapshot.rs` | Immutable belief state snapshot for planning |
| 4 | `planning_state.rs` | Mutable planning simulation state |
| 5 | `budget.rs` | Planning budget constraints |
| 6 | `candidate_generation.rs` | Goal candidate enumeration |
| 7 | `ranking.rs` | Goal priority scoring |
| 8 | `plan_revalidation.rs` | Plan step revalidation |
| 9 | `planner_ops.rs` | Action-type semantics for planner |
| 10 | `interrupts.rs` | Action interrupt evaluation |
| 11 | `affordance_query.rs` | Available action queries — NOTE: lives in `crates/worldwake-sim/src/` |

### Core crate (`crates/worldwake-core/src/`)

| # | File | Domain |
|---|------|--------|
| 12 | `topology.rs` | Place graph, Dijkstra pathfinding, travel edges |
| 13 | `relations.rs` | Relation tables, placement/ownership/reservation APIs |
| 14 | `component_tables.rs` | Macro-generated typed component storage |

## Immutable Files

- All test files (`crates/worldwake-ai/tests/golden_*.rs`)
- `campaigns/golden-top3-perf/harness.sh`
- `campaigns/golden-top3-perf/program.md`

## Experiment Categories

| Category | Description |
|----------|-------------|
| `route-optimization` | Dijkstra/pathfinding allocation and algorithm improvements |
| `snapshot-optimization` | Planning snapshot construction and reuse |
| `search-pruning` | Plan search frontier pruning and early termination |
| `caching` | Memoization and result caching across calls |
| `clone-reduction` | Reducing unnecessary clone/allocation in hot paths |
| `budget-tuning` | Planning budget parameter adjustments |
| `candidate-reduction` | Reducing redundant goal candidate generation |
| `replan-reduction` | Avoiding unnecessary replanning cycles |
| `reservation-optimization` | Reservation lookup and reverse-index improvements |
| `other` | Anything not fitting above categories |

## Root Cause Hypotheses

Ranked by estimated impact (highest first):

### H1: Route cloning in Dijkstra (`topology.rs`) — `route-optimization`
The Dijkstra implementation clones `Route` (which contains `Vec<TravelEdgeId>`) on every frontier pop. For the prototype world's small graph this may not dominate, but `golden_determinism`'s double-run amplifies it.

### H2: Reservation reserver scan (`relations.rs`) — `reservation-optimization`
`reservations_by_reserver()` performs O(k) full scan over all reservations. A reverse index (reserver → reservation set) would make this O(1) lookup.

### H3: Snapshot construction cost (`planning_snapshot.rs`) — `snapshot-optimization`
`SnapshotEntity` construction iterates O(entities * ~20 component lookups) per plan search invocation. If the world state hasn't changed since last snapshot, the snapshot could be cached and incrementally updated.

### H4: Travel pruning per-candidate cost (`search.rs`) — `search-pruning`
`min_travel_ticks_to_any` is called repeatedly during search expansion. Pre-computing a distance matrix or caching results per (source, target-set) would eliminate redundant pathfinding.

### H5: PlanningState clone cost (`planning_state.rs`) — `clone-reduction`
`PlanningState` contains 13 `BTreeMap` fields, all cloned per successor node during search expansion. Structural sharing (e.g., persistent data structures or diff-based state) could reduce this.

### H6: Budget over-provisioning (`budget.rs`) — `budget-tuning`
Default beam_width=8 and max_expansions=512 may be excessive for simple 1-2 step goals. Adaptive budgets based on goal complexity could reduce wasted search.

### H7: Floyd-Warshall recomputation (`planning_snapshot.rs`) — `caching`
All-pairs shortest path recomputed per snapshot. Since topology is static during a tick, this can be cached at the tick level or globally.

### H8: Affordance entities_at caching (`affordance_query.rs`) — `caching`
`get_affordances()` may query `entities_at()` repeatedly for the same place within a single tick when multiple agents share a location.

### H9: Component tables remove_all (`component_tables.rs`) — `other`
`remove_all_components()` iterates 30+ typed maps. If called frequently during entity lifecycle, a bitset tracking which tables have data for an entity would skip empty maps.

### H10: Vec<PlannedStep> cloning per successor (`search.rs`) — `clone-reduction`
Each search node clones its full step list when generating successors. An Rc-linked list or arena-allocated path would make successor generation O(1) instead of O(plan_length).

## Critical Invariants

Every experiment MUST preserve:

1. **Determinism**: `golden_determinism` suite passes (replay produces identical state hashes)
2. **Conservation**: Item quantities are conserved across all operations (`verify_conservation`)
3. **Belief-only planning**: Agents never read `World` state directly — only through `BeliefView`
4. **All tests pass**: `cargo test --workspace` must pass before and after each experiment
5. **No clippy warnings**: `cargo clippy --workspace` must be clean
6. **Behavioral equivalence**: Golden test assertions (agent actions, state outcomes) must not change — only wall time should improve
