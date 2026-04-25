# Worldwake Visualizer

`worldwake-visualizer` is a developer-only desktop observer for loading Worldwake scenarios and inspecting live simulation state. It hosts the simulation, supports step/play/reset controls, builds frame snapshots, computes deterministic place layout, and renders the current snapshot on an interactive egui canvas.

## Manual QA

Run:

```bash
cargo run -p worldwake-visualizer -- scenarios/survival-baseline.ron
```

Confirm:

- Places render as dark rounded rectangles with tag pills.
- Directed edges render as dashed lines with tick-count labels.
- Agents render at places with deterministic fan-out; agents in transit lerp along their current travel edge.
- Mouse wheel zooms the canvas and middle-drag pans it.
- Space advances one tick; Play advances ticks continuously; Reset returns the scenario to tick 0.
