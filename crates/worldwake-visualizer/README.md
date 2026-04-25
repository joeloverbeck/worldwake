# Worldwake Visualizer

`worldwake-visualizer` is a developer-only desktop observer for loading
Worldwake scenarios and inspecting live simulation state. It hosts the
simulation, supports step/play/reset controls, builds frame snapshots, computes
deterministic place layout, and renders the current snapshot on an interactive
egui canvas.

## How to Run

Open a scenario:

```bash
cargo run -p worldwake-visualizer -- scenarios/<name>.ron
```

Open the baseline scenario:

```bash
cargo run -p worldwake-visualizer -- scenarios/survival-baseline.ron
```

Print CLI usage:

```bash
cargo run -p worldwake-visualizer -- --help
```

Bypass scenario lint failures for ad-hoc debugging:

```bash
cargo run -p worldwake-visualizer -- --ignore-lints scenarios/<name>.ron
```

## Manual QA Checklist

For each landed scenario under `scenarios/`:

1. `cargo run -p worldwake-visualizer -- scenarios/<name>.ron` opens window
   within 2s.
2. `cargo run -p worldwake-visualizer -- --help` prints the clap-derived
   usage and exits.
3. Places render without overlap; graph fits in window via auto-fit on first
   frame.
4. Dashed edges render with tick-count labels at midpoints.
5. Pan (middle-drag) and zoom (wheel) work on the canvas - confirms
   `egui::Scene` is reachable on the pinned `egui` version. If `egui::Scene`
   is unavailable on the resolved version, fall back to a hand-rolled pan/zoom
   container before continuing.
6. Space advances exactly one tick (tick counter in header increments by 1).
7. Play + speed slider: tick counter advances at approximately the configured
   rate.
8. Reset returns tick to 0 and places agents at their initial locations.
9. Hover agent -> tooltip with zone-colored need bars, including Pain/Danger
   when non-zero; bars match numeric values.
10. Click agent -> modal opens; all 6 tabs render without panic.
11. Traces tab populates with entries after several ticks.
12. Beliefs tab shows entries from `AgentBeliefStore`, `LastSeenMemory`,
    `ExpectationStore`, and `SourceReliability` after the agent has observed
    something.
13. Transit: for `survival-scattered.ron`, an agent on a multi-tick edge is
    visibly lerped across ticks.

## Known Scenarios

At the T01DEBVIS-010 landing pass, the workspace `scenarios/` directory
contains:

- `cli-evaluation.ron`
- `final-integration.ron`
- `survival-ask-consult.ron`
- `survival-baseline.ron`
- `survival-combat.ron`
- `survival-contested.ron`
- `survival-drive-escalation.ron`
- `survival-escort.ron`
- `survival-items-decay.ron`
- `survival-justice.ron`
- `survival-offices.ron`
- `survival-patrol.ron`
- `survival-preferences.ron`
- `survival-production.ron`
- `survival-scattered.ron`
- `survival-tell.ron`
- `survival-theft.ron`
- `survival-trade.ron`

## Screenshots

TBD.
