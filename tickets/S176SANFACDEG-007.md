# S176SANFACDEG-007: Observer basin/latrine condition display

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None (observer/CLI only)
**Deps**: S176SANFACDEG-001 (`WashBasinState.max_effective_dirtiness` for display context)

## Problem

The observer shows only basin *presence*, not condition, and shows no latrine state at all (S176 D10, observer half). The controlled agent cannot see how dirty/full a co-located facility is, so the human player faces different information than the AI. This ticket surfaces basin/latrine condition for the controlled agent's co-located facilities under FND-14A.

## Assumption Reassessment (2026-05-29)

1. The observer renders basin presence at `crates/worldwake-cli/src/bin/observer.rs:2101` (`wash={}` via `yes_no(summary.wash_basin_present)`); detection is at `:2374-2376` (workstation-tag presence only). There is no condition display and no latrine-fullness display.
2. Belief-view accessors `facility_wash_basin_state` (`belief_view.rs:495`), `wash_basin_state` (`:561`), and `latrine_fullness` (`:557`) expose `dirtiness_level` / `clean_water_units` / `fill`; the observer must read condition only for the controlled agent's co-located facilities (FND-14A), never remote authoritative state.
3. Shared boundary under audit: the player-POV place summary in the observer — a read-only view; this ticket adds no engine state and no simulation mutation. Items 4-15 are inapplicable (observer-only).

## Architecture Check

1. Reuses the existing belief-view accessors and the existing place-summary rendering path; no new accessor, no engine change (FND-19 agent symmetry — the human sees only what the controlled agent lawfully perceives).
2. FND-14A: condition is shown only for co-located facilities; remote condition is not surfaced as authoritative.

## Verification Layers

1. Display content for co-located condition → headless observer render test (basin dirtiness/clean-units + latrine fill appear for the controlled agent's place).
2. FND-14A locality → render test confirming remote facility condition is not displayed as authoritative.

## What to Change

### 1. Place-summary condition fields

Extend the observer place summary to surface basin `dirtiness_level` / `clean_water_units` and latrine `fill` for the controlled agent's co-located facilities, read via the belief-view accessors.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- The `WashBasinState` field and scenario contract — S176SANFACDEG-001.
- Any engine, planner, or simulation-state change.

## Acceptance Criteria

### Tests That Must Pass

1. The observer place summary shows basin dirtiness/clean-units and latrine fill for the controlled agent's co-located facilities.
2. Remote facility condition is not surfaced as authoritative.
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Observer reads only belief-view/co-located condition (FND-14A); no remote authoritative read.
2. No engine or simulation-state change (`Engine Changes: None` holds).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli` observer render test — new: co-located basin/latrine condition appears; remote condition does not.

### Commands

1. `cargo test -p worldwake-cli observer`
2. `cargo test -p worldwake-cli`
3. `scripts/verify.sh`
