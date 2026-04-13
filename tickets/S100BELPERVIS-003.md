# S100BELPERVIS-003: Update scenario RON perception profiles

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: archive/tickets/S100BELPERVIS-001.md

## Problem

After `archive/tickets/S100BELPERVIS-001.md` adds `infrastructure_retention_ticks` to `PerceptionProfile`, any scenario RON file with an explicit `perception_profile` block will fail to parse because `PerceptionProfile` does not use `#[serde(default)]` on individual fields. Currently only `scenarios/cli-evaluation.ron` has explicit perception profiles (Kael and Guard Theron). Without this update, the CLI evaluation scenario cannot load.

## Assumption Reassessment (2026-04-13)

1. `PerceptionProfile` at `belief.rs:2179` now derives `Serialize, Deserialize` with 9 explicit fields including `infrastructure_retention_ticks`, and still has no `#[serde(default)]` on individual fields. Omitting the new field in RON causes a parse error. Confirmed against the landed implementation.
2. `scenarios/cli-evaluation.ron` — Kael's perception profile at lines 91-108 and Guard Theron's at lines 330-347 still explicitly enumerate the pre-001 field set and therefore need the new `infrastructure_retention_ticks` entry added. Neither uses shorthand or omission-tolerant defaults. Confirmed via read.
3. `scenarios/default.ron` — no explicit perception profiles. Agents use `unwrap_or_default()` in `spawn_agent()` (line 364 of `scenario/mod.rs`), so they automatically get the Default value including the new field. Confirmed via read.

## Architecture Check

1. Adding the field to existing RON blocks is the minimal change. No new serde attributes, no structural changes to the scenario format.
2. No backward-compatibility shims. The RON files are updated to match the new struct shape. Old RON without the field will fail to parse (correct behavior per P28).

## Verification Layers

1. Scenario loads successfully → CLI smoke test (load cli-evaluation.ron)
2. Single-layer ticket (scenario configuration). No cross-system or mixed-layer concerns.

## What to Change

### 1. Update Kael's perception profile in cli-evaluation.ron

In `scenarios/cli-evaluation.ron`, add `infrastructure_retention_ticks: 640,` to Kael's `perception_profile` block (after `memory_retention_ticks: 64,` at line 94) to preserve the 10x ratio already used by the landed shared default.

### 2. Update Guard Theron's perception profile in cli-evaluation.ron

In `scenarios/cli-evaluation.ron`, add `infrastructure_retention_ticks: 640,` to Guard Theron's `perception_profile` block (after `memory_retention_ticks: 64,` at line 333) to match Kael's explicit profile ratio.

## Files to Touch

- `scenarios/cli-evaluation.ron` (modify — two perception_profile blocks)

## Out of Scope

- Modifying `scenarios/default.ron` (no explicit perception profiles — uses Default automatically)
- Adding new scenario files
- Changing retention logic (ticket 002)
- Modifying the `PerceptionProfile` struct (ticket 001)

## Acceptance Criteria

### Tests That Must Pass

1. `scenarios/cli-evaluation.ron` loads without parse errors
2. Kael and Guard Theron agents spawn with `infrastructure_retention_ticks` values
3. Existing suite: `cargo test --workspace`

### Invariants

1. All agents with explicit perception profiles in RON include the new field
2. Agents without explicit profiles get the Default value (480) via `unwrap_or_default()`

## Test Plan

### New/Modified Tests

1. None — scenario configuration ticket; verification is command-based. Existing golden tests that load cli-evaluation.ron will exercise the parsing path.

### Commands

1. `cargo test --workspace` — verifies scenario parsing via any test that loads cli-evaluation.ron
2. `cargo clippy --workspace --all-targets -- -D warnings` — lint clean
