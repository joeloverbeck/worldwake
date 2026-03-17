**Status**: DRAFT

# S09: Travel-Aware Plan Search

## Summary

Replace the current blind uniform-cost search in the GOAP planner with an A*-style search that uses shortest-path travel distances as a heuristic, and add goal-directed travel pruning to eliminate wasted node expansions. This addresses the fundamental architectural gap where the planner exhausts its budget at hub nodes because it explores all travel directions equally, with no knowledge of which direction leads toward goal-relevant locations.

## Why This Exists

### The Problem

The GOAP plan search (`search.rs`) currently orders frontier nodes by `total_estimated_ticks` (g-cost only) with no goal-directed heuristic. This is Dijkstra/uniform-cost search. When an agent is at a hub node (e.g., VillageSquare with 7+ outgoing travel edges), the search expands travel actions in all 7 directions equally. For a 6-step plan through a hub, this creates a combinatorial explosion:

- Depth 1: ~7 travel options + non-travel actions
- Depth 2: ~7 × 7 = 49 travel combinations
- ...
- Depth 4 (at VillageSquare): 7+ branches explored before GeneralStore (1 hop away) is found

With `max_node_expansions: 512` and `beam_width: 8`, the search exhausts its budget before finding multi-hop plans from remote locations. Even 1024 expansions is insufficient for a 6-step MoveCargo plan from OrchardFarm through VillageSquare to GeneralStore when 3+ agents are active.

### Evidence

The SUPPLYCHAINFIX-001 golden test diagnosed this via the S08 decision trace system:

1. **Segment test (2 agents)**: Merchant plans RestockCommodity at tick 0, plan continuation preserves the plan across SnapshotChanged ticks. Works with default 512 budget.

2. **Combined test (3 agents)**: When the merchant's goal transitions from RestockCommodity to MoveCargo at OrchardFarm, plan continuation cannot apply (goal changed). The fresh plan search for MoveCargo(Apple, GeneralStore) — requiring pick_up → 4 travels → put_down (6 steps, depth 6) — exhausts the budget at VillageSquare. The merchant is permanently stuck. Even 1024 expansions is insufficient.

3. **Root cause confirmed**: `compare_search_nodes` (search.rs:466-471) orders by `total_estimated_ticks` then `steps.len()` then `steps` identity. There is no heuristic component. The search is spatially blind.

### Why Budget Tuning Is Not the Fix

Increasing `max_node_expansions` is a patch, not a solution:

- It increases worst-case planning time linearly.
- The required budget grows exponentially with hub connectivity. Adding one more hub node to the world could make 2048 insufficient.
- It violates Principle 18 (Resource-Bounded Practical Reasoning) — agents should plan efficiently, not just with more budget.
- The real issue is that 99%+ of expanded nodes are in wrong directions. The search should not expand them.

## Phase

Post-Phase-2 hardening, no dependency on E14 or later epics. Depends on the existing search infrastructure in `worldwake-ai/src/search.rs` and `PlanningSnapshot` in `worldwake-ai/src/planning_snapshot.rs`.

## Crates

- `worldwake-ai` (primary — distance matrix, heuristic, pruning, goal-place resolution)
- `worldwake-core` (minor — optional: expose all-pairs shortest-path utility on `Topology`)

## Design Principles

1. **Admissible heuristic** — the h-cost never overestimates actual travel cost, preserving plan optimality.
2. **Zero regression** — all existing golden tests pass with the same or lower expansion counts.
3. **Goal-kind extensible** — new GoalKind variants automatically get travel guidance by implementing one method.
4. **Budget-neutral** — the default `max_node_expansions: 512` should suffice for all current scenarios including the 3-agent supply chain.
5. **Deterministic** — `BTreeMap` for distance matrices, deterministic iteration order throughout.

## Architecture

### Component 1: Distance Matrix on PlanningSnapshot

Precompute all-pairs shortest travel times during `PlanningSnapshot` construction. The snapshot already captures `adjacent_places_with_travel_ticks` for places within `snapshot_travel_horizon` hops. This component extends that to a full distance matrix.

```rust
pub struct PlanningSnapshot {
    // ... existing fields ...
    /// All-pairs shortest travel times between snapshot places.
    /// Computed via Floyd-Warshall during construction. O(n^3) where n is
    /// the number of places in the snapshot (typically 10-20, so < 8000 ops).
    shortest_travel_ticks: BTreeMap<(EntityId, EntityId), u32>,
}

impl PlanningSnapshot {
    /// Minimum travel ticks from `from` to `to`, or None if unreachable.
    pub fn min_travel_ticks(&self, from: EntityId, to: EntityId) -> Option<u32> {
        if from == to { return Some(0); }
        self.shortest_travel_ticks.get(&(from, to)).copied()
    }

    /// Minimum travel ticks from `from` to the nearest place in `destinations`.
    pub fn min_travel_ticks_to_any(
        &self,
        from: EntityId,
        destinations: &[EntityId],
    ) -> Option<u32> {
        if destinations.contains(&from) { return Some(0); }
        destinations.iter()
            .filter_map(|dest| self.shortest_travel_ticks.get(&(from, *dest)))
            .copied()
            .min()
    }
}
```

**Algorithm**: Floyd-Warshall on the snapshot's place adjacency graph. Initialize the distance matrix from `adjacent_places_with_travel_ticks`. Iterate all triples `(k, i, j)` and relax `dist[i][j] = min(dist[i][j], dist[i][k] + dist[k][j])`. Use `BTreeMap<(EntityId, EntityId), u32>` for the matrix (sparse, deterministic iteration).

**Cost**: For 15 places, this is 15^3 = 3,375 operations — negligible compared to the plan search itself. Computed once per `PlanningSnapshot` construction (once per agent per tick when planning).

### Component 2: Goal-Relevant Places Resolution

Each `GoalKind` must declare which places are relevant for achieving the goal. This enables the heuristic to guide travel toward those places.

```rust
/// Extension to GoalKindPlannerExt trait.
trait GoalKindPlannerExt {
    // ... existing methods ...

    /// Places where this goal can potentially be achieved.
    /// Used by the A* heuristic to guide travel.
    /// Returns empty if the goal has no spatial preference (e.g., Sleep
    /// can happen anywhere with a bed — all places with beds are relevant).
    fn goal_relevant_places(&self, state: &PlanningState<'_>) -> Vec<EntityId>;
}
```

**Goal-to-place mapping**:

| GoalKind | Relevant Places |
|----------|----------------|
| `ConsumeOwnedCommodity` | Actor's current place (already possesses the commodity). If not possessed: places where commodity exists. |
| `AcquireCommodity` | Places with resource sources for the commodity, places with merchants selling it. |
| `Sleep` | Places with sleep-compatible entities. |
| `Relieve` | Places with latrine/relief facilities. |
| `Wash` | Places with wash basins. |
| `ProduceCommodity` | Places with required workstations. |
| `RestockCommodity` | Places with resource sources for the commodity (outbound leg) OR home market (return leg, if commodity already held). |
| `MoveCargo { destination }` | The destination place. |
| `SellCommodity` | Places with potential buyers. |
| `EngageHostile { target }` | Place of the target entity. |
| `Heal` | Actor's current place (if treatment available) or places with healers. |
| `LootCorpse { corpse }` | Place of the corpse. |
| `ShareBelief { target }` | Place of the target entity. |

**Key insight for RestockCommodity**: The relevant places change based on the agent's current state. If the agent doesn't have the commodity yet, relevant places are sources. If the agent already has the commodity, the relevant place is the home market. This dual-phase behavior is what makes RestockCommodity plans span the full outbound+return trip — the heuristic guides the agent toward the source first, then toward home.

### Component 3: A* Heuristic in Plan Search

Replace the pure g-cost ordering with f = g + h:

```rust
struct SearchNode<'snapshot> {
    state: PlanningState<'snapshot>,
    steps: Vec<PlannedStep>,
    total_estimated_ticks: u32,
    /// Heuristic: minimum travel ticks from current simulated position
    /// to the nearest goal-relevant place. Zero when already at a
    /// goal-relevant place or when no spatial guidance is available.
    heuristic_ticks: u32,
}

fn compare_search_nodes(left: &SearchNode<'_>, right: &SearchNode<'_>) -> Ordering {
    let left_f = left.total_estimated_ticks.saturating_add(left.heuristic_ticks);
    let right_f = right.total_estimated_ticks.saturating_add(right.heuristic_ticks);
    left_f.cmp(&right_f)
        .then_with(|| left.total_estimated_ticks.cmp(&right.total_estimated_ticks))
        .then_with(|| left.steps.len().cmp(&right.steps.len()))
        .then_with(|| left.steps.cmp(&right.steps))
}
```

**Heuristic computation** in `build_successor`:

```rust
let current_place = successor_state.actor_place();
let heuristic_ticks = snapshot
    .min_travel_ticks_to_any(current_place, &goal_relevant_places)
    .unwrap_or(0);
```

The heuristic is **admissible** (shortest-path distance never overestimates actual travel, since the agent must traverse at least those edges) and **consistent** (h(n) ≤ cost(n, n') + h(n') for any successor n'), guaranteeing A* optimality.

**Tie-breaking**: When f-costs are equal, prefer lower g-cost (more heuristic, less committed cost). Then step count, then step identity — preserving existing determinism.

### Component 4: Goal-Directed Travel Pruning

As a secondary optimization, prune travel actions that move the agent farther from ALL goal-relevant places. This reduces the branching factor at hub nodes from 7+ to typically 1-3.

```rust
fn prune_travel_away_from_goal(
    candidates: &mut Vec<SearchCandidate>,
    current_place: EntityId,
    goal_places: &[EntityId],
    snapshot: &PlanningSnapshot,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) {
    if goal_places.is_empty() {
        return; // No spatial guidance — don't prune.
    }
    let current_min = snapshot
        .min_travel_ticks_to_any(current_place, goal_places)
        .unwrap_or(u32::MAX);

    candidates.retain(|c| {
        let Some(sem) = semantics_table.get(&c.def_id) else { return true };
        if sem.op_kind != PlannerOpKind::Travel { return true; }
        let Some(dest) = c.authoritative_targets.first() else { return true; };

        let dest_min = snapshot
            .min_travel_ticks_to_any(*dest, goal_places)
            .unwrap_or(u32::MAX);

        // Keep if destination is closer to (or same distance from) any goal place.
        dest_min <= current_min
    });
}
```

**Safety**: Only prunes Travel actions. Non-travel actions (pick_up, harvest, trade, etc.) are never pruned. If `goal_places` is empty (goal has no spatial preference), no pruning occurs. This is conservative — it only removes travel that provably moves away from all known goal-relevant locations.

**Interaction with beam_width**: Travel pruning runs BEFORE beam_width truncation. This means beam_width slots are not wasted on wrong-direction travel.

## FND-01 Section H Analysis

### Information-path analysis

The distance matrix is derived from topology data already present in the `PlanningSnapshot`. No new information paths are created. The heuristic is a read-side computation over existing spatial data. Agents do not gain new information about the world — they simply use existing topology knowledge more efficiently during planning.

### Positive-feedback analysis

No positive feedback loops. The heuristic and pruning are stateless computations within a single plan search invocation. They do not modify world state, agent memory, or any persistent data structure.

### Concrete dampeners

N/A — no feedback loops to dampen.

### Stored state vs. derived read-model list

- **Stored state**: `PlanningSnapshot.shortest_travel_ticks: BTreeMap<(EntityId, EntityId), u32>` — computed during snapshot construction, immutable thereafter.
- **Derived**: `heuristic_ticks` on `SearchNode` — computed per-node during search, not stored beyond the search's lifetime.
- **Derived**: `goal_relevant_places` — computed once at the start of `search_plan`, not stored.
- **Derived**: Travel pruning decisions — computed inline during `search_candidates`.

No derived value is stored as authoritative state.

## Implementation Plan

### Ticket 1: Distance Matrix on PlanningSnapshot

- Add Floyd-Warshall all-pairs shortest path computation to `PlanningSnapshot` construction in `planning_snapshot.rs`.
- Add `min_travel_ticks` and `min_travel_ticks_to_any` query methods.
- Unit tests: verify distances on the prototype world topology (GeneralStore→OrchardFarm should be 1+3+2+2 = 8 ticks via VillageSquare→SouthGate→EastFieldTrail→OrchardFarm).
- Determinism test: same snapshot produces same distance matrix.

### Ticket 2: Goal-Relevant Places Resolution

- Add `goal_relevant_places(&self, state: &PlanningState<'_>) -> Vec<EntityId>` to `GoalKindPlannerExt` trait in `goal_model.rs`.
- Implement for all `GoalKind` variants per the mapping table above.
- Unit tests: verify correct places for `MoveCargo`, `RestockCommodity` (both phases), `ConsumeOwnedCommodity`, `AcquireCommodity`.

### Ticket 3: A* Heuristic in Plan Search

- Add `heuristic_ticks` field to `SearchNode`.
- Pass `goal_relevant_places` and distance matrix through `search_plan`.
- Modify `build_successor` to compute `heuristic_ticks` for each successor.
- Modify `compare_search_nodes` to use f = g + h ordering.
- Unit tests: verify that the search finds the same plans with fewer expansions. Golden tests must not regress.

### Ticket 4: Goal-Directed Travel Pruning

- Add `prune_travel_away_from_goal` in `search.rs`, called from `search_candidates`.
- Unit tests: verify that at VillageSquare with goal GeneralStore, only the GeneralStore travel edge survives pruning.
- Integration test: the 3-agent combined supply chain test with default 512 budget.

### Ticket 5: Enable Full Supply Chain Golden Tests

- Remove `#[ignore]` from `test_full_supply_chain` and `test_full_supply_chain_replay` in `golden_supply_chain.rs`.
- Reduce the combined test's budget from 1024 back to `PlanningBudget::default()` (512).
- Both tests must pass with default budget and realistic agent metabolism.
- Deterministic replay must hold.

## Acceptance Criteria

1. All existing golden tests pass with default `PlanningBudget` (512 expansions, beam width 8).
2. The full 3-agent supply chain test (`test_full_supply_chain`) passes with default budget.
3. `test_full_supply_chain_replay` passes — deterministic replay is preserved.
4. `cargo test --workspace` — no regressions.
5. `cargo clippy --workspace` — clean.
6. Node expansion counts for multi-hop plans through VillageSquare are demonstrably lower (verifiable via decision traces or explicit test assertions on expansion counts).

## What This Does Not Cover

- **Budget auto-scaling** per agent position. The A* heuristic and travel pruning should make the fixed 512 budget sufficient. If future world maps have higher connectivity, budget scaling may be revisited.
- **Hierarchical planning** (HTN-style task decomposition). The flat GOAP search with A* heuristic is sufficient for the current world scale and plan depths.
- **Perception (E14)** for consumer observation of merchant arrivals. The combined test uses belief seeding. Perception is a separate system.
- **Separating movement from planning (F.E.A.R. pattern)**. Travel cost is a first-class planning concern in a place-graph world. The current architecture of treating travel as a regular planner action with duration costs is correct.

## Principles Alignment

| Principle | How This Spec Serves It |
|-----------|------------------------|
| 7. Locality of Information | The heuristic uses only topology data already in the snapshot — no global queries. |
| 8. Feedback Dampening | No feedback loops introduced. Heuristic is a stateless read-side computation. |
| 12. System Decoupling | Changes are contained in worldwake-ai. No changes to action framework, affordance system, or execution pipeline. |
| 18. Resource-Bounded Practical Reasoning | The heuristic makes planning efficient within fixed budgets, rather than requiring budget increases. Agents plan smarter, not longer. |
| 27. Debuggability | Decision traces already capture `BudgetExhausted` and `expansions_used`. The heuristic values can optionally be included in `PlannedStepSummary` for diagnostic inspection. |

## Key References

- Jeff Orkin, "Three States and a Plan: The A.I. of F.E.A.R." (GDC 2006) — canonical GOAP reference; demonstrates separation of movement from planning (pattern we deliberately do NOT follow).
- Eric Jacopin, "Optimizing Practical Planning for Game AI" (Game AI Pro 2, 2014) — analysis of GOAP plan lengths and optimization patterns.
- Existing worldwake infrastructure: `Topology::shortest_path` (Dijkstra, core/topology.rs), `PlanningSnapshot` adjacency data, `PlanningBudget` configuration.
