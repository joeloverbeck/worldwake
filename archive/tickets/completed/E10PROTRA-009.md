# E10PROTRA-009: Craft action in worldwake-systems

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new ActionDef + ActionHandler in systems crate
**Deps**: E10PROTRA-004 (KnownRecipes), E10PROTRA-005 (WorkstationMarker, ProductionJob), E10PROTRA-006 (RecipeRegistry)

## Problem

Craft transforms input materials into output products through a work-in-progress process. Unlike Harvest, Craft consumes explicit inputs from the agent's accessible inventory, stages them in a WIP container, and after the required work ticks, produces the recipe's outputs. Interrupted crafts must leave WIP state and staged inputs in the world — no silent destruction of progress.

## Assumption Reassessment (2026-03-10)

1. `KnownRecipes`, `WorkstationMarker`, and `ProductionJob` already exist in `crates/worldwake-core/src/production.rs` via completed E10 tickets.
2. `RecipeRegistry` already exists in `crates/worldwake-sim/src/recipe_registry.rs` and is already owned by `SimulationState`.
3. Harvest is already implemented in `crates/worldwake-systems/src/production_actions.rs` through the shared action framework (`register_*_actions`, typed `ActionPayload`, standard `ActionHandler` callbacks). Craft should extend that same architecture, not introduce a parallel dispatch shape.
4. The existing `Container`/`ItemLot`/`WorldTxn` APIs are sufficient for concrete staged-input WIP: containers can be created as entities, lots can be split, moved into containers, merged, and archived, and `ProductionJob` can point at the staged container.
5. Workstation reservation should reuse the existing generic reservation pipeline that `start_action` and `ReservationReq` already use; craft should not add workstation-specific reservation logic.
6. `RecipeDefinition` intentionally allows empty `inputs` and empty `outputs` at the schema level. Craft action registration must therefore filter for recipes that are actually craft-shaped instead of assuming the registry enforces that invariant.
7. There is no craft-specific `EventTag` today, and the current action/event architecture already records `ActionStarted`, `ActionCommitted`, `ActionAborted`, plus recipe/action-specific causal tags. Add a new tag only if the broader event taxonomy truly needs it.
8. The ticket originally referenced `specs/E10-PROTRA-economic-production-trade.md`, but the active epic spec is `specs/E10-production-transport.md`.

## Architecture Check

1. Craft should use the same recipe-backed action-registration pattern as Harvest. The clean extension point is `register_craft_actions(...)` beside `register_harvest_actions(...)`, sharing `production_actions.rs` and typed `ActionPayload` rather than inventing bespoke action plumbing.
2. `ProductionJob` on the workstation is the right persistent WIP authority. Action-local state is currently only `ActionState::Empty`, so hidden in-action progress bookkeeping would be less robust than explicit world state.
3. On start: reserve workstation through the standard action reservation path, create a staged container at the workstation's place, split/move concrete controlled input lots into that container, and set `ProductionJob` on the workstation.
4. On tick: increment `ProductionJob.progress_ticks`. Body cost should continue to flow through the existing action-definition `body_cost_per_tick` path and the needs system, not a craft-specific side channel.
5. On commit: consume staged inputs concretely, create only recipe-defined output lots, clear `ProductionJob`, archive the emptied staged container, and let the standard action-finalization path release reservations.
6. On abort/interruption: leave `ProductionJob` and the staged container in the world. The current ticket does not need resumable craft logic yet, but it must preserve resumable state.
7. Craft should remain recipe-driven and multi-output capable. Do not narrow the architecture to a single hardcoded commodity transform just to mirror the current harvest payload shape.
8. This preserves Principle 3 and Principle 12: WIP is concrete world state, and later systems can react to it without direct cross-system calls.

## What to Change

### 1. Extend the shared production action surface for craft

In `crates/worldwake-systems/src/production_actions.rs`:
- Add `register_craft_actions(...)` alongside the existing harvest registration.
- Extend `ActionPayload` in `crates/worldwake-sim/src/action_payload.rs` with a craft-specific payload carrying the recipe id plus the explicit input/output/tool/workstation requirements needed at execution time.
- If necessary, extend `ActionState` only when it materially improves the architecture. Prefer authoritative `ProductionJob` world state over duplicative action-local state.
- Define craft start/tick/commit/abort callbacks within the existing `ActionHandler` model:
  - `start`: validate recipe shape, create staged container, move concrete controlled inputs into it, set `ProductionJob`
  - `tick`: increment `ProductionJob.progress_ticks`
  - `commit`: consume staged inputs, create recipe-defined outputs, clear `ProductionJob`, archive staged container
  - `abort`: preserve `ProductionJob` and staged inputs
- Define craft `ActionDef`s that:
  - require `KnownRecipes`
  - target a co-located workstation entity with matching `WorkstationMarker`
  - enforce required tools through existing actor constraints
  - enforce material availability through actor constraints plus concrete start-time staging
  - use recipe `work_ticks` and `body_cost_per_tick`
  - reserve the workstation through the standard reservation mechanism

### 2. Keep recipe registration data-driven

Do not hardcode global default craft recipes just for this ticket. Tests may register concrete craft recipes inline in a local `RecipeRegistry` fixture (for example, `Bake Bread` from Grain to Bread at a Mill) to verify behavior without mutating broader simulation bootstrap state.

### 3. Export and wire into the existing production API

## Files to Touch

- `crates/worldwake-systems/src/production_actions.rs` (modify — add craft handler + def)
- `crates/worldwake-systems/src/lib.rs` (modify — registration if needed)
- `crates/worldwake-sim/src/action_payload.rs` (modify — add craft payload variant)
- `crates/worldwake-sim/src/action_state.rs` (modify only if a concrete non-duplicative action-local state is actually justified)
- `crates/worldwake-core/src/event_tag.rs` (modify only if a broader event taxonomy review shows a craft-specific tag is warranted)

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
14. **Registration filters invalid recipe shapes**: harvest-only recipes (`inputs.is_empty()`) and degenerate recipes (`outputs.is_empty()`) do not produce craft action defs.
14. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Conservation: inputs consumed = recipe inputs; outputs created = recipe outputs. No hidden loss or gain.
2. Interrupted WIP is traceable — no silent destruction of progress or materials.
3. No floating-point arithmetic.
4. Deterministic behavior.
5. Staged container exists as a real entity with real item lots — not abstract bookkeeping.
6. No duplicate hidden progress authority: active craft progress is represented by `ProductionJob` on the workstation, not by an untracked parallel action-only counter.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production_actions.rs` — craft registration filtering, happy path, interrupted WIP persistence, unknown recipe, missing inputs, missing tools, occupied workstation, workstation type mismatch, co-location, duration/body-cost flow, event emission, conservation check
2. `crates/worldwake-sim/src/action_payload.rs` — craft payload serialization / enum coverage
3. `crates/worldwake-sim/src/action_state.rs` — only if action state grows beyond `Empty`

### Commands

1. `cargo test -p worldwake-systems craft`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - Added recipe-backed craft action registration in `crates/worldwake-systems/src/production_actions.rs` beside harvest.
  - Extended `ActionPayload` with a craft payload carrying explicit recipe inputs, outputs, workstation, and tool requirements.
  - Added `TargetLacksProductionJob` support to the shared action semantics so occupied workstations are expressed declaratively in action definitions rather than hidden in handler logic.
  - Implemented concrete craft WIP behavior: action start now creates a staged container, moves controlled input lots into it, and writes `ProductionJob` on the workstation; ticks increment `progress_ticks`; commit consumes staged inputs, creates outputs, clears the job, and archives the empty staged container; abort preserves the job and staged materials.
  - Exported craft registration from `crates/worldwake-systems/src/lib.rs`.
  - Added focused craft tests in `crates/worldwake-systems/src/production_actions.rs` plus payload coverage in `crates/worldwake-sim/src/action_payload.rs`.
  - Extended the action framework so `ActionHandler::on_start` receives `&mut WorldTxn`, allowing long-running actions to perform authoritative start-time mutations cleanly.
- Deviations from original plan:
  - Did not add a craft-specific `EventTag`. The existing `ActionStarted` / `ActionCommitted` / `WorldMutation` taxonomy was sufficient.
  - Did not add global default craft recipe bootstrap data. Test fixtures register concrete craft recipes locally, which keeps recipe registration data-driven and avoids hidden simulation defaults.
  - Broadened the implementation slightly into `worldwake-sim` because the pre-existing non-transactional start hook was the wrong abstraction for concrete WIP staging. Updating the shared action framework was cleaner and more extensible than pushing craft staging into the first tick.
- Verification results:
  - `cargo test -p worldwake-systems production_actions -- --nocapture` ✅
  - `cargo test -p worldwake-sim` ✅
  - `cargo test -p worldwake-systems` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo test --workspace` ✅
