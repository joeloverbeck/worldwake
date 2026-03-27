**Status**: PENDING

# S29: Planning State Structural Sharing

## Summary

Replace the clone-per-expansion `PlanningState` pattern in GOAP search with a persistent (structurally-shared) overlay design. Each search expansion currently clones 16 BTreeMaps; the new design shares unmodified subtrees between parent and child nodes, reducing per-expansion cost from O(total state) to O(mutations). This is a pure Principle 11 optimization: it compresses computation without changing any world meaning or planning outcome.

## Why This Exists

Profiling during the golden-perf campaign (exp-007, exp-012) identified `PlanningState::clone()` as the single most expensive per-expansion operation in `search_plan`. The struct contains 16 BTreeMap/BTreeSet fields that are fully cloned for every successor candidate at every expansion. With a budget of 256 max expansions and beam_width=8, a single search can clone 2000+ instances. Cost grows with search depth as states accumulate overrides. This dominates found-search time (47% of AI cost at the 30s baseline).

The budget tuning campaign reduced total golden test time from 92s to 30s, but the remaining 30s is still dominated by this clone pattern. No further parameter tuning can improve it without reducing plan quality below acceptable thresholds (beam_width < 8 and max_expansions < 256 both break golden tests).

## Phase

Phase 3+: AI Architecture Overhaul

## Crates

- `worldwake-ai`

## Dependencies

- None (self-contained within the planning search module)

## Design Goals

1. **O(mutations) per expansion**: Each search node expansion costs proportional to the number of state mutations, not the total accumulated state.
2. **Zero behavioral change**: Identical planning outcomes for any given search. The optimization is invisible to goal selection, plan validity, and all downstream systems.
3. **No external dependencies**: Use Rust's `Rc<BTreeMap>` or equivalent; do not introduce external persistent-data-structure crates.
4. **Determinism preserved**: `BTreeMap` ordering guarantees retained. No `HashMap`. No floating point.
5. **Save/load unaffected**: `PlanningState` is transient (lives only within a search invocation) and is never serialized. No save format changes.

## Deliverables

### 1. `SharedMap<K, V>` wrapper type

A copy-on-write BTreeMap wrapper that shares the underlying allocation until mutation:

```rust
/// A copy-on-write BTreeMap that shares allocation across search expansions.
/// Cloning an unmodified SharedMap is O(1) (reference count increment).
/// First mutation triggers a full clone (copy-on-write).
#[derive(Clone)]
struct SharedMap<K: Ord, V: Clone>(Rc<BTreeMap<K, V>>);

impl<K: Ord + Clone, V: Clone> SharedMap<K, V> {
    fn new() -> Self { Self(Rc::new(BTreeMap::new())) }

    fn get(&self, key: &K) -> Option<&V> { self.0.get(key) }

    fn insert(&mut self, key: K, value: V) -> Option<V> {
        Rc::make_mut(&mut self.0).insert(key, value)
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        Rc::make_mut(&mut self.0).remove(key)
    }

    fn iter(&self) -> impl Iterator<Item = (&K, &V)> { self.0.iter() }

    fn is_empty(&self) -> bool { self.0.is_empty() }

    fn contains_key(&self, key: &K) -> bool { self.0.contains_key(key) }
}
```

`Rc::make_mut` handles the copy-on-write semantics: if the reference count is 1 (sole owner), it mutates in place; otherwise it clones the inner BTreeMap and returns a mutable reference to the new allocation.

### 2. Replace BTreeMap fields in `PlanningState` with `SharedMap`

All 14 `BTreeMap` fields and the `BTreeSet` field in `PlanningState` are replaced with `SharedMap` (or `SharedSet` for the BTreeSet). The `snapshot` reference field is unchanged.

### 3. Replace `BTreeSet` with `SharedSet`

```rust
#[derive(Clone)]
struct SharedSet<K: Ord>(Rc<BTreeSet<K>>);
```

Same pattern as `SharedMap` but for the `removed_entities: BTreeSet<PlanningEntityRef>` field.

### 4. Benchmark before/after

Add a `#[cfg(feature = "bench-profiling")]` microbenchmark in the search module that measures per-expansion clone cost. Run before and after the change on the `golden_world_runs_without_observers` test to quantify the improvement.

## Component Registration

No new components. `PlanningState` is not a component; it is a transient struct internal to the search module.

## SystemFn Integration

No system changes. This is entirely within `worldwake-ai::search` and `worldwake-ai::planning_state`.

## Cross-System Interactions (Principle 12)

None. `PlanningState` is internal to the AI crate's search module. No other crate reads or writes it.

## FND-01 Section H

### H.1 Information-Path Analysis

No information paths are affected. `PlanningState` is a hypothetical simulation state used within a single `search_plan` invocation. It never enters the event log, world state, or belief stores.

### H.2 Positive-Feedback Analysis

No feedback loops introduced. The change is purely structural (data representation), not behavioral.

### H.3 Concrete Dampeners

N/A — no feedback loops.

### H.4 Stored vs Derived State

- **Stored (authoritative)**: None. `PlanningState` is never stored.
- **Derived (transient)**: The entire `PlanningState` struct is a transient derived computation from the `PlanningSnapshot` (which is itself derived from the agent's belief view). The `SharedMap`/`SharedSet` wrappers change the sharing pattern of this transient state, not its semantics.

## Invariants

1. `SharedMap::get` returns identical results to `BTreeMap::get` for any key.
2. Iteration order of `SharedMap` is identical to `BTreeMap` (BTree ordering preserved).
3. `PlanningState::clone()` produces a logically identical state (same query results for all fields).
4. Search outcomes are bit-identical before and after the change (same plans found, same expansion counts, same terminal kinds).
5. No `Rc` reference leaks across search invocations — all `SharedMap` instances are dropped when `search_plan` returns.

## Tests

- [ ] Unit tests for `SharedMap` and `SharedSet` (insert, get, remove, clone sharing, mutation after clone).
- [ ] Determinism test: `search_plan` with shared state produces identical results to `search_plan` with BTreeMap state (compare plans step-by-step).
- [ ] All existing golden tests pass unchanged.
- [ ] Profiling benchmark shows per-expansion clone cost reduction (target: >50% reduction in clone time for depth-6+ searches).

## Acceptance Criteria

1. `PlanningState::clone()` is O(1) when no fields have been mutated since the last clone.
2. `PlanningState::clone()` is O(k) where k = number of mutated fields (not total fields).
3. All 2700+ workspace tests pass with zero behavioral changes.
4. Golden test hash values are identical before and after (determinism preserved).
5. Profiling shows measurable reduction in search time for the `golden_world_runs_without_observers` test.

## References

- golden-perf campaign: exp-007 profiling, exp-012 search analysis
- `crates/worldwake-ai/src/planning_state.rs` — current implementation
- `crates/worldwake-ai/src/search/mod.rs` — search loop that clones PlanningState
