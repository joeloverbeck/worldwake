# S117CONMAIOBS-002: `GeographicConvergence` detector

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: `archive/tickets/S117CONMAIOBS-001.md`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

Four-agent survival-contested runs have shown 78–84% occupancy on a single place (East Orchard) with no mechanical detector surfacing the pattern. Currently the analyst must eyeball Section 2 "Locations visited" rows to spot convergence, and only the `/scenario-analysis` skill's LLM-driven Layer 3 catches it. This ticket adds a mechanical detector that emits `GEOGRAPHIC_CONVERGENCE` when 2+ agents anchor on the same place for ≥60% of a 200-tick rolling window.

## Assumption Reassessment (2026-04-18)

1. Per-tick location data is already collected by the observer via `AgentStats.location_ticks` (rendered in Section 2 as "Locations visited"), and per-tick `(agent, tick, place)` records are reconstructible from the action trace and the existing per-tick sampling loop at `bin/observer.rs:2544-2551`. No new world-state reads are required.
2. The `Anomaly` struct's `additional_agent_names: Option<Vec<String>>` field lands in 001; this ticket depends on that field being present. No other detector in this spec emits multi-agent anomalies, so this ticket is the sole consumer of the multi-agent header branch.
3. Shared abstraction boundary under audit: the `detect_anomalies()` function's per-agent scan at `bin/observer.rs:752` — this ticket adds a new detector function invoked from the same orchestrator.
4. Determinism: agent-set dedup keys must use `BTreeSet<EntityId>` (not `HashSet`) per CLAUDE.md's Determinism invariant. Iteration order of the dedup map drives anomaly emission order, which drives Section 3 numbering.

## Architecture Check

1. The detector is a pure read-side scan over already-collected per-tick location data. No new ECS components, no authoritative mutations, no cross-system calls. Alternative — storing a running "convergence score" component on each agent — would be a P3/P27 violation (a score masquerading as concrete state); scanning authoritative per-tick records on demand is the correct architecture.
2. Threshold (60% occupancy over 200 ticks) is a compile-time constant local to the detector function with a justification comment, revisable without engine changes. No backward-compatibility shim — this is a greenfield detector.

## Verification Layers

1. Detector fires on a forced-hub fixture (3 agents pinned to one place for 200+ consecutive ticks) → golden E2E covered by 007; focused assertion in this ticket's unit test is on the detector function's behavior given a hand-constructed `Vec<AgentStats>` input.
2. Detector does NOT fire on `survival-baseline.ron` where agents rotate normally → regression guard asserted as a command in 007; this ticket's scope is the detector function, not the scenario-level regression.
3. Dedup: the same (agent-set, place) combination emits one anomaly across overlapping windows → focused unit test on the detector function with a synthetic trajectory that qualifies for three overlapping 200-tick windows.
4. Single-layer ticket (observer read-side); no action trace or event-log proof surface applies.

## What to Change

### 1. Per-tick location reconstruction

Within the `detect_anomalies()` orchestrator (or a new helper it calls), build a `BTreeMap<Tick, BTreeMap<EntityId, EntityId>>` (tick → agent → place) from the observer's existing per-tick sampling. If the current sampling already stores this, reuse it; otherwise extend the sampling loop at `bin/observer.rs:2544-2551` to emit per-tick location records keyed in a deterministic container.

### 2. New detector function

Add `fn detect_geographic_convergence(stats_by_agent: &BTreeMap<EntityId, AgentStats>, per_tick_location: &BTreeMap<Tick, BTreeMap<EntityId, EntityId>>, names: &BTreeMap<EntityId, String>, anomalies: &mut Vec<Anomaly>)` below the existing `detect_sustained_critical_needs` / `detect_unaddressed_needs` detectors.

Logic:

- Define `WINDOW_TICKS: u64 = 200` and `SHARE_THRESHOLD_PERMILLE: u32 = 600` (60%) as file-local constants with a justification comment.
- Iterate 200-tick rolling windows with a step of 1 tick; for each window, for each place, count per-agent ticks spent there.
- For each window, gather agents whose per-place share ≥ `SHARE_THRESHOLD_PERMILLE` into a `BTreeSet<EntityId>`. If the set has ≥ 2 agents, record a qualifying `(BTreeSet<EntityId>, place_id, window_start, window_end)` candidate.
- Merge adjacent/overlapping qualifying windows with the same `(agent-set, place_id)` into a single span (earliest start, latest end).
- For each merged span, push one `Anomaly`:
  - `kind: AnomalyKind::GeographicConvergence`
  - `agent_name: names[&lead_agent]` where `lead_agent` = smallest `EntityId` in the set
  - `additional_agent_names: Some(vec![names[&agent] for agent in sorted_remaining])`
  - `description: format!("{} agents spent {:.1}% of ticks {}–{} at {}.", set.len(), max_share_as_percent, window_start, window_end, place_label)`
  - `tick_range: Some((window_start, window_end))`

### 3. Wire into `detect_anomalies()`

Call the new detector from the orchestrator loop at `bin/observer.rs:752`. Order: after `detect_sustained_critical_needs` and before any subsequent detectors this spec adds (so Section 3 ordering is stable).

### 4. Focused unit test

Add to the existing `#[cfg(test)] mod tests`:

- `test_geographic_convergence_fires_when_three_agents_share_place_for_window` — synthetic per-tick trajectory with 3 agents at place P for 250 consecutive ticks; assert one `Anomaly` of kind `GeographicConvergence`, with lead agent name correct and `additional_agent_names.unwrap().len() == 2`.
- `test_geographic_convergence_deduplicates_overlapping_windows` — 250-tick shared-place trajectory should produce exactly one anomaly, not `(250 - 200 + 1) = 51`.
- `test_geographic_convergence_does_not_fire_on_rotating_agents` — agents alternate between two places every 50 ticks; assert zero anomalies.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Multi-agent render-path work (landed in 001).
- Other S117 detectors (003–005).
- Goldens against real scenario fixtures (007).
- Auto-dismiss of legitimate trade-hub convergence — the spec's Risk §1 defers this to scenario-level annotation in the `/scenario-analysis` skill.

## Acceptance Criteria

### Tests That Must Pass

1. `test_geographic_convergence_fires_when_three_agents_share_place_for_window` passes.
2. `test_geographic_convergence_deduplicates_overlapping_windows` passes.
3. `test_geographic_convergence_does_not_fire_on_rotating_agents` passes.
4. Existing suite: `cargo test -p worldwake-cli`.
5. Existing observer integration: `test_observer_mode_simulation_runs` still passes.

### Invariants

1. Per-run anomaly count for `GeographicConvergence` equals the number of distinct (agent-set, place) pairs with qualifying windows — never the raw qualifying-window count.
2. Agent-set dedup uses `BTreeSet<EntityId>` to preserve determinism (CLAUDE.md Determinism invariant).
3. `additional_agent_names` contents are sorted by `EntityId` ordering, not by String ordering, to stay deterministic across locale / name formatting changes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` — three new focused unit tests above.

### Commands

1. `cargo test -p worldwake-cli --bin observer geographic_convergence`
2. `cargo test -p worldwake-cli`
3. `cargo clippy --workspace --all-targets -- -D warnings`
