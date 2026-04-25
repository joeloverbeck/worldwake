# T01DEBVIS-006: Tooltip + need_bar widget

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: [T01DEBVIS-005](T01DEBVIS-005.md)

## Problem

Hovering an agent on the canvas should reveal a compact ~240px tooltip with name, control source, location/transit, active action, active goal, and zone-colored need bars per spec T01 §D6. The need-bar widget is reusable: T01DEBVIS-007's Needs tab (full-width version) consumes the same widget. Zones come from per-need `DriveThresholds` (`<low → green`, `low..medium → yellow-green`, `medium..high → amber`, `high..critical → orange-red`, `>=critical → red with pulsing 1px outline`).

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `DriveThresholds` at `crates/worldwake-core/src/drives.rs:58` exposes per-need `ThresholdBand` with `low()`, `medium()`, `high()`, `critical()` accessors validated at construction (low < medium < high < critical). Reassessment 2026-04-25 confirmed the zone order matches spec §D6.
2. `HomeostaticNeeds` at `crates/worldwake-core/src/needs.rs:9` carries `hunger / thirst / fatigue / bladder / dirtiness` as `Permille`. `Pain` and `Danger` threshold bands exist on `DriveThresholds`, but `FrameSnapshot::AgentView` does not yet carry derived pain/danger pressure values. This ticket therefore renders the five embodied needs; derived pressure rows are deferred to [T01DEBVIS-011](../../tickets/T01DEBVIS-011.md).
3. `FrameSnapshot.agents[id]` already carries `needs: HomeostaticNeeds` and `drive_thresholds: DriveThresholds` per T01DEBVIS-003 — the tooltip reads from the snapshot, not directly from `World`. The tooltip helper also receives the `FrameSnapshot` so it can resolve place names from `AgentPosition` IDs without widening `AgentView`.
4. Tooling-only ticket — UI widget; no engine state, no shared abstraction boundary.

## Architecture Check

1. The need-bar widget is a leaf visual component reused by both the hover tooltip and the modal Needs tab. Single definition, two callers — no duplication.
2. Zone classification is a pure function of `(value, thresholds)`; the widget signature accepts both rather than a pre-computed zone enum, so the same widget renders correctly even if thresholds change between agents.
3. `egui::Response::on_hover_ui` integrates with the canvas hit-test from T01DEBVIS-005 — no separate event channel.

## Verification Layers

1. Need-bar zone classification correctness → focused unit test (`need_bar_zone_classification`) parameterized over the 5 zones against a known `DriveThresholds`. Each zone produces the expected `Color32`.
2. Per template item 6: tooltip rendering is a pure function of snapshot + thresholds; action/decision-trace layers are not relevant.

## What to Change

### 1. Implement `need_bar.rs`

Create `crates/worldwake-visualizer/src/need_bar.rs`:

- `pub fn need_bar(ui: &mut egui::Ui, label: &str, value: Permille, thresholds: &ThresholdBand, width: f32) -> egui::Response`.
- Width parameter (140px in tooltip, full-width in modal) so the same widget serves both contexts.
- Bar fill proportional to `value.as_u16() as f32 / 1000.0`.
- Zone-colored fill per spec §D6 zones.
- Threshold tick marks as 1px vertical lines at low/medium/high/critical positions.
- `>=critical` zone applies the 1px outline pulse `alpha = 0.6 + 0.4 * sin(time * 4.0)`.
- Numeric value right-aligned after the bar.

### 2. Implement `tooltip.rs`

Create `crates/worldwake-visualizer/src/tooltip.rs`:

- `pub fn show_tooltip(ui: &mut egui::Ui, snapshot: &FrameSnapshot, agent: &AgentView)` invoked from `on_hover_ui` registered on each agent's hit-rect in `canvas.rs`.
- Layout per spec §D6:
  - Row 1: bold name · control-source badge · alive/dead glyph.
  - Row 2: location string (`"@ {place_name}"` or `"→ {dest_name} ({k}/{n})"`).
  - Row 3: active action (`"travel [k/n]"` / `"{action_def_id}"` / `"—"`). The live snapshot carries `ActionDefId`, not an action-definition display name.
  - Row 4: active goal (`{goal_kind}` debug-name · `motive_score` from `AgendaState.committed`, or `"no goal"`).
  - Need bars: vertical stack — Hunger, Thirst, Fatigue, Bladder, Dirtiness. Pain and Danger pressure rows are deferred until those derived values are present in `AgentView`.

### 3. Wire tooltip into `canvas.rs`

Modify `crates/worldwake-visualizer/src/canvas.rs` from T01DEBVIS-005 — wrap each agent's hit-rect with `.on_hover_ui(|ui| tooltip::show_tooltip(ui, snapshot, agent))`.

### 4. Wire modules into lib.rs

Add `pub mod need_bar;` and `pub mod tooltip;` to `crates/worldwake-visualizer/src/lib.rs`.

## Files to Touch

- `crates/worldwake-visualizer/src/need_bar.rs` (new)
- `crates/worldwake-visualizer/src/tooltip.rs` (new)
- `crates/worldwake-visualizer/src/canvas.rs` (modify — wire `on_hover_ui` for agents)
- `crates/worldwake-visualizer/src/lib.rs` (modify — add module declarations)

## Out of Scope

- Detail modal Needs tab (T01DEBVIS-007 — reuses the `need_bar` widget at full width).
- Other modal tabs (T01DEBVIS-007, -008, -009).
- Search/filter on tooltip content.
- Derived Pain/Danger pressure rows; this ticket lands only the values currently carried by `HomeostaticNeeds`. Follow-up: [T01DEBVIS-011](../../tickets/T01DEBVIS-011.md).

## Acceptance Criteria

### Tests That Must Pass

1. `need_bar_zone_classification` — parameterized test covering all 5 zones; for `(value, thresholds)` pairs at each zone boundary, the widget returns the expected `Color32` (or equivalent test surface — expose a `pub fn classify_zone(value, thresholds) -> NeedZone` helper if needed for testability).
2. Manual QA: hover an agent on the canvas; tooltip appears with all 4 rows and need bars; bar colors match numeric values (e.g., a hunger value above the critical threshold renders red).
3. Existing suite: `cargo test -p worldwake-visualizer` passes.

### Invariants

1. The same `need_bar` widget renders both in tooltip and Needs-tab contexts — no parallel implementation.
2. Zone classification is deterministic from `(value, thresholds)`; same inputs always produce the same color.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-visualizer/src/need_bar.rs` (`#[cfg(test)] mod tests`) — `need_bar_zone_classification` parameterized over 5 zones.

### Commands

1. `cargo test -p worldwake-visualizer --lib need_bar::tests::need_bar_zone_classification -- --exact`
2. `cargo test -p worldwake-visualizer`
3. `cargo run -p worldwake-visualizer -- scenarios/survival-baseline.ron` (manual hover smoke)
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-25.

- Added `crates/worldwake-visualizer/src/need_bar.rs` with the reusable bar widget, deterministic `classify_zone`, threshold tick marks, numeric value rendering, and critical-zone pulse outline.
- Added `crates/worldwake-visualizer/src/tooltip.rs` with the compact hover tooltip for name/control/alive state, location/transit, active action, active goal, and five core need bars.
- Wired canvas agent hit-rects to `egui::Response::on_hover_ui` and exported the new modules from `lib.rs`.

## Deviations

- The live `AgentView` carries `ActionDefId`, not an action-definition display name, so non-travel active actions render by ID.
- The live `AgentView` carries `HomeostaticNeeds` but not derived pain/danger pressure values, so this ticket renders the five embodied needs only and leaves Pain/Danger rows to [T01DEBVIS-011](../../tickets/T01DEBVIS-011.md).
- The manual GUI hover smoke was not run in this headless session; automated tooltip/widget smoke tests and full verify passed.

## Verification Result

- Passed `cargo test -p worldwake-visualizer --lib -- --list` to resolve exact selectors.
- Passed `cargo test -p worldwake-visualizer --lib need_bar::tests::need_bar_zone_classification -- --exact`.
- Passed `cargo test -p worldwake-visualizer`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh` before the final ASCII-only tooltip separator cleanup.
- Passed final post-cleanup `cargo test -p worldwake-visualizer`.
- Passed final post-cleanup `cargo clippy --workspace --all-targets -- -D warnings`.
