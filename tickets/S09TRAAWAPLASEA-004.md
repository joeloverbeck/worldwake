# S09TRAAWAPLASEA-004: Add goal-directed travel pruning

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new pruning step in search candidate generation
**Deps**: S09TRAAWAPLASEA-001 (distance matrix), S09TRAAWAPLASEA-002 (goal-relevant places), S09TRAAWAPLASEA-003 (heuristic integration — uses same goal_relevant_places plumbing)

## Problem

Even with A* ordering (ticket 003), the search still generates successor nodes for all travel directions before pruning via beam width. At hub nodes like VillageSquare (7+ edges), this wastes beam width slots on wrong-direction travel. This ticket adds a pruning step that eliminates travel actions moving the agent farther from ALL goal-relevant places, reducing the branching factor at hubs from 7+ to typically 1-3.

## Assumption Reassessment (2026-03-17)

1. Search candidates are generated via `search_candidates()` function in `search.rs` — confirmed. This is where pruning should be inserted.
2. `PlannerOpSemantics` has `op_kind: PlannerOpKind` field — confirmed at `planner_ops.rs:36`. `PlannerOpKind::Travel` exists at line 13.
3. The `semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>` is passed through `search_plan` — confirmed.
4. `SearchCandidate` (or equivalent) has `def_id: ActionDefId` and target entities — need to verify exact field names during implementation.
5. `PlanningSnapshot::min_travel_ticks_to_any` will exist after ticket 001 — dependency.
6. `goal_relevant_places` Vec will be available in the search loop after ticket 003 — dependency.

## Architecture Check

1. Pruning before beam width truncation is correct — we don't want wrong-direction travel consuming beam slots.
2. Only pruning `Travel` actions is conservative and safe — non-travel actions (pick_up, harvest, trade, etc.) are never pruned.
3. When `goal_places` is empty (no spatial preference), no pruning occurs — preserves correctness.
4. The `<=` comparison (keep if destination is closer OR same distance) prevents pruning the only path forward in linear topologies.

## What to Change

### 1. Add `prune_travel_away_from_goal` function

Add a function in `search.rs` that filters travel candidates:

```rust
fn prune_travel_away_from_goal(
    candidates: &mut Vec<SearchCandidate>,
    current_place: EntityId,
    goal_places: &[EntityId],
    snapshot: &PlanningSnapshot,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) {
    if goal_places.is_empty() { return; }
    let current_min = snapshot
        .min_travel_ticks_to_any(current_place, goal_places)
        .unwrap_or(u32::MAX);
    candidates.retain(|c| {
        let Some(sem) = semantics_table.get(&c.def_id) else { return true; };
        if sem.op_kind != PlannerOpKind::Travel { return true; }
        let Some(dest) = c.authoritative_targets.first() else { return true; };
        let dest_min = snapshot
            .min_travel_ticks_to_any(*dest, goal_places)
            .unwrap_or(u32::MAX);
        dest_min <= current_min
    });
}
```

Adapt field names (`def_id`, `authoritative_targets`) to match the actual `SearchCandidate` struct.

### 2. Call pruning from search loop

Insert the call to `prune_travel_away_from_goal` after `search_candidates()` returns and BEFORE beam width truncation. The `current_place` is the actor's simulated place in the current search node's state. The `goal_places` are the same `goal_relevant_places` computed once at the start of `search_plan` (plumbed in ticket 003).

## Files to Touch

- `crates/worldwake-ai/src/search.rs` (modify — add pruning function, call from search loop)

## Out of Scope

- Distance matrix computation (ticket 001)
- Goal-relevant places implementation (ticket 002)
- Heuristic ordering changes (ticket 003)
- Golden test changes (ticket 005)
- Pruning non-travel actions (explicitly forbidden by spec)
- Modifying `PlannerOpKind`, `PlannerOpSemantics`, or any core types
- Budget adjustments

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: At VillageSquare with goal-relevant place = GeneralStore, only the travel edge toward GeneralStore survives pruning (others are eliminated).
2. Unit test: With `goal_places` empty, no candidates are pruned (all retained).
3. Unit test: Non-travel candidates (Harvest, Craft, Trade, etc.) are never pruned regardless of goal places.
4. Unit test: Travel to a place at equal distance to a goal-relevant place is retained (not pruned).
5. Unit test: Travel to a place that is the ONLY path forward (linear topology) is retained.
6. All existing golden tests pass: `cargo test -p worldwake-ai` — no regressions.
7. `cargo clippy --workspace`

### Invariants

1. Only `PlannerOpKind::Travel` actions are subject to pruning — all other action types pass through.
2. When `goal_places` is empty, the function is a no-op.
3. Pruning is applied BEFORE beam width truncation.
4. The pruning condition is `dest_min <= current_min` (not strict `<`), preserving lateral movement options.
5. Determinism is preserved — `retain` preserves ordering of the candidate vector.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` — unit tests for pruning behavior with mock candidates and distance matrix queries

### Commands

1. `cargo test -p worldwake-ai search`
2. `cargo test -p worldwake-ai` (all golden tests)
3. `cargo test --workspace && cargo clippy --workspace`
