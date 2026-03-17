# S09TRAAWAPLASEA-002: Add goal_relevant_places to GoalKindPlannerExt

**Status**: COMPLETED
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
fn goal_relevant_places(&self, state: &PlanningState<'_>, recipes: &RecipeRegistry) -> Vec<EntityId>;
```

Returns the list of place EntityIds where this goal can potentially be achieved. Returns empty if no spatial preference (heuristic will default to 0).

The `&RecipeRegistry` parameter follows the existing trait pattern (`relevant_observed_commodities` also takes `&RecipeRegistry`). It is needed by `ProduceCommodity` to look up the recipe's `required_workstation_tag` so the heuristic can target the *specific* workstation places rather than all workstation places (Principle 18: precise guidance over imprecise approximation; Principle 25: derive from the most specific concrete state available).

### 2. Implement for all GoalKind variants

Per the spec mapping table, corrected against actual action constraints:

| GoalKind | Relevant Places | Rationale |
|----------|----------------|-----------|
| `ConsumeOwnedCommodity` | Actor's current place (already possesses the commodity). If not possessed: places where commodity exists. | Consumption requires possession; if possessed, can consume at current place. |
| `AcquireCommodity` | Places with resource sources for the commodity, places with merchants selling it. | Agent needs to travel to sources or sellers. |
| `Sleep` | Empty (no spatial preference). | Sleep action has only `Constraint::ActorAlive` — no place constraint (`needs_actions.rs:105`). Returning specific places would create an inadmissible heuristic (h > 0 when actual cost is 0 at current place). |
| `Relieve` | Places with `PlaceTag::Latrine`. | Toilet action has explicit `Constraint::ActorAtPlaceTag(PlaceTag::Latrine)` (`needs_actions.rs:103`). |
| `Wash` | Empty (no spatial preference). | Wash action requires possessed Water item lot but has no place constraint (`needs_actions.rs:105`). Same admissibility concern as Sleep. |
| `ProduceCommodity` | Places with the *specific* `WorkstationTag` from `recipes.get(recipe_id).required_workstation_tag`. If recipe has no workstation requirement, empty vec. | Uses `RecipeRegistry` for precise heuristic per Principle 18. |
| `RestockCommodity` | If agent doesn't have commodity: places with resource sources. If agent has commodity: home market places (from demand memory). | Dual-phase behavior guides outbound then return trip. |
| `MoveCargo { destination }` | The destination place. | Direct spatial target. |
| `SellCommodity` | Places with potential buyers (merchants). | Agent needs to travel to buyers. |
| `EngageHostile { target }` | Place of the target entity. | Must co-locate with target. |
| `Heal` | Actor's current place or places with healers. | Treatment may require co-location with healer. |
| `LootCorpse { corpse }` | Place of the corpse. | Must co-locate with corpse. |
| `ShareBelief { listener }` | Place of the listener entity. | Must co-locate with listener (Principle 7: locality). |
| `ReduceDanger` | Empty (no spatial preference). | Danger reduction depends on context (flee, defend, heal). |
| `BuryCorpse { corpse }` | Place of the corpse. | Must co-locate with corpse. |
| `ClaimOffice` | Empty (no spatial preference for now). | |
| `SupportCandidateForOffice` | Empty (no spatial preference for now). | |

Use `PlanningState` methods to query entity positions, inventory, and properties. Scan `snapshot.places` for matching tags/entities where needed.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — add trait method + implement for all variants)

## Out of Scope

- Modifying `search.rs` (ticket 003)
- Distance matrix / `PlanningSnapshot` changes (ticket 001)
- Travel pruning logic (ticket 004)
- Golden test changes (ticket 005)
- Adding new `GoalKind` variants
- Any changes to `worldwake-core` or `worldwake-sim`

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `MoveCargo { destination: place_x }.goal_relevant_places(state, recipes)` returns `[place_x]`.
2. Unit test: `RestockCommodity` when agent does NOT hold the commodity returns places with resource sources for that commodity.
3. Unit test: `RestockCommodity` when agent DOES hold the commodity returns the home market place.
4. Unit test: `ConsumeOwnedCommodity` when agent possesses the commodity returns actor's current place.
5. Unit test: `AcquireCommodity` returns places with resource sources or merchants for the commodity.
6. Unit test: `EngageHostile { target }` returns the place where the target entity is located.
7. Unit test: `LootCorpse { corpse }` returns the place where the corpse is located.
8. Unit test: `ReduceDanger` returns empty vec (no spatial preference).
9. Unit test: `Sleep` returns empty vec (no place constraint).
10. Unit test: `Wash` returns empty vec (no place constraint).
11. Unit test: `Relieve` returns places with `PlaceTag::Latrine`.
12. Unit test: `ProduceCommodity` returns places with the specific workstation tag from the recipe.
13. Unit test: All 17 GoalKind variants have a `goal_relevant_places` implementation (exhaustive match — adding a variant causes a compile error).
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

## Outcome

**Completion date**: 2026-03-17

**What changed**:
- Added `goal_relevant_places(&self, state: &PlanningState<'_>, recipes: &RecipeRegistry) -> Vec<EntityId>` to `GoalKindPlannerExt` trait in `crates/worldwake-ai/src/goal_model.rs`
- Exhaustive match implementation for all 17 `GoalKind` variants (no wildcard arm)
- 5 helper functions: `places_with_resource_source`, `places_with_sellers`, `places_with_place_tag`, `places_with_workstation`, `demand_memory_places`
- Extended `TestBeliefView` with `resource_sources`, `workstation_tags`, `place_tags` fields and `place_has_tag`/`resource_sources_at` overrides
- 14 new unit tests covering all acceptance criteria

**Deviations from original plan**:
1. Method signature changed from `fn goal_relevant_places(&self, state: &PlanningState<'_>)` to include `recipes: &RecipeRegistry` — follows existing trait pattern (`relevant_observed_commodities`), enables precise workstation targeting for `ProduceCommodity` per Principle 18.
2. `Sleep` → empty vec (original ticket said "places with beds" but sleep action has no place constraint; returning places would violate A* admissibility).
3. `Wash` → empty vec (original ticket said "places with wash basins" but wash action has no place constraint; same admissibility concern).
4. `Relieve` → places with `PlaceTag::Latrine` (confirmed via `Constraint::ActorAtPlaceTag(PlaceTag::Latrine)` in `needs_actions.rs`).
5. Removed "Changing existing `GoalKindPlannerExt` method signatures" from Out of Scope since the new method was added to the trait.

**Verification**:
- `cargo test --workspace` → 1,794 passed, 0 failed, 2 ignored
- `cargo clippy --workspace` → clean
