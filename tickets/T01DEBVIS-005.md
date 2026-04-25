# T01DEBVIS-005: Canvas rendering

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: T01DEBVIS-001, [T01DEBVIS-002](../archive/tickets/T01DEBVIS-002.md), T01DEBVIS-003, T01DEBVIS-004

## Problem

With the simulation host (T01DEBVIS-004), force-directed layout (T01DEBVIS-002), and frame snapshot (T01DEBVIS-003) in place, the visualizer needs the actual canvas drawing per spec T01 §D4. Places render as rounded rects with tag pills; edges as dashed lines with travel-tick labels; agents at places fan out around the place center; agents in transit are lerped along the edge with a directional chevron. Distributed §D11 sub-cases land here: multi-agent fan-out by BTreeMap-ordered ID, dead-agent striped-grey at last known place, self-loop and parallel-edge offsets, and `PlaceTag` exhaustive matching (compile-time enforced by the closed enum).

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PlaceTag` at `crates/worldwake-core/src/topology.rs:11` is a closed 14-variant enum (not `#[non_exhaustive]`). Reassessment 2026-04-25 confirmed: every variant must have an explicit color/style mapping; adding a new variant fails compilation at the rendering match site, which is the desired behavior.
2. `egui::Scene::new(id, scene_rect)` was introduced in egui 0.31 and is reachable on the pinned `egui = 0.34` per spec T01 §D1. Implementation must verify reachability before relying on it; T01DEBVIS-010 includes a manual QA checklist item that confirms pan + zoom work end-to-end. If `Scene` is unavailable, fall back to a hand-rolled pan/zoom container as named in spec T01 §D13 step 5.
3. Tooling-only ticket — canvas reads from `FrameSnapshot` (a derived view per FND-27) and writes nothing back. No engine surface is mutated.

## Architecture Check

1. Drawing reads exclusively from `FrameSnapshot` produced by T01DEBVIS-003 — the canvas never queries `World`/`Scheduler`/`AgentTickDriver` directly. This separation keeps rendering swappable (a future ASCII renderer or PNG dump could consume the same snapshot).
2. `PlaceTag` exhaustive match at the rendering site forces compile-time coverage of new variants — preferred over a runtime "unknown → grey" fallback that would silently drop new tags.
3. Agent fan-out angles are derived deterministically from `BTreeMap`-ordered `EntityId` so re-rendering the same snapshot produces identical positions (FND-27 cache property).

## Verification Layers

1. Snapshot-to-canvas mapping correctness → focused unit test (`canvas_smoke_no_panic_on_baseline_scenario`) building one snapshot from `survival-baseline.ron` and invoking the canvas draw routine via a headless `egui::Context` test harness; asserts no panic and that all snapshot agents are accounted for in the draw call list.
2. Multi-agent fan-out determinism → focused unit test (`agent_fan_out_angles_are_btreemap_stable`) constructing three agents at the same place and asserting their angles are sorted by `EntityId` and total `2π`.
3. Per template item 6: action/decision-trace layers are not relevant — canvas drawing is a pure function of the snapshot.

## What to Change

### 1. Implement `canvas.rs`

Create `crates/worldwake-visualizer/src/canvas.rs` with the public draw routine:

```rust
pub fn draw_canvas(
    ui: &mut egui::Ui,
    snapshot: &FrameSnapshot,
    scene_rect: egui::Rect,
    selected_agent: &mut Option<EntityId>,
    hovered_agent: &mut Option<EntityId>,
);
```

Implementation per spec T01 §D4:

- **Wrap in `egui::Scene`** for pan + zoom.
- **Places**: `Shape::Rect` with 8px rounded corners, stroke width 1.5; stroke color via exhaustive `PlaceTag` match; fill `Color32::from_rgb(36, 36, 42)`; place name top-left + tag pills below name.
- **Edges**: `Shape::dashed_line` with `Stroke::new(3.0, Color32::GRAY)`, dash 8/gap 4; "{travel_ticks} ticks" label at midpoint on a background-filled rounded-rect.
- **Agents at place**: circle radius 10 fanned at ring-radius 22; angle = `(stable_index * TAU / agent_count)` with `stable_index` from BTreeMap-ordered EntityId.
- **Agents in transit**: circle at `lerp(snapshot.places[from].position, snapshot.places[to].position, progress.as_f32())`; small chevron at the leading edge.
- **Agent color**: `ControlSource::Ai = blue-400`, `Human = gold`, `None = grey`. Dead agents: striped grey fill (per §D11).
- **Agent label**: 11pt text 14px below circle, elided to 16 chars.
- **Hit-test**: track hovered_agent on hover; on click, set selected_agent (modal opens in T01DEBVIS-007 — until then, click is a no-op visual indicator).

### 2. Distributed D11 sub-cases

- Multi-agent fan-out per BTreeMap order.
- Dead agents drawn striped-grey at `effective_place(agent)` resolved at last-snapshot-time.
- Self-loops render as a small arc above the place; parallel edges offset perpendicularly by 6px per duplicate (defensive — current scenarios have neither).
- `PlaceTag` exhaustive match — compile-time enforced. No runtime fallback.

### 3. Wire canvas into `app.rs::update()`

Replace the placeholder canvas region from T01DEBVIS-004 with a call to `canvas::draw_canvas(...)`. Pass `&mut self.selected_agent` and `&mut self.hovered_agent` so the canvas can mutate selection state.

### 4. Wire module into lib.rs

Add `pub mod canvas;` to `crates/worldwake-visualizer/src/lib.rs`.

## Files to Touch

- `crates/worldwake-visualizer/src/canvas.rs` (new)
- `crates/worldwake-visualizer/src/app.rs` (modify — replace placeholder canvas with `canvas::draw_canvas` call)
- `crates/worldwake-visualizer/src/lib.rs` (modify — add `canvas` module declaration)

## Out of Scope

- Tooltip on hover (T01DEBVIS-006).
- Detail modal on click (T01DEBVIS-007 onward).
- Trace tab integration (T01DEBVIS-009).
- Optimization for >30 places (explicit non-goal in T01).

## Acceptance Criteria

### Tests That Must Pass

1. `canvas_smoke_no_panic_on_baseline_scenario` — load `survival-baseline.ron`, build one snapshot, invoke `draw_canvas` against a headless egui context; assert no panic.
2. `agent_fan_out_angles_are_btreemap_stable` — three agents at the same place produce angles sorted by EntityId and summing to (within `f32` tolerance) `2π`.
3. Manual QA: `cargo run -p worldwake-visualizer -- scenarios/survival-baseline.ron` shows places with tag pills, dashed edges with tick labels, agents at places, and at least one agent visibly lerped along an edge if scenario produces travel.
4. Existing suite: `cargo test -p worldwake-visualizer` passes.

### Invariants

1. Canvas drawing is a pure function of `FrameSnapshot` + selection/hover state; it does not query authoritative engine state directly.
2. `PlaceTag` rendering is exhaustive at compile time — no `_ =>` catch-all arm.
3. Agent positions for the same snapshot are identical across re-renders (BTreeMap-ordered determinism).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-visualizer/src/canvas.rs` (`#[cfg(test)] mod tests`) — `canvas_smoke_no_panic_on_baseline_scenario`, `agent_fan_out_angles_are_btreemap_stable`. Headless egui context for the smoke test.

### Commands

1. `cargo test -p worldwake-visualizer canvas::`
2. `cargo test -p worldwake-visualizer`
3. `cargo run -p worldwake-visualizer -- scenarios/survival-baseline.ron` (manual smoke)
4. `./scripts/verify.sh`
