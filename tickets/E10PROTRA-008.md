# E10PROTRA-008: Harvest action in worldwake-systems

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new ActionDef + ActionHandler in systems crate
**Deps**: E10PROTRA-002 (ResourceSource), E10PROTRA-004 (KnownRecipes), E10PROTRA-005 (WorkstationMarker), E10PROTRA-006 (RecipeRegistry)

## Problem

Harvest transfers material out of a concrete `ResourceSource` stock. It is not a Craft (no input transformation) — it is a direct extraction from a depletable natural source. The action must enforce: recipe knowledge gating, workstation co-location, source stock availability, tool possession, and body cost.

## Assumption Reassessment (2026-03-10)

1. `ResourceSource` component will exist on Facility/Place entities — confirmed dependency.
2. `KnownRecipes` component will exist on Agent entities — confirmed dependency.
3. `RecipeRegistry` will exist in `SimulationState` — confirmed dependency.
4. `WorkstationMarker` component will exist on Facility entities — confirmed dependency.
5. The action framework pattern is well-established in `needs_actions.rs`: register `ActionHandler` + `ActionDef` with preconditions, constraints, targets, duration, body cost.
6. Harvest recipes have `inputs: []` (nothing consumed from agent inventory) and `outputs: [(commodity, quantity)]`. The source of material is the `ResourceSource`, not an explicit recipe input.
7. `LotOperation::Produced` exists for provenance tracking of created item lots.
8. The reservation system exists in `relations.rs` — workstation reservation uses existing APIs.

## Architecture Check

1. Harvest follows the ActionDef/ActionHandler pattern established by E09 needs actions.
2. Preconditions enforce all spec requirements: recipe knowledge, co-location, source availability, tool possession.
3. The handler's commit phase reduces `ResourceSource.available_quantity` and creates/increases an output item lot.
4. If the source becomes empty mid-harvest (another agent harvested it), the action fails at commit — no partial goods conjured.
5. Duration comes from `RecipeDefinition.work_ticks`.
6. Body cost comes from `RecipeDefinition.body_cost_per_tick`.

## What to Change

### 1. Add harvest action registration to `crates/worldwake-systems/src/production.rs`

Or create `crates/worldwake-systems/src/production_actions.rs` (parallel to `needs_actions.rs`).

Define:
- `harvest_handler`: ActionHandler with start/tick/commit/abort callbacks
  - **start**: Validate preconditions, reserve workstation
  - **tick**: Accumulate progress, apply body cost
  - **commit**: Reduce `ResourceSource.available_quantity`, create output lot, release workstation
  - **abort**: Release workstation reservation, no material lost (nothing was consumed from source yet)
- `ActionDef` for Harvest with:
  - Actor constraints: must have `KnownRecipes` containing the recipe
  - Targets: workstation or resource source entity
  - Preconditions: co-location, source has sufficient stock, tools possessed
  - Duration: from recipe definition
  - Body cost: from recipe definition
  - Reservation: workstation entity

### 2. Register harvest recipes

Register at least one concrete harvest recipe (e.g., "Harvest Apples" from orchard) so the system is testable.

### 3. Export and wire into dispatch

## Files to Touch

- `crates/worldwake-systems/src/production_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module + export registration function)
- `crates/worldwake-core/src/event_tag.rs` (modify — add Harvest event tag if needed)

## Out of Scope

- Craft action (E10PROTRA-009)
- ResourceSource regeneration (E10PROTRA-007)
- AI decision to harvest (E13)
- Seasonal modifiers
- Tool degradation (future)
- Multiple output lots from a single harvest (keep to single recipe output for now)

## Acceptance Criteria

### Tests That Must Pass

1. **Harvest reduces `ResourceSource.available_quantity`** by the recipe output amount.
2. **Harvest fails when the source is empty** — action does not commit, no goods created.
3. **Harvest creates output item lot** at the location or in a container.
4. **Known recipe gating works**: agent without the recipe in `KnownRecipes` cannot start harvest.
5. **Workstation co-location enforced**: agent not at the workstation's location cannot harvest.
6. **Tool requirement enforced**: agent without required tools cannot start harvest.
7. **Workstation reservation**: second agent cannot start harvest on an already-reserved workstation.
8. **Body cost applied**: agent's `HomeostaticNeeds` are affected by `body_cost_per_tick`.
9. **Duration correct**: harvest takes `work_ticks` ticks to complete.
10. **Abort preserves source**: if harvest is interrupted, `ResourceSource` quantity is unchanged.
11. **Event emitted**: harvest completion emits a causal event.
12. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. No production path creates goods from a tag alone — material comes from `ResourceSource`.
2. Conservation: `ResourceSource` decrease = output lot increase.
3. No floating-point arithmetic.
4. Deterministic behavior.
5. Aborted harvest does not leak or destroy material.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production_actions.rs` — happy path, empty source, unknown recipe, missing tools, occupied workstation, abort safety, body cost, event emission

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
