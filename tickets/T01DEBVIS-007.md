# T01DEBVIS-007: Detail modal + Overview/Needs/Inventory tabs

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: [T01DEBVIS-005](../archive/tickets/T01DEBVIS-005.md), [T01DEBVIS-006](../archive/tickets/T01DEBVIS-006.md)

## Problem

Clicking an agent on the canvas should open a tabbed inspection modal per spec T01 §D7. This ticket lands the modal shell, the tab routing infrastructure, and the three simpler tabs (Overview, Needs, Inventory). The remaining three tabs (Beliefs, Plan, Traces) land in T01DEBVIS-008 and T01DEBVIS-009. The modal is `egui::Modal`-based, 820×640 default size, with an `egui::CollapsingHeader`-based tab strip.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `AgendaState` at `crates/worldwake-ai/src/agenda_types.rs:13` carries `committed: Option<AgendaEntry>`, `pending: BTreeMap<…, AgendaEntry>`, `suspended: BTreeMap<…, AgendaEntry>`. `AgendaEntry.motive_score: u32` at line 30; `AgendaEntry.provenance: Option<RankedGoalProvenance>` at line 31. Reassessment 2026-04-25 confirmed all three fields are public.
2. `MetabolismProfile` at `crates/worldwake-core/src/needs.rs:117` and `DriveEscalationProfile` at `crates/worldwake-core/src/drive_escalation_profile.rs:38` are reachable via `world.get_component_metabolism_profile(agent)` / `get_component_drive_escalation_profile(agent)` (component_schema-generated accessors). Both are universal agent profiles per existing scenario contract.
3. `World::possessions_of(agent) -> Vec<EntityId>` at `crates/worldwake-core/src/world/ownership.rs:50` returns the entities held by an agent. Each held entity is typically of `EntityKind::ItemLot`; commodity/quantity/`GroundSince` come from per-entity component reads.
4. `AgendaEntry` carries a `goal_kind: GoalKind` — confirm exact field name during implementation (the spec's Overview tab description names `GoalKind` debug-name + score). If the field is named differently (e.g., wrapped inside an inner struct), surface in the implementation phase rather than baking the field name into this ticket.
5. Tooling-only ticket — modal reads through public component accessors; no engine state mutation.

## Architecture Check

1. Tab modules live under `src/tabs/` so each tab is a separately reviewable file. Modal shell only owns the tab-strip rendering and the active-tab state — no per-tab logic in `modal.rs`.
2. Inventory tab uses `EntityId` (not a fictional `LotId`) since item lots are `EntityKind::ItemLot` entities — this matches the corrected spec text from `/reassess-spec`.
3. Needs tab reuses the `need_bar` widget from T01DEBVIS-006 at full width, no parallel rendering implementation.

## Verification Layers

1. Modal open/close state correctness → focused unit test (`modal_opens_on_agent_select`) verifying that setting `selected_agent = Some(id)` produces the modal-rendering branch in `update()`.
2. Inventory tab read correctness → focused unit test (`inventory_tab_renders_possessions`) loading `survival-baseline.ron`, taking an agent with at least one possession, and asserting the rendered row count matches `world.possessions_of(agent).len()`.
3. Per template item 6: action/decision-trace layers are not relevant — modal is a read view.

## What to Change

### 1. Modal shell — `modal.rs`

Create `crates/worldwake-visualizer/src/modal.rs`:

- `pub fn show_modal(ctx: &egui::Context, app: &mut VisualizerApp, agent_id: EntityId)` invoked from `update()` when `selected_agent.is_some()`.
- `egui::Modal` anchored centered, initial size 820×640, resizable.
- Tab strip across the top using `egui::CollapsingHeader` per spec §D7. Persist active-tab in `VisualizerApp.ui_settings` so it survives modal close/reopen.
- Esc closes the modal (already wired in T01DEBVIS-004 — clear `selected_agent`).

### 2. Tab routing — `tabs/mod.rs`

Create `crates/worldwake-visualizer/src/tabs/mod.rs`:

- `pub enum DetailTab { Overview, Needs, Beliefs, Inventory, Plan, Traces }` — all six variants declared up front (Beliefs/Plan/Traces are placeholder branches in this ticket; T01DEBVIS-008 and -009 fill them in).
- `pub fn render_tab(ui: &mut egui::Ui, tab: DetailTab, app: &VisualizerApp, agent_id: EntityId)` dispatches to the appropriate module.

### 3. Overview tab — `tabs/overview.rs`

- Tooltip content expanded.
- `AgendaState.committed` rendering: GoalKind + motive_score + provenance.
- `pending` and `suspended` entries as collapsible sub-sections.
- Top-N candidates shown as a table.

### 4. Needs tab — `tabs/needs.rs`

- Full-width need bars (one per `Permille` need) using the `need_bar` widget at increased width.
- `MetabolismProfile` field display below the bars (decay rates per need).
- `DriveEscalationProfile` field display when registered.

### 5. Inventory tab — `tabs/inventory.rs`

- `world.possessions_of(agent)` produces `Vec<EntityId>` of held lot entities.
- Table columns: `CommodityKind | Quantity | LotEntity (EntityId) | GroundSince (if any)`. Pull commodity/quantity via `world.get_component_commodity(lot_entity)` / `get_component_quantity(lot_entity)` (or analogous component accessors — confirm exact names during implementation).
- Totals per `CommodityKind` at the bottom.

### 6. Wire selection-click into modal open

Modify `crates/worldwake-visualizer/src/app.rs::update()` — when `selected_agent.is_some()`, call `modal::show_modal(ctx, self, id)`.

### 7. Wire modules into lib.rs

Add `pub mod modal;` and `pub mod tabs;` to `crates/worldwake-visualizer/src/lib.rs`.

## Files to Touch

- `crates/worldwake-visualizer/src/modal.rs` (new)
- `crates/worldwake-visualizer/src/tabs/mod.rs` (new)
- `crates/worldwake-visualizer/src/tabs/overview.rs` (new)
- `crates/worldwake-visualizer/src/tabs/needs.rs` (new)
- `crates/worldwake-visualizer/src/tabs/inventory.rs` (new)
- `crates/worldwake-visualizer/src/app.rs` (modify — wire modal call into `update()`)
- `crates/worldwake-visualizer/src/lib.rs` (modify — add module declarations)

## Out of Scope

- Beliefs tab (T01DEBVIS-008).
- Plan tab (T01DEBVIS-008).
- Traces tab (T01DEBVIS-009).
- Cross-agent inspection / diffing.
- Modal docking or persistence (deferred per spec Open Questions §2).

## Acceptance Criteria

### Tests That Must Pass

1. `modal_opens_on_agent_select` — setting `selected_agent = Some(id)` causes the modal-render branch to execute in `update()`; clearing it via Esc returns to the no-modal branch.
2. `inventory_tab_renders_possessions` — load `survival-baseline.ron`, advance until at least one agent has a possession, assert the rendered row count equals `world.possessions_of(agent).len()`.
3. `needs_tab_renders_all_five_core_needs` — Hunger, Thirst, Fatigue, Bladder, Dirtiness rows render for any agent with a `HomeostaticNeeds` component.
4. Existing suite: `cargo test -p worldwake-visualizer` passes.

### Invariants

1. The modal reads only from `World`, `Scheduler`, `AgentTickDriver` and the `FrameSnapshot` — no parallel state mutation.
2. Inventory tab uses `EntityId` for lot identity (matching the simulation's actual model — no fictional `LotId`).
3. The same `need_bar` widget serves both tooltip (T01DEBVIS-006) and Needs tab — no duplicate widget code.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-visualizer/src/modal.rs` and `tabs/inventory.rs` (`#[cfg(test)] mod tests`) — three unit tests above using a headless egui context and a baseline scenario.

### Commands

1. `cargo test -p worldwake-visualizer modal:: tabs::`
2. `cargo test -p worldwake-visualizer`
3. `cargo run -p worldwake-visualizer -- scenarios/survival-baseline.ron` (manual click smoke)
4. `./scripts/verify.sh`
