# T01DEBVIS-001: Crate skeleton + workspace registration + entry point

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

T01 (debug visualizer) ships as a brand-new workspace crate `worldwake-visualizer` that consumes existing public APIs from `worldwake-core` / `-sim` / `-systems` / `-ai` / `-cli`. Before any rendering, snapshot, or app-loop logic can land, the crate must exist as a buildable workspace member with a working CLI entry point and a minimal `eframe` app shell. This ticket lands that foundation so subsequent tickets (002–009) can each ship reviewable diffs that compile in isolation.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Workspace `Cargo.toml` currently lists 5 members at `/Cargo.toml:3`: `worldwake-core`, `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, `worldwake-cli`. No existing `worldwake-visualizer` directory or member entry — this ticket adds both.
2. `worldwake_cli::scenario::load_scenario_file` and `spawn_scenario` / `spawn_scenario_ignoring_lints` exist as `pub` at `crates/worldwake-cli/src/scenario/mod.rs:111,125,134`. Spec T01 §D1 names these as the loader pair the visualizer reuses; reassessment 2026-04-25 confirmed both are still public.
3. Tooling-only ticket — no shared abstraction boundary or simulation data contract is under audit. The ticket adds a new binary crate that is a read-only client of existing public APIs.

## Architecture Check

1. The crate is a standalone binary with no engine code changes. It depends on the existing public surfaces of `core`/`sim`/`systems`/`ai`/`cli` and follows the observer-binary pattern already established by `crates/worldwake-cli/src/bin/observer.rs`. New crate = no shims, no aliasing.
2. `--ignore-lints` is wired by selecting between `spawn_scenario` and `spawn_scenario_ignoring_lints` exactly as observer.rs does at `crates/worldwake-cli/src/bin/observer.rs:3631-3635` — no parallel lint-suppression path.

## Verification Layers

1. Single-layer ticket — `cargo check -p worldwake-visualizer` and `cargo build -p worldwake-visualizer` are the proof surfaces. The visualizer writes nothing to engine state; no decision/action/event-log layer is involved. Per template item 6: additional layer mapping is not applicable to a foundational crate-skeleton ticket.

## What to Change

### 1. Create the new crate directory and Cargo.toml

Create `crates/worldwake-visualizer/Cargo.toml` with the dependency set per spec T01 §D1:

```toml
[package]
name = "worldwake-visualizer"
version = "0.1.0"
edition = "2021"

[dependencies]
worldwake-core = { path = "../worldwake-core" }
worldwake-sim = { path = "../worldwake-sim" }
worldwake-systems = { path = "../worldwake-systems" }
worldwake-ai = { path = "../worldwake-ai" }
worldwake-cli = { path = "../worldwake-cli" }
eframe = { version = "0.34", default-features = false, features = ["default_fonts", "glow", "wayland", "x11"] }
egui = "0.34"
rand_chacha = "0.3"
rfd = { version = "0.14", default-features = false, features = ["xdg-portal"] }
clap = { version = "4", features = ["derive"] }
```

### 2. Register the new crate as a workspace member

Modify `Cargo.toml` (workspace root) to add `crates/worldwake-visualizer` to the `members` array.

### 3. Implement minimal `main.rs` and `app.rs` shell

`crates/worldwake-visualizer/src/main.rs` — `VisualizerCli` (clap-derived: optional positional `scenario: PathBuf`, flag `--ignore-lints: bool`), `eframe::run_native` boilerplate per spec T01 §D2.

`crates/worldwake-visualizer/src/app.rs` — `VisualizerApp` shell with the persistent fields named by spec T01 §D3 (`sim: Option<SimulationState>`, `action_registries`, `dispatch_table`, `driver: AgentTickDriver`, `scenario_path: Option<PathBuf>`, trace sinks as default-constructed placeholders, `play_state`, `speed`, `tick_carry`, `selected_agent`, `hovered_agent`). `new(cli: VisualizerCli)` loads the scenario via `load_scenario_file` + the lint-conditional spawn pair. `update()` is a placeholder that draws an empty-state panel ("Load scenario…" if no scenario was provided) — full update logic lands in T01DEBVIS-004. `step_one_tick()` is unimplemented in this ticket (stub).

### 4. Placeholder README and lib.rs

`crates/worldwake-visualizer/README.md` — one-paragraph stub. Manual QA checklist lands in T01DEBVIS-010.

`crates/worldwake-visualizer/src/lib.rs` — minimal `pub mod app;` so the binary and any future integration tests share the type definitions.

## Files to Touch

- `Cargo.toml` (modify) — workspace `members` array
- `crates/worldwake-visualizer/Cargo.toml` (new)
- `crates/worldwake-visualizer/README.md` (new — stub only)
- `crates/worldwake-visualizer/src/main.rs` (new)
- `crates/worldwake-visualizer/src/lib.rs` (new)
- `crates/worldwake-visualizer/src/app.rs` (new — shell only)

## Out of Scope

- Force-directed layout (T01DEBVIS-002).
- Frame snapshot construction (T01DEBVIS-003).
- Per-tick step routine, full `update()` logic, and step controls (T01DEBVIS-004) — the `VisualizerApp` shell here has stub `step_one_tick` and a placeholder `update()`; both are replaced by T01DEBVIS-004.
- Canvas rendering (T01DEBVIS-005).
- Tooltip, modal, tabs, trace ring buffers (T01DEBVIS-006 through -009).
- Manual QA checklist content in README.md (T01DEBVIS-010).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo check -p worldwake-visualizer` succeeds.
2. `cargo build -p worldwake-visualizer` succeeds.
3. `cargo run -p worldwake-visualizer -- --help` exits 0 and prints the clap-derived usage including `--ignore-lints` and the optional `scenario` argument.
4. Existing suite: `cargo test --workspace` passes (no regressions in any pre-existing crate).

### Invariants

1. The new crate's `Cargo.toml` is the only lint-suppression authority for the visualizer; the `--ignore-lints` flag selects between `spawn_scenario` and `spawn_scenario_ignoring_lints` rather than introducing a parallel lint-bypass path.
2. The `VisualizerApp` shell never writes to authoritative engine state — engine APIs are read-only consumers from this crate's perspective.

## Test Plan

### New/Modified Tests

1. None — documentation/skeleton ticket; verification is command-based (build + `--help` smoke).

### Commands

1. `cargo check -p worldwake-visualizer`
2. `cargo build -p worldwake-visualizer`
3. `cargo run -p worldwake-visualizer -- --help`
4. `./scripts/verify.sh`
