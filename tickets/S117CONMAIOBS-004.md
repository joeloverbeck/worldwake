# S117CONMAIOBS-004: `RecipeMonoculture` detector

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: `archive/tickets/S117CONMAIOBS-001.md`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

In the survival-contested run, every agent knew both Harvest Apples and Harvest Grain, but Harvest Grain saw zero commits across 1440 ticks while Harvest Apples saw 64. The divergence is a real behavioral signal — an agent is not exercising its full known-recipe repertoire for a given need category — but no current detector surfaces it. This ticket adds a detector that flags a per-need-category recipe share ≥ 95% when the agent had the belief substrate to execute alternatives.

## Assumption Reassessment (2026-04-18)

1. `KnownRecipes` component is defined at `crates/worldwake-core/src/production.rs:40` as `pub struct KnownRecipes { pub recipes: BTreeSet<RecipeId> }` and implements `Component`. The observer can read it per-agent via the standard `get_component_known_recipes(agent)` accessor (pattern matches `get_component_drive_thresholds` and `get_component_homeostatic_needs`).
2. `RecipeDefinition` is defined at `crates/worldwake-sim/src/recipe_def.rs:6-15` with fields `name`, `inputs: Vec<(CommodityKind, Quantity)>`, `outputs: Vec<(CommodityKind, Quantity)>`, `work_ticks`, `required_workstation_tag`, `required_tool_kinds`, `body_cost_per_tick`. There is no `need_category` field — classification is derived from `outputs[0]`'s `CommodityKind::spec().consumable_profile`.
3. `CommodityKindSpec` is defined at `crates/worldwake-core/src/items.rs:122-128` with `consumable_profile: Option<CommodityConsumableProfile>`. `CommodityConsumableProfile` at line 135 has `hunger_relief_per_unit: Permille`, `thirst_relief_per_unit: Permille`, `bladder_fill_per_unit: Permille` (all per-unit positive-delta amounts). Fatigue and dirtiness reliefs are NOT exposed through `CommodityConsumableProfile` — they come from actions (sleep, wash), not from consuming items.
4. Recipe commit counts are already counted in the observer via the action trace (commits per recipe name). The Section 2 "Recipe usage" table landed in 006 relies on the same data; this ticket reads action-trace commit counts per agent per recipe.
5. Belief-gate requires "alternative recipe's required facility / workstation / resource source was known to the agent" — the observer's Section 5 already collects per-agent belief data. This ticket reads the same belief substrate to check whether an alternative recipe's prerequisites were observed by the agent at some point during the run.
6. Shared abstraction boundary under audit: the `detect_anomalies()` orchestrator at `bin/observer.rs:752` plus the recipe → need derivation helper introduced in this ticket. The derivation is a read-side computation per FND-27 — it is not cached or stored.

## Architecture Check

1. Inlining the recipe → need classification inside the detector module (as a private helper `fn primary_satisfied_need(recipe: &RecipeDefinition) -> Option<HomeostaticNeedId>`) respects YAGNI: only this detector consumes the classification today. Alternative — promoting the helper into `worldwake-sim` as a public API — would add a speculative cross-crate surface; if a second consumer emerges, the helper can be promoted then.
2. The derivation reads `CommodityKind::spec()` (a `const fn` lookup) plus the recipe's `outputs` field. Both are authoritative static data; no simulation state is mutated. FND-27 (Derived Summaries Are Caches): the classification is recomputed on every run, never stored.
3. Belief-gate prevents false positives for agents who knew a recipe but never discovered a facility. This respects FND-14 (belief-only reasoning reflected in detection) — the detector distinguishes "didn't know how" from "chose not to."

## Verification Layers

1. Detector fires when agent has ≥2 known food recipes, commits ≥95% to one, and has belief evidence of the alternative's facility → focused unit test with hand-constructed `KnownRecipes`, action counts, and belief snapshot.
2. Detector does NOT fire when belief substrate for the alternative is missing → focused unit test (belief-gate control case).
3. Detector does NOT fire when the only known recipe for a need has no alternatives → focused unit test (trivial single-recipe case).
4. Recipe → need derivation maps `CommodityKind::Apple` / `CommodityKind::Grain` to `HomeostaticNeedId::Hunger` (via `hunger_relief_per_unit > 0`) and `CommodityKind::Water` to `HomeostaticNeedId::Thirst` → focused unit test on the derivation helper.
5. Recipes producing non-consumable outputs (e.g., tools, weapons, waste) return `None` from the derivation and are excluded from monoculture analysis → focused unit test.
6. Single-layer ticket (observer read-side); no action-trace or event-log proof surface applies beyond reading the already-captured commit counts.

## What to Change

### 1. Recipe → need derivation helper

Add a private module-local helper inside `bin/observer.rs`:

```rust
fn primary_satisfied_need(recipe: &RecipeDefinition) -> Option<HomeostaticNeedId> {
    // Pure derivation: outputs[0] -> CommodityKind::spec().consumable_profile -> first non-zero relief.
    // FND-27: not cached, recomputed per-call. Per spec D4 semantics:
    //   thirst_relief > 0 -> Thirst
    //   hunger_relief > 0 -> Hunger
    //   otherwise          -> None
    let (commodity, _qty) = recipe.outputs.first()?;
    let profile = commodity.spec().consumable_profile?;
    if profile.thirst_relief_per_unit.as_u16() > 0 {
        Some(HomeostaticNeedId::Thirst)
    } else if profile.hunger_relief_per_unit.as_u16() > 0 {
        Some(HomeostaticNeedId::Hunger)
    } else {
        None // bladder-fill-only or non-consumable outputs
    }
}
```

Use the project's existing `Permille` accessor (whatever `.as_u16()` or `.0` convention is current at the time of implementation — match the existing pattern in `bin/observer.rs`).

### 2. Belief-gate helper

Add `fn agent_believes_recipe_facility_reachable(agent: EntityId, recipe: &RecipeDefinition, belief_snapshot: &AgentBeliefSnapshot) -> bool` that scans the agent's run-long belief history for evidence of the recipe's `required_workstation_tag` or primary input's resource source. If either appears in the agent's belief record at any tick during the run, return `true`. The belief data is already collected by the observer for Section 5 — reuse that collection rather than re-sampling.

### 3. New detector function

Add `fn detect_recipe_monoculture(stats_by_agent: &BTreeMap<EntityId, AgentStats>, known_recipes_by_agent: &BTreeMap<EntityId, BTreeSet<RecipeId>>, recipe_registry: &ActionRecipeRegistry, beliefs_by_agent: &BTreeMap<EntityId, AgentBeliefSnapshot>, names: &BTreeMap<EntityId, String>, anomalies: &mut Vec<Anomaly>)` below `detect_maintenance_starvation`.

Logic per agent:

- Group known recipes by `primary_satisfied_need` classification; skip recipes that return `None`.
- For each need bucket with ≥2 recipes:
  - Count commits per recipe across the run (from existing action-trace data).
  - If the top recipe's share (commits / total_bucket_commits) ≥ 95% (as a permille integer comparison: `>= 950`) AND `total_bucket_commits > 0`:
    - Belief-gate: for each non-top recipe in the bucket, check `agent_believes_recipe_facility_reachable`. If NONE of the alternatives satisfies the belief-gate, skip (no false positive).
    - Otherwise emit one anomaly:
      - `kind: AnomalyKind::RecipeMonoculture`
      - `agent_name: names[&agent]`
      - `additional_agent_names: None`
      - `description: format!("{} actions: {:.0}% {} ({} actions), {:.0}% {} ({} actions). Both recipes known; {} facility belief present at tick {}.", need_category_label, top_percent, top_name, top_count, other_percent, other_name, other_facility_label, belief_tick)` (extend with additional "`, {:.0}% ...`" entries if bucket has >2 recipes)
      - `tick_range: Some((0, run_end_tick))`

### 4. Wire into `detect_anomalies()`

Call from the orchestrator after `detect_maintenance_starvation`. Collect per-agent `KnownRecipes` via `world.get_component_known_recipes(agent).cloned().unwrap_or_default()` into the required BTreeMap.

### 5. Focused unit tests

Add to the existing `#[cfg(test)] mod tests`:

- `test_primary_satisfied_need_classifies_apple_as_hunger` — build a recipe with `outputs = vec![(CommodityKind::Apple, Quantity(1))]`; assert `Some(HomeostaticNeedId::Hunger)`.
- `test_primary_satisfied_need_classifies_water_as_thirst` — same pattern for water; assert `Some(HomeostaticNeedId::Thirst)`.
- `test_primary_satisfied_need_returns_none_for_non_consumable` — recipe producing a non-consumable commodity; assert `None`.
- `test_recipe_monoculture_fires_on_100_percent_apple_share` — agent knows Apples + Grain, commits 16 Apples / 0 Grain, belief snapshot includes grainfield facility; assert one anomaly of kind `RecipeMonoculture`.
- `test_recipe_monoculture_does_not_fire_without_belief_gate` — same setup but belief snapshot lacks grainfield; assert zero anomalies.
- `test_recipe_monoculture_does_not_fire_on_single_known_recipe` — agent knows only Apples; assert zero anomalies.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Promoting `primary_satisfied_need` into `worldwake-sim` as a public API (spec Non-Goals §5; inline for now).
- Fatigue/Dirtiness recipe classification — no recipes produce fatigue- or dirtiness-relieving commodities in the current data model; this is a natural consequence of the derivation, not a gap.
- Multi-output recipes that satisfy multiple needs via different output slots — `primary_satisfied_need` uses `outputs[0]` only, matching the "primary output" convention already in the codebase.
- Goldens against real scenario fixtures (007).

## Acceptance Criteria

### Tests That Must Pass

1. `test_primary_satisfied_need_classifies_apple_as_hunger` passes.
2. `test_primary_satisfied_need_classifies_water_as_thirst` passes.
3. `test_primary_satisfied_need_returns_none_for_non_consumable` passes.
4. `test_recipe_monoculture_fires_on_100_percent_apple_share` passes.
5. `test_recipe_monoculture_does_not_fire_without_belief_gate` passes.
6. `test_recipe_monoculture_does_not_fire_on_single_known_recipe` passes.
7. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. `primary_satisfied_need` is a pure function of `RecipeDefinition.outputs[0]` + `CommodityKind::spec()` — no mutable state, no caching, deterministic.
2. Belief-gate never emits a false positive: if the agent has literally no belief-record evidence of any alternative recipe's facility during the run, no anomaly is emitted regardless of commit-count distribution.
3. The derivation helper is private to `bin/observer.rs` — it is not exposed to other crates. If a future consumer needs it, that is a separate promotion ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` — six new focused unit tests above.

### Commands

1. `cargo test -p worldwake-cli --bin observer recipe_monoculture`
2. `cargo test -p worldwake-cli --bin observer primary_satisfied_need`
3. `cargo test -p worldwake-cli`
4. `cargo clippy --workspace --all-targets -- -D warnings`
