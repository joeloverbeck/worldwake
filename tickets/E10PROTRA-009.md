# E10PROTRA-009: Craft action in worldwake-systems

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new ActionDef + ActionHandler in systems crate
**Deps**: E10PROTRA-004 (KnownRecipes), E10PROTRA-005 (WorkstationMarker, ProductionJob), E10PROTRA-006 (RecipeRegistry)

## Problem

Craft transforms input materials into output products through a work-in-progress process. Unlike Harvest, Craft consumes explicit inputs from the agent's accessible inventory, stages them in a WIP container, and after the required work ticks, produces the recipe's outputs. Interrupted crafts must leave WIP state and staged inputs in the world — no silent destruction of progress.

## Assumption Reassessment (2026-03-10)

1. `KnownRecipes` on agents and `RecipeRegistry` in `SimulationState` will exist — confirmed dependencies.
2. `WorkstationMarker` on Facility entities will exist — confirmed dependency.
3. `ProductionJob` on Facility entities will exist — confirmed dependency.
4. The existing Container and ItemLot systems support creating staged input containers — `Container` can be created as an entity, items moved into it via `LotOperation`.
5. The reservation system in `relations.rs` handles workstation reservation.
6. Craft recipes have non-empty `inputs` and `outputs`.

## Architecture Check

1. Craft uses the `ProductionJob` component on the workstation to track WIP state.
2. On start: reserve workstation, create staged container, move inputs from accessible inventory into staged container, set `ProductionJob` on workstation.
3. On tick: increment `progress_ticks`, apply body cost.
4. On commit: remove staged inputs (they are consumed), create output lots, remove `ProductionJob`, release workstation.
5. On abort/interruption: `ProductionJob` persists on the workstation. Staged inputs remain in the staged container. Another agent (or the same one later) could potentially resume or abandon the job.
6. This ensures Principle 3: no abstract progress — concrete WIP state with concrete staged materials.

## What to Change

### 1. Add craft action to `crates/worldwake-systems/src/production_actions.rs`

Define:
- `craft_handler`: ActionHandler with start/tick/commit/abort callbacks
  - **start**: Validate preconditions, reserve workstation, create staged container, move inputs into it, set `ProductionJob`
  - **tick**: Increment `ProductionJob.progress_ticks`, apply body cost
  - **commit**: Consume staged inputs, create output lots, remove `ProductionJob`, release workstation, remove staged container
  - **abort**: Leave `ProductionJob` and staged inputs in place — do NOT destroy progress or materials
- `ActionDef` for Craft with:
  - Actor constraints: must have `KnownRecipes` containing the recipe
  - Targets: workstation entity with matching `WorkstationMarker`
  - Preconditions: co-location, inputs accessible, tools possessed
  - Duration: from recipe definition (`work_ticks`)
  - Body cost: from recipe definition
  - Reservation: workstation entity

### 2. Register concrete craft recipes

Register at least one concrete craft recipe (e.g., "Bake Bread" from Grain→Bread at Mill) for testability.

### 3. Export and wire into dispatch

## Files to Touch

- `crates/worldwake-systems/src/production_actions.rs` (modify — add craft handler + def)
- `crates/worldwake-systems/src/lib.rs` (modify — registration if needed)
- `crates/worldwake-core/src/event_tag.rs` (modify — add Craft event tag if needed)

## Out of Scope

- Harvest action (E10PROTRA-008)
- Job resumption by a different agent (future — the data model supports it but no action for it yet)
- Job abandonment action (future — explicit cleanup of WIP state)
- Tool degradation
- AI decision to craft (E13)
- Multi-step recipes or recipe chains

## Acceptance Criteria

### Tests That Must Pass

1. **Craft stages inputs into WIP container** — inputs are moved from accessible inventory to staged container.
2. **Craft produces only recipe-defined outputs** — no hidden creation.
3. **Craft consumes only recipe-defined inputs** — no hidden destruction.
4. **Interrupted craft leaves WIP / staged inputs in the world** — `ProductionJob` persists, staged container with inputs remains.
5. **Known recipe gating works**: agent without the recipe cannot start craft.
6. **Workstation co-location enforced**: agent must be at the workstation's location.
7. **Workstation type matching**: only workstations with matching `WorkstationMarker` tag are valid.
8. **Input availability enforced**: craft fails if required inputs are not accessible.
9. **Tool requirement enforced**: craft fails without required tools.
10. **Workstation concurrency enforced**: cannot start craft on an occupied workstation (already has `ProductionJob`).
11. **Body cost applied**: agent's `HomeostaticNeeds` affected by `body_cost_per_tick`.
12. **Duration correct**: craft takes `work_ticks` ticks.
13. **Event emitted**: craft completion emits a causal event.
14. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Conservation: inputs consumed = recipe inputs; outputs created = recipe outputs. No hidden loss or gain.
2. Interrupted WIP is traceable — no silent destruction of progress or materials.
3. No floating-point arithmetic.
4. Deterministic behavior.
5. Staged container exists as a real entity with real item lots — not abstract bookkeeping.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production_actions.rs` — happy path, interrupted WIP persistence, unknown recipe, missing inputs, missing tools, occupied workstation, workstation type mismatch, body cost, event emission, conservation check

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
