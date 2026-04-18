# S117CONMAIOBS-003: `MaintenanceStarvation` detector

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: `archive/tickets/S117CONMAIOBS-001.md`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

Per the Layer 3 report, the survival-contested scenario shows agents accumulating dirtiness faster than they relieve it — for 200+ consecutive ticks relief rate trails accumulation rate — but the existing `SUSTAINED_CRITICAL_NEED` detector only flags the symptom after 100 ticks above the critical threshold. The upstream frequency mismatch (relief cadence losing to metabolism + action penalties) is invisible today. This ticket adds a detector that directly measures accumulation vs. relief per-agent per-need over a rolling 200-tick window, gated on `avg_need > medium_threshold` so a chronically-elevated need surfaces even before it crosses critical.

## Assumption Reassessment (2026-04-18)

1. Per-tick `HomeostaticNeeds` samples are already collected in `AgentStats.needs_samples: Vec<NeedsSample>` (captured in the observer's per-tick sampling loop at `bin/observer.rs:2544-2551`; rendered in Section 2 "Needs trajectory" at `bin/observer.rs:1602-1658`). Per-tick positive deltas sum to accumulation; per-tick negative deltas sum to relief. No new sampling is required.
2. `DriveThresholds` component is read per-agent via `World::get_component_drive_thresholds(agent)` — the observer already uses this exact API at `bin/observer.rs:2585-2588` for the forensics extractor. `DriveThresholds` has per-need fields `hunger`, `thirst`, `fatigue`, `bladder`, `dirtiness` (each a `ThresholdBand`) and exposes `ThresholdBand::medium() -> Permille` at `crates/worldwake-core/src/drives.rs:41`. `DriveThresholds::critical(need)` exists at line 92 but no `medium(need)` helper exists — this ticket performs the per-need match locally per spec Non-Goals §6.
3. Shared abstraction boundary under audit: the `detect_anomalies()` orchestrator at `bin/observer.rs:752` and the `AgentStats.needs_samples` data structure. This ticket is a pure read-side extension.
4. The intended invariant surfaced by this detector: "if mean need is chronically elevated AND net per-tick delta is positive over a 200-tick window, the agent's maintenance cadence is failing regardless of whether the need ever crosses critical." The detector measures the cadence gap directly, not a symptom proxy.
5. Cumulative arithmetic: the detector reads `needs_samples[i].hunger - needs_samples[i-1].hunger` per tick (same pattern for other needs), summing positive deltas into `accumulation_permille` and negative deltas into `relief_permille` across the window. Values are `u16` permille in `NeedsSample`; accumulators are `u32` to avoid overflow over a 200-tick window.

## Architecture Check

1. Using the agent's own per-need `medium` threshold via `DriveThresholds` respects FND-14 (per-agent parameterization) and FND-22 (agent diversity through concrete variation) — a global medium constant would flatten this. Alternative — computing a global medium from aggregate statistics — would obscure per-agent starvation for agents with non-default drive profiles.
2. The detector is a pure read-side aggregation with no stored state. Deleting the detector code changes nothing about the simulation. No backward-compatibility shim.
3. `DriveThresholds::medium(need)` helper is intentionally NOT added to `worldwake-core` (per spec Non-Goals §6): keeping the match local to the single detector call site avoids a speculative cross-crate API whose only consumer is this detector.

## Verification Layers

1. Detector fires on synthetic trajectory where accumulation > relief over a 200-tick window with avg > medium → focused unit test on the detector function.
2. Detector does NOT fire when accumulation > relief but avg ≤ medium (transient spike with immediate relief) → focused unit test.
3. Detector does NOT fire when accumulation ≤ relief even if avg > medium (chronic-but-balanced case: agent lives at 600 permille dirtiness but keeps it steady) → focused unit test.
4. Dedup: multiple overlapping qualifying windows for the same (agent, need) merge into one anomaly with combined span → focused unit test.
5. Single-layer ticket (observer read-side); no action-trace or event-log proof surface applies.

## What to Change

### 1. New detector function

Add `fn detect_maintenance_starvation(stats_by_agent: &BTreeMap<EntityId, AgentStats>, thresholds_by_agent: &BTreeMap<EntityId, DriveThresholds>, names: &BTreeMap<EntityId, String>, anomalies: &mut Vec<Anomaly>)` below the `detect_geographic_convergence` detector added in 002.

Logic per agent per `HomeostaticNeedId`:

- Let `WINDOW_TICKS: usize = 200` (file-local constant, shared with 002 — extract into a module-level constant).
- Accessor for per-need current value: `fn need_value(sample: &NeedsSample, need: HomeostaticNeedId) -> u16` returning the appropriate field (`sample.hunger`, `sample.thirst`, etc.).
- Look up `medium` threshold via the per-need match:

```rust
let medium_permille = match need {
    HomeostaticNeedId::Hunger    => thresholds.hunger.medium(),
    HomeostaticNeedId::Thirst    => thresholds.thirst.medium(),
    HomeostaticNeedId::Fatigue   => thresholds.fatigue.medium(),
    HomeostaticNeedId::Bladder   => thresholds.bladder.medium(),
    HomeostaticNeedId::Dirtiness => thresholds.dirtiness.medium(),
}.as_u16(); // or the equivalent extractor on Permille
```

- Slide a 200-tick window across `needs_samples`. For each window, compute `accumulation`, `relief`, `avg`. Qualify if `relief < accumulation && avg > medium_permille`.
- Merge adjacent qualifying windows per `(agent, need)` into a single span (earliest start, latest end).
- Emit one `Anomaly` per merged span:
  - `kind: AnomalyKind::MaintenanceStarvation`
  - `agent_name: names[&agent]`
  - `additional_agent_names: None`
  - `description: format!("{} accumulated {} permille but was relieved only {} permille over ticks {}–{}. Average {} in window: {} permille (above medium threshold {}).", need_label, accumulation, relief, start, end, need_label, avg, medium_permille)` where `need_label` is the lowercase need name (`"dirtiness"`, `"hunger"`, etc.)
  - `tick_range: Some((start, end))`

### 2. Wire into `detect_anomalies()`

Call the new detector from the orchestrator. Order: after `detect_geographic_convergence` to keep Section 3 ordering stable.

### 3. Thresholds collection

At the `detect_anomalies()` call site, collect per-agent `DriveThresholds` into the `BTreeMap<EntityId, DriveThresholds>` passed into the detector. Use `world.get_component_drive_thresholds(*agent).copied().unwrap_or_default()` — matches the existing pattern at `bin/observer.rs:2585-2588`.

### 4. Focused unit tests

Add to the existing `#[cfg(test)] mod tests`:

- `test_maintenance_starvation_fires_on_rising_dirtiness_over_window` — synthetic dirtiness trajectory rising from 550 → 850 over 200 ticks, average ≈ 700 (> medium 550 for dirtiness default), per-tick deltas all positive. Assert one anomaly with correct accumulation/relief/avg values in description.
- `test_maintenance_starvation_does_not_fire_when_balanced` — dirtiness oscillating 600 ± 50 over 200 ticks (accumulation ≈ relief). Assert zero anomalies.
- `test_maintenance_starvation_does_not_fire_when_avg_below_medium` — dirtiness oscillating 400 ± 100 over 200 ticks (avg < medium 550). Assert zero anomalies.
- `test_maintenance_starvation_merges_adjacent_windows` — 400-tick rising trajectory. Assert exactly one anomaly with span covering the full qualifying range, not `(400 - 200 + 1) = 201`.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Adding a `DriveThresholds::medium(need)` helper (spec Non-Goals §6; deferred).
- Fatigue/Bladder/Hunger/Thirst scenarios — the default thresholds apply uniformly across all five needs; unit tests use dirtiness as a representative need, but the detector handles all five.
- Cross-scenario regression guards (landed in 007).

## Acceptance Criteria

### Tests That Must Pass

1. `test_maintenance_starvation_fires_on_rising_dirtiness_over_window` passes.
2. `test_maintenance_starvation_does_not_fire_when_balanced` passes.
3. `test_maintenance_starvation_does_not_fire_when_avg_below_medium` passes.
4. `test_maintenance_starvation_merges_adjacent_windows` passes.
5. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. At most one `MaintenanceStarvation` anomaly emits per (agent, need) pair in a single run — overlapping qualifying windows merge rather than multiplying.
2. The `medium` threshold in the description text equals the agent's actual `DriveThresholds.<need>.medium().as_u16()` — not a hardcoded global constant.
3. Accumulation and relief in the description sum to the absolute value of the net delta across the qualifying span (modulo saturating arithmetic at need-value boundaries).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` — four new focused unit tests above.

### Commands

1. `cargo test -p worldwake-cli --bin observer maintenance_starvation`
2. `cargo test -p worldwake-cli`
3. `cargo clippy --workspace --all-targets -- -D warnings`
