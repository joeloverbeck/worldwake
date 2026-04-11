# S85OBSBEHENR-001: Death tick and cause display in observer

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S85 (Observer Behavioral Enrichment)

## Problem

When an agent dies during a simulation, the observer reports `dead_ticks` in the tick breakdown but does not surface the exact tick of death or the cause. Diagnosing mortality requires cross-referencing event logs manually.

## Assumption Reassessment (2026-04-10)

1. `DeadAt { tick: Tick, cause: DeathCause }` exists at `crates/worldwake-core/src/combat.rs:66-68`. `DeathCause` enum has variants `NeedDeprivation { need: HomeostaticNeedId }` and `CombatWounds` at `combat.rs:59-62`. `get_component_dead_at(entity)` accessor confirmed via `component_schema.rs:117`. Observer already counts `dead_ticks` at `observer.rs:1057-1064` but never queries the `DeadAt` component directly.
2. S85 spec (Deliverable 1) describes this change. S81 (completed) introduced `DeadAt` with cause field.
3. Single-layer ticket: observer-only read of existing component. No shared abstraction boundary.

## Architecture Check

1. Reads an existing component through the standard accessor — no new queries, no new coupling. Placing the death line at the top of the per-agent summary section gives immediate context before action/needs analysis.
2. No backwards-compatibility aliasing or shims introduced.

## Verification Layers

1. Death tick and cause displayed correctly → focused unit test with mock `DeadAt` component
2. No display when agent is alive → focused unit test without `DeadAt` component
3. Single-layer observer-only ticket; no action/planning/event-log layer mapping applicable.

## What to Change

### 1. Import `DeadAt` and `DeathCause` in observer.rs

Add `use worldwake_core::combat::{DeadAt, DeathCause};` to the observer imports (near the top of the file).

### 2. Emit death line in per-agent summary

In Section 2 — Per-Agent Summary (around `observer.rs:541-549` after implementation), after `writeln!(out, "### {}\n", stats.name)` and before the action breakdown, query `world.get_component_dead_at(agent_id)`. If `Some(dead_at)`, emit:

```
**Death**: Tick {tick} (cause: {formatted_cause})
```

Format `DeathCause` variants as:
- `DeathCause::NeedDeprivation { need }` → `"NeedDeprivation { {need:?} }"`
- `DeathCause::CombatWounds` → `"CombatWounds"`

### 3. Add unit test

Add a test that constructs a world with an agent that has a `DeadAt` component, runs the relevant rendering logic, and asserts the output contains the expected death line. Add a negative test for an alive agent.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Modifying simulation behavior or AI decision-making
- Adding new components or systems to the engine
- Changing how `dead_ticks` is counted in the tick breakdown
- Interactive observer features or live dashboards

## Acceptance Criteria

### Tests That Must Pass

1. New test: observer output contains `**Death**: Tick N (cause: NeedDeprivation { Hunger })` when agent has `DeadAt` component
2. New test: observer output does not contain `**Death**:` when agent has no `DeadAt` component
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Observer remains read-only — no mutation of world state
2. Existing observer output sections are unchanged for alive agents

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (inline test) — verifies death display formatting for both dead and alive agents

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-10.

- Added observer-local `format_death_cause()` and `death_summary_line()` helpers in `crates/worldwake-cli/src/bin/observer.rs`.
- Section 2 — Per-Agent Summary now emits `**Death**: Tick N (cause: ...)` before the action table when an agent has `DeadAt`.
- Added focused observer tests covering `NeedDeprivation`, `CombatWounds`, and the alive-agent negative case.

## Verification Result

- Passed `cargo test -p worldwake-cli death_summary_line_includes_tick_and_cause_for_dead_agent`
- Passed `cargo test -p worldwake-cli death_summary_line_is_absent_for_alive_agent`
- Passed `cargo test -p worldwake-cli format_death_cause_renders_spec_strings`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
