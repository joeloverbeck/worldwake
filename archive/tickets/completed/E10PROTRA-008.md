# E10PROTRA-008: Harvest action in worldwake-systems

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — production action support in sim + harvest registration/handler in systems crate
**Deps**: E10PROTRA-002 (ResourceSource), E10PROTRA-004 (KnownRecipes), E10PROTRA-005 (WorkstationMarker), E10PROTRA-006 (RecipeRegistry)

## Problem

Harvest transfers material out of a concrete `ResourceSource` stock. It is not a Craft (no input transformation) — it is a direct extraction from a depletable natural source. The action must enforce: recipe knowledge gating, workstation co-location, source stock availability, tool possession, and body cost.

## Assumption Reassessment (2026-03-10)

1. `ResourceSource`, `KnownRecipes`, `WorkstationMarker`, and `RecipeRegistry` already exist in the codebase. This ticket should consume them rather than treating them as future dependencies.
2. The current action framework pattern is established in `needs_actions.rs`, but its semantic vocabulary is still needs-oriented. Today it cannot express recipe knowledge checks, workstation-tag checks, or resource-source availability checks through the shared affordance/precondition layer.
3. `RecipeRegistry` lives in `SimulationState`, while action handlers currently execute with `ActionInstance` + `WorldTxn` and do not receive `SimulationState`. A clean harvest implementation therefore needs recipe-derived executable data copied into action definitions at registration time rather than ad hoc runtime registry reach-through.
4. `BeliefView` / affordance evaluation currently exposes only generic kind, place, commodity, and consumable facts. Harvest affordances need belief-side access to known recipes, workstation markers, and resource-source state.
5. `LotOperation::Produced` exists in core provenance types, but `WorldTxn` currently exposes only `create_item_lot(...)`, which produces `LotOperation::Created` provenance. This ticket should rely on causal event-log records for harvest traceability and not promise produced-provenance wiring unless the transaction API is explicitly extended.
6. The reservation system already exists through world reservation APIs. Harvest should reuse those APIs directly.
7. `RecipeDefinition.required_tool_kinds` should model required possessed tools as `UniqueItemKind`, not `CommodityKind`. Tools in Worldwake are already represented as unique items, so harvest should reuse that authoritative item model instead of treating tools as stackable commodities.
8. To keep the architecture coherent with workstation reservations and the current affordance model, this ticket should scope the initial harvest path to facility-attached resource sources (`Facility` entities carrying both `WorkstationMarker` and `ResourceSource`). Place-attached harvest sources can be added later once the action surface expands further.

## Architecture Reassessment

1. A clean harvest implementation is more beneficial than a handler-only shortcut that bypasses the shared affordance/precondition model. If recipe knowledge, workstation tag matching, and source availability live only inside handler callbacks, affordance enumeration becomes misleading and later production actions will accumulate one-off validation paths.
2. The right long-lived architecture is to keep recipe data authoritative in `RecipeRegistry`, but register executable harvest `ActionDef`s from those recipes by copying the recipe-derived execution payload needed at runtime. This avoids hidden global lookups from handlers while keeping actions deterministic and replay-friendly.
3. Extending the shared action semantics slightly is better than inventing production-only side channels. Harvest needs first-class shared predicates for recipe knowledge, workstation-tag matching, and source availability so both `BeliefView` affordance checks and authoritative start/commit validation speak the same language.
4. No new backward-compatibility layer should be introduced. The action framework should be updated directly so later production tickets build on the same clean surface.

## Architecture Check

1. Harvest must still follow the ActionDef/ActionHandler pattern established by E09, but with a small sim-level extension so recipe-backed executable payloads are available to handlers without runtime registry reach-through.
2. Shared constraints/preconditions must cover recipe knowledge, workstation tag matching, co-location, source availability, and possessed unique-item tool requirements when the recipe demands them.
3. The handler commit phase reduces `ResourceSource.available_quantity` and creates a concrete output item lot at the workstation's place.
4. Body cost comes from the recipe-derived action definition and is applied by the existing needs system via active-action aggregation.
5. Because the initial implementation reserves the concrete workstation/resource entity for the action duration, concurrent depletion of that same harvest source is prevented by reservation rather than by a late commit race.

## What to Change

### 1. Extend shared action semantics in `worldwake-sim`

Add the minimum generic support needed for recipe-backed production actions:

- Action definitions need an explicit production/harvest payload carrying the recipe-derived execution data required by handlers.
- Handler callbacks should receive the resolved `ActionDef` so they can read that payload directly instead of relying on implicit registration order or globals.
- Shared action constraints / preconditions and belief-view evaluation need enough vocabulary to express:
  - actor knows recipe
  - target workstation tag matches
  - target resource source exists with sufficient quantity

### 2. Add harvest action registration to `crates/worldwake-systems/src/production_actions.rs`

Define:
- `register_harvest_actions(defs, handlers, recipes)`: registers one harvest action per eligible harvest recipe in the provided registry
- `harvest_handler`: ActionHandler with start/tick/commit/abort callbacks
  - **start**: validate the harvest payload is present and structurally coherent
  - **tick**: no direct mutation; progress is represented by action duration, and body cost comes from the action definition
  - **commit**: reduce `ResourceSource.available_quantity`, create concrete output lot on the ground at the workstation place, keep all changes inside `WorldTxn`
  - **abort**: no material lost; reservations are released by the shared action framework
- Harvest `ActionDef`s must:
  - require `KnownRecipes` membership for the matching `RecipeId`
  - target a co-located `Facility`
  - require the matching `WorkstationMarker`
  - require sufficient `ResourceSource.available_quantity`
  - reserve the workstation/resource entity for the action duration
  - copy duration and body cost from the recipe definition

### 3. Register at least one concrete harvest recipe for tests

Use a concrete facility-based orchard-row recipe such as "Harvest Apples":
- `inputs: []`
- `outputs: [(Apple, quantity)]`
- `required_workstation_tag: Some(WorkstationTag::OrchardRow)`
- `required_tool_kinds: [UniqueItemKind::SimpleTool]`

The recipe remains authoritative in `RecipeRegistry`; action registration derives the executable action from it.

### 4. Export registration API from `worldwake-systems`

## Files to Touch

- `crates/worldwake-sim/src/action_def.rs` (modify — action payload field)
- `crates/worldwake-sim/src/action_handler.rs` (modify — handler signatures receive `ActionDef`)
- `crates/worldwake-sim/src/action_semantics.rs` (modify — harvest-relevant constraint/precondition variants)
- `crates/worldwake-sim/src/action_validation.rs` (modify — authoritative evaluation for new semantics)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — belief-side evaluation for new semantics)
- `crates/worldwake-sim/src/belief_view.rs` (modify — expose known recipes / workstation / resource facts)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (modify — authoritative implementation)
- `crates/worldwake-sim/src/lib.rs` (modify — export any new shared action types)
- `crates/worldwake-systems/src/production_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module + export registration function)

## Out of Scope

- Craft action (E10PROTRA-009)
- ResourceSource regeneration (E10PROTRA-007)
- AI decision to harvest (E13)
- Seasonal modifiers
- Tool degradation (future)
- Place-attached harvest sources without a concrete workstation facility
- Produced-provenance transaction support in `WorldTxn`
- Multiple output lots from a single harvest (keep to single recipe output for now)

## Acceptance Criteria

### Tests That Must Pass

1. **Harvest reduces `ResourceSource.available_quantity`** by the recipe output amount.
2. **Harvest fails when the source lacks sufficient stock** — action does not start or commit, and no goods are created.
3. **Harvest creates output item lot** on the ground at the workstation's location.
4. **Known recipe gating works**: agent without the recipe in `KnownRecipes` cannot start harvest.
5. **Workstation co-location enforced**: agent not at the workstation's location cannot harvest.
6. **Workstation tag matching enforced**: only facilities with the recipe's `WorkstationMarker` are valid harvest targets.
7. **Workstation reservation**: second agent cannot start harvest on an already-reserved workstation/resource entity.
8. **Body cost applied**: agent's `HomeostaticNeeds` are affected by recipe-derived `body_cost_per_tick` through the existing needs system.
9. **Duration correct**: harvest takes `work_ticks` ticks to complete.
10. **Abort preserves source**: if harvest is interrupted, `ResourceSource` quantity is unchanged.
11. **Event emitted**: harvest completion emits a causal event-log record using the existing generic action/world-mutation tags.
12. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. No production path creates goods from a tag alone — material comes from concrete `ResourceSource` stock on a concrete workstation entity.
2. Conservation: `ResourceSource` decrease = output lot increase for the harvested commodity.
3. No floating-point arithmetic.
4. Deterministic behavior.
5. Aborted harvest does not leak or destroy material.
6. No runtime reach-through from handlers into `SimulationState`; recipe-derived execution data is copied into action definitions at registration time.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production_actions.rs` — happy path, insufficient stock, unknown recipe, wrong workstation tag, occupied workstation, abort safety, body cost, event emission
2. `crates/worldwake-sim/src/action_semantics.rs` / `action_validation.rs` / `affordance_query.rs` / `omniscient_belief_view.rs` — coverage for new harvest-relevant shared semantics

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - Added recipe-backed harvest action registration in `crates/worldwake-systems/src/production_actions.rs`.
  - Added a small sim-layer action payload mechanism so executable harvest data is copied out of `RecipeRegistry` into `ActionDef`s at registration time.
  - Updated action handlers to receive the resolved `ActionDef`, which lets handlers consume recipe-backed payloads without global state or `SimulationState` reach-through.
  - Extended shared action semantics, authoritative validation, and belief-view affordance checks to cover recipe knowledge, workstation tags, and resource-source availability.
  - Reworked recipe tool requirements to use possessed `UniqueItemKind`s, added authoritative/belief-side unique-item tool checks, and wired harvest affordances to require the concrete tool when a recipe specifies one.
  - Exported `register_harvest_actions` from `worldwake-systems` and kept harvest body-cost application on the existing needs-system path.
  - Added focused harvest tests plus sim-layer tests for the new shared semantics.
- Deviations from original plan:
  - Corrected the ticket first: the dependencies already existed, so the real architectural gap was in the sim action framework rather than in core production schema work.
  - No `EventTag` expansion was needed. Existing `ActionCommitted` + `WorldMutation` tags already cover harvest causality cleanly.
  - No produced-provenance transaction change was made. `WorldTxn` still creates lots through the existing creation path, so causal event records remain the source of harvest traceability for now.
  - Outcome amended on 2026-03-10 after follow-up architecture work: the initial temporary limitation around commodity-based tool requirements was removed by moving recipe tool requirements onto possessed `UniqueItemKind`s, which matches the existing item model and avoids a parallel tool taxonomy.
- Verification results:
  - `cargo test -p worldwake-sim` ✅
  - `cargo test -p worldwake-systems` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo test --workspace` ✅
