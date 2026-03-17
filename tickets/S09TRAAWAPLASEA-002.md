# S09TRAAWAPLASEA-002: Add goal_relevant_places to GoalKindPlannerExt

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new method on `GoalKindPlannerExt` trait
**Deps**: None (parallel with S09TRAAWAPLASEA-001)

## Problem

The A* heuristic (ticket 003) and travel pruning (ticket 004) need to know which places are relevant for achieving a given goal. This ticket adds a `goal_relevant_places` method to the `GoalKindPlannerExt` trait, implemented for all `GoalKind` variants.

## Assumption Reassessment (2026-03-17)

1. `GoalKindPlannerExt` trait is defined in `crates/worldwake-ai/src/goal_model.rs` with methods `goal_kind_tag`, `relevant_op_kinds`, `relevant_observed_commodities`, `build_payload_override`, `apply_planner_step`, `is_satisfied`, `is_progress_barrier` — confirmed.
2. `GoalKind` has 17 variants: ConsumeOwnedCommodity, AcquireCommodity, Sleep, Relieve, Wash, EngageHostile, ReduceDanger, Heal, ProduceCommodity, SellCommodity, RestockCommodity, MoveCargo, LootCorpse, BuryCorpse, ShareBelief, ClaimOffice, SupportCandidateForOffice — confirmed in `crates/worldwake-core/src/goal.rs`.
3. `PlanningState` provides `effective_place_ref()`, `snapshot()`, and traversal of entities/places — confirmed. No `actor_place()` method exists; use `snapshot().actor()` + `effective_place_ref()`.
4. `PlanningSnapshot.places` is `BTreeMap<EntityId, SnapshotPlace>` with `SnapshotPlace.tags: BTreeSet<PlaceTag>` and `SnapshotPlace.entities: BTreeSet<EntityId>` — confirmed.
5. `SnapshotEntity` has fields for `commodity_kind`, `resource_source_commodity`, `workstation_tag`, `merchandise_profile`, etc. — confirmed.
6. The spec's goal-to-place mapping table lists 13 goal kinds with spatial preferences — confirmed.

## Architecture Check

1. Adding a new method to `GoalKindPlannerExt` is the cleanest extension point — it follows the existing pattern where each goal kind declares its planner-relevant properties.
2. The method takes `&PlanningState` (not `&PlanningSnapshot`) because some goals' relevant places depend on the simulated state (e.g., RestockCommodity changes based on whether the agent already holds the commodity in the planning state).

## What to Change

### 1. Add `goal_relevant_places` method to `GoalKindPlannerExt` trait

```rust
fn goal_relevant_places(&self, state: &PlanningState<'_>) -> Vec<EntityId>;
```

Returns the list of place EntityIds where this goal can potentially be achieved. Returns empty if no spatial preference (heuristic will default to 0).

### 2. Implement for all GoalKind variants

Per the spec mapping table:

| GoalKind | Relevant Places |
|----------|----------------|
| `ConsumeOwnedCommodity` | Actor's current place (already possesses the commodity). If not possessed: places where commodity exists. |
| `AcquireCommodity` | Places with resource sources for the commodity, places with merchants selling it. |
| `Sleep` | Places with sleep-compatible entities (beds). |
| `Relieve` | Places with latrine/relief facilities. |
| `Wash` | Places with wash basins. |
| `ProduceCommodity` | Places with required workstations. |
| `RestockCommodity` | If agent doesn't have commodity: places with resource sources. If agent has commodity: home market place. |
| `MoveCargo { destination }` | The destination place. |
| `SellCommodity` | Places with potential buyers (merchants). |
| `EngageHostile { target }` | Place of the target entity. |
| `Heal` | Actor's current place or places with healers. |
| `LootCorpse { corpse }` | Place of the corpse. |
| `ShareBelief { target }` | Place of the target entity. |
| `ReduceDanger` | Empty (no spatial preference — danger reduction depends on context). |
| `BuryCorpse` | Place of the corpse. |
| `ClaimOffice` | Empty (no spatial preference for now). |
| `SupportCandidateForOffice` | Empty (no spatial preference for now). |

Use `PlanningState` methods to query entity positions, inventory, and properties. Scan `snapshot.places` for matching tags/entities where needed.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — add trait method + implement for all variants)

## Out of Scope

- Modifying `search.rs` (ticket 003)
- Distance matrix / `PlanningSnapshot` changes (ticket 001)
- Travel pruning logic (ticket 004)
- Golden test changes (ticket 005)
- Adding new `GoalKind` variants
- Changing existing `GoalKindPlannerExt` method signatures
- Any changes to `worldwake-core` or `worldwake-sim`

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `MoveCargo { destination: place_x }.goal_relevant_places(state)` returns `[place_x]`.
2. Unit test: `RestockCommodity` when agent does NOT hold the commodity returns places with resource sources for that commodity.
3. Unit test: `RestockCommodity` when agent DOES hold the commodity returns the home market place.
4. Unit test: `ConsumeOwnedCommodity` when agent possesses the commodity returns actor's current place.
5. Unit test: `AcquireCommodity` returns places with resource sources or merchants for the commodity.
6. Unit test: `EngageHostile { target }` returns the place where the target entity is located.
7. Unit test: `LootCorpse { corpse }` returns the place where the corpse is located.
8. Unit test: `ReduceDanger` returns empty vec (no spatial preference).
9. Unit test: All 17 GoalKind variants have a `goal_relevant_places` implementation (exhaustive match — adding a variant causes a compile error).
10. Existing suite: `cargo test -p worldwake-ai`
11. `cargo clippy --workspace`

### Invariants

1. The match in `goal_relevant_places` is exhaustive — no wildcard arm.
2. Returns `Vec<EntityId>` of place IDs only (not entity IDs that aren't places).
3. Empty return means "no spatial preference" — the heuristic will treat this as h=0.
4. No new authoritative state is stored — this is a pure computation.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — unit tests for goal-relevant-place resolution for key goal kinds using test snapshots/states

### Commands

1. `cargo test -p worldwake-ai goal_model`
2. `cargo test --workspace && cargo clippy --workspace`
