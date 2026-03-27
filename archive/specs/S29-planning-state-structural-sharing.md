**Status**: COMPLETED

# S29: Planning State Structural Sharing

## Summary

Replace the clone-per-expansion `PlanningState` and `Vec<PlannedStep>` patterns in GOAP search with copy-on-write wrappers using `Rc`. Each search expansion currently deep-clones 15 BTreeMaps, 1 BTreeSet, and a growing Vec of planned steps; the new design shares unmodified data between parent and child nodes via reference counting, reducing per-expansion cost from O(total state) to O(mutated fields). This is a pure Principle 11 optimization: it compresses computation without changing any world meaning or planning outcome.

## Why This Exists

Profiling during the golden-perf campaign (exp-007, exp-012) identified `PlanningState::clone()` as the single most expensive per-expansion operation in `search_plan`. The struct contains 15 BTreeMap fields and 1 BTreeSet field that are fully deep-cloned for every successor candidate at every expansion. With a budget of 256 max expansions and beam_width=8, a single search can clone 2000+ instances. Cost grows with search depth as states accumulate overrides. This dominates found-search time (47% of AI cost at the 30s baseline).

The budget tuning campaign reduced total golden test time from 92s to 30s, but the remaining 30s is still dominated by this clone pattern. No further parameter tuning can improve it without reducing plan quality below acceptable thresholds (the default beam_width=8 and max_expansions=256 cannot be reduced without breaking golden tests; some golden tests require even higher budgets — beam_width=16, max_expansions=1024 — making the clone cost proportionally worse).

### Mutation Pattern Analysis

Analysis of `apply_hypothetical_transition` and the `PlanningState` mutation methods shows that most expansions touch only a small fraction of the 16 collection fields:

| Action Type | Fields Mutated | Typical Count |
|---|---|---|
| Travel | entity_place_overrides | 1 |
| Consume / Sleep / Wash / Relieve | needs_overrides | 1 |
| Heal | pain_overrides | 1 |
| PickUp / PutDown / Steal | direct_possessor_overrides, direct_container_overrides, entity_place_overrides, commodity_quantity_overrides | 4 |
| PickUp with lot split | above + hypothetical_registry, commodity_quantity_overrides (second entry) | 6-7 |
| Loot | same as PickUp, per corpse lot | 4-7 |
| DeclareSupport / Bribe / Threaten | support_declaration_overrides + belief overrides | 2-4 |
| FacilityQueue operations | facility_queue_membership_overrides, facility_grant_overrides | 2 |

**Conclusion**: Typical expansions touch **1-4 fields**. With `Rc<BTreeMap>` copy-on-write, the 12-15 untouched fields are shared via O(1) reference count increment. Only the mutated fields pay a clone cost. This makes `Rc<BTreeMap>` highly effective without the complexity of per-key persistent data structures.

Additionally, `SearchNode.steps: Vec<PlannedStep>` is cloned at `search/transition.rs:124` for every successor. This Vec grows linearly with search depth (up to max_plan_depth=8, or 12 in expanded-budget tests). Wrapping it in `Rc<Vec>` with the same CoW pattern avoids deep-cloning the accumulated step history.

## Phase

Phase 3+: AI Architecture Overhaul

## Crates

- `worldwake-ai`

## Dependencies

- None (self-contained within the planning search module)

## Design Goals

1. **O(mutated fields) per expansion**: Each search node clone costs proportional to the number of mutated collection fields, not the total field count.
2. **Zero behavioral change**: Identical planning outcomes for any given search. The optimization is invisible to goal selection, plan validity, and all downstream systems.
3. **No external dependencies**: Use Rust's `Rc<BTreeMap>` / `Rc<Vec>` via `Rc::make_mut`; do not introduce external persistent-data-structure crates.
4. **Determinism preserved**: `BTreeMap` ordering guarantees retained. No `HashMap`. No floating point.
5. **Save/load unaffected**: `PlanningState` is transient (lives only within a search invocation) and is never serialized. No save format changes.

## Deliverables

### 1. `SharedMap<K, V>` wrapper type

A copy-on-write BTreeMap wrapper that shares the underlying allocation until mutation:

```rust
/// A copy-on-write BTreeMap that shares allocation across search expansions.
/// Cloning an unmodified SharedMap is O(1) (reference count increment).
/// First mutation triggers a full clone (copy-on-write via Rc::make_mut).
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

    fn entry(&mut self, key: K) -> std::collections::btree_map::Entry<'_, K, V> {
        Rc::make_mut(&mut self.0).entry(key)
    }

    fn iter(&self) -> impl Iterator<Item = (&K, &V)> { self.0.iter() }

    fn keys(&self) -> impl Iterator<Item = &K> { self.0.keys() }

    fn is_empty(&self) -> bool { self.0.is_empty() }

    fn contains_key(&self, key: &K) -> bool { self.0.contains_key(key) }
}
```

`Rc::make_mut` handles the copy-on-write semantics: if the reference count is 1 (sole owner), it mutates in place; otherwise it clones the inner BTreeMap and returns a mutable reference to the new allocation.

Note: `entry()` calls `Rc::make_mut` eagerly (before knowing whether insertion will happen). This is acceptable because `entry()` is only used in mutation contexts (`reservation_shadows.entry(entity).or_default().push(range)`) where a write is always intended.

### 2. `SharedSet<K>` wrapper type

```rust
#[derive(Clone)]
struct SharedSet<K: Ord>(Rc<BTreeSet<K>>);

impl<K: Ord + Clone> SharedSet<K> {
    fn new() -> Self { Self(Rc::new(BTreeSet::new())) }

    fn contains(&self, key: &K) -> bool { self.0.contains(key) }

    fn insert(&mut self, key: K) -> bool {
        Rc::make_mut(&mut self.0).insert(key)
    }

    fn is_empty(&self) -> bool { self.0.is_empty() }
}
```

Same CoW pattern as `SharedMap` but for the `removed_entities: BTreeSet<PlanningEntityRef>` field.

### 3. `SharedVec<T>` wrapper type

```rust
/// A copy-on-write Vec for sharing planned step history across search nodes.
/// Cloning is O(1); first mutation (push) triggers a full clone.
#[derive(Clone)]
struct SharedVec<T: Clone>(Rc<Vec<T>>);

impl<T: Clone> SharedVec<T> {
    fn new() -> Self { Self(Rc::new(Vec::new())) }

    fn push(&mut self, value: T) {
        Rc::make_mut(&mut self.0).push(value);
    }

    fn as_slice(&self) -> &[T] { &self.0 }

    fn len(&self) -> usize { self.0.len() }

    fn is_empty(&self) -> bool { self.0.is_empty() }

    fn iter(&self) -> std::slice::Iter<'_, T> { self.0.iter() }

    fn into_vec(self) -> Vec<T> {
        Rc::try_unwrap(self.0).unwrap_or_else(|rc| (*rc).clone())
    }
}
```

Used for `SearchNode.steps: SharedVec<PlannedStep>` to avoid cloning the accumulated step history at `search/transition.rs:124`.

### 4. Replace BTreeMap/BTreeSet fields in `PlanningState` with `SharedMap`/`SharedSet`

All 15 `BTreeMap` fields in `PlanningState` are replaced with `SharedMap`. The `removed_entities` `BTreeSet` field is replaced with `SharedSet`. The `snapshot` reference field and `next_hypothetical_id: u32` are unchanged.

### 5. Replace `Vec<PlannedStep>` in `SearchNode` with `SharedVec<PlannedStep>`

The `steps` field in `SearchNode` (used in `search/transition.rs`) is changed from `Vec<PlannedStep>` to `SharedVec<PlannedStep>`. The `into_vec()` method converts back to `Vec<PlannedStep>` when constructing the final `PlannedPlan` result.

### 6. Benchmark before/after

Add microbenchmarks measuring per-expansion clone cost. Run before and after the change on:
- `golden_world_runs_without_observers` (default budget: beam_width=8, max_expansions=256)
- At least one high-budget golden test from `golden_supply_chain.rs` (max_expansions=1024) or `golden_offices.rs` (beam_width=16)

Benchmarks should measure wall-clock time for the full `search_plan` invocation, not just clone cost in isolation, to capture the end-to-end impact.

## Component Registration

No new components. `PlanningState` is not a component; it is a transient struct internal to the search module. `SharedMap`, `SharedSet`, and `SharedVec` are private implementation details.

## SystemFn Integration

No system changes. This is entirely within `worldwake-ai::search`, `worldwake-ai::planning_state`, and the `SharedMap`/`SharedSet`/`SharedVec` wrapper types.

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
- **Derived (transient)**: The entire `PlanningState` struct is a transient derived computation from the `PlanningSnapshot` (which is itself derived from the agent's belief view). The `SharedMap`/`SharedSet`/`SharedVec` wrappers change the sharing pattern of this transient state, not its semantics.

## Invariants

1. `SharedMap::get` returns identical results to `BTreeMap::get` for any key.
2. Iteration order of `SharedMap` is identical to `BTreeMap` (BTree ordering preserved).
3. `SharedSet::contains` returns identical results to `BTreeSet::contains` for any key.
4. `SharedVec` maintains insertion order identical to `Vec`.
5. `PlanningState::clone()` produces a logically identical state (same query results for all fields).
6. Search outcomes are bit-identical before and after the change (same plans found, same expansion counts, same terminal kinds).
7. No `Rc` reference leaks across search invocations — all shared wrappers are dropped when `search_plan` returns.
8. `SharedMap::entry()` is semantically equivalent to `BTreeMap::entry()` — the eager `Rc::make_mut` call does not change observable behavior.

## Tests

- [ ] Unit tests for `SharedMap` (insert, get, remove, entry, keys, clone sharing, mutation after clone, independence of cloned copies).
- [ ] Unit tests for `SharedSet` (insert, contains, is_empty, clone sharing, mutation after clone).
- [ ] Unit tests for `SharedVec` (push, as_slice, clone sharing, into_vec, mutation after clone).
- [ ] Determinism test: `search_plan` with shared state produces identical results to the pre-change implementation (compare plans step-by-step).
- [ ] All existing golden tests pass unchanged.
- [ ] Benchmark shows per-expansion clone cost reduction on both default-budget and high-budget tests (target: >50% reduction in clone time for depth-6+ searches).

## Acceptance Criteria

1. `PlanningState::clone()` is O(1) when no fields have been mutated since the last clone.
2. `PlanningState::clone()` is O(k) where k = number of mutated fields (not total fields).
3. `SearchNode` clone (including steps) is O(1) when steps have not been modified since the last clone.
4. All workspace tests pass with zero behavioral changes.
5. Golden test hash values are identical before and after (determinism preserved).
6. Benchmarks show measurable reduction in search time for both default-budget and high-budget golden tests.

## Ticket Decomposition

### S29-001: SharedMap, SharedSet, and SharedVec wrapper types

**Scope**: Create `SharedMap<K, V>`, `SharedSet<K>`, and `SharedVec<T>` as private types in `worldwake-ai`. Full API as specified in Deliverables 1-3. Comprehensive unit tests for each type covering: construction, get/insert/remove/entry/keys, clone sharing (verify Rc refcount), mutation independence after clone, iteration order matches BTreeMap/BTreeSet/Vec.

**Files**: New file `crates/worldwake-ai/src/shared_collections.rs` (or inline in `planning_state.rs`).

### S29-002: Migrate PlanningState to SharedMap/SharedSet

**Scope**: Replace all 15 BTreeMap fields with `SharedMap` and the `removed_entities` BTreeSet with `SharedSet` in `PlanningState`. Update all mutation methods. Verify all existing tests pass with zero behavioral change.

**Files**: `crates/worldwake-ai/src/planning_state.rs`.

### S29-003: Migrate SearchNode steps to SharedVec

**Scope**: Replace `Vec<PlannedStep>` in `SearchNode` with `SharedVec<PlannedStep>`. Update `search/transition.rs` clone+push site and the final plan construction to use `into_vec()`. Verify all existing tests pass.

**Files**: `crates/worldwake-ai/src/search/mod.rs`, `crates/worldwake-ai/src/search/transition.rs`.

### S29-004: Benchmarking and verification

**Scope**: Add before/after benchmarks for `search_plan` on default-budget and high-budget golden tests. Run full workspace test suite. Verify golden test hash determinism. Document measured improvement.

**Files**: Benchmark harness in `crates/worldwake-ai/` (test or bench module).

## References

- golden-perf campaign: exp-007 profiling, exp-012 search analysis
- `crates/worldwake-ai/src/planning_state.rs` — current implementation (15 BTreeMap + 1 BTreeSet, lines 38-60)
- `crates/worldwake-ai/src/search/mod.rs` — search loop
- `crates/worldwake-ai/src/search/transition.rs` — state clone (line 96), steps clone (line 124)
- `crates/worldwake-ai/src/planner_ops.rs` — `apply_hypothetical_transition` (lines 419-453)
- `crates/worldwake-ai/src/budget.rs` — PlanningBudget defaults (beam_width=8, max_expansions=256)

## Outcome

- Completed: 2026-03-27
- What changed: `SharedMap`, `SharedSet`, and `SharedVec` shipped in `worldwake-ai`; `PlanningState` and `SearchNode` now use copy-on-write structural sharing; ignored benchmark wrappers were added to the existing golden determinism, supply-chain, and offices suites for manual perf checks.
- Deviations from original plan: the implementation landed across `S29-001` through `S29-003` before this closeout ticket. Benchmark proof was also more nuanced than the original plan predicted: `golden_world_runs_without_observers` improved from `3.188s` average on pre-S29 `1791bf0` to `1.908s` average on current `main` (about 40% faster), `bench_high_budget_prerequisite_replan` was effectively flat (`425.9ms` -> `419.5ms` average), and the branch-heavy office coalition benchmark regressed modestly (`160.0ms` -> `187.0ms` average). The optimization is therefore a clear architectural/asymptotic improvement with workload-dependent end-to-end payoff, not a universal speedup across every golden.
- Verification results: `cargo test -p worldwake-ai --test golden_determinism bench_world_runs_without_observers -- --ignored --nocapture`, `cargo test -p worldwake-ai --test golden_supply_chain bench_high_budget_prerequisite_replan -- --ignored --nocapture`, `cargo test -p worldwake-ai --test golden_offices bench_branchy_office_coalition -- --ignored --nocapture`, `cargo test --workspace`, and `cargo clippy --workspace` all passed on the completed implementation.
