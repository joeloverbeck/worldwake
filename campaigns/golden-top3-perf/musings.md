# Musings: golden-top3-perf

## Learnings from Previous Campaign (golden-ai-perf)

The previous `golden-ai-perf` campaign targeted a different subset of golden tests and achieved -49% wall time reduction through affordance pre-filtering. Key takeaways:

### What Worked

- **Affordance pre-filtering** was the single biggest win: filtering out irrelevant action candidates before expensive plan search dramatically reduced search iterations.
- **Profile-guided approach**: Measuring first, then targeting the actual hot paths (not assumed ones) prevented wasted experiments.
- **Small, isolated changes**: Each experiment touched 1-2 files, making it easy to attribute gains and roll back failures.

### What Didn't Work / Diminishing Returns

- Micro-optimizations to already-fast paths yielded <1% gains.
- Over-aggressive pruning risked changing agent behavior (caught by golden test assertions).

### Key Observations

- `golden_determinism` dominates at ~81% of combined time and runs the full simulation **twice** (normal + replay verification). Any per-tick optimization gets doubled impact there.
- Affordance filtering low-hanging fruit is now captured on main. Remaining gains require **structural changes**: allocation reduction, caching, algorithm improvements.
- The prototype world is small (few places, few agents), so algorithmic complexity improvements may show modest absolute gains but establish patterns that scale.

## Hypotheses Priority

Starting with H1 (route cloning) and H7 (Floyd-Warshall caching) since topology operations are called deep in hot paths and the fixes are well-contained. H2 (reservation reverse index) is a close third due to clear O(k) → O(1) improvement.

H5 (PlanningState clone) and H10 (step list cloning) are higher-effort structural changes — save for later iterations after capturing easier wins.

## Running Notes

*(Updated during campaign execution)*

## exp-001: Replace Floyd-Warshall BTreeMap with Vec-based distance matrix
**Hypothesis**: The Floyd-Warshall in `compute_shortest_travel_ticks` uses `BTreeMap<(EntityId, EntityId), u32>` with O(log n) per access. Replacing with a flat `Vec<u32>` indexed by place position should reduce constant factors in the O(n³) loop and all subsequent lookups via `min_travel_ticks`/`min_travel_ticks_to_any`.
**Result**: ACCEPT (44869 -> 41760 ms, -6.9%)
**Learning**: BTreeMap overhead for small-but-frequently-accessed structures is significant. The Vec-based matrix with binary_search for index lookup is much faster. This affects both construction (Floyd-Warshall inner loop) and every lookup during search heuristic computation.

## exp-002: Hoist relevant_action_defs out of per-node search_candidates
**Hypothesis**: `search_candidates()` calls `relevant_action_defs()` per node expansion, re-creating a `BTreeSet<ActionDefId>` each time. Since the result depends only on the goal (constant across all nodes), hoisting it to `search_plan()` and passing it down eliminates redundant allocations proportional to total node expansions.
**Result**: NEAR_MISS (41760 -> 41605 ms, -0.4%)
**Learning**: The BTreeSet creation per expansion is a minor cost — the set is small (typically 2-5 entries). The real cost per expansion is `get_affordances_for_defs` which iterates entity lists. Stashed for potential combination.

## exp-003: Optimize build_snapshot_places to avoid O(places*entities) effective_place queries
**Hypothesis**: `build_snapshot_places` calls `view.effective_place(entity)` for every entity for every place — O(P*E). Building a single entity→place map first and then grouping reduces this to O(E) view queries + O(E) grouping.
**Result**: REJECT (41760 -> 43684 ms, +4.6%)
**Learning**: For small entity/place counts (~12 places, ~50 entities), the BTreeMap allocation overhead of the pre-grouping approach exceeds the saved view queries. The O(P*E) with ~600 cheap lookups is faster than creating and populating a BTreeMap grouping structure. Don't optimize small-N loops with complex data structures.

## exp-004: Skip read phase for agents with non-interruptible active actions
**Hypothesis**: The read phase (candidate generation, ranking, observation snapshot comparison) runs unconditionally for all agents every tick. For agents currently executing a NonInterruptible action, evaluate_interrupt always returns NoInterrupt, making the entire read phase wasted work. Skipping it should eliminate ~50% of per-agent-per-tick cost for multi-tick non-interruptible actions (harvest, craft, etc.).
**Result**: NEAR_MISS (41760 -> 42098 ms, within noise)
**Learning**: Most actions in golden tests are FreelyInterruptible (travel, eat, drink, harvest). Very few are NonInterruptible. The fast path fires rarely. Need to target FreelyInterruptible action paths or the planning phase itself.

## exp-005: Replace BTreeMap-based component lookups with Vec in PlanningState RuntimeBeliefView
**Hypothesis**: PlanningState implements RuntimeBeliefView with 13 BTreeMap override fields. During search, each `get_affordances_for_defs` call queries these maps. Since most overrides are empty for early search nodes, the cost is in BTreeMap::get which always traverses the tree. But empty BTreeMap::get should be O(1). Let me focus on something else.

## exp-005: Share a single merged snapshot across all candidates for the same agent
**Hypothesis**: build_candidate_plans builds a separate PlanningSnapshot per candidate (up to 4). Each snapshot construction involves collect_places (BFS), collect_entities (BFS), build_snapshot_entity (30 queries per entity), and DistanceMatrix (Floyd-Warshall). Building one merged snapshot with the union of all evidence sets and reusing it for all candidates should save 3 snapshot constructions worth of work.
**Result**: ACCEPT (41760 -> 41051 ms, -1.7%)
**Learning**: Snapshot sharing works. The merged snapshot may include slightly more entities than individual snapshots would, but the cost of building those extra entities is far less than building 3 additional complete snapshots. The Floyd-Warshall and entity query savings dominate.

## exp-006: Optimize PlanningState::entities_at to use snapshot place index
**Hypothesis**: `entities_at()` iterates ALL snapshot entities and calls `effective_place()` (recursive with BTreeSet cycle detection) per entity. Instead, start from the SnapshotPlace's pre-indexed entity set, add entities with place overrides pointing here, and remove entities with overrides pointing elsewhere. This reduces O(N) full-entity scans to O(P_entities + overrides).
**Result**: ACCEPT (41051 -> 39851 ms, -2.9%)
**Learning**: The fast path fires on the root search node (no overrides yet) and is a massive win because `entities_at` is called during affordance generation — which happens per node expansion. For root nodes of all goal candidates, this eliminates the most expensive per-expansion call. The complex per-override path was too risky; just checking for empty overrides is simple and effective.

## exp-007: Fast-path direct_possessions using snapshot data
**Hypothesis**: Same approach as entities_at — skip full scan when no overrides exist.
**Result**: REJECT (39851 -> 40500 ms, +1.6%)
**Learning**: direct_possessions is called less frequently than entities_at in affordance generation, so the overhead of the extra condition checks isn't amortized. Or the measurement was noisy. Either way, not worth the complexity.

## exp-008: Fast-path effective_place for unmodified states
**Hypothesis**: effective_place creates BTreeSet::new() for cycle detection on every call. Skip the resolve chain entirely when no overrides exist.
**Result**: NEAR_MISS (39851 -> 39970 ms, within noise)
**Learning**: The entities_at fast path already eliminates most effective_place calls on root nodes. Adding the fast path to effective_place itself has diminishing returns because the remaining calls happen in the slow path of entities_at (when overrides exist), where the override check would be true anyway.

## exp-011: Batch commodity signature via single possession traversal
**Hypothesis**: Abandoned — semantics mismatch with belief view (CRASH).

## exp-012 through exp-014: Various optimizations
- exp-012: Precompute goal distances — too many test changes, abandoned.
- exp-013: BTreeMap index in DistanceMatrix — REJECT, slower than binary_search for 12-element Vec.
- exp-014: Kind-aware snapshot entity construction — NEAR_MISS, branching overhead offsets savings.

## Status Summary (after exp-014)
**Baseline**: 44,869ms → **Current best**: 39,851ms → **Reduction**: -11.2%
**Target**: <32,000ms (-29%) — still need ~20% more reduction.

Accepted optimizations:
1. Vec-based distance matrix (exp-001): -6.9%
2. Merged snapshot across candidates (exp-005): -1.7%
3. Fast-path entities_at for unmodified states (exp-006): -2.9%

Remaining bottleneck analysis: The overhead is spread across many small operations (component table lookups, candidate generation, ranking) rather than concentrated in one hot spot.

## Post-expansion experiments (exp-018)
After expanding mutable files to include ownership.rs, belief_view.rs, per_agent_belief_view.rs:
- exp-018: Batch commodity/unique item queries via single BFS — NEAR_MISS
- **Key learning**: The prototype world has tiny possession hierarchies (1-2 items per agent). BFS is O(3), so doing it 10x = O(30) — barely more than 1x with accumulation overhead. Algorithmic improvements require larger N to show impact.

The remaining ~20% to target requires either:
1. **Reducing total work volume** (fewer ticks, fewer candidates, fewer expansions) — blocked by immutable test files and behavioral equivalence
2. **Profile-guided micro-optimization** with actual profiling tools (perf, flamegraph) to find the true hot spots
3. **Compiler-level optimizations** (release mode, LTO) which would shrink everything proportionally but don't apply to debug-mode tests
