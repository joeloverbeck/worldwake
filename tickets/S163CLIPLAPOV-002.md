# S163CLIPLAPOV-002: Mark debug/observer console surfaces + play-surface guard

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — `worldwake-cli` doc comments + a boundary test
**Deps**: S163CLIPLAPOV-001 (the play-surface guard passes only after D1 removes the omniscient label call from `actions.rs`)

## Problem

The CLI's omniscient read surfaces carry no marker preventing a future normal-play
UI from importing them. `display.rs` (`entity_display_name`/`resolve_entity`/
`format_location`) and the console handlers `control.rs` (`switch`/`observe`),
`world_overview.rs` (`world`/`places`/`agents`/`goods`), `inspect.rs`
(`inspect`/`relations`), `events.rs` (`events`/`event`/`trace`), and `tick.rs`'s
action-trace output all read authoritative `World` truth ungated by belief or
control mode. They are legitimate observer/debug console surfaces, but unmarked.
This ticket marks them debug/observer-only and adds an enforceable guard so the
play surface (`actions.rs`) cannot silently regain an omniscient display
dependency. This is S163 Deliverable 3.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Current module docs (verified 2026-05-22): `display.rs:1-3` says "Display and
   formatting helpers for the CLI. / All functions are pure read-only — no world
   mutation."; `world_overview.rs:1-3` and `inspect.rs:1-3` and `events.rs:1-3` each
   say "...read-only — zero world mutation."; `control.rs:1` says "Handlers for
   agent control commands: `switch` and `observe`."; `tick.rs:1` says "Tick and
   status command handlers." None state "observer/debug-only; normal player UI must
   not depend on this." The "read-only" claim is true but orthogonal to the FND-19
   concern (read-only ≠ POV-safe).
2. Spec contract: S163 Deliverable 3
   (`specs/S163-cli-player-pov-boundary.md:133-146`) and Non-Goals (`:96-101`).
   `dispatch_command` (`crates/worldwake-cli/src/handlers/mod.rs:32-71`) routes all
   these commands ungated.
3. Shared boundary under audit: the play-surface vs. debug-console split. The play
   surface is exactly the action-menu path `handle_actions`/`handle_do` in
   `actions.rs`; everything else routed by `dispatch_command` is debug/observer.
4. `tick.rs` nuance (precision rule 2 — layer precision): `tick.rs` contains both
   `handle_tick` (advances ticks and renders action traces naming all agents at
   `tick.rs:80-89` — omniscient) **and** `handle_status` (self-scoped status about
   the controlled agent — lawful play). A module-level debug marker would wrongly
   tar `handle_status`. Therefore the marker on `tick.rs` is **function-level** on
   the trace-rendering path, not the module. The other five surfaces (`display.rs`,
   `control.rs`, `world_overview.rs`, `inspect.rs`, `events.rs`) are wholly debug
   and take module-level markers.
5. Play-surface guard mechanism (ticket-time choice per spec): after
   S163CLIPLAPOV-001, `actions.rs` no longer imports or calls
   `entity_display_name`/`resolve_entity`/`format_location`. The enforceable guard
   asserts this stays true — either (a) a unit test that reads the `actions.rs`
   source and asserts none of the three forbidden symbols appear in the
   `handle_actions`/`handle_do` bodies, or (b) a structural guarantee via no `use
   crate::display::{...}` of those helpers in `actions.rs`. Prefer (a) as the
   "enforceable guard" the spec asks for. The guard must fail if a future change
   reintroduces an omniscient display dependency in the play surface.
6. Adjacent contradiction classification: doc-marking is a required consequence of
   this deliverable; POV-gating the console commands is explicit future work
   (S163 Non-Goals), not this ticket.

## Architecture Check

1. Doc comments plus a single boundary guard are the lightest correct fix: they
   make the debug/observer classification explicit (FND-29: debuggability is a
   product feature; FND-27: derived views are caches, not truth) without rewriting
   any working debug surface. The function-level marker on `tick.rs` preserves the
   lawful `handle_status` play path rather than over-marking the module.
2. No backward-compatibility shim and no behavioral change: only doc comments and a
   test are added. The guard enforces the play-surface boundary established by
   S163CLIPLAPOV-001 rather than introducing a parallel path.

## Verification Layers

1. Debug/observer classification is explicit → presence of the module-level
   (`display.rs`/`control.rs`/`world_overview.rs`/`inspect.rs`/`events.rs`) and
   function-level (`tick.rs` trace render) doc markers; reviewed in diff, not a
   runtime surface.
2. Play surface stays POV-safe → focused boundary unit test asserting
   `handle_actions`/`handle_do` reference none of
   `entity_display_name`/`resolve_entity`/`format_location` for player-visible
   output. The test fails if the dependency returns.
3. Single-layer ticket: doc comments + one boundary test in `worldwake-cli`; no
   decision trace / action trace / event-log delta applies (no engine change).

## What to Change

### 1. Module-level debug/observer markers

Add a module-level doc comment to each of `display.rs`, `handlers/control.rs`,
`handlers/world_overview.rs`, `handlers/inspect.rs`, and `handlers/events.rs`
stating: these surfaces read authoritative world truth and are for
observer/debug/replay tooling only; normal player-facing UI must not depend on
them. Keep the existing "read-only" line where present.

### 2. Function-level marker on the tick trace render

Add a doc comment to the trace-rendering path in `handlers/tick.rs` (the
`handle_tick` action-trace output around `:80-89`, not `handle_status`) noting it
renders omniscient action traces for observer/debug use, and that a normal
player-facing UI must route per-agent feedback through a POV-safe surface instead.

### 3. Play-surface boundary guard

Add an enforceable guard (preferred: a `#[cfg(test)]` unit test in `actions.rs` or a
`crates/worldwake-cli/tests/` file) asserting the play surface
(`handle_actions`/`handle_do`) does not call
`entity_display_name`/`resolve_entity`/`format_location` for player-visible output.
The guard must fail if a future change reintroduces an omniscient display dependency
in the play path.

## Files to Touch

- `crates/worldwake-cli/src/display.rs` (modify — module doc)
- `crates/worldwake-cli/src/handlers/control.rs` (modify — module doc)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify — module doc)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify — module doc)
- `crates/worldwake-cli/src/handlers/events.rs` (modify — module doc)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify — function-level doc on the trace render)
- `crates/worldwake-cli/src/handlers/actions.rs` (modify — add the boundary guard test) OR `crates/worldwake-cli/tests/` (new test file), implementer's choice

## Out of Scope

- POV-safe label resolution itself (D1) — S163CLIPLAPOV-001.
- The `handle_cancel` regression guard (D2) and FND-19 symmetry test (D4) —
  S163CLIPLAPOV-003.
- POV-gating any console command (`world`/`inspect`/`events`/`switch`/`observe`/
  `tick` trace) — explicit S163 Non-Goal; this ticket only marks them.
- A `DebugWorldView`/`ObserverUi` capability layer — future spec (S163 Non-Goals).

## Acceptance Criteria

### Tests That Must Pass

1. Boundary guard fails if `handle_actions`/`handle_do` reference
   `entity_display_name`/`resolve_entity`/`format_location` for player-visible
   output; passes against the post-S163CLIPLAPOV-001 code.
2. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. The play surface carries no omniscient display dependency; every wholly-debug
   console surface and the `tick` trace render are explicitly marked
   observer/debug-only so a future player UI cannot inherit them silently. (FND-19,
   FND-27, FND-29.)
2. `handle_status` (self-scoped play) is **not** marked debug-only — only the
   omniscient trace-rendering path in `tick.rs` is.

## Test Plan

### New/Modified Tests

1. Play-surface boundary guard — new `#[cfg(test)]` test in
   `crates/worldwake-cli/src/handlers/actions.rs` (or a `tests/` file) asserting the
   action-menu path does not depend on the three omniscient display helpers.

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
3. `scripts/verify.sh` (before PR push)
